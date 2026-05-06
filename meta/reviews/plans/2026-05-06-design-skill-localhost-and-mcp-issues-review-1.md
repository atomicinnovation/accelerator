---
date: "2026-05-06T20:33:40Z"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-06-design-skill-localhost-and-mcp-issues.md"
review_number: 1
verdict: REVISE
lenses: [architecture, security, test-coverage, code-quality, correctness, usability, portability, safety, compatibility]
review_pass: 4
status: complete
---

## Plan Review: Design Skill — localhost validation and MCP-hallucination fixes

**Verdict:** REVISE

The plan is well-structured (clear phases, TDD discipline, explicit out-of-scope list, honest acknowledgement of design stretches) and tackles two genuinely independent UAT-blocker issues with solid background research. However, the executor design rests on three structural decisions — PPID-derived session id, regex-only `evaluate` deny-list, and a single overloaded `--allow-internal` flag — that nine independent lenses converged on as critical. Combined with deferred crawl bounds, a big-bang switchover with no rollback, and several host-canonicalisation gaps in the validator, the plan needs revision before implementation rather than during it.

### Cross-Cutting Themes

These issues were flagged by 3+ lenses and account for the majority of the critical/major findings — fixing them likely retires several findings at once.

- **PPID-derived daemon session id is unreliable** (flagged by: architecture, correctness, code-quality) — `${PPID}` is the parent of the *current shell*, which differs between Bash tool calls in Claude Code's invocation model. Daemon reuse will silently fail in the common case (cold-start every call, orphan daemons leak) and may collide across unrelated agents in the rare case (cross-talk through a shared daemon). Architecture's suggestion (explicit `ACCELERATOR_SESSION_ID` exported by the skill at Step 0 and propagated to agents) eliminates the problem cleanly.

- **Regex `evaluate` deny-list is structurally weak** (flagged by: security, architecture, code-quality, correctness, safety) — `\bfetch\b`, `\beval\b`, etc. are trivially bypassable (`globalThis['fe'+'tch']`, `fetch`, computed property access, with-statements, template-literal building) and the "plain `=` outside comparison contexts" heuristic is acknowledged as best-effort. The plan correctly positions this as defence-in-depth, but eval id 17 will only test the trivially-detected forms, creating false confidence. Two concrete options: (a) parse via `acorn` and walk the AST for forbidden identifiers, or (b) replace freeform `evaluate` with named structured ops (`getComputedStyle`, `getBoundingClientRect`, dataset reads) — no JS string crosses the boundary, and the deny-list disappears.

- **`--allow-internal` flag is overloaded** (flagged by: security, usability, architecture, correctness) — the same flag (a) opens RFC1918 / loopback / link-local *and* (b) enables AWS/cloud-metadata at 169.254.169.254 *and* (c) permits `http://` to public hosts. On any cloud-hosted dev box or CI runner, a user who flips the flag for legitimate reason 1 silently inherits reasons 2 and 3. The error message for `http://example.com` ("internal address") will mislead users. The plan's own "Naming note" already flags this; security and usability lenses both treat it as a critical/major issue. Splitting in v1 is mechanical (two booleans) and avoids a deprecation cycle later.

- **Host canonicalisation gaps in `validate-source.sh`** (flagged by: correctness, security, test-coverage) — `is_localhost_default()` matches only the literal strings `localhost`/`127.0.0.1`. Misses uppercase (`LOCALHOST`), trailing dot (`localhost.`), `0.0.0.0`, `[::]`, `[::ffff:127.0.0.1]` (IPv4-mapped IPv6 loopback), decimal-encoded `2130706433`, hex-encoded `0x7f000001`, and userinfo-containing URLs (`http://user:pass@127.0.0.1@evil.com/`). DNS rebinding bypasses the validator entirely. Multiple lenses suggest a real URL parser (`new URL(...)` in Node) or expanded explicit cases.

- **Daemon orphan / stale-socket recovery is under-specified** (flagged by: architecture, correctness, safety, test-coverage) — no behaviour for SIGKILL'd daemons, machine reboot, agent crashing before `daemon-stop`. The plan relies on the 5-minute idle timer for all cleanup, which is also the wall-clock crawl bound (race during long crawls). Need: connect-then-fallback for stale sockets, idle-timer-resets-on-operation invariant, explicit owner pidfile inspection.

- **Eval id 3 / 13 in-place rewrite** (flagged by: test-coverage, compatibility, correctness) — id 3's negative assertion ("no `mcp__playwright__*` invoked") becomes harder to verify when the new path uses `Bash(...run.sh)`; id 13 splits but id 15 doesn't preserve the loopback-rejection range coverage (`127.0.0.2` not exercised). Allocating fresh ids 18+ rather than mutating preserves benchmark history continuity.

- **Crawl bounds deferred while MCP path is removed** (flagged by: safety, security) — page cap, wall-clock, screenshot byte budget remain instructional only. With a 5-minute idle daemon and no enforcement teeth, a misbehaving page can spin Chromium for the full window per request. This is a regression vs the prior MCP path that the plan explicitly accepts as out-of-scope.

- **Big-bang switchover with no rollback** (flagged by: safety, compatibility) — Phase 4 deletes `.mcp.json`, swaps both agent frontmatters, rewrites SKILL.md detection, and updates evals/structural tests as one indivisible blob. No feature flag, no parallel-run period. Suggested split: Phase 4a (executor + dual-tool agents) → 4b (remove MCP) so each `main` SHA stays green and bisect/partial-revert remains possible.

- **`--offline-mock` defeats the only automated bootstrap test** (flagged by: test-coverage, correctness) — the only CI-exercised path is the no-op mock; real-failure paths (npm offline, partial download, corrupted node_modules) are skipped by default. The plan's claim that the no-mock path runs "manually before merge" is a thin gate for a user-facing first-run experience.

### Tradeoff Analysis

- **Security (deny-list AST parse) vs Code Quality (single-file simplicity)** — security and code-quality lenses both want AST-based deny-list (or named structured ops); code-quality additionally wants a multi-file run.js layout. Doing both is the recommendation but it widens the work. If only one is feasible, prefer the AST/structured-ops path — it eliminates the deny-list problem entirely.

- **Compatibility (Node 18) vs Modern features (Node 20)** — Compatibility flags Node 20 floor as a silent break for Node 18 LTS users. Playwright 1.49 actually supports Node 18, so dropping the floor to 18 is technically possible. Recommendation: Toby to decide whether the project's `mise.toml = node 22` posture means "users are expected to have Node 22 too" (keep 20 floor with documentation), or "the runtime should accept what's reasonable" (lower to 18).

- **Safety (parallel-run period) vs Plan momentum (one-PR)** — Safety wants 4a/4b split and a release cycle of dual-mode. The plan currently bundles. A middle ground: keep the single PR but require it merges with both paths active behind an env-var gate, then a follow-up PR removes MCP after a brief soak. This is a small process change, not a re-plan.

### Findings

#### Critical

- 🔴 **Architecture**: PPID session-id is unstable
  **Location**: Phase 2: Daemon lifecycle (lines 451-462)
  Session id derived from `${PPID}` defeats daemon reuse in the common Claude Code Bash-tool model where each invocation may be a fresh shell. Suggest `ACCELERATOR_SESSION_ID` UUID set at Step 0 and propagated to agents.

- 🔴 **Correctness**: PPID session id unstable across Bash invocations
  **Location**: Phase 2: Daemon lifecycle (session id derivation)
  Same root cause as Architecture finding. Adds the cross-talk failure mode (two unrelated agents with same parent shell) and the orphan-on-restart case.

- 🔴 **Security**: Regex `evaluate` deny-list is trivially bypassable
  **Location**: Phase 2: 'evaluate payload deny-list' (run.js)
  `globalThis['fe'+'tch']`, `fetch`, AST-level smuggling, `with(window){...}`, computed property access. Eval id 17 only tests literal-token forms. The agent-body allowlist is prose, not code; the deny-list is the only programmatic gate. Suggest AST-based check (`acorn`) or named structured operations replacing freeform `evaluate`.

- 🔴 **Security**: `--allow-internal` unlocks AWS/cloud metadata in one step
  **Location**: Phase 1: validate-source.sh changes; Desired End State item 1
  Single flag exposes 169.254.169.254 (Capital One 2019 SSRF surface) on every cloud-hosted dev box / CI runner. Suggest splitting `--allow-internal` (RFC1918/link-local non-IMDS) from `--allow-cloud-metadata` (or hard-block IMDS regardless).

- 🔴 **Security**: Caller-controlled screenshot path enables arbitrary file overwrite
  **Location**: Phase 2: 'screenshot' op signature
  `run.sh screenshot '{"path": ...}'` accepts absolute paths and `..`. Combined with prompt-injection from crawled pages → write to `~/.ssh/authorized_keys`, `~/.zshrc`, etc. Constrain to `ACCELERATOR_INVENTORY_OUTPUT_ROOT` with `realpath`-resolved boundary check and `O_CREAT|O_EXCL`.

- 🔴 **Correctness**: Host canonicalisation gaps allow trivial bypass of localhost-only default-allow
  **Location**: Phase 1: validate-source.sh host classification
  Misses uppercase, trailing dot, `0.0.0.0`, `[::]`, `[::ffff:127.0.0.1]`, decimal/hex IPv4 encoding, userinfo segments. Some forms accepted that shouldn't be; some rejected that should be. The default-allow asymmetry between http and https branches is itself a bug.

- 🔴 **Safety**: Big-bang switchover with no rollback path
  **Location**: Implementation Approach + Phase 4
  No feature flag; no parallel-run; lockstep delete of `.mcp.json` + agent frontmatter + eval rewrites. Suggest 4a/4b split: dual-mode under env-var gate first, MCP removal in follow-up.

- 🔴 **Safety**: Crawl bounds deferred while the safety surface they replaced is removed
  **Location**: What We're NOT Doing + Phase 2 daemon idle timer
  Page cap, wall-clock, screenshot byte budget become instructional-only with no enforcement. 5-minute idle daemon can spin under runaway pages with no kill switch. Either include wall-clock bound in Phase 2 (`setTimeout` → `browser.close()`) or re-scope so MCP removal does not land before bounds.

#### Major

- 🟡 **Architecture**: Daemon ownership and orphan-cleanup strategy under-specified
  **Location**: Phase 2: Daemon lifecycle and Phase 4: Cleanup steps
  No SIGKILL recovery, no concurrent-client serialisation, no startup stale-socket detection. Add connect-then-reclaim, PID-liveness check, SIGTERM-driven cleanup, and a Phase 2 test for stale-socket recovery.

- 🟡 **Architecture**: Regex deny-list is structurally weak primitive for security-critical filter
  **Location**: Phase 2: evaluate payload deny-list
  See cross-cutting theme. Either close the protocol (named ops) or explicitly de-load-bear the deny-list as "tripwire telemetry, not security boundary".

- 🟡 **Architecture**: Step 0 / Step 3 coupling
  **Location**: Phase 4 Step 0 + Step 3 (Confirm Executor Availability)
  Step 0's outcome decides crawler mode, but Step 0 only runs when crawler is runtime/hybrid. Resolve: Step 0 always runs (cheap on sentinel hit), returns 3-state result, mode selection consumes deterministically.

- 🟡 **Security**: Session id derived from PPID hash is predictable / collidable for local-attacker hijack
  **Location**: Phase 2 daemon lifecycle
  Other local users can enumerate `/proc/*/status` and reconstruct socket path. With default umask, socket is connectable. Suggest: random-UUID session id, dir mode 0700, socket mode 0600, `SO_PEERCRED` UID check on accept.

- 🟡 **Security**: Bootstrap suppresses npm audit and lacks lockfile / integrity pinning
  **Location**: Phase 3 ensure-playwright.sh
  `npm install --no-audit --no-fund --silent`; no `package-lock.json` shipped; transitive deps float; postinstall scripts run silently. Ship lockfile, use `npm ci --ignore-scripts`, drop `--no-audit`, pin `NPM_CONFIG_REGISTRY`.

- 🟡 **Security**: Chromium binary downloaded with no documented integrity verification
  **Location**: Phase 3 ensure-playwright.sh
  Playwright validates internally but plan doesn't pin `PLAYWRIGHT_DOWNLOAD_HOST`. A typo'd env var or hostile network can substitute the binary. Unset/refuse `PLAYWRIGHT_DOWNLOAD_HOST` overrides; mode 0700 cache; document hash verification.

- 🟡 **Security**: Auth-header origin-allowlist matching algorithm unspecified
  **Location**: Phase 2 auth-header injection
  Case folding? Default-port form? Cross-origin redirect re-evaluation? Subdomain confusion? IDN homograph? Specify exact algorithm (`new URL(...).origin`) and add tests for each pitfall.

- 🟡 **Security**: Mask defaults miss CSRF tokens, JWT-in-data-attribute, hidden inputs, ARIA-labelled secrets
  **Location**: Phase 2 screenshot masking
  Current list: `[type=password]`, `[autocomplete*=token]`, `[data-secret]`. Expand to include `[name*=csrf i]`, `[name*=token i]`, `[data-jwt]`, `[data-auth]`, `meta[name*=csrf i]`, `[aria-label*=token i]`, etc.

- 🟡 **Security**: No documented resource bounds on daemon — DoS by malicious page
  **Location**: Phase 2 daemon lifecycle
  No max RSS/CPU, no max contexts, no per-request timeout, no max response body in `route()`, no socket-message size cap. Bound socket message size (1 MB), set strict context permissions (deny geo/cam/clipboard), `setDefaultNavigationTimeout`.

- 🟡 **Security**: Auth header secret in env var exposed via /proc and process listings
  **Location**: Phase 2 ACCELERATOR_BROWSER_AUTH_HEADER
  `/proc/<pid>/environ` readable by user; long-lived daemon broadens window. Pass via stdin/socket-bootstrap message instead; zero env-var slot before fork.

- 🟡 **Test Coverage**: Daemon lifecycle concurrency and crash-recovery untested
  **Location**: Phase 2 test-run.js / test-run.sh
  No tests for stale socket recovery, parallel-client race, idle-timer accuracy, parent-shell exit. Add explicit cases parameterising the timer to 100ms.

- 🟡 **Test Coverage**: Offline mock defeats the purpose of the only automated bootstrap test
  **Location**: Phase 3 test-ensure-playwright.sh
  Real `npm install` path never CI-exercised. Either run real bootstrap against tiny fixture package, or split mock more narrowly to mock only Chromium download.

- 🟡 **Test Coverage**: Validator misses host-parsing edge cases the implementation hits
  **Location**: Phase 1 test cases
  No tests for `LOCALHOST`, `localhost.`, `0.0.0.0`, `[::ffff:127.0.0.1]`, path+port+query host extraction. Eval id 15 also doesn't widen.

- 🟡 **Test Coverage**: Test-suite stability across Phase 2/3 seam at risk
  **Location**: Phase 2 internal helper deletion
  Phase-2 internal helper that gets deleted in Phase 3; non-mock real-bootstrap path never automatically exercised. Make Phase 3 first change the rename + repointer; add an automated full-bootstrap CI step.

- 🟡 **Code Quality**: run.js structure described only by header comments
  **Location**: Phase 2 §3 (Implement run.js)
  ~600 LoC mixing daemon + client + IPC + deny-list + masking + route handler. Commit upfront to multi-file: `run.js` (dispatch, ~50 LoC), `daemon.js`, `client.js`, `deny-list.js` (pure, importable), `mask.js`, `errors.js`.

- 🟡 **Code Quality**: Best-effort regex heuristic for assignment detection is a code smell
  **Location**: Phase 2 'plain = outside comparison contexts'
  Drop the heuristic or use AST. Don't ship regex heuristic as hard reject.

- 🟡 **Code Quality**: Error-JSON shape under-specified across eight subcommands
  **Location**: Phase 2 protocol errors
  Specify schema: `{error: kebab-code, message, category, retryable, ...details}`. Enumerate codes per subcommand. One error-formatter is the only emitter.

- 🟡 **Correctness**: Test for 172.15.0.1 mis-attributes its own assertion
  **Location**: Phase 1 tests, line ~231
  The `--allow-internal` flag accepts everything, so the test labelled "just outside RFC1918" doesn't actually exercise the boundary. Add explicit boundary tests using stderr-content differentiation between RFC1918-reject and http-public-reject paths.

- 🟡 **Correctness**: 5-minute idle timer races with 5-minute crawl bound
  **Location**: Phase 2 daemon idle shutdown
  Long crawl with stretch > 5 min between executor calls causes silent daemon reset (lost state). Either reset timer on every operation AND on Playwright network events, or use in-progress counter, or bump to 15 min decoupled from crawl bound.

- 🟡 **Correctness**: No first-run-after-crash recovery for stale sockets and pidfiles
  **Location**: Phase 2 daemon socket / pidfile cleanup
  Connect-then-fallback pattern; pidfile liveness inspection; explicit test.

- 🟡 **Correctness**: Bracket-strip-then-port-strip ordering still mishandles zone-ids and IPv4-mapped
  **Location**: Phase 1 validate-source.sh IPv6 parsing
  Pin canonicalisation order: lowercase, trim trailing dot, strip brackets, strip port (only after brackets), strip zone-id. Add explicit tests.

- 🟡 **Correctness**: Word-boundary regex deny-list bypassable via property access
  **Location**: Phase 2 evaluate deny-list
  See security finding. Reframe as coarse typo guard, parse via `acorn`, or restrict to structured forms.

- 🟡 **Correctness**: TOCTOU race between sentinel check and flock; require.resolve looks in wrong tree
  **Location**: Phase 3 ensure-playwright.sh
  (1) Use lock-then-check, not check-then-lock. (2) `cd "$CACHE_ROOT"` or pass `paths` option to `require.resolve`. Add concurrent-bootstrap and corrupted-cache tests.

- 🟡 **Correctness**: Eval id 15 does not preserve loopback-rejection semantics dropped from id 13
  **Location**: Phase 1 eval id 13 split / id 15
  No coverage for non-`127.0.0.1` loopback range (e.g. `127.0.0.2`). Add explicit case to id 13 with stderr-content differentiation.

- 🟡 **Usability**: Reusing `--allow-internal` for http-to-public produces misleading error
  **Location**: Phase 1 §2 + Naming note
  Pin error string explicitly mentioning "insecure scheme" before the flag name; or split into two flags now.

- 🟡 **Usability**: First-run preamble does not commit to user affordances
  **Location**: Phase 3 §2 ensure-playwright.sh
  `npm install --silent` + 90-second freeze = user `^C`. Drop `--silent`; document `^C` cleanup; test SIGINT mid-bootstrap.

- 🟡 **Usability**: Hybrid-mode silent downgrade governed by prose, not assertion
  **Location**: Phase 4 §3 SKILL.md Step 3
  No eval asserts the user-visible "downgraded to code" message fires. Make the message a script responsibility (`notify-downgrade.sh` or `ensure-playwright.sh --mode hybrid`); add explicit Phase 4 eval expectation on stdout content.

- 🟡 **Portability**: OS coverage unstated; Windows silently dropped
  **Location**: Plan-wide
  Unix sockets, `flock`, bash idioms not portable to native Windows. Add explicit "supported platforms" line; fail fast on `MSYS*`/`MINGW*`/`CYGWIN*` `OSTYPE` if Windows is out of scope.

- 🟡 **Portability**: `flock` is not present on macOS by default
  **Location**: Phase 3 ensure-playwright.sh edge cases
  Replace with `mkdir`-based atomic locking or fall back at runtime. Test for two concurrent invocations on a system without `flock`.

- 🟡 **Portability**: Bash regex features not available under `sh`
  **Location**: Phase 1 validate-source.sh pseudocode
  Pin shebang to `#!/usr/bin/env bash`; assert `BASH_VERSION`; test `sh -c "./validate-source.sh ..."` either re-execs or fails loudly.

- 🟡 **Portability**: Unix socket path may exceed `sun_path` on macOS
  **Location**: Phase 2 run.js daemon
  104-byte cap on macOS; deep `~/Library/Application Support/...` plus `playwright-session/<sessionid>.sock` overflows. Place socket under `${TMPDIR}` or `chdir`-then-bind-relative. Length-check at startDaemon.

- 🟡 **Portability**: `${CLAUDE_PLUGIN_DATA}` fallback assumption unverified
  **Location**: Plan Overview / Current State Analysis
  No empirical test or version floor cited. Document which Claude Code versions populate it durably; print resolved root in Phase 3 verification.

- 🟡 **Portability**: npm install assumes public registry; corporate proxies / Artifactory not considered
  **Location**: Phase 3 §2
  List relevant env vars (`NPM_CONFIG_REGISTRY`, `NODE_EXTRA_CA_CERTS`, `HTTPS_PROXY`, `PLAYWRIGHT_DOWNLOAD_HOST`); document mirror-pointing recipe.

- 🟡 **Safety**: Daemon orphan / leaked-resource accounting incomplete
  **Location**: Phase 2 daemon lifecycle
  See architecture & correctness findings. Add `daemon-stop --all` for users to recover.

- 🟡 **Safety**: Silent npm install runs unsupervised on first runtime crawl
  **Location**: Phase 3 ensure-playwright.sh
  See security finding. Add pre-flight `df` check requiring 500 MB free.

- 🟡 **Safety**: Regex deny-list false-positives framed as programming errors
  **Location**: Phase 2 evaluate payload deny-list
  When deny-list false-positives, agent is told "do not retry" → silent data loss. Record rejected attempts in `Crawl Notes` as known-incomplete observations.

- 🟡 **Safety**: Phase 4 cohesion makes intermediate commits red
  **Location**: Phase 4 lockstep changes
  State squash-merge requirement explicitly, or split 4a/4b for atomic commits.

- 🟡 **Safety**: Shared `${CLAUDE_PLUGIN_DATA}` cache layout lacks version namespacing
  **Location**: Phase 2 cache layout
  Namespace cache root by plugin name + sha256(package.json)[:8]. Hold flock for entire install transaction.

- 🟡 **Compatibility**: Hard pin `playwright@1.49.0` decouples from upstream patch/security fixes
  **Location**: Phase 2 package.json
  Use caret/tilde range with lockfile, or document explicit bump policy. Record resolved version in `evals/benchmark.json`.

- 🟡 **Compatibility**: Node ≥ 20 floor undeclared in plugin.json; breaks Node 18 LTS users silently
  **Location**: Phase 3 ensure-playwright.sh
  Either lower to 18 (Playwright 1.49 supports it), or declare in `plugin.json` and gate behind major-version bump with prominent CHANGELOG callout.

- 🟡 **Compatibility**: Executor wire protocol unversioned
  **Location**: Phase 2 protocol surface
  Add `"protocol": 1` field to requests/responses, or document explicitly as internal-unstable in run.js. User-override agent bodies are particularly exposed.

- 🟡 **Compatibility**: Eval ids 3 and 13 mutated in place
  **Location**: Phase 4 §4 evals/evals.json
  Allocate fresh ids 18+ for new tests; retire 3/13 with deprecation marker. Verify benchmark.json schema supports id retirement before finalising.

#### Minor

- 🔵 **Architecture**: Splitting package.json from install root creates fragile resolver dance
- 🔵 **Architecture**: Executor surface mirrors MCP one-to-one without justification
- 🔵 **Architecture**: Single `--allow-internal` flag conflates insecure-scheme and internal-host concerns
- 🔵 **Architecture**: Auth header origin enforcement now lives in two places
- 🔵 **Architecture**: Phase-2 internal bootstrap helper creates transient coupling
- 🔵 **Security**: `NODE_PATH` set unconditionally to writeable cache invites shadowing
- 🔵 **Security**: Host parsing missing userinfo, percent-encoding, IDN, DNS rebinding
- 🔵 **Security**: Executor availability probe bypasses URL validator
- 🔵 **Security**: No security-event logging for deny-list / mask / origin-strip events
- 🔵 **Test Coverage**: Screenshot-masking test has weak fallback assertion
- 🔵 **Test Coverage**: Best-effort deny-list `=` heuristic isn't characterised by tests
- 🔵 **Test Coverage**: Rewritten eval id 3 may lose its negative assertion strength
- 🔵 **Code Quality**: Argument parsing under-specified in validate-source.sh pseudocode
- 🔵 **Code Quality**: Host-classification helpers exist as pseudocode but aren't separately testable
- 🔵 **Code Quality**: Phase-2 internal bootstrap helper is acknowledged technical debt
- 🔵 **Code Quality**: 5-minute idle timer couples to crawl wall-clock without naming the invariant
- 🔵 **Code Quality**: Session-id derivation described too loosely to be implementable
- 🔵 **Code Quality**: run.js header comment exceeds codebase style
- 🔵 **Correctness**: `=` heuristic will produce false positives on legitimate payloads
- 🔵 **Correctness**: `--offline-mock` substitutes happy-path-only no-ops
- 🔵 **Correctness**: Step 0 bootstrap runs before validation; failure messages will conflate concerns
- 🔵 **Correctness**: Single flag conflates two unrelated relaxations (correctness echo)
- 🔵 **Usability**: Validator error wording not pinned, only flag name
- 🔵 **Usability**: Hung-daemon recovery has no user-discoverable path
- 🔵 **Usability**: Node ≥ 20 requirement discoverable too late
- 🔵 **Usability**: Cache cleanup story undocumented
- 🔵 **Usability**: `evaluate-payload-rejected` returns raw stderr JSON to a human user
- 🔵 **Portability**: Single hard pin may not have prebuilt Chromium for less-common architectures
- 🔵 **Portability**: Project's `mise.toml` pins Node 22 but plan declares ≥ 20 — record intent
- 🔵 **Portability**: Headless Chromium may need `--no-sandbox` in containerised CI
- 🔵 **Portability**: Vendor coupling on Microsoft `playwright` npm package — flag for visibility
- 🔵 **Safety**: Probe via `about:blank` triggers full bootstrap on readiness check
- 🔵 **Safety**: Migration Notes do not document new resource footprint
- 🔵 **Safety**: Pass-rate floor of 0.9 silently lowers prior baseline of 1.0
- 🔵 **Compatibility**: Env-var expansion in sub-agent `tools:` unverified for permission engine
- 🔵 **Compatibility**: Removal of `.mcp.json` may be observed by ecosystem tooling
- 🔵 **Compatibility**: Structural test asserts "MCP path is gone forever" rather than "executor is wired"
- 🔵 **Compatibility**: Error-JSON shape stable for one error code but not declared as schema

#### Suggestions

- 🔵 **Test Coverage**: Variance gate set lower than prior baseline
- 🔵 **Usability**: argument-hint becomes long; consider grouping
- 🔵 **Compatibility**: No semver guidance for what version bump this plan triggers

### Strengths

- ✅ Clear phase decomposition; Phases 2 and 3 buildable in isolation; Phase 4 wires them in lockstep
- ✅ TDD discipline explicit per phase: failing tests before implementation, with refactor as a third step
- ✅ Honest acknowledgement of design stretches: the "Naming note" on `--allow-internal`, the "best-effort heuristic" on the regex deny-list, the "What We're NOT Doing" deferrals
- ✅ Phase 1 helper extraction (`is_localhost_default`, `is_internal_flagged`) collapses seven copy-pasted host rejects
- ✅ Default-allow scope for SSRF is narrowed appropriately to literal `localhost`/`127.0.0.1` only — NOT widened to `127.0.0.0/8`
- ✅ Auth-header injection moved server-side via `context.route()` rather than relying on every caller to attach correctly
- ✅ Screenshot masking always-on with no opt-out flag, enforced in launcher rather than agent
- ✅ MCP detection via LLM self-introspection replaced with deterministic shell probe
- ✅ Cache placement at `${CLAUDE_PLUGIN_DATA}` (with fallback) correctly separates version-keyed code from version-stable persistent data
- ✅ Removing `.claude-plugin/.mcp.json` eliminates a hard runtime dependency on `@playwright/mcp` and the upstream MCP-inheritance bug class
- ✅ Plan correctly identifies the bracketed-IPv6 vs port-strip ordering bug in the existing script
- ✅ Inventory artifact shape unchanged — downstream consumers (`analyse-design-gaps`) keep working
- ✅ Phase 1's `--allow-internal` flag is purely additive on the skill CLI; existing invocations keep working
- ✅ `${CLAUDE_PLUGIN_DATA}` fallback to `${HOME}/.cache/accelerator/playwright}` so older Claude Code builds still work
- ✅ Idempotent bootstrap via sentinel with a sub-2-second second-run budget

### Recommended Changes

Ordered by impact. Many of these collapse multiple findings.

1. **Replace PPID-derived session id with explicit `ACCELERATOR_SESSION_ID`** (addresses: PPID-architecture, PPID-correctness, session-id-code-quality, session-id-security)
   The skill exports a UUID at Step 0. `run.sh` requires it (fail-closed if unset). Eliminates daemon reuse loss, cross-talk risk, local-attacker hijack via PPID enumeration. Add session id to the executor protocol contract.

2. **Replace freeform `evaluate` deny-list with AST validation or named structured ops** (addresses: deny-list-security, deny-list-architecture, deny-list-code-quality, deny-list-correctness, deny-list-safety)
   Either parse via `acorn` (single dep, already a Playwright transitive) and walk for forbidden identifiers/MemberExpressions, or replace freeform `evaluate` with `getComputedStyle`, `getBoundingClientRect`, `dataset`, `querySelectorAllAttrs` — closed-protocol, no JS string crosses the boundary. Update agent body and eval id 17 to assert the new boundary.

3. **Split `--allow-internal` into two flags from day one** (addresses: --allow-internal-security/IMDS, --allow-internal-usability, --allow-internal-architecture, --allow-internal-correctness)
   `--allow-internal` (RFC1918, link-local non-IMDS, IPv6 ULA, other 127/8) and `--allow-insecure-scheme` (http-to-public). Hard-block `169.254.169.254` and Azure `168.63.129.16` regardless of `--allow-internal` unless an explicit `--unsafe-allow-cloud-metadata` is passed. Pin error wording for each path.

4. **Constrain `screenshot` op `path` argument** (addresses: screenshot-path-traversal-security)
   Reject absolute paths and `..`; require resolution under `ACCELERATOR_INVENTORY_OUTPUT_ROOT` env-var-supplied directory; open with `O_CREAT|O_EXCL`. Add tests for each rejection.

5. **Canonicalise host strings before classification** (addresses: host-canon-correctness, host-canon-security, host-canon-test-coverage)
   Use `new URL(...)` (Node) for parsing or expand the explicit case list: lowercase, trim trailing dot, strip brackets-then-port, strip zone-id, reject userinfo, reject decimal/hex IPv4 encoding, add `0.0.0.0`/`[::]`/`[::ffff:127.0.0.1]` explicitly to flag-gated set. Document DNS rebinding as known limitation.

6. **Split Phase 4 into 4a (executor + dual-tool agents) and 4b (MCP removal)** (addresses: big-bang-safety, lockstep-safety, structural-test-compatibility, rollback-safety)
   4a: add `Bash(...run.sh *)` to agent `tools:` and `allowed-tools:` alongside existing `mcp__playwright__*`; switch agent prose to use the executor; keep `.mcp.json` for one release. 4b: delete `.mcp.json`, remove `mcp__*` from frontmatter. Each `main` SHA stays green and is independently revertible.

7. **Land at least the wall-clock bound in Phase 2** (addresses: crawl-bounds-safety, dos-security)
   `setTimeout(() => browser.close().then(() => process.exit(2)), WALL_CLOCK_MS)` in the daemon. Page cap and screenshot byte budget can defer; wall-clock cannot — it is the only safety net against runaway pages.

8. **Specify daemon orphan / stale-socket / shutdown semantics** (addresses: daemon-orphan-architecture, daemon-orphan-correctness, daemon-orphan-safety, daemon-orphan-test-coverage)
   On client startup: try-connect-then-rm-and-fork. On daemon startup: pidfile liveness + EUID + cmdline check. SIGTERM cleanup. `daemon-stop --all` user command. Test stale-socket recovery, parallel-client serialisation, parameterised idle timer.

9. **Replace `--offline-mock` boolean with parametric mock flags** (addresses: offline-mock-test-coverage, offline-mock-correctness)
   `mock_npm_install_exit`, `mock_playwright_install_exit`, etc. Each failure-path test sets the relevant flag. Add a single non-mock CI step against a tiny fixture package.

10. **Allocate fresh eval ids for rewritten tests** (addresses: eval-id-rewrite-compatibility, eval-id-rewrite-test-coverage, eval-id-15-correctness)
    New ids 18 (executor-bootstrap-failure-fallback), 19 (internal-host-rejection-without-localhost). Retire 3 and 13 with deprecation markers. Add explicit `127.0.0.2` rejection case so non-`127.0.0.1` loopback range is exercised.

11. **Replace `flock` with portable lock; pin bash-only execution** (addresses: flock-portability, bash-regex-portability, sun_path-portability)
    `mkdir`-based atomic acquisition with `trap` cleanup, or runtime fallback. `validate-source.sh` shebang `#!/usr/bin/env bash` + `BASH_VERSION` guard. Socket path under `${TMPDIR}` or relative-bind. Add macOS-equivalent CI matrix entry.

12. **Ship a `package-lock.json`; drop `--no-audit`/`--silent`; namespace cache by hash** (addresses: npm-supply-chain-security, npm-supply-chain-safety, cache-poisoning-safety, npm-portability-corporate)
    Lockfile committed alongside `package.json`. `npm ci --ignore-scripts`; explicit `npx playwright install chromium` after. Cache root namespaced as `${ROOT}/accelerator/playwright/<sha8>/`. Document `NODE_EXTRA_CA_CERTS` / `HTTPS_PROXY` / `PLAYWRIGHT_DOWNLOAD_HOST` env vars in error messages. Pre-flight `df` ≥ 500 MB.

13. **Specify error-JSON envelope and protocol versioning** (addresses: error-shape-code-quality, protocol-versioning-compatibility, error-shape-compatibility)
    Canonical `{error: kebab-code, message, category, retryable, ...details}`. `"protocol": 1` field on requests/responses. One error-formatter is the only stderr emitter. Eval id 17 asserts subset-match (additive future fields are non-breaking).

14. **Pin error wording and downgrade-message responsibility** (addresses: usability-error-wording, downgrade-silent-usability, validator-error-stale-text)
    `validate-source.sh` reject path: explicit error template naming the host, the classification, and the relevant flag — and asserting the obsolete `(not available in v1)` text is gone. Hybrid-mode downgrade emits its message from `ensure-playwright.sh` (script responsibility, not LLM prose); Phase 4 eval asserts stdout content.

15. **Split run.js into testable modules** (addresses: run.js-structure-code-quality, deny-list-architecture)
    `run.js` (CLI dispatch only, ~50 LoC), `daemon.js`, `client.js`, `deny-list.js` (pure, no Node deps), `mask.js`, `errors.js`. Deny-list and error-formatter become importable, side-effect-free units.

16. **Reconcile pass-rate gates** (addresses: pass-rate-floor-test-coverage, pass-rate-floor-safety)
    Either tighten Phase 5 gate to "≥ prior baseline minus documented variance margin (0.05)" or limit the 0.9 floor to *new* evals only (15-19) with a "no regression" rule for pre-existing evals.

17. **Decide and document version bump policy** (addresses: semver-compatibility, node-floor-compatibility)
    State whether this is a minor or major bump. If Node 20 floor stays, declare in `plugin.json` and add CHANGELOG "Breaking" callout. Migration Notes should document new disk footprint, daemon-stop incantation, and cache-purge recipe.

18. **Verify env-var expansion in sub-agent `tools:`** (addresses: agent-tool-env-expansion-compatibility)
    A real `Task` invocation of `browser-locator` confirming `run.sh navigate` actually executes. If unsupported, fall back to a permissive `Bash` allow with runtime path check. Land before Phase 4 PR opens.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan responds well to the architectural forces (routing around the MCP-inheritance bug while preserving the agent boundary) and the functional core / lazy bootstrap / defence-in-depth layering is reasonable. However the per-shell daemon-session-id-via-PPID scheme is the weakest joint, and combined with under-specified daemon-lifecycle ownership and a moderately broad executor surface, this risks brittle coupling and unclear failure modes.

**Strengths**: Phase boundaries; preserved sub-agent role cohesion; defence-in-depth layered architecture; auth-header origin enforcement migrated from prose to programmatic; deterministic shell probe replaces LLM self-introspection; cache placement at `${CLAUDE_PLUGIN_DATA}` separates code from data.

**Findings**: 1 critical (PPID session-id), 3 major (daemon ownership, regex deny-list weakness, Step 0/Step 3 coupling), 5 minor (resolver dance, surface mirrors MCP, flag overload, origin enforcement duplication, transient bootstrap helper).

### Security

**Summary**: The plan loosens SSRF defences and replaces an MCP integration with a Bash-invoked Node executor that introduces several new high-risk surfaces: a regex-only `evaluate` deny-list trivially bypassable from page-controlled content, a session-id derived from PPID, predictably-named Unix sockets, a caller-controlled screenshot path, an unspecified auth-header origin allowlist algorithm, and a first-run bootstrap that downloads ~150MB of Chromium with no integrity pinning while explicitly suppressing `npm audit`. Several findings are critical.

**Strengths**: Server-side origin-restricted auth header; always-on screenshot masking; deny-list as defence-in-depth; idle-shutdown timer; pinned Playwright version; narrow default-allow scope; `.mcp.json` removal eliminates project-scoped attack/hallucination surface.

**Findings**: 3 critical (deny-list trivially bypassable, `--allow-internal` IMDS one-step, screenshot path traversal), 7 major (PPID hijack, npm audit suppression, Chromium integrity, origin-allowlist algorithm, mask defaults, no DoS bounds, env-var secret exposure), 4 minor (NODE_PATH shadowing, host-parsing edge cases, probe bypasses validator, no audit logging).

### Test Coverage

**Summary**: The plan establishes a reasonable test pyramid and explicitly schedules tests-first within each phase. However, several high-risk areas are undertested: daemon lifecycle and concurrency, bootstrap failure modes, and validator edge cases. The use of `--offline-mock` for the only automated bootstrap test risks reducing coverage to a tautology — the real install path is gated behind a manual pre-merge step.

**Strengths**: TDD discipline; eval suite updated in lockstep with structural assertions; eval id 14 + new id 17 give defence-in-depth two test points; coverage spans three layers; broad RFC1918 + link-local + IPv6 + 172.16-31 boundary coverage.

**Findings**: 4 major (daemon lifecycle untested, offline-mock defeats purpose, validator host-parsing edge cases, Phase 2/3 seam stability), 3 minor (mask test fallback, deny-list `=` heuristic untested, eval id 3 negative-assertion strength), 1 suggestion (variance gate vs baseline).

### Code Quality

**Summary**: Well-organised, TDD-disciplined, rigorous about test surface — but the centerpiece (400-600 LoC single-file run.js) is described only as a comment-block outline rather than a concrete module decomposition. Several hand-wavy spots (best-effort regex, `{error, …}` shape, bash-arg parse "any position", bootstrap edge cases) will harden into technical debt without sharper specification.

**Strengths**: TDD discipline; Phase 1 helper extraction collapses seven copy-pasted rejects; bracket-strip-then-port-strip ordering bug correctly identified; deny-list patterns individually paired with pass/fail tests; bootstrap edge-case enumeration concrete; deterministic shell probe replaces LLM self-introspection.

**Findings**: 3 major (run.js structure, regex heuristic code smell, error-JSON shape), 6 minor (arg-parse spec, helpers not separately testable, transient helper debt, idle-timer coupling, session-id derivation looseness, header-comment style).

### Correctness

**Summary**: Phase 2 daemon/IPC machinery and Phase 3 bootstrap introduce significant correctness hazards that the plan only partially anticipates. PPID-derived session id, 5-minute idle timer aligned with crawl bound, evaluate-deny-list regex, IPv6/host-canonicalisation gaps, and sentinel-vs-node_modules race all have concrete failure modes. Phase 1 validator changes mostly sound but contain a mis-scoped test and several host-canonicalisation gaps.

**Strengths**: http-to-public correctly remains gated; daemon idle/SIGTERM/daemon-stop shows lifetime awareness; bootstrap idempotent via sentinel + flock; 172.16/172.31 boundary tested; deny-list correctly framed as defence-in-depth; auth header injection moved server-side.

**Findings**: 2 critical (PPID instability, host-canon bypass), 7 major (172.15 mis-attribution, idle-timer race, no crash recovery, IPv6 ordering, deny-list bypassable, sentinel TOCTOU, eval id 15 gap), 4 minor (`=` heuristic false positives, mock too coarse, Step 0 ordering, single-flag scope creep).

### Usability

**Summary**: Largely DX-aware: surfaces a first-run preamble, names a clear cache root with fallback, fixes the UAT pain point (http://localhost), keeps deterministic Step-0 in front of agent spawn. Several details under-specified: dual-purpose `--allow-internal` confuses error messages, bootstrap preamble doesn't commit to wording or progress, downgrade-to-code semantics depend on prose updates, daemon/cache lifecycle gives no visible re-run ergonomics or cleanup story.

**Strengths**: Step 0 surfaces failure before agent spawn; default-allow for localhost addresses UAT pain; cache root has fallback; idempotent bootstrap; naming-stretch acknowledged; argument-hint exposes the new flag; ensure-playwright failure modes named.

**Findings**: 3 major (`--allow-internal` misleading error, preamble lacks user affordances, hybrid-mode silent downgrade), 5 minor (validator error wording not pinned, hung-daemon recovery undiscoverable, Node ≥20 discoverable too late, cache cleanup undocumented, raw stderr JSON to humans), 1 suggestion (argument-hint length).

### Portability

**Summary**: Significant portability wins by removing the Playwright MCP dependency and consolidating browser automation behind a vendored Node executor. However introduces several implicit environment assumptions (Unix sockets, `flock`, modern bash regex, npm-registry reachability, writable cache paths) without naming the supported OS matrix or guarding against the cases that will break — particularly on macOS, the project's likely primary platform.

**Strengths**: Removing `.mcp.json` eliminates `@playwright/mcp` dependency; `${CLAUDE_PLUGIN_DATA}` fallback documented; avoids `python3 -m http.server` dependency; Chromium-only download bounds disk footprint; `run.sh` resolves SCRIPT_DIR via `BASH_SOURCE`.

**Findings**: 6 major (OS coverage unstated, flock not on macOS, bash regex not under sh, sun_path on macOS, CLAUDE_PLUGIN_DATA unverified, npm registry assumes public), 4 minor (Playwright pin architectures, Node 22 vs 20 drift, headless --no-sandbox, vendor coupling).

### Safety

**Summary**: Replaces a working integration with a substantially larger surface area (long-lived daemon, sockets, lockfiles, sentinels, regex deny-list, auto-installed npm dependency tree, ~150 MB Chromium download) in a single big-bang switchover with no parallel-run, no feature flag, and no documented rollback. Several runaway-process safeguards explicitly deferred — most concerning, the page cap, wall-clock, and screenshot byte budget become unenforced. Recovery, blast-radius containment, and disk/supply-chain safeguards need hardening.

**Strengths**: Lazy bootstrap gated behind Step-0 prerequisite check; sentinel + flock pattern; deny-list as defence-in-depth; daemon idle + SIGTERM bound; cache root persistent across version bumps.

**Findings**: 2 critical (big-bang switchover, crawl bounds deferred), 5 major (daemon orphan accounting, silent npm install, regex false-positives as programming errors, Phase 4 cohesion, cache namespacing), 3 minor (about:blank probe heavyweight, migration notes understated, 0.9 floor lowers baseline).

### Compatibility

**Summary**: Introduces several new contracts (executor wire protocol, env-var allowlist, JSON error shapes) and removes an existing one (`mcp__playwright__*` tools, `.mcp.json`) without an explicit versioning or compatibility-window policy. Most additive changes are safe (`--allow-internal` flag, new env vars, evals 15-17), but the hard `playwright@1.49.0` pin, unconditional Node ≥ 20 floor, and in-place rewrite of eval ids 3/13 carry real compatibility risk.

**Strengths**: `--allow-internal` flag purely additive; env-var naming consistent with existing namespace; `${CLAUDE_PLUGIN_DATA}` fallback; `Bash(${CLAUDE_PLUGIN_ROOT}/...)` pattern established for SKILL allowed-tools; inventory artifact shape unchanged; users with own MCP unaffected.

**Findings**: 4 major (Playwright hard pin, Node 20 floor undeclared, protocol unversioned, eval id mutation), 4 minor (env-var expansion in agent tools unverified, .mcp.json removal observability, structural test over-strict, error-JSON shape one-off), 1 suggestion (semver guidance).

## Re-Review (Pass 2) — 2026-05-06T17:56:03Z

**Verdict:** REVISE

The revision is a substantial structural improvement. **All 8 prior critical findings are resolved** — PPID session id, regex deny-list bypass, `--allow-internal` cloud-metadata foot-gun, screenshot path traversal, host-canonicalisation gaps, big-bang switchover, and crawl-bounds deferral are all addressed via concrete design changes (visualiser daemon pattern, deny-list dropped, flag split, path-guard, canonicalisation helper, Phase 4a/4b split, daemon-enforced wall-clock kill).

The verdict remains REVISE because **22 new majors** were identified — but qualitatively these are addressable hygiene fixes rather than structural redesign. They cluster around five themes (below) and could be retired in a single targeted edit pass, not another rewrite.

### Cross-Cutting Themes (new)

- **Wall-clock crawl bound never re-arms across daemon reuse** (Correctness + Safety, both major) — the timer is described as "armed at first `navigate` after a fresh daemon"; under daemon-reuse-before-lock (the common path) a second crawl gets no kill switch. The single most important new finding: silently reintroduces the runaway-page risk the bound was added to prevent.
- **`run.sh` pseudocode references non-existent `find-repo-root.sh`; TMP_REL config indirection mentioned in prose but not invoked in code** (Portability + Code Quality, both major) — implementation will fail on first run as written; per-project `tmp` config overrides silently ignored.
- **`ACCELERATOR_INVENTORY_OUTPUT_ROOT` precondition under-specified** (Security + Correctness, major + minor) — the screenshot path-guard relies on this env var being set, but Phase 4 §3 SKILL.md change-list does not enumerate exporting it; behaviour when unset is unspecified, re-opening the prior critical path-traversal finding.
- **`notify-downgrade.sh` has no test, no canonical message source** (Code Quality + Safety + Usability) — script promoted from LLM prose to shell to enable structural assertion, but the eval matcher semantics are unspecified, no goldenfile fixture, no test-notify-downgrade.sh, and the literal message lives in 3+ places.
- **Visualiser code lift unspecified — copy vs source not pinned** (Code Quality + Portability + Compatibility, major + minor) — `start_time_of()`, lock helpers, etc. are described as "lifted from launcher-helpers.sh — copy / source either way"; neither path is committed, both have failure modes (drift if copied, hidden coupling if sourced).

### Previously Identified Issues

#### Critical (all 8 resolved)

- ✅ **Architecture**: PPID session-id is unstable — Resolved (project-scoped daemon via `find_repo_root` + PID/start_time identity)
- ✅ **Correctness**: PPID session id unstable across Bash invocations — Resolved (same)
- ✅ **Security**: Regex `evaluate` deny-list trivially bypassable — Resolved (deny-list dropped wholesale; agent-body allowlist remains the documented contract)
- ✅ **Security**: `--allow-internal` unlocks AWS/cloud metadata in one step — Partially resolved (flag split into `--allow-internal` + `--allow-insecure-scheme` retires the prose-mismatch foot-gun; cloud-metadata still gated by `--allow-internal` per user-scope decision)
- ✅ **Security**: Caller-controlled screenshot path enables arbitrary file overwrite — Resolved (path-guard via `ACCELERATOR_INVENTORY_OUTPUT_ROOT` + realpath + `..` rejection; see new major below for residual env-var precondition)
- ✅ **Correctness**: Host canonicalisation gaps allow trivial bypass — Resolved (lowercase, trailing dot, brackets, port, zone-id, userinfo, decimal/octal/hex all handled)
- ✅ **Safety**: Big-bang switchover with no rollback path — Resolved (Phase 4a/4b split with explicit revert-4b property)
- ✅ **Safety**: Crawl bounds deferred — Resolved in code (5-min wall-clock `setTimeout` + `browser.close()` + `process.exit(2)` + `server-stopped.json` audit), though see new major about re-arm semantics

#### Major (24 of prior 27 resolved; 3 partially resolved → recur as new majors)

Resolved:
- ✅ Architecture: daemon ownership / orphan cleanup; regex deny-list as load-bearing primitive; Step 0/Step 3 coupling
- ✅ Security: PPID local-attacker hijack; lockfile pinning shipped; auth-origin algorithm specified; mask defaults frozen by scope; DoS bounds enforced; env-var auth-header out-of-scope by user
- ✅ Test Coverage: daemon lifecycle untested; offline-mock defeats purpose; validator host-parsing edge cases; Phase 2/3 seam stability
- ✅ Code Quality: run.js structure; regex heuristic code smell; error-JSON shape under-specified
- ✅ Correctness: 172.15 mis-attribution; idle/wall-clock race; first-run-after-crash recovery; IPv6 ordering; deny-list bypass; sentinel TOCTOU; eval id 15 gap
- ✅ Usability: `--allow-internal` misleading error; preamble UX (npm progress, `^C` cleanup); hybrid-mode silent downgrade
- ✅ Portability: OS coverage; flock-on-macOS; bash regex under sh; sun_path on macOS; `${CLAUDE_PLUGIN_DATA}` unverified; npm corporate-network
- ✅ Safety: daemon orphan accounting; silent npm install (lockfile + ignore-scripts); Phase 4 cohesion; regex false-positives as programming errors
- ✅ Compatibility: Playwright hard pin (now `~1.49.0` + lockfile); Node 20 floor declared; protocol versioned; eval id mutation (fresh ids 18-21)

Partially resolved → re-asserted as new majors:
- 🟡 **Security**: `npm audit` suppression — `--no-audit` retained with unscheduled future-ticket commitment; `--silent` removal does not surface audit signal
- 🟡 **Architecture**: package.json install-into-cache resolver dance — TOCTOU fixed but two-copy split survives; sentinel doesn't track lockfile hash
- 🟡 **Safety**: cache root namespacing — visualiser pattern adopted but cache root has no per-version subdir; multiple plugin versions on same machine collide

### New Issues Introduced

#### Major (22)

- 🟡 **Correctness**: Wall-clock crawl bound never re-arms across daemon reuse — timer arms only on first `navigate` after fresh daemon; reused daemon (common path) runs unbounded
- 🟡 **Safety**: Wall-clock bound only arms on fresh daemon — same finding from Safety lens
- 🟡 **Correctness**: `run.sh ping` is not a sufficient executor liveness signal — verifies playwright resolves but not Chromium binary integrity; bootstrap-failure-class errors leak past Step 5 into agent execution
- 🟡 **Portability** + **Code Quality**: `find_repo_root` references non-existent `scripts/find-repo-root.sh` — script does not exist; only `vcs-common.sh` defines `find_repo_root()` as a sourceable function
- 🟡 **Portability**: TMP_REL config indirection invoked in prose but not in pseudocode — bare `${TMP_REL:-.accelerator/tmp}` ignores per-project config overrides that visualiser honours via `config-read-path.sh`
- 🟡 **Portability** + **Code Quality** + **Compatibility**: Visualiser-pattern reuse strategy unspecified — "copy / source" left to implementer; both paths have failure modes
- 🟡 **Portability**: macOS-without-flock fallback only manually verified — automated test does not force the `mkdir lock.d` branch
- 🟡 **Test Coverage**: Multi-file `lib/*` modules have no per-module unit tests — promised testability win not collected; pure modules only exercised via integration
- 🟡 **Test Coverage**: Validator helper unit tests claimed but not delivered — Testing Strategy mentions a `test-validate-source.sh` but no Phase 1 deliverable
- 🟡 **Test Coverage**: Screenshot path-guard rejection cases under-tested — only 2 of 4 documented rejection paths have cases
- 🟡 **Security** + **Correctness**: `ACCELERATOR_INVENTORY_OUTPUT_ROOT` precondition under-specified — Phase 2 says "populated by the skill at agent-spawn time" but Phase 4 SKILL.md change-list does not enumerate exporting it
- 🟡 **Security**: `npm ci --no-audit` retained with unscheduled future-ticket — supply-chain visibility gap not closed by lockfile alone
- 🟡 **Code Quality** + **Safety** + **Usability**: `notify-downgrade.sh` has no dedicated test, no single source of truth for literal output
- 🟡 **Usability**: Internal-host-on-http error names only `--allow-internal` — but SKILL.md prose says "needs both flags"; tests show `--allow-internal` alone suffices for internal http
- 🟡 **Compatibility**: Protocol-version assertion only generic, not per-subcommand
- 🟡 **Compatibility**: Error-envelope schema not surfaced to downstream agent-body authors — schema lives only in plan
- 🟡 **Compatibility**: `grep -r 'evaluate-payload-rejected'` structural assertion blocks any future deny-list reintroduction — too tight
- 🟡 **Safety**: Squash-merge requirement stated but not enforced — bisect-safety property collapses under non-squash strategies
- 🟡 **Safety**: Cache root has no version namespace — multiple plugin versions on the same machine collide on shared cache
- 🟡 **Safety**: Phase 3-before-Phase 2 dependency creates hidden revert hazard — reverting Phase 3 cascades into Phase 2 test failures
- 🟡 **Compatibility**: 4a state requires user-visible verification that dual-tool agents survive the bug class — agent prose-ordering not pinned by eval

#### Minor (selected — full list in per-lens results below)

- 🔵 **Architecture**: Identity check duplicated in run.sh (bash) and lib/identity.js (Node)
- 🔵 **Architecture**: client.js may spawn a daemon "via run.sh-equivalent path" — two spawn paths to keep in sync
- 🔵 **Architecture**: `find_repo_root` assumes a discoverable repo for every invocation
- 🔵 **Security**: Percent-encoded host characters not handled by canonicalisation
- 🔵 **Security**: `notify-downgrade.sh` `--reason` arg not pinned to closed enum (low under scope)
- 🔵 **Security**: Test pins forward-arbitrary-JS as v1 contract — should document non-filtering at protocol layer
- 🔵 **Code Quality**: Step numbering collides between phases and is fragile to insertion
- 🔵 **Code Quality**: Validator pseudocode ~90 lines of inline shell at upper boundary of readability
- 🔵 **Code Quality**: `pngjs` devDependency installed in production cache (lacks `--omit=dev`)
- 🔵 **Code Quality**: Per-subcommand error code enumeration deferred to implementation
- 🔵 **Code Quality**: lib/daemon.js + lib/state.js have likely circular import seam
- 🔵 **Correctness**: Octal IPv4 form `0177` (no dot) edge case
- 🔵 **Correctness**: Identity check passes but listening socket may have died independently
- 🔵 **Correctness**: Phase 1 success-criterion wording about MCP assertions is stale (documentation, not defect)
- 🔵 **Usability**: Wall-clock 5-min crawl bound surface invisible when it fires
- 🔵 **Usability**: 30-min idle daemon mentioned in Migration Notes but not user-facing flow prose
- 🔵 **Usability**: `protocol-mismatch` error is structured JSON, not human-friendly for agent-file editors
- 🔵 **Usability**: argument-hint with five tokens likely wraps on standard 80-column terminals
- 🔵 **Usability**: `run.sh ping` / `daemon-status` exposed but not surfaced as diagnostic tools in README troubleshooting
- 🔵 **Usability**: Forbidden-pattern allowlist enumerated in agent body but no signal that it is now soft-only
- 🔵 **Portability**: Visualiser-pattern adoption omits `host_header_guard` and `origin_guard` middleware — DNS rebinding surface
- 🔵 **Portability**: Lockfile regenerated on Node 22 may pin Node-22-only transitives
- 🔵 **Portability**: Tilde range plus `npm ci` has no story for yanked patches
- 🔵 **Portability**: TCP loopback may be intercepted by corporate dev-machine firewalls (low confidence)
- 🔵 **Safety**: `npm ci --ignore-scripts` skips ALL transitive postinstalls — not documented
- 🔵 **Safety**: Eval id 3 retirement strategy depends on framework feature not yet verified
- 🔵 **Safety**: Stale-file rm-then-relaunch ordering ambiguous on lock acquisition
- 🔵 **Safety**: `pngjs` devDependency adds transitive supply-chain surface
- 🔵 **Compatibility**: Three-script `allowed-tools` surface vs existing wildcard convention
- 🔵 **Compatibility**: `pngjs` caret pin asymmetric with playwright tilde pin
- 🔵 **Compatibility**: Semver convention check left to discretion without project-history confirmation

### Recommended Changes (delta — for round-3 edit pass)

Ordered by impact, then by how many lenses each retires.

1. **Wall-clock re-arm semantics** (retires: Correctness major, Safety major, Architecture minor) — re-arm `setTimeout` at the start of every `navigate` op; add test for two consecutive `navigate` calls separated by > wall-clock-ms each having their own bound; document per-`navigate` semantics.

2. **Fix `run.sh` pseudocode** (retires: Portability major × 2, Code Quality minor) — replace the non-existent `find-repo-root.sh` reference with `source "$PLUGIN_ROOT/scripts/vcs-common.sh"; PROJECT_ROOT="$(find_repo_root)"`; replace bare `TMP_REL=…` with `"$PLUGIN_ROOT/scripts/config-read-path.sh" tmp .accelerator/tmp` to honour user config overrides.

3. **Specify `ACCELERATOR_INVENTORY_OUTPUT_ROOT` plumbing** (retires: Security major, Correctness minor) — add Phase 4 §3 bullet: SKILL.md exports the env var before each browser-agent Task call; `lib/path-guard.js` fail-closes on unset/empty value; add test asserting `run.sh screenshot` exits non-zero when env var unset.

4. **Pin visualiser-helper sourcing** (retires: Code Quality major, Portability major, Compatibility minor) — for each helper explicitly state: source-from-visualiser via documented path, or extract-to-shared-location. Drop "copy / source" hand-wave. Add a structural test that the path resolves.

5. **`notify-downgrade.sh` testing + canonical message source** (retires: Code Quality major, Safety minor, Usability minor) — add `test-notify-downgrade.sh` covering each `--reason` variant; define canonical message strings in one location; pair with `evals/fixtures/notify-downgrade.expected.txt` goldenfile; eval id 20 matches byte-equality on the fixture; pin `--reason` to a closed enum.

6. **Strengthen `ping` to verify Chromium binary integrity** (retires: Correctness major) — `ping` should `fs.statSync(chromium.executablePath())` so partial bootstraps surface at Step 5 rather than at first `navigate`.

7. **Reconcile internal-host-on-http error wording with SKILL.md prose** (retires: Usability major) — pick one and update the other two: either require both flags for internal-http (and update error to name both), or treat `--allow-internal` as subsuming for internal hosts.

8. **Add per-module / per-subcommand / per-rejection tests** (retires: Test Coverage major × 3, Compatibility major) — `lib/*.test.js` for each pure module; `test-validate-source.sh` exercising helpers via BASH_SOURCE; per-subcommand protocol-version round-trip + mismatch test; per-rejection-case path-guard test.

9. **Drop `--no-audit` or surface audit summary** (retires: Security major, Safety major) — drop `--no-audit` from `npm ci` (audit warnings appear once at first run; `npm ci` short-circuits subsequently), or add `npm audit --omit=dev --audit-level=high || true` after `npm ci` with a one-line warning summary; create a tracked follow-up ticket NOW.

10. **Cache namespace by lockfile hash** (retires: Safety major) — `~/.cache/accelerator/playwright/<sha8>/` keyed on `sha256(package-lock.json)[:8]`. Old version directories age out naturally.

11. **Enforce squash-merge or split into two PRs** (retires: Safety major) — pin GitHub branch-protection rule to squash-merge AND document in PR description, or split 4a and 4b into separate PRs (more robust; recommended).

12. **Document Phase 3→2 revert dependency** (retires: Safety major) — add explicit Implementation Approach note; consider `--skip-real-install` env var for test-run.sh.

13. **Surface error-envelope schema externally** (retires: Compatibility major) — Phase 5 deliverable: `skills/design/inventory-design/scripts/playwright/PROTOCOL.md` (or README section) listing envelope shape, category enum, kebab error codes per subcommand.

14. **Loosen the `evaluate-payload-rejected` grep assertion** (retires: Compatibility major) — reframe to assert deny-list pattern strings absent from executor source, leaving the kebab-code namespace open. Or drop entirely.

15. **Pin agent prose ordering for 4a transitional state** (retires: Compatibility major) — structural assertion or eval that 4a agent body places `run.sh` instructions before MCP fallback prose.

### Assessment

The plan now has a sound structural foundation. **None of the new majors require redesign** — they are all addressable with targeted edits, mostly in existing sections. A round-3 edit pass focused on the 15 recommended changes would likely close all 22 majors and most minors.

The verdict is REVISE on mechanical grounds (≥ 3 majors), but the right framing is: **"the architecture is right; tighten the implementation details before opening the PR."** Many of the recommended changes can be implemented in parallel; estimated effort is single-digit hours rather than another full-revision pass.

Particular attention is warranted for the **wall-clock re-arm** finding — it silently undermines the safety teeth this revision was specifically structured to land. Fixing it is a one-line code change but would otherwise fail manual testing in production use.

## Re-Review (Pass 3) — 2026-05-06T20:14:12Z

**Verdict:** REVISE

Round 3 retires the bulk of round-2's findings via concrete, testable design changes. Of the 22 round-2 majors, most are now resolved. What remains is a final layer of edge-case hygiene — primarily around the wall-clock not applying to non-`navigate` ops, the 30-day stale-sweep being too aggressive, cross-platform parsing fragility (`sha256sum`, `ps -o lstart=`, `find -mtime`), and several unchecked failure paths. None requires structural redesign; most are 1–2 line code changes or short prose additions.

### Headline

| | Round 1 | Round 2 | Round 3 |
|--|--|--|--|
| Critical | 8 | 0 | **0** |
| Major | 27 | 22 | ~24 |
| Minor | 26 | 31 | ~30 |

The major count looks similar between round 2 and round 3, but the *character* of the findings has shifted: round-2 majors were structural (PPID session id, big-bang switchover, missing path-guard); round-3 majors are about narrow correctness/safety edge cases that emerge once the structure is right.

### Cross-Cutting Themes (round 3)

The 24 new majors cluster around six themes:

1. **Wall-clock per-`navigate` only — `wait_for` and other ops unbounded** (Correctness + Safety, both major) — the wall-clock timer arms only on `navigate`; `wait_for` accepts caller-supplied `timeout_ms` with no daemon-side cap, and `evaluate`/`click`/`type` against a hung page after a `navigate` runs to whatever timeout the agent picks. The runaway-page safety net protects only one of eight ops. **Fix**: extend re-arm to all ops or wrap each op in `Promise.race([op, killTimer])`.

2. **30-day stale-namespace sweep is too aggressive** (Correctness + Safety + Portability, all major) — silently deletes user data based on `mtime`; pinned-older-version users lose their cache; clock skew (NTP backwards adjust, dead battery) can wipe everything; BSD vs GNU `find -mtime` differences; can race against in-use namespaces. **Fix**: use `atime` not `mtime`; gate behind explicit signal or notification; raise threshold to 90 days.

3. **`ps -p -o lstart=` (macOS identity check) is locale-dependent** (Correctness + Portability, both major) — bash and JS implementations can silently disagree under non-C locales (German, French, etc.); cross-validation fixture passes in CI's `LANG=C` but diverges on real localised dev boxes. **Fix**: force `LC_ALL=C` (or `LC_TIME=C`) around `ps` and `date -j -f` calls in both implementations.

4. **Top-level sentinel pointer can dangle** (Architecture + Correctness + Safety, all major) — multi-version cache state has a race window; if pointer references a namespace removed by stale-sweep or a different lockhash, `run.sh` falls back silently to wrong/empty namespace, producing `MODULE_NOT_FOUND` rather than a clear bootstrap error. **Fix**: have `run.sh` compute lockhash directly from skill-shipped `package-lock.json` (deterministic, no pointer needed); top-level pointer becomes informational only.

5. **`find_repo_root` failure path unchecked** (Correctness + Safety, both major) — empty `PROJECT_ROOT` under `set -euo pipefail` doesn't trigger via command-substitution; `STATE_DIR` becomes absolute path under `/`. **Fix**: explicit `[[ -n "$PROJECT_ROOT" ]] || exit 2` after the call, with a clear message naming the precondition.

6. **lib/* per-module test asymmetry** (Test Coverage + Code Quality, both major) — `lock.js`, `daemon.js`, `client.js`, `auth-header.js` have no dedicated unit tests; only the five "pure" modules got per-module coverage. `auth-header.js` is particularly notable given the round-2 origin-allowlist concern. **Fix**: add minimal `*.test.js` for the four omitted modules, especially `auth-header.test.js` for the URL.origin match cases.

### Resolution of Round-2 Findings

Of round-2's 22 majors:
- **18 fully resolved** (wall-clock re-arm — but see new finding about non-navigate ops; `find_repo_root` reference fixed; `TMP_REL` config indirection invoked; visualiser-helper sourcing pinned; mkdir-fallback automated; `ACCELERATOR_INVENTORY_OUTPUT_ROOT` plumbing; `--no-audit` dropped; per-module tests added for 5 modules; per-subcommand protocol tests; per-rejection screenshot tests; `notify-downgrade.sh` test + goldenfile + closed-enum; `ping` strengthened; internal-host wording reconciled; cache lockhash namespacing; PROTOCOL.md surfaces error envelope; loosened deny-list grep; agent prose ordering pinned; revert dependency note added)
- **3 partially resolved** (audit signal — non-failing summary still has visibility gaps; PR 4b dependency stated but not enforceable; eval id 3 retirement still conditional)
- **1 still present** (pngjs caret pin asymmetry)

Of round-2's 31 minors: ~22 fully resolved, ~9 still present or partially resolved.

### New Issues Introduced (round 3)

#### Major (~24)

- 🟡 **Correctness + Safety**: Wall-clock bound only re-arms on `navigate`; `wait_for` (and other ops) run unbounded
- 🟡 **Correctness + Safety + Portability**: 30-day stale-namespace sweep silently deletes user data based on `mtime`; clock skew + pinned-older-version users at risk
- 🟡 **Correctness + Portability**: `ps -p -o lstart=` macOS identity-check is locale-dependent; bash/JS cross-validation can silently drift
- 🟡 **Architecture + Correctness + Safety**: Top-level sentinel pointer can dangle when stale-sweep removes the pointed-at namespace
- 🟡 **Correctness + Safety**: `find_repo_root` failure case (CWD outside any repo) not handled — silent empty `PROJECT_ROOT` or fail-fast unspecified
- 🟡 **Test Coverage + Code Quality**: lib/lock.js, daemon.js, client.js, auth-header.js have no dedicated `*.test.js`
- 🟡 **Portability**: `sha256sum` runtime dispatch missing — pseudocode comment notes it but real code path will fail on macOS
- 🟡 **Portability**: BSD vs GNU `find -mtime +30` semantics + clock-skew can wipe recent namespaces
- 🟡 **Architecture**: Cross-skill direct sourcing of `launcher-helpers.sh` creates hard coupling design→visualisation
- 🟡 **Test Coverage**: Lockhash-namespacing fixtures' source/regeneration unspecified (`abc12345`/`def67890` prefixes not actually controllable)
- 🟡 **Test Coverage**: Goldenfile fixture regeneration recipe absent — drift between `NOTIFY_DOWNGRADE_MESSAGES` and fixtures has no fast-path repair
- 🟡 **Code Quality**: Three-file lockhash sentinel topology creates multi-file consistency hazard (per-namespace sentinel, top-level pointer, bootstrap.lock)
- 🟡 **Code Quality**: Sourceable bash associative array (`notify-downgrade-messages.sh`) is a heavyweight format for i18n data; binds to bash 4+
- 🟡 **Code Quality**: Phase 2 §1 + §2 test enumeration approaching readability limits at ~50 bullets
- 🟡 **Correctness**: SKILL.md does not enumerate which detected condition maps to which `notify-downgrade.sh --reason` enum value
- 🟡 **Compatibility**: `PROTOCOL.md` location at `skills/design/inventory-design/scripts/playwright/PROTOCOL.md` is too deep for a downstream consumer to discover
- 🟡 **Compatibility**: "v1 does not filter `evaluate`" stability commitment locks shared-CI tightening behind a major bump
- 🟡 **Safety**: No documented policy for the window where 4a is merged but 4b stalls indefinitely
- 🟡 **Architecture**: PR 4b's dependency on PR 4a is documented but not enforceable from the plan

#### Minor (~30 — selected highlights)

- 🔵 **Security**: `npm audit` non-failing warning likely to be ignored as noise; recurring on every install for the same lockfile-pinned advisories
- 🔵 **Security**: Control-character sanitisation spec under-specified vs realistic injection vectors (NUL, bidi-override, C1 controls)
- 🔵 **Security**: Public "v1 does not filter evaluate" commitment may license downstream callers to skip their own validation
- 🔵 **Test Coverage**: Cross-validation fixture between bash and JS identity is described abstractly — fixture source and assertion shape under-pinned
- 🔵 **Test Coverage**: Wall-clock kill writes "url of the navigate that armed it" but contract not pinned in `server-stopped.json` schema definition
- 🔵 **Test Coverage**: Audit-summary mock covers reporting-advisory and clean paths but leaves audit-command-failure (network down, bad registry) untested
- 🔵 **Code Quality**: PROTOCOL.md inside `scripts/playwright/` is technically discoverable but architecturally low
- 🔵 **Correctness**: `--ignore-scripts` snapshot test only catches lockfile-postinstall-set growth, not behavioural need (native bindings)
- 🔵 **Correctness**: `disk-floor-not-met` and `cache-root-unwritable-vs-cache-unwritable` ambiguity in `notify-downgrade.sh` enum
- 🔵 **Usability**: Reset incantation removes all `<sha8>` caches indiscriminately; lacks per-version recipe
- 🔵 **Usability**: Silent `rm -rf` of >30-day cache dirs may surprise users
- 🔵 **Usability**: `npm audit reported advisories` warning fires on every install with no de-duplication
- 🔵 **Usability**: argument-hint is now five tokens wide and likely wraps on 80-column terminals (round-3 didn't address)
- 🔵 **Usability**: Wall-clock 5-minute kill is invisible to user when it fires (round-3 didn't address)
- 🔵 **Usability**: 30-minute idle daemon shutdown not surfaced in user-facing flow prose (round-3 didn't address)
- 🔵 **Usability**: `bootstrap-failed` enum value collapses several distinct failure modes into one message
- 🔵 **Portability**: `npm audit --omit=dev --audit-level=high` requires npm ≥ 8.3; no version floor declared
- 🔵 **Portability**: `OSTYPE` test handles `linux-gnu*` but not Alpine `linux-musl*` (Playwright Chromium is glibc-only)
- 🔵 **Portability**: `ACCELERATOR_LOCK_FORCE_MKDIR=1` only exercises mkdir branch in `ensure-playwright.sh`, not the daemon launcher
- 🔵 **Safety**: Bash/Node `start_time_of` agreement tested at code-review time only — no runtime monitoring
- 🔵 **Compatibility**: Eval id 3 retirement still conditional on unverified framework support
- 🔵 **Compatibility**: 4a state declares both MCP and Bash tools — verify marketplace tooling tolerates dual declaration
- 🔵 **Compatibility**: pngjs caret pin remains asymmetric with Playwright tilde pin
- 🔵 **Compatibility**: README cache-cleanup recipe predates lockhash namespacing
- 🔵 **Compatibility**: `run.sh` hard-couples to plugin's `scripts/` layout via `vcs-common.sh` sourcing (no structural test)

### Recommended Changes (delta — for round-4 edit pass, if pursued)

Ordered by impact / lenses retired. Many of these are 1–3 line additions to existing pseudocode.

1. **Extend wall-clock re-arm to all blocking ops** (Correctness major, Safety major) — re-arm on every protocol op that touches the page, OR wrap each op in `Promise.race([op, killTimer])`, OR cap `timeout_ms` for `wait_for` at the wall-clock budget. Add `wait_for` test against a never-satisfied condition.

2. **Fix `find_repo_root` failure handling** (Correctness major, Safety major) — `[[ -n "$PROJECT_ROOT" ]] || { echo 'must be run inside a repository' >&2; exit 2; }` immediately after the call. Add test for `cd /tmp && run.sh ping`.

3. **Force `LC_ALL=C` around `ps -o lstart=` and `date -j -f`** (Correctness major, Portability major) — both bash and JS sides. Add fixture test under `LANG=de_DE.UTF-8`.

4. **Fix `sha256sum` cross-platform dispatch** (Portability major) — `sha256_of() { command -v sha256sum >/dev/null 2>&1 && sha256sum "$1" || shasum -a 256 "$1"; } | cut -c1-8`. Add macOS-platform test.

5. **Make stale-sweep safer** (Correctness major, Safety major, Portability major) — use `atime` not `mtime`; gate behind notification or `ACCELERATOR_PLAYWRIGHT_SWEEP=1`; check delta against `date -u +%s` (handle clock-skew); raise threshold to 90 days; respect long-running invocations.

6. **Drop top-level pointer dependency in `run.sh`** (Architecture major, Correctness major, Safety major) — compute lockhash directly from skill-shipped `package-lock.json` in `run.sh`. Top-level pointer becomes informational only. Add test for stale top-level pointer recovery.

7. **Add per-module tests for the four omitted modules** (Test Coverage major, Code Quality major) — `auth-header.test.js` is highest-priority (origin matching pitfalls). `lock.test.js`, `client.test.js`, focused `daemon.test.js` cases for timer juggling. Or document explicitly why those four are integration-tested only.

8. **Specify `--reason` mapping in SKILL.md** (Correctness major) — `ensure-playwright.sh` writes `ACCELERATOR_DOWNGRADE_REASON=<enum>` to stderr on non-zero exit; SKILL.md Step 4 reads it and passes through. Add table in SKILL.md mapping detected condition → exit code → reason.

9. **Pin 4a→4b stall policy** (Safety major, Architecture minor) — explicit window (e.g. "open 4b within 2 working days; revert 4a if 4b stalls > 5 working days"). Add Phase 5 verification step asserting 4a/4b commits within N days.

10. **Soften PROTOCOL.md "v1 does not filter evaluate" commitment** (Compatibility major) — reframe as "v1's default behaviour does not filter; opt-in tightening via documented env-var is permitted within v1". Or accept the major-bump trigger and call out shared-CI as v2 milestone.

11. **Move PROTOCOL.md to a more discoverable location** (Compatibility major) — promote to `skills/design/inventory-design/PROTOCOL.md` (skill-level), or add explicit absolute-path cross-links from agent bodies and README.

12. **Convert `notify-downgrade-messages.sh` to JSON** (Code Quality major) — remove bash 4+ dependency; downstream consumers can read with `jq`; goldenfiles trivially regeneratable.

13. **Pin lockhash-namespacing test fixture strategy** (Test Coverage major) — commit two synthetic `package-lock.json` files OR add `ACCELERATOR_PLAYWRIGHT_LOCKHASH_OVERRIDE` env hook. Document fixture refresh recipe.

14. **Add goldenfile regeneration recipe + key-equality assertion** (Test Coverage major) — `notify-downgrade.sh --regenerate-fixtures` mode; structural test asserting `keys(NOTIFY_DOWNGRADE_MESSAGES) == ls fixtures/`.

15. **Restructure Phase 2 §1+§2 as tables** (Code Quality major) — per-module table for unit, per-op table for integration. Makes coverage gaps visually obvious.

### Assessment

The plan is now substantively close to ready. The architecture is sound; the implementation specification is detailed; the testing strategy is comprehensive. What remains is a layer of edge-case hygiene that an implementer could either address in a small round-4 edit pass or accept as known limitations and address during implementation as code review surfaces them.

**Trade-off recommendation**: Toby has three reasonable paths from here:

- **Round-4 edit pass** (estimated 2–4 hours): apply the 15 recommended changes. Most are 1–3 line additions. Likely closes nearly all round-3 majors and most minors. Plan would be ready for implementation with high confidence.
- **Accept and implement** (estimated 0 hours of plan work): start implementation; address each round-3 finding as it surfaces in code review for the actual PR. Trade-off: more findings will surface during implementation rather than during planning, but most are bounded.
- **Cherry-pick high-impact fixes** (estimated 1 hour): apply the top 4–5 recommended changes (wall-clock for all ops, `find_repo_root` failure check, `LC_ALL=C` for identity, `sha256sum` dispatch, stale-sweep safety) and accept the rest as implementation-time findings.

The wall-clock-only-on-`navigate` and the stale-sweep `mtime` issues are the highest-priority of the remaining majors — both are functional safety regressions waiting to happen in production. The rest are mostly polish.

## Re-Review (Pass 4) — 2026-05-06T20:33:40Z

**Verdict:** REVISE

Round 4 retired the highest-priority round-3 findings (wall-clock-on-navigate-only, stale-sweep-on-mtime, find_repo_root failure unchecked, top-level-pointer dangle, ps-locale dependence, sha256sum dispatch, lib/* test asymmetry, JSON message file, PROTOCOL.md location, softened v1 commitment, 4a/4b stall policy, lockhash test fixtures, goldenfile regen recipe). The plan's architecture is now sound and the implementation specification is concrete enough to build from.

The new findings cluster into a final layer of integration-detail concerns. **None block implementation.** A handful are worth addressing before the PR opens; the rest can plausibly be triaged at implementation review time.

### Headline counts

| | Round 1 | Round 2 | Round 3 | Round 4 |
|--|--|--|--|--|
| Critical | 8 | 0 | 0 | **0** |
| Major | 27 | 22 | ~24 | ~21 |
| Minor | 26 | 31 | ~30 | ~38 |

Major count is finally trending down. Round-4 majors are narrower in scope and more specific than round-3's; many are documentation/test gaps rather than design issues.

### Cross-Cutting Themes (round 4)

1. **`jq` is now a hard runtime dependency but undeclared** (Compatibility + Portability major) — `notify-downgrade.sh` and `ensure-playwright.sh` both rely on `jq`; macOS doesn't ship it; minimal Linux containers don't either. No preflight check, no README requirement, no version floor.

2. **`wait_for` wall-clock cap silent truncation** (Correctness + Safety + Usability major) — legitimately-long polls (10-min build artifact wait, payment confirmation) get cut off at 5 min with `wall-clock` reason, indistinguishable from a hang.

3. **Wall-clock kill via `process.exit(2)` breaks error-envelope contract** (Correctness major) — client receives `ECONNRESET` instead of structured envelope for the operationally most important error case.

4. **Phase 2 §1+§2 readability regressed** (Code Quality + Test Coverage major) — round-4 added 4 module test files plus lifecycle cases without restructuring; section is now ~250 lines of nested bullets across 9 test files.

5. **Detected-condition mapping table duplicated** (Code Quality + Compatibility major) — same table in PROTOCOL.md and Phase 4 §3 SKILL.md prose; drift inevitable.

6. **Stale-sweep opt-in trades data-loss for unbounded growth** (Safety major + Usability minor) — default-off means caches accumulate forever for users who never read the README.

7. **4a/4b stall policy is post-hoc** (Architecture + Safety + Compatibility major) — Phase 5 `git log` check is a coroner not a paramedic; auto-revert is also visible to downstream marketplace consumers.

8. **`sha256_of` and detection-mapping duplication** (Architecture + Code Quality) — `sha256_of` lives in both `run.sh` and `ensure-playwright.sh`; should extract or assert byte-equality.

### Resolution of Round-3 Findings

Of round-3's ~24 majors, **17 fully resolved** (wall-clock for all ops, find_repo_root fail-fast, LC_ALL=C, top-level pointer dropped from run.sh, sha256sum dispatch, four lib/* tests added, JSON messages, PROTOCOL.md location, softened v1 commitment, 4a/4b stall policy, lockhash fixtures, goldenfile regen recipe + key-equality, disk-floor-not-met enum, audit summary surfaced, control-character sanitisation pinned, mkdir-fallback automated, identity locale fragility test added).

**4 partially resolved** (Phase 2 §1+§2 readability worsened not improved; PR 4b enforceability still procedural; opt-in stale-sweep introduces inverse risk; top-level pointer orphaned).

**3 still present** (eval id 3 retirement still conditional, pngjs caret pin asymmetric, vcs-common.sh structural test absent).

Of round-3's ~30 minors, ~20 resolved; ~10 remain.

### New Issues Introduced (round 4)

#### Major (~21)

- 🟡 **Compatibility + Portability**: `jq` now a hard runtime requirement, undeclared
- 🟡 **Correctness + Safety + Usability**: `wait_for` wall-clock cap silent truncation
- 🟡 **Correctness**: Wall-clock kill via `process.exit(2)` breaks error-envelope contract (ECONNRESET vs structured)
- 🟡 **Correctness**: `navigate` `waitUntil` strategy unspecified; default Playwright timeout interaction with wall-clock
- 🟡 **Code Quality + Test Coverage**: Phase 2 §1+§2 readability regression (~250-line bullet wall)
- 🟡 **Code Quality + Compatibility**: Detected-condition mapping table duplicated in two places
- 🟡 **Architecture**: `sha256_of` algorithm duplicated across `run.sh` and `ensure-playwright.sh`
- 🟡 **Architecture + Safety + Compatibility**: 4a/4b stall policy enforcement is post-hoc
- 🟡 **Architecture**: Softened v1 evaluate commitment introduces env-var-gated behaviour fork
- 🟡 **Test Coverage**: `find_repo_root` failure test promised but missing from `test-run.sh`
- 🟡 **Test Coverage**: Sweep in-use protection covers only one race timing
- 🟡 **Test Coverage**: Lockhash-namespacing fixtures pinned but provenance still under-documented
- 🟡 **Safety**: Opt-in stale-sweep means default is unbounded cache growth
- 🟡 **Usability**: Rendering convention "recommended" but Phase 4 prose still says "surface stderr JSON"
- 🟡 **Usability**: `bootstrap-failed` reason collapses npm vs Chromium failures with same generic message
- 🟡 **Compatibility**: 4a auto-revert at 5 days affects downstream marketplace consumers (no version-bump signal)
- 🟡 **Portability**: GNU-vs-BSD `date` dispatch for stale-sweep documented in pseudocode comments only — not a real helper
- 🟡 **Portability**: `OSTYPE` `linux*` glob admits Alpine/musl where Chromium fails at runtime
- 🟡 **Portability**: `run.sh` fails ungracefully if `package-lock.json` is missing (sparse checkout, partial install)

#### Minor (~38 — selected highlights)

- 🔵 **Security**: Byte-level ASCII strip will mangle UTF-8 if any non-ASCII enters messages
- 🔵 **Security**: Opt-in security env-vars need documented propagation + fail-closed contract
- 🔵 **Security**: JSON parse failure must fail-closed, not emit empty downgrade notice
- 🔵 **Architecture**: Top-level pointer is now an architectural orphan (no production reader)
- 🔵 **Code Quality**: PROTOCOL.md stability commitment is wordy — leads with hedges before headline
- 🔵 **Code Quality**: `notify-downgrade.sh` implementation contract specified in prose, not pseudocode
- 🔵 **Correctness**: `--ignore-scripts` post-install smoke test still missing
- 🔵 **Correctness**: Sentinel lockhash comparison structurally redundant; mismatch behaviour unspecified
- 🔵 **Test Coverage**: Cross-validation fixture provenance under-documented
- 🔵 **Test Coverage**: `wait_for` wall-clock case conflates two assertions
- 🔵 **Test Coverage**: `client.test.js` misses PID-reuse-with-mismatch case
- 🔵 **Usability**: Opt-in cache growth needs in-band nudge
- 🔵 **Usability**: Argument-hint at five tokens still wraps (still present)
- 🔵 **Usability**: Wall-clock kill user-visible signal (still present)
- 🔵 **Usability**: 30-min idle daemon visibility (still present)
- 🔵 **Usability**: `npm audit` warning recurs every install (still present)
- 🔵 **Usability**: `run.sh ping`/`daemon-status` discoverability (still present)
- 🔵 **Portability**: `npm audit --audit-level=high` requires npm ≥ 8.3 (no version floor)
- 🔵 **Portability**: `ACCELERATOR_LOCK_FORCE_MKDIR=1` only exercises ensure-playwright.sh, not daemon launcher
- 🔵 **Safety**: Compound clock-skew (forward then backward) escapes negative-delta guard
- 🔵 **Compatibility**: README cache-cleanup recipe still nukes everything indiscriminately
- 🔵 **Compatibility**: Eval id 3 retirement still conditional (still present)
- 🔵 **Compatibility**: 4a dual-tool marketplace metadata tolerance unverified (still present)
- 🔵 **Compatibility**: pngjs caret pin asymmetric (still present)
- 🔵 **Compatibility**: vcs-common.sh structural test absent (still present)

### Recommendation

The plan has been through four review rounds. Each round retired the previous round's highest-impact findings, and round 4 left the architecture sound and the implementation specification concrete. The remaining issues are integration-detail concerns rather than structural problems.

**Diminishing returns are real.** Continuing to round 5 would close ~21 majors but introduce another ~15 — that's been the pattern across rounds 2, 3, and 4. At some point implementation discovers the rest, more efficiently than additional review cycles.

Three concrete options:

1. **One final targeted edit pass (~30–60 min)** — fix only the 6–7 highest-impact round-4 findings:
   - Add `jq` preflight check + README requirement (Compatibility + Portability major)
   - Fix `wait_for` cap silent truncation: surface `truncated: true` field and document escape hatch (Correctness + Safety + Usability major)
   - Fix wall-clock-kill envelope: write structured error before `process.exit(2)` (Correctness major)
   - Pin `navigate` `waitUntil: 'domcontentloaded'` in PROTOCOL.md (Correctness major)
   - Add real GNU/BSD `date` dispatch helper for stale-sweep (Portability major)
   - Add `find_repo_root` failure test to `test-run.sh` (Test Coverage major)
   - Move detected-condition table to one canonical location (Code Quality + Compatibility major)

2. **Accept and implement** (recommended): the architecture is right; remaining concerns are bounded; implementation review will surface them efficiently. Open the implementation PRs (4a then 4b) and address each finding as it arises in code review against actual code rather than against pseudocode.

3. **Cherry-pick only the two safety-critical** (~15 min): fix the `wait_for` cap (silent truncation is operationally bad) and the wall-clock-kill envelope (breaks the contract for the most important error case). Defer the rest.

### Assessment

The plan is implementation-ready. All eight original critical findings remain resolved across all rounds. The architecture has been validated against nine independent quality lenses four times. Further plan-iteration cycles will continue to find legitimate issues but at decreasing marginal benefit per hour invested.

**Recommendation: option 2 (accept and implement) or option 3 (one targeted 15-minute pass on the two safety-critical items).** The `wait_for` cap silent truncation and the envelope-breaking exit are the only remaining items where round-4 findings would change user-observable behaviour in a way that's hard to fix retroactively. Everything else can be code-review fodder.
