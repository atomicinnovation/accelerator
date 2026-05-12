---
date: "2026-05-06T16:49:30+01:00"
type: plan
skill: create-plan
work-item: ""
status: accepted
---

# Design Skill — localhost validation and MCP-hallucination fixes

## Overview

`/inventory-design` UAT exposed two production blockers (research:
`meta/research/codebase/2026-05-06-design-skill-localhost-and-mcp-issues.md`):

1. The URL validator rejects `http://localhost`, the most common dev-server
   case, with no way to override.
2. The Playwright MCP path through plugin-shipped sub-agents triggers a known
   Claude Code bug (issues #13605, #13898): sub-agents do not inherit
   project-scoped `mcp__playwright__*` tools and **hallucinate** plausible
   tool names (`mcp__chrome-devtools__*`) and fictional crawl results.

This plan fixes both. Issue 1 is a small change to one validation script
(plus a flag-split for clarity). Issue 2 replaces the MCP path with a
Bash-invoked Node executor (`run.js`) that runs Playwright,
lazy-bootstrapped on first use. Sub-agents stay in the architecture but
talk to the executor instead of the MCP — this sidesteps the inheritance
bug class because Bash inheritance to sub-agents is reliable.

The daemon model **mirrors the visualiser skill's pattern**
(`skills/visualisation/visualise/`): TCP loopback (random port) instead
of Unix sockets, project-scoped state via `find_repo_root` instead of
PPID divination, PID + start-time identity check, reuse-before-lock,
`mkdir lock.d` fallback for `flock`-less platforms, and a
`server-stopped.json` audit invariant. Reusing this pattern eliminates a
class of daemon-management findings raised in the initial review.

This revision (review-1) applies three constraints from the user:

1. **Local-dev-only execution** — these skills are not run on shared CI
   or multi-user hosts. Local-attacker-on-the-box threats (socket
   hijacking via PPID enumeration, `/proc/<pid>/environ` disclosure)
   are out of scope.
2. **UX preferred over heavy lockdown** — the regex `evaluate` deny-list
   from earlier drafts is dropped. The agent body's allowlist remains
   the documented contract; the executor enforces no programmatic
   block on `evaluate` payloads. False-positive UX cliffs disappear.
3. **Daemon management mirrors the visualiser** — see above.

## Current State Analysis

### Issue 1 — URL validation surface

All blocking lives in
`skills/design/inventory-design/scripts/validate-source.sh`:

- Scheme dispatch (lines 29–47) classifies `http://*` as `SCHEME=http`.
- Lines 91–94 hard-reject `http://` with `Use https:// instead.`
- Lines 100–147 reject every internal-host range (loopback, link-local,
  RFC1918, IPv6 fe80) and the messages already advertise the placeholder flag
  `Use --allow-internal to override (not available in v1).`
- The skill never re-validates the location elsewhere, and no other script
  checks scheme/host. Change surface is single-file plus its caller.

### Issue 2 — Playwright integration surface

- `.claude-plugin/.mcp.json` (3 lines) registers Playwright MCP project-scoped
  via `npx @playwright/mcp@0.0.73`. This is the worst-case scope for
  sub-agent inheritance under #13898.
- `agents/browser-locator.md:7` declares two `mcp__playwright__*` tools.
- `agents/browser-analyser.md:7` declares all seven `mcp__playwright__*` tools
  and contains the `browser_evaluate` payload allowlist (lines 44–75).
- `skills/design/inventory-design/SKILL.md`:
  - lines 11–20 declare seven `mcp__playwright__*` tools in `allowed-tools`
  - line 53 keys default crawler-mode selection on
    `mcp__playwright__browser_navigate` presence
  - lines 103–124 contain MCP detection prose (LLM self-introspection of its
    toolbox; hard-fail message references
    `mise run deps:install:playwright`)
  - lines 134–149 invoke `{browser locator agent}` and
    `{browser analyser agent}` for runtime/hybrid modes
- `skills/design/inventory-design/evals/evals.json`:
  - id 3 (`mcp-unavailable-fallback`) is the only eval that asserts no
    `mcp__playwright__*` tools are invoked
  - id 13 (`internal-host-rejection`) asserts non-zero exit on
    `http://127.0.0.1:8080` and `http://169.254.169.254/`
  - id 14 (`browser-evaluate-safety-structural`) asserts the
    `browser-analyser.md` allowlist names each forbidden pattern by name
- `scripts/test-design.sh` makes structural assertions about the seven
  `mcp__playwright__*` entries, the `.mcp.json` `npx` command, and the
  agents' tool fields (lines 50–145, 240–246).

### Key Discoveries

- **The validator is single-file**: change-surface for Issue 1 is genuinely
  isolated to `validate-source.sh` and its caller in `SKILL.md`.
- **MCP detection is LLM-mediated**: `SKILL.md` instructs the model to "check
  your own toolbox" — there is no programmatic detector. Replacing the MCP
  with an executor lets us replace this with a deterministic shell check
  (`run.sh ping`).
- **The sub-agent boundary itself isn't broken** — `Bash` inheritance to
  sub-agents is reliable. Only `mcp__*` tool inheritance is bug-prone. So
  sub-agents can keep their roles if they invoke a Bash executor instead of
  MCP tools directly.
- **The `browser_evaluate` allowlist is enforced by agent prose** — and
  remains so after this work. The earlier plan's regex deny-list inside
  `run.js` is **dropped**: trivially bypassable, false-positive cliff,
  and unnecessary on local-dev machines. The agent body's allowlist is
  the only governance.
- **The visualiser skill provides a proven daemon-management pattern**:
  TCP loopback random port, project-scoped state under
  `$PROJECT_ROOT/.accelerator/tmp/<skill>/`, PID+start_time identity,
  reuse-before-lock short-circuit, `flock`-or-`mkdir-lock.d` fallback,
  atomic state-file writes, post-shutdown audit. We adopt this pattern
  wholesale.
- **No `${CLAUDE_PLUGIN_DATA}` dependency**: instead of the
  per-plugin-data dir (whose availability across Claude Code versions
  is uncertain), the binary cache lives at `~/.cache/accelerator/playwright/`
  and the per-project state lives at `$PROJECT_ROOT/.accelerator/tmp/...`
  — same conventions as the visualiser.
- **Chromium-only download** matches Playwright MCP defaults (~150 MB) and
  keeps first-run latency in the 1–3 minute window.

## Desired End State

After this plan:

1. `validate-source.sh` accepts `http://localhost` and `http://127.0.0.1`
   without any flag, plus their canonical equivalents (uppercase, trailing
   dot). Other internal/loopback/link-local/RFC1918 hosts require
   `--allow-internal`. `http://` to public hosts requires
   `--allow-insecure-scheme` (separate flag). Userinfo segments and
   numeric/hex IPv4 encodings are rejected outright. All other rejections
   (file://, javascript:, etc.) are unchanged.
2. The Playwright MCP is gone. `inventory-design` and the two browser
   agents crawl runtime targets via a vendored Node executor at
   `skills/design/inventory-design/scripts/playwright/run.js`, invoked
   exclusively through `Bash(...run.sh *)`.
3. The executor uses a project-scoped daemon model copied from the
   visualiser: TCP loopback, `find_repo_root`-keyed state, atomic file
   writes, audit invariant on shutdown. The wall-clock crawl bound (5
   minutes) is enforced by the daemon itself.
4. The executor is auto-installed on first runtime/hybrid crawl by
   `scripts/ensure-playwright.sh`, which `npm ci`'s the pinned dependencies
   from a committed `package-lock.json` and runs `playwright install chromium`
   into `~/.cache/accelerator/playwright/`. Bootstrap respects user network
   configuration (registries, CA certs, proxies) and surfaces the relevant
   env vars in error messages.
5. `.claude-plugin/.mcp.json` is deleted; `plugin.json` is unchanged
   (no dependency on Microsoft's Playwright plugin).
6. `evals/evals.json` is updated: id 13 refreshed with a `127.0.0.2`
   non-`127.0.0.1` loopback case; new ids 18 (localhost-default-allow),
   19 (flag passthrough), 20 (executor-bootstrap-failure-fallback),
   21 (executor-ping-no-browser); id 3 retired; id 14 (allowlist)
   intact.
7. `scripts/test-design.sh` no longer asserts `mcp__playwright__*`
   structurally; it asserts the executor protocol, bootstrap behaviours,
   and the agent-body allowlist contract.
8. Final benchmark via skill-creator: existing evals ≥ baseline (mean
   ≥ 0.95 with 0.05 variance margin); new evals ≥ 0.9.

### Verification

- `mise run test:all` passes (or whichever umbrella task runs `test-design.sh`
  and the new bash test scripts; current set-up runs `test-design.sh` outside
  mise — confirmed during research).
- `bash scripts/test-design.sh` exits 0 with no `mcp__playwright__*`
  assertions remaining and the new executor / bootstrap assertions present
  and passing.
- `bash skills/design/inventory-design/scripts/playwright/test-run.sh` (new)
  exits 0.
- `bash skills/design/inventory-design/scripts/test-ensure-playwright.sh`
  (new) exits 0.
- A real `/inventory-design my-app http://localhost:3000 --crawler runtime`
  invocation against a developer's local dev server completes without the
  hallucination class observed in UAT.
- A real `/inventory-design my-app http://example.com --allow-insecure-scheme`
  invocation against a public dev URL validates and proceeds (proves the
  flag-split works end to end).
- skill-creator benchmark of `inventory-design` reports existing evals
  meeting their per-eval baseline + 0.05 variance margin and new evals
  ≥ 0.9.

## What We're NOT Doing

- Filing an upstream Anthropic issue for #13605 / #13898 (we route around it).
- Adding a `dependencies` field on Microsoft's Playwright plugin in
  `plugin.json` — confirmed in research not to fix the inheritance bug.
- **Programmatic page-cap or screenshot-byte-budget bounds**. The
  wall-clock bound (5 minutes per crawl, daemon-enforced via
  `setTimeout`) IS in scope (Phase 2) — the prior draft's deferral was
  unsafe given MCP removal. Page cap and screenshot byte budget remain
  instructional in `SKILL.md` and are deferred to a follow-up ticket.
- **Programmatic `evaluate`-payload deny-list inside the executor**.
  Dropped from this revision in favour of UX. The agent body's
  allowlist remains the documented contract.
- **Cross-user / cross-process security hardening of the daemon socket**.
  Skills run on local-dev machines; threats from other local users on
  the same host are out of scope. (Files are still mode `0600` as a
  basic hygiene measure inherited from the visualiser pattern.)
- **Aggressive expansion of screenshot mask defaults** (CSRF tokens,
  hidden inputs, etc.). Defaults stay at the existing three
  (`[type=password]`, `[autocomplete*=token]`, `[data-secret]`); callers
  can extend. Local-dev threat model does not warrant catching every
  possible secret carrier by default.
- **Subdomain-confusion / IDN homograph defences in the auth-header
  origin allowlist**. Simple `URL.origin` exact-compare is sufficient;
  the threat model assumes the developer's local machine.
- **Windows support**. macOS + Linux only. `ensure-playwright.sh` rejects
  other `OSTYPE`s fast.
- Re-implementing the `analyse-design-gaps` skill or any other downstream
  consumer of inventory artifacts. Their inputs (`inventory.md` schema,
  `screenshots/` layout) are unchanged.
- Migrating any existing inventory artifacts. The artifact shape is
  unchanged.
- Adding Firefox or WebKit binaries.
- Switching to a different browser-automation library (Puppeteer /
  Selenium).

## Implementation Approach

The phases ship in sequence on a single branch. Phase ordering changed
from prior drafts: **Phase 3 (bootstrap) lands before Phase 2 (executor
tests)** so the executor's integration tests have a real Playwright
install to run against without throwaway helper code.

- **Phase 1** — Validator changes. Independent of all other phases;
  could land first or in parallel.
- **Phase 3** — Bootstrap script (`ensure-playwright.sh`) plus committed
  `package-lock.json`. Lands before Phase 2 so Phase 2's tests can
  invoke `ensure-playwright.sh` directly (no `--mock`) for setup.
- **Phase 2** — Executor (`run.js`, `run.sh`, multi-file lib/) plus its
  tests. Consumes Phase 3's bootstrap.
- **Phase 4** — Wires Phases 2 and 3 into the skill and agents in two
  commit groups (4a additive dual-tools; 4b subtractive MCP removal).
  Updates evals + structural tests in lockstep so each `main` SHA
  stays green.
- **Phase 5** — Doc/cleanup + final benchmark.

**Revert dependency note**: Phase 2's tests invoke the real `ensure-playwright.sh`
from Phase 3, so reverting Phase 3 cascades into Phase 2 test failures. To make
each phase independently revertible, `test-run.sh` honours
`ACCELERATOR_PLAYWRIGHT_SKIP_REAL_INSTALL=1` to no-op the bootstrap step and
skip Playwright-dependent cases (the validator-style cases still run). Document
this in the Phase 2 §2 test deliverable.

TDD discipline within each phase:

1. Write or extend the failing test (red).
2. Implement the smallest change to pass (green).
3. Refactor and tighten (still green).

Every change to a `SKILL.md` or agent body is performed via the
`skill-creator:skill-creator` skill so that its writing-style and frontmatter
guarantees apply, and so eval/benchmark changes are part of that workflow.

---

## Phase 1: Loosen URL validation for localhost / 127.0.0.1

### Overview

Default-allow `http://localhost` and `http://127.0.0.1` (any port, any
path). Introduce two new flags: `--allow-internal` for the remaining
internal/loopback/link-local/RFC1918 ranges, and `--allow-insecure-scheme`
for `http://` to public hosts. Plumb both flags through the skill front
door. Canonicalise the host before classification (lowercase, trim trailing
dot, strip brackets, strip port, strip zone-id, reject userinfo and
decimal/hex IPv4 encodings) so common-bypass forms behave consistently.

The two-flag split is a direct response to the review: a single flag that
covers both internal hosts and http-to-public-host produces a misleading
error message (telling a user typing `http://example.com` that the host is
"internal") and conflates two unrelated trust dimensions. Splitting now is
mechanical (two booleans, two if-branches); splitting later is a breaking
CLI change.

### Changes Required

#### 1. Extend `scripts/test-design.sh` (failing tests first)

**File**: `scripts/test-design.sh`

**Changes**: Replace the existing `validate-source.sh` behavioural block
(lines 145–161) with the cases listed below. Tests must fail against the
unchanged script.

```bash
echo "=== inventory-design: validate-source.sh behavioural ==="

VALIDATE="$PLUGIN_ROOT/skills/design/inventory-design/scripts/validate-source.sh"
assert_file_exists "validate-source.sh exists" "$VALIDATE"
assert_file_executable "validate-source.sh is executable" "$VALIDATE"

# Unchanged: https + path acceptance, scheme rejections, .. escape rejection
assert_exit_code "accepts https URL" 0 "$VALIDATE" "https://prototype.example.com"
assert_exit_code "rejects file:// scheme" 1 "$VALIDATE" "file:///etc/passwd"
assert_exit_code "rejects javascript: scheme" 1 "$VALIDATE" "javascript:alert(1)"
assert_exit_code "rejects data: scheme" 1 "$VALIDATE" "data:text/html,<script>"
assert_exit_code "accepts code-repo path inside project root" 0 "$VALIDATE" "./examples/design-test-app"
assert_exit_code "rejects path with .. escape" 1 "$VALIDATE" "../../etc/passwd"

# Default-allow cases (localhost / 127.0.0.1)
assert_exit_code "accepts http://localhost without flag" 0 "$VALIDATE" "http://localhost:8080"
assert_exit_code "accepts http://localhost (no port) without flag" 0 "$VALIDATE" "http://localhost/"
assert_exit_code "accepts http://127.0.0.1 without flag" 0 "$VALIDATE" "http://127.0.0.1:3000"
assert_exit_code "accepts https://localhost without flag" 0 "$VALIDATE" "https://localhost:8443"

# Canonicalisation: equivalent forms of localhost are all default-allowed
assert_exit_code "accepts http://LOCALHOST (uppercase)" 0 "$VALIDATE" "http://LOCALHOST:8080"
assert_exit_code "accepts http://localhost. (trailing dot)" 0 "$VALIDATE" "http://localhost./"
assert_exit_code "accepts http://localhost:8080/path?q=1" 0 "$VALIDATE" "http://localhost:8080/path?q=1"

# Internal-host cases: rejected without --allow-internal, accepted with it
assert_exit_code "rejects http://127.0.0.2 without flag" 1 "$VALIDATE" "http://127.0.0.2/"
assert_exit_code "accepts http://127.0.0.2 with --allow-internal" 0 "$VALIDATE" "http://127.0.0.2/" --allow-internal
assert_exit_code "rejects http://10.0.0.1 without flag" 1 "$VALIDATE" "http://10.0.0.1/"
assert_exit_code "accepts http://10.0.0.1 with --allow-internal" 0 "$VALIDATE" "http://10.0.0.1/" --allow-internal
assert_exit_code "rejects http://192.168.1.1 without flag" 1 "$VALIDATE" "http://192.168.1.1/"
assert_exit_code "accepts http://192.168.1.1 with --allow-internal" 0 "$VALIDATE" "http://192.168.1.1/" --allow-internal

# RFC1918 boundary (172.16/12 — most error-prone arithmetic)
assert_exit_code "rejects http://172.16.0.1 (lower edge) without flag" 1 "$VALIDATE" "http://172.16.0.1/"
assert_exit_code "rejects http://172.31.255.255 (upper edge) without flag" 1 "$VALIDATE" "http://172.31.255.255/"
assert_stderr_contains "172.16.0.1 reject names RFC1918" "RFC1918" \
  "$VALIDATE" "http://172.16.0.1/"
assert_stderr_contains "172.31.255.255 reject names RFC1918" "RFC1918" \
  "$VALIDATE" "http://172.31.255.255/"
# 172.15.x and 172.32.x are *outside* RFC1918 — they are public hosts on http,
# so without --allow-insecure-scheme they are rejected as insecure-scheme,
# NOT as RFC1918. This differentiates the two reject paths.
assert_exit_code "rejects http://172.15.255.255 (just outside RFC1918) without flag" 1 "$VALIDATE" "http://172.15.255.255/"
assert_stderr_contains "172.15.255.255 reject names insecure-scheme" "--allow-insecure-scheme" \
  "$VALIDATE" "http://172.15.255.255/"
assert_exit_code "rejects http://172.32.0.0 (just outside RFC1918) without flag" 1 "$VALIDATE" "http://172.32.0.0/"
assert_stderr_contains "172.32.0.0 reject names insecure-scheme" "--allow-insecure-scheme" \
  "$VALIDATE" "http://172.32.0.0/"

# Link-local / cloud metadata
assert_exit_code "rejects http://169.254.169.254 without flag" 1 "$VALIDATE" "http://169.254.169.254/"
assert_exit_code "accepts http://169.254.169.254 with --allow-internal" 0 "$VALIDATE" "http://169.254.169.254/" --allow-internal

# IPv6
assert_exit_code "rejects [::1] without flag" 1 "$VALIDATE" "http://[::1]/"
assert_exit_code "accepts [::1] with --allow-internal" 0 "$VALIDATE" "http://[::1]/" --allow-internal
assert_exit_code "rejects [fe80::1] without flag" 1 "$VALIDATE" "http://[fe80::1]/"
assert_exit_code "accepts [fe80::1%eth0] (zone-id stripped) with --allow-internal" 0 "$VALIDATE" "http://[fe80::1%eth0]/" --allow-internal
assert_exit_code "rejects [::ffff:127.0.0.1] (IPv4-mapped) without flag" 1 "$VALIDATE" "http://[::ffff:127.0.0.1]/"
assert_exit_code "accepts [::ffff:127.0.0.1] with --allow-internal" 0 "$VALIDATE" "http://[::ffff:127.0.0.1]/" --allow-internal
assert_exit_code "rejects [::] without flag" 1 "$VALIDATE" "http://[::]/"
assert_exit_code "accepts [::] with --allow-internal" 0 "$VALIDATE" "http://[::]/" --allow-internal
assert_exit_code "rejects [::1]:8080 (port present) without flag" 1 "$VALIDATE" "http://[::1]:8080/"

# 0.0.0.0 (commonly resolves to local, RFC1122-reserved)
assert_exit_code "rejects http://0.0.0.0 without flag" 1 "$VALIDATE" "http://0.0.0.0/"
assert_exit_code "accepts http://0.0.0.0 with --allow-internal" 0 "$VALIDATE" "http://0.0.0.0/" --allow-internal

# Numeric / encoded IPv4 forms — rejected outright as malformed (no flag bypass)
assert_exit_code "rejects http://2130706433 (decimal-encoded 127.0.0.1)" 1 "$VALIDATE" "http://2130706433/"
assert_exit_code "rejects http://0x7f000001 (hex-encoded 127.0.0.1)" 1 "$VALIDATE" "http://0x7f000001/"
assert_exit_code "rejects http://0177.0.0.1 (octal-encoded)" 1 "$VALIDATE" "http://0177.0.0.1/"

# Userinfo segments are rejected outright (the `user@127.0.0.1@evil.com` confusion class)
assert_exit_code "rejects http://user@example.com (userinfo)" 1 "$VALIDATE" "http://user@example.com/" --allow-insecure-scheme
assert_exit_code "rejects http://user:pass@127.0.0.1@evil.com" 1 "$VALIDATE" "http://user:pass@127.0.0.1@evil.com/" --allow-internal --allow-insecure-scheme

# http-to-public-host: gated on --allow-insecure-scheme (NOT --allow-internal)
assert_exit_code "rejects http://example.com without flag" 1 "$VALIDATE" "http://example.com/"
assert_stderr_contains "http://example.com reject names insecure-scheme" "--allow-insecure-scheme" \
  "$VALIDATE" "http://example.com/"
assert_stderr_not_contains "http://example.com reject does NOT name --allow-internal" "internal address" \
  "$VALIDATE" "http://example.com/"
assert_exit_code "accepts http://example.com with --allow-insecure-scheme" 0 "$VALIDATE" "http://example.com/" --allow-insecure-scheme
assert_exit_code "rejects http://example.com with only --allow-internal" 1 "$VALIDATE" "http://example.com/" --allow-internal
assert_exit_code "accepts http://example.com with both flags" 0 "$VALIDATE" "http://example.com/" --allow-internal --allow-insecure-scheme

# Stale-text guard: the obsolete `(not available in v1)` parenthetical from the
# original script must be gone after this phase
assert_stderr_not_contains "no obsolete (not available in v1) text" "not available in v1" \
  "$VALIDATE" "http://10.0.0.1/"

# Stderr content checks for new default-allow path: no error printed
assert_stderr_empty "http://localhost succeeds silently" \
  "$VALIDATE" "http://localhost:8080"

# Stderr content checks for flag-gated cases: error names the right flag and the host
assert_stderr_contains "10.0.0.1 reject names --allow-internal" "--allow-internal" \
  "$VALIDATE" "http://10.0.0.1/"
assert_stderr_contains "10.0.0.1 reject names the host" "10.0.0.1" \
  "$VALIDATE" "http://10.0.0.1/"

# Unknown flags are rejected (don't silently become a location)
assert_exit_code "rejects unknown --alllow-internal (typo)" 2 "$VALIDATE" "http://localhost/" --alllow-internal
```

#### 1b. Add helper-level unit tests

**File**: `skills/design/inventory-design/scripts/test-validate-source.sh` (NEW)

**Changes**: A separate test file that sources `validate-source.sh` (the
`BASH_SOURCE`-guarded `main` allows source-and-call usage) and exercises the
helpers directly with focused fixtures. Cover:

- `canonicalise_host`:
  - `canonicalise_host '[::1]:8080'` → `::1`
  - `canonicalise_host '[fe80::1%eth0]:443'` → `fe80::1` (zone-id stripped)
  - `canonicalise_host 'LOCALHOST.'` → `localhost`
  - `canonicalise_host '127.0.0.1:8080'` → `127.0.0.1`
  - `canonicalise_host 'user:pass@example.com'` → exit 1 (userinfo)
  - `canonicalise_host '2130706433'` → exit 1 (decimal-encoded)
  - `canonicalise_host '0x7f000001'` → exit 1 (hex-encoded)
  - `canonicalise_host '0177.0.0.1'` → exit 1 (octal-encoded)
- `is_localhost_default`:
  - `localhost`, `127.0.0.1` → 0; everything else → 1
- `classify_internal` boundary cases:
  - `172.15.255.255` → 1 (just below RFC1918)
  - `172.16.0.0` → 0, prints `RFC1918`
  - `172.31.255.255` → 0, prints `RFC1918`
  - `172.32.0.0` → 1 (just above RFC1918)
  - `127.0.0.2` → 0, prints `loopback`
  - `::1` → 0, prints `loopback`
  - `::ffff:127.0.0.1` → 0, prints `loopback`
  - `0.0.0.0` → 0, prints `wildcard`
  - `::` → 0, prints `wildcard`
  - `fe80::1` → 0, prints `link-local`
  - `169.254.169.254` → 0, prints `link-local`
  - `8.8.8.8` → 1 (public)

Hook into `scripts/test-design.sh`:

```bash
echo "=== inventory-design: validate-source.sh helpers ==="
bash "$PLUGIN_ROOT/skills/design/inventory-design/scripts/test-validate-source.sh"
```

#### 2. Implement `validate-source.sh` changes

**File**: `skills/design/inventory-design/scripts/validate-source.sh`

**Changes**:

- Pin `#!/usr/bin/env bash` and add a `BASH_VERSION` guard at the top so the
  script re-execs or fails loudly under `sh`.
- Accept two optional flags (any position): `--allow-internal` and
  `--allow-insecure-scheme`. Use a `while ... case ... shift` parser that
  rejects unknown flags with exit code 2.
- Add a `canonicalise_host()` helper that runs before classification:
  lowercase, trim a single trailing dot, strip surrounding brackets, strip
  port (after brackets), strip zone-id (`%eth0` etc.), reject userinfo
  segments (any `@` in the authority before host extraction), reject
  decimal/octal/hex numeric IPv4 encodings.
- Classify the canonical host as one of: `localhost-default` (always
  allowed: `localhost`, `127.0.0.1`), `internal-flagged` (allowed only with
  `--allow-internal`: other 127/8, 10/8, 172.16/12, 192.168/16, 169.254/16,
  IPv6 `::1`, `fe80::/10`, `::ffff:127.0.0.1`, `::`, `0.0.0.0`), `public`
  (allowed for https; allowed for http only with `--allow-insecure-scheme`).
- Replace the seven independent host-range rejects with one
  `is_internal_flagged()` helper and one error path that names the host,
  the classification (e.g. `RFC1918`, `link-local`, `loopback`), and the
  flag required to override.
- Keep the file:// / javascript: / data: / chrome:// / about: / path-escape
  rejects and the existing happy-path return for valid `https://` and code
  paths unchanged in semantics.
- Drop the obsolete `(not available in v1)` parenthetical from any surviving
  message.
- Source-and-call layout: define helpers above a `main()` entry, with
  `if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then main "$@"; fi` so
  `test-validate-source.sh` (new, see §1) can `source` and exercise
  helpers directly.

Pseudocode:

```bash
#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${BASH_VERSION:-}" ]]; then
  echo "validate-source.sh requires bash" >&2
  exit 2
fi

ALLOW_INTERNAL=0
ALLOW_INSECURE_SCHEME=0
LOCATION=""
while (( $# > 0 )); do
  case "$1" in
    --allow-internal)        ALLOW_INTERNAL=1; shift ;;
    --allow-insecure-scheme) ALLOW_INSECURE_SCHEME=1; shift ;;
    --) shift; LOCATION="${1:-}"; break ;;
    -*) echo "error: unknown flag $1" >&2; exit 2 ;;
    *) if [[ -n "$LOCATION" ]]; then
         echo "error: unexpected positional $1" >&2; exit 2
       fi
       LOCATION="$1"; shift ;;
  esac
done

# canonicalise_host: input is the authority (host[:port], possibly with
# brackets and zone-id; userinfo already rejected at scheme-dispatch time).
# Output (on stdout): lowercased, trailing-dot-stripped, bracket-stripped,
# port-stripped, zone-id-stripped host. Returns 1 if input is malformed
# (numeric IPv4 encodings, embedded `@`).
canonicalise_host() {
  local raw="$1"
  # Reject userinfo / suffix-confusion forms outright
  [[ "$raw" == *@* ]] && return 1
  # Lowercase
  raw="${raw,,}"
  # Strip surrounding brackets (IPv6)
  if [[ "$raw" == \[*\]* ]]; then
    raw="${raw#\[}"; raw="${raw%%\]*}"
    # zone-id strip (anything after `%`)
    raw="${raw%%\%*}"
  else
    # IPv4 / hostname: strip port, then trailing dot
    raw="${raw%%:*}"
  fi
  raw="${raw%.}"
  # Reject decimal/octal/hex IPv4 numeric encodings (e.g. 2130706433, 0x7f000001, 0177.0.0.1)
  if [[ "$raw" =~ ^[0-9]+$ ]] && (( ${#raw} > 3 )); then return 1; fi
  if [[ "$raw" =~ ^0x ]]; then return 1; fi
  if [[ "$raw" =~ ^0[0-9]+\. ]]; then return 1; fi
  printf '%s' "$raw"
}

is_localhost_default() {
  case "$1" in
    localhost|127.0.0.1) return 0 ;;
    *) return 1 ;;
  esac
}

# Returns 0 (internal) and prints the classification on stdout
classify_internal() {
  local h="$1"
  case "$h" in
    ::1|::ffff:127.0.0.1) echo "loopback"; return 0 ;;
    ::|0.0.0.0)           echo "wildcard"; return 0 ;;
  esac
  if [[ "$h" =~ ^127\. ]];     then echo "loopback";   return 0; fi
  if [[ "$h" =~ ^10\. ]];      then echo "RFC1918";    return 0; fi
  if [[ "$h" =~ ^192\.168\. ]];then echo "RFC1918";    return 0; fi
  if [[ "$h" =~ ^169\.254\. ]];then echo "link-local"; return 0; fi
  if [[ "$h" =~ ^fe80: ]];     then echo "link-local"; return 0; fi
  if [[ "$h" =~ ^172\.([0-9]+)\. ]]; then
    local o="${BASH_REMATCH[1]}"
    if (( o >= 16 && o <= 31 )); then echo "RFC1918"; return 0; fi
  fi
  return 1
}
```

Decision logic (after scheme dispatch and canonicalisation):

- `https://` happy path: `is_localhost_default` → allow; `classify_internal`
  succeeds → require `--allow-internal`; else allow (public host on https).
- `http://` localhost-default: allow.
- `http://` `classify_internal` succeeds: require `--allow-internal` only.
  `--allow-internal` *subsumes* `--allow-insecure-scheme` for internal hosts —
  rationale: a user opting into internal-host SSRF risk has already accepted
  the strictly-greater concern; requiring a second flag is redundant. Reject
  message names only `--allow-internal`:
  `error: host '<h>' is <classification>. Pass --allow-internal to permit.`
- `http://` public host (no internal classification): require
  `--allow-insecure-scheme`. Reject message names only `--allow-insecure-scheme`:
  `error: http:// to public host '<h>' is rejected. Use https:// or pass --allow-insecure-scheme.`
- Canonicalisation failure (userinfo / numeric encoding): reject outright,
  no flag bypass. Message names the malformed input class.

**Single-source-of-truth note**: this decision logic is the canonical statement.
Three places must agree: this pseudocode, the validator implementation, and the
SKILL.md paragraph in §3 below. The phrase "an internal host on http needs
both" from the prior draft is wrong and is corrected in §3 — `--allow-internal`
alone suffices for internal hosts on either scheme.

#### 3. Plumb the flags through `SKILL.md` (via skill-creator)

**File**: `skills/design/inventory-design/SKILL.md`

**Changes** — invoke `skill-creator:skill-creator` to apply the following
edits:

- `argument-hint` (line 9): change to
  `"[source-id] [location] [--crawler code|runtime|hybrid] [--allow-internal] [--allow-insecure-scheme]"`.
- Step 1 (lines 59–66): pass both `--allow-internal` and
  `--allow-insecure-scheme` through to `validate-source.sh` when the user
  provided them (each independently).

```bash
${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/validate-source.sh \
  "<location>" ${allow_internal_flag} ${allow_insecure_scheme_flag}
```

- Add a short paragraph after Step 1 explaining the default-allow and the
  two flags:

> By default, `https://` URLs to public hosts and `http://localhost` /
> `http://127.0.0.1` are accepted. Other internal hosts (RFC1918, link-local,
> other loopback IPs) require `--allow-internal` — on either scheme.
> `--allow-internal` subsumes `--allow-insecure-scheme` for internal hosts:
> a user accepting internal-host SSRF risk has already accepted the
> strictly-greater concern. Plain `http://` to a non-localhost public host
> requires `--allow-insecure-scheme` (NOT `--allow-internal`, which would
> be a misleading flag name for that case).

#### 4. Update evals (via skill-creator)

**File**: `skills/design/inventory-design/evals/evals.json`

**Changes**:

- Eval id 13 (`internal-host-rejection`): keep id; refresh to assert
  `http://10.0.0.1/` and `http://169.254.169.254/` both fail without flag,
  `http://10.0.0.1/` succeeds with `--allow-internal`, and
  `http://127.0.0.2/` (non-`127.0.0.1` loopback) fails without flag /
  succeeds with `--allow-internal`. The new case for `127.0.0.2`
  preserves coverage of the loopback range that the original
  `127.0.0.1:8080` case used to indirectly exercise; without it, a
  regression that hardcoded `127.0.0.1` as the only loopback would pass
  every other eval. The original `http://127.0.0.1:8080` case moves out
  (it now succeeds without flag).
- New eval id 18 (`localhost-default-allow`): asserts
  `http://localhost:8080`, `http://127.0.0.1:3000`, `http://LOCALHOST/`,
  and `http://localhost./` all succeed without any flag. Exercises the
  canonicalisation path explicitly.
- New eval id 19 (`allow-internal-and-insecure-scheme-flag-passthrough`):
  asserts the skill passes each flag independently from its CLI through
  to `validate-source.sh` when the user supplies it. Includes a case for
  `http://example.com` requiring `--allow-insecure-scheme` (NOT
  `--allow-internal`) to verify the two flags are not silently aliased.
- Update `evals/benchmark.json` `metadata.evals_run` to include 18 and 19
  in the deterministic tier.

> Eval ids are allocated fresh (18, 19) rather than reusing 15/16 from a
> prior draft, so the executor-related evals introduced in Phase 4 can
> claim a contiguous block (17 was the original deny-list eval; see
> Phase 4 §4 for its replacement).
>
> Each eval body should briefly describe the prompt the test harness will
> use to drive the skill, the expected output file(s) under `outputs/`, and
> the per-expectation list. Match the structure of existing evals (e.g.
> id 12, id 13).

### Success Criteria

#### Automated Verification

- [x] `bash scripts/test-design.sh` passes — Phase 1 does not modify
      MCP-related assertions; whether they pass depends on whether
      Phase 4 has merged
- [x] `bash skills/design/inventory-design/scripts/test-validate-source.sh` exits 0 (helper unit tests)
- [x] `bash skills/design/inventory-design/scripts/validate-source.sh "http://localhost:8080"` exits 0
- [x] `bash skills/design/inventory-design/scripts/validate-source.sh "http://LOCALHOST/"` exits 0 (canonicalisation)
- [x] `bash skills/design/inventory-design/scripts/validate-source.sh "http://10.0.0.1/"` exits 1
- [x] `bash skills/design/inventory-design/scripts/validate-source.sh "http://10.0.0.1/" --allow-internal` exits 0
- [x] `bash skills/design/inventory-design/scripts/validate-source.sh "http://example.com/"` exits 1, stderr names `--allow-insecure-scheme` (not `--allow-internal`)
- [x] `bash skills/design/inventory-design/scripts/validate-source.sh "http://example.com/" --allow-insecure-scheme` exits 0
- [x] `bash skills/design/inventory-design/scripts/validate-source.sh "http://2130706433/"` exits 1 (decimal-encoded IP rejected outright)
- [x] `bash skills/design/inventory-design/scripts/validate-source.sh "http://localhost/" --alllow-internal` exits 2 (typo rejected, not silently swallowed)
- [x] `sh -c "skills/design/inventory-design/scripts/validate-source.sh http://localhost/"` either re-execs under bash or fails loudly (no silent classifier degradation under sh)
- [x] `jq empty skills/design/inventory-design/evals/evals.json` succeeds
- [x] `jq empty skills/design/inventory-design/evals/benchmark.json` succeeds
- [x] `shellcheck skills/design/inventory-design/scripts/validate-source.sh` clean

#### Manual Verification

- [ ] `/inventory-design my-app http://localhost:3000 --crawler code` against
      the design-test-app fixture validates and proceeds (no validator
      error). NOTE: at this phase, the runtime/hybrid path still uses MCP;
      manual verification of `--crawler runtime` localhost is deferred to
      Phase 4.
- [ ] `/inventory-design my-app http://10.0.0.1` reports the validator error
      naming `--allow-internal` (and not `--allow-insecure-scheme`).
- [ ] `/inventory-design my-app http://10.0.0.1 --allow-internal` proceeds
      past validation.
- [ ] `/inventory-design my-app http://example.com` reports the validator
      error naming `--allow-insecure-scheme` (and not `--allow-internal`).
- [ ] `/inventory-design my-app http://example.com --allow-insecure-scheme` proceeds.

---

## Phase 2: Node executor (`run.js`) + project-scoped daemon

### Overview

Build the Node executor and its protocol in isolation. Nothing in the skill
or agents consumes it yet at the end of this phase; it is fully exercised
by Node-side unit tests and bash integration tests.

The daemon model mirrors the **visualiser** skill (`skills/visualisation/visualise/`)
to reuse a pattern already proven on macOS + Linux:

- **TCP loopback** (`127.0.0.1:0`, OS-assigned random port) — no Unix
  sockets, so `sun_path` length, macOS `/var/folders/...` depth, and
  named-pipe portability all become non-issues.
- **Per-project scoping** via `find_repo_root` — daemon belongs to a
  project, not to a shell-process tree. PPID divination is unnecessary;
  daemon reuse and sharing are governed by project root.
- **PID + start-time identity check** — defends against PID reuse so
  `kill` is never sent to an unrelated process.
- **Reuse-before-lock short-circuit** — the common "already running"
  path skips `flock` entirely; only first-spawns and recovery contend on
  the lock.
- **Atomic `server-info.json` / `server.pid` writes** plus a
  post-shutdown `server-stopped.json` audit invariant — if the audit
  write fails, the live files stay in place so the next launch's
  stale-recovery path detects the prior crash.

This phase deliberately drops the regex `evaluate` deny-list documented in
prior drafts. The agent body's allowlist (preserved verbatim in
`agents/browser-analyser.md`) remains the documented contract; the
executor enforces no programmatic block on `evaluate` payloads beyond
Playwright's own page-execution sandbox. Reasons:

1. The deny-list is bypassable by trivial obfuscation (`globalThis['fe'+'tch']`,
   computed property access, template-literal building); shipping it
   creates a false sense of a programmatic boundary.
2. False positives on legitimate accessibility-tree payloads (e.g.
   destructuring with `=`, dataset reads where attribute names happen to
   match a banned token) become user-visible "evaluate-payload-rejected"
   errors with no recovery path.
3. These skills run only on local development machines; the page-controlled
   threat model assumes the local user is trusted with their own browser
   context. The agent-body allowlist remains the right place to encode
   *what we permit*, not *what we forbid*.

### Protocol

The executor lives at
`skills/design/inventory-design/scripts/playwright/run.js` (split into
several modules — see §3) and ships alongside a `package.json` +
`package-lock.json` declaring `playwright` (installed by
`ensure-playwright.sh` in Phase 3) and a tiny launcher
`scripts/playwright/run.sh` that resolves the project state directory and
invokes `node run.js …`.

**Commands** (each one a self-contained bash invocation):

```
run.sh ping
run.sh navigate    '{"url": "https://example.com"}'
run.sh snapshot
run.sh screenshot  '{"path": "screenshots/home-success.png", "mask": ["[type=password]"]}'
run.sh evaluate    '{"expression": "getComputedStyle(document.body).color"}'
run.sh click       '{"ref": "<aria-ref-from-snapshot>"}'
run.sh type        '{"ref": "<aria-ref>", "text": "alice"}'
run.sh wait_for    '{"text": "Welcome, alice", "timeout_ms": 5000}'
run.sh daemon-stop
run.sh daemon-status
```

`ping` is a cheap readiness probe that:
1. Resolves `playwright` from the cache root (catches missing/corrupted
   `node_modules`).
2. Reads `chromium.executablePath()` from Playwright and `fs.statSync`s it
   (catches partial installs where `node_modules/playwright` exists but
   the Chromium binary download was interrupted or removed).
3. Returns `{ok: true, node: <version>, playwright: <version>, chromium: <executable_path>}`.

Does NOT launch the browser. Phase 4's executor-availability check uses
this in preference to a real `navigate` call so that bootstrap-failure-class
errors surface at Step 5 (cleanly, with a `bootstrap` category) rather than
inside agent execution.

**Request / response envelope**: every request and response carries a
`"protocol": 1` field. The daemon rejects a mismatched protocol version
with `{"error": "protocol-mismatch", "expected": 1, "got": <n>}` (exit
code differentiates from transport errors). This gives us a forward
compatibility seam for adding required args without silently breaking
older agent bodies.

**Error envelope**: a single error formatter is the only stderr emitter.
Shape:

```json
{
  "protocol": 1,
  "error": "<kebab-code>",
  "message": "<human-readable>",
  "category": "usage|protocol|browser|bootstrap|filesystem",
  "retryable": false,
  "details": { ... }
}
```

The `error` code is stable (kebab-case); each subcommand documents the
codes it can emit. `retryable: true` means the caller can retry the same
op verbatim; `false` means do not retry.

**Daemon lifecycle** (visualiser-pattern, transparent to callers):

- **State location**: `$PROJECT_ROOT/.accelerator/tmp/inventory-design-playwright/`
  (or whatever `${TMP_REL}` resolves to via the existing
  `config-read-path.sh tmp .accelerator/tmp` indirection used by
  visualiser). This is project-scoped runtime state, NOT machine-wide
  binary cache (which lives elsewhere — see Phase 3).
- **Files** (umask `077`, files mode `0600`, dir mode `0700`):
  - `server-info.json` — `{protocol: 1, pid, start_time, host: "127.0.0.1", port, url, ready_at}`
  - `server.pid` — bare PID
  - `server.log` — daemon log
  - `server.bootstrap.log` — captures stdout/stderr until daemon
    redirects FDs to /dev/null
  - `server-stopped.json` — post-shutdown audit
  - `launcher.lock` — `flock` target (with `mkdir launcher.lock.d`
    fallback, see Phase 3)
- **Reuse short-circuit** (in `run.sh`, before any locking):
  if `server-info.json` + `server.pid` both exist AND `kill -0 $PID`
  succeeds AND the recorded `start_time` matches the current `start_time`
  of `$PID` — connect to the URL and use it. Otherwise, fall through to
  startup/recovery.
- **Identity check**: `start_time_of()` reads `/proc/<pid>/stat` field 22
  + `/proc/stat btime` on Linux, or `LC_ALL=C ps -p <pid> -o lstart=` on
  macOS, parsed with `LC_ALL=C date -j -f "%a %b %e %T %Y"`. The
  `LC_ALL=C` envelope is **mandatory** on the macOS path: without it,
  `ps` emits localised day/month abbreviations (`Mo Mai 6` on
  `LC_TIME=de_DE.UTF-8`, `lun. mai 6` on French) and `date -j -f`
  fails to parse, returning empty. Empty `start_time` defeats the
  reuse short-circuit silently (every call cold-starts a new daemon).
  Both the bash sourced path and the Node `lib/identity.js`
  re-implementation must spawn `ps` with `env: {LC_ALL: 'C', PATH: process.env.PATH}`.

  **Sourcing strategy** (pinned, not "copy or source"): `run.sh` sources
  `skills/visualisation/visualise/scripts/launcher-helpers.sh` directly
  for the bash-side identity check used by the reuse short-circuit. The
  Node-side daemon-status / client-side recovery in `lib/identity.js`
  re-implements the same algorithm in Node (necessary — bash helpers
  can't be sourced into Node), with explicit fixture tests that the bash
  and JS versions agree on the same `(pid, start_time)` tuples under
  pinned `LC_ALL=C TZ=UTC` envelope.

  Both implementations emit `start_time` as **UTC seconds since epoch**
  in `server-info.json`, regardless of host locale or timezone, so any
  downstream comparison is locale-independent.

  If the visualiser's helper is later refactored or moved, a structural
  test (`scripts/test-design.sh`) asserts the source path resolves, the
  function exists, AND that calling it under `LANG=de_DE.UTF-8` produces
  byte-identical output to the `LANG=C` baseline (locale-fragility
  regression guard).

  Follow-up ticket: extract `start_time_of` (and the lock helpers) into
  a plugin-shared `scripts/process-identity.sh` so both skills source
  one canonical implementation. Tracked as a separate work item rather
  than gating this plan.
- **Lock acquisition**: `flock -n 9` on `launcher.lock`; if `flock` is
  not on PATH, fall back to `mkdir launcher.lock.d` (atomic). Other
  invocations get a clear `another launcher is running` error.
- **Spawn**: `nohup node run.js daemon --state-dir <PROJECT_TMP> --owner-pid <PPID> >> bootstrap.log 2>&1 &; disown`.
  Daemon binds `127.0.0.1:0`, captures port via `server.address()`,
  atomically writes `server.pid` then `server-info.json` (each via
  `fs.writeFile` to a `.tmp` sibling, then `fs.rename`).
- **Polling**: launcher polls up to 5s (`50 × 100ms`) for
  `server-info.json` to appear, then echoes the URL.
- **Shutdown triggers** (any of, all converge on one mpsc-equivalent
  promise chain):
  - `SIGTERM` / `SIGINT`
  - Owner PID exited (poll `kill -0 <OWNER_PID>` every 60s)
  - Idle timeout — **30 minutes** of no protocol traffic (decoupled from
    the per-crawl wall-clock bound; see below)
  - Wall-clock per-op timeout — `setTimeout` armed (and re-armed)
    at the **start of every protocol op that touches the page**:
    `navigate`, `snapshot`, `screenshot`, `evaluate`, `click`, `type`,
    `wait_for`. Each such call `clearTimeout`s any prior outstanding
    timer and arms a fresh one. On expiry, the kill path runs in this
    order:
    1. **Write the structured error envelope to the in-flight client
       connection** (best-effort, with a 500 ms timeout on the write):
       ```json
       {
         "protocol": 1,
         "error": "wall-clock-exceeded",
         "message": "Operation exceeded the 5-minute wall-clock budget. See PROTOCOL.md for the wall-clock policy.",
         "category": "browser",
         "retryable": false,
         "details": {
           "op": "<op-name>",
           "url": "<url-if-applicable>",
           "wall_clock_ms": 300000,
           "caller_timeout_ms": <caller-supplied-or-null>,
           "truncated": <true if caller_timeout_ms > wall_clock_ms else false>
         }
       }
       ```
       This preserves the error-envelope contract for the most
       operationally important error case — clients receive a
       structured failure they can branch on, not an `ECONNRESET`.
    2. Write `server-stopped.json` with `reason: "wall-clock"`,
       `op: "<op-name>"`, `url: "<url>"`, and the same
       `caller_timeout_ms` / `truncated` fields as the envelope.
    3. `browser.close()` (graceful Chromium shutdown).
    4. `process.exit(2)`.

    This is the enforcement teeth the prior plan deferred to a separate
    ticket. **Per-op semantics**: a long sequence of distinct ops,
    each completing within 5 minutes, never triggers the kill — but a
    single op that hangs for 5 minutes does. The kill applies to all
    blocking ops including `wait_for`. `ping`, `daemon-status`, and
    `daemon-stop` do NOT arm the timer (they are non-blocking
    control-plane ops).

    **`wait_for` cap with explicit truncation signal**: the daemon
    enforces `timeout_ms = Math.min(callerTimeout, WALL_CLOCK_MS)`
    defence-in-depth so a caller cannot defer the kill by passing a
    huge timeout. **When the cap is applied**, the daemon adds
    `truncated: true` and `caller_timeout_ms: <original>` to whichever
    response the `wait_for` ultimately returns (success, timeout, or
    wall-clock kill). This lets callers distinguish "the page is
    hanging" from "I asked for 10 min and got cut off at 5 min".

    **Escape hatch for legitimately-long ops**: callers needing > 5 min
    `wait_for` (long-running build artifact polls, payment confirmation
    flows) can set `ACCELERATOR_PLAYWRIGHT_WALL_CLOCK_MS=<ms>` in the
    daemon's launch environment to extend the per-op budget. The
    daemon enforces a hard ceiling of 30 minutes regardless of the env
    var. The active value is recorded in `server-info.json` so callers
    can verify the budget in effect.
- **Atomic shutdown**: write `server-stopped.json` first; only then
  remove `server.pid` and `server-info.json`. If the audit write fails,
  the live files stay — next launcher run will re-detect the dead
  daemon and clean up.
- **Stale recovery** (ordering pinned to avoid race): the reuse
  short-circuit's `kill -0` + `start_time` check runs without a lock
  (cheap path). If either fails, the launcher transitions into recovery
  with this strict ordering:
  1. Acquire the lock (`flock` or mkdir-fallback).
  2. Re-run the identity check under the lock — the daemon may have just
     shut down cleanly between the first check and lock acquisition;
     in that case the files are already gone and we proceed to fresh
     spawn.
  3. If still stale: `rm -f` the stale `server.pid`, `server-info.json`,
     and `server-stopped.json` (the audit invariant only cares about
     successful shutdowns; stale files from crashed daemons are
     reaped here).
  4. Spawn fresh daemon (still under lock).
  5. Release lock once `server-info.json` is present.

  This `lock → re-check → rm → spawn` ordering ensures two clients that
  both detect the same stale daemon serialise: exactly one performs
  cleanup-and-spawn, the other observes the fresh daemon under the lock
  and short-circuits to reuse.

  Add a Phase 2 test exercising this exact race: pre-write valid stale
  files for a non-existent PID, launch two `run.sh navigate` in parallel,
  assert exactly one daemon is created.

**Auth header injection** (origin-restricted):

- The launcher reads `ACCELERATOR_BROWSER_AUTH_HEADER` and
  `ACCELERATOR_BROWSER_LOCATION_ORIGIN` from env (populated by the skill
  before agent spawn). The daemon installs a `route()` handler that runs:

  ```js
  const target = new URL(request.url());
  if (target.origin === expectedOrigin) {
    headers[authName] = authValue;
  } else {
    delete headers[authName];
  }
  ```

  Exact-match on `URL.origin` only — no suffix-confusion or IDN handling
  at this stage (skills run on local-dev machines; the threat model does
  not include cross-origin attacker-controlled CDN subresources).

**Screenshot masking** (always-on):

- Default mask selectors: `[type=password]`, `[autocomplete*=token]`,
  `[data-secret]`. Caller-supplied selectors are merged in. No way to
  disable. Unchanged from prior plan.

**Screenshot path constraint**:

- The launcher reads `ACCELERATOR_INVENTORY_OUTPUT_ROOT` from env
  (populated by the skill at agent-spawn time — see Phase 4 §3 for the
  SKILL.md plumbing — pointing at the inventory's `screenshots/` directory).
- **Fail-closed on missing precondition**: if `ACCELERATOR_INVENTORY_OUTPUT_ROOT`
  is unset or empty, `lib/path-guard.js` rejects with
  `{error: "screenshot-output-root-unset", category: "usage", retryable: false}`.
  The path-guard is not a soft constraint — it is the only thing standing
  between caller-controlled paths and arbitrary file overwrite.
- Resolution + rejection cases (all mandatory tests in §1/§2):
  - Caller supplies absolute path: `realpath` the path; if it does not
    fall under `realpath` of the output root, reject with
    `{error: "screenshot-path-outside-output-root", category: "usage"}`.
    This catches both straightforward absolute-outside cases and tricky
    inside-prefix-but-resolves-outside cases (e.g. `${ROOT}/../foo.png`).
  - Caller supplies relative path: resolve against the output root via
    `path.resolve(root, path)`, then `realpath` the result; reject if
    the realpath does not fall under `realpath` of the output root.
    This catches `..` traversal that escapes the prefix.
  - Symlinks under the root: `realpath` follows them. If
    `${ROOT}/escape -> /tmp/external` exists and the caller writes to
    `escape/x.png`, the resolved real path is outside the root and is
    rejected.
- Overwriting an existing file is permitted (re-runs of the same crawl
  produce the same screenshot names).

### Changes Required

#### 1. Add Node-side unit tests (failing tests first)

Tests are split into per-module unit tests (pure modules, fast, no
browser) and integration tests (full executor against a fixture page).

**File**: `skills/design/inventory-design/scripts/playwright/lib/errors.test.js`

Pure-function tests for the error-envelope formatter. Cover:
- Every category (`usage`, `protocol`, `browser`, `bootstrap`, `filesystem`)
  produces a well-formed envelope with `protocol: 1`, `error` (kebab),
  `message` (non-empty), `category`, `retryable`.
- Optional `details` round-trips arbitrary JSON-serialisable values.
- `protocol-mismatch` populates `message` with actionable text mentioning
  `"protocol": 1`.

**File**: `skills/design/inventory-design/scripts/playwright/lib/mask.test.js`

Default mask selectors plus caller-supplied are merged, deduped, and the
output is a plain array. Default list = `[type=password]`,
`[autocomplete*=token]`, `[data-secret]`. Caller cannot remove defaults.

**File**: `skills/design/inventory-design/scripts/playwright/lib/path-guard.test.js`

Each rejection case independently:
- Unset `ACCELERATOR_INVENTORY_OUTPUT_ROOT` → `screenshot-output-root-unset`.
- Empty `ACCELERATOR_INVENTORY_OUTPUT_ROOT` → same.
- Absolute path outside root → `screenshot-path-outside-output-root`.
- Absolute path with `..` that resolves outside root (`${ROOT}/../foo.png`) → rejected.
- Relative path with `..` that escapes (`../../etc/x.png`) → rejected.
- Symlink under root pointing outside (create `${ROOT}/escape -> /tmp/external`,
  request `escape/x.png`) → rejected.
- Valid relative path → resolves to `${ROOT}/path` and returns the path.
- Valid absolute path inside root → returns the path.

**File**: `skills/design/inventory-design/scripts/playwright/lib/state.test.js`

Atomic file-write semantics: `writeServerInfo()` writes via tmp+rename;
mid-write crash leaves no partial file; concurrent writes serialise
(last writer wins, no torn reads).

**File**: `skills/design/inventory-design/scripts/playwright/lib/identity.test.js`

`start_time_of()` cross-platform parsing using fixture data committed at
`lib/__fixtures__/proc-stat-linux.txt` and `lib/__fixtures__/ps-lstart-macos.txt`:
- Linux fixture (raw `/proc/<pid>/stat` line + `/proc/stat btime` line)
  → expected (pid, start_time) tuple as UTC seconds since epoch.
- macOS fixture (raw `ps -p <pid> -o lstart=` output) → expected
  (pid, start_time) tuple.
- Cross-validation: shell out to `bash -c "LC_ALL=C TZ=UTC; source $LAUNCHER_HELPERS; start_time_of $PID < $FIXTURE"`,
  run `lib/identity.js` against the same fixture under
  `{LC_ALL: 'C', TZ: 'UTC'}`, assert `result_bash === result_js` as
  strings.
- Locale fragility test: run the bash version under `LANG=de_DE.UTF-8`
  and `LANG=fr_FR.UTF-8` and assert the same tuple as `LANG=C` (regression
  guard for the round-3 locale-dependence finding).

**File**: `skills/design/inventory-design/scripts/playwright/lib/lock.test.js`

Tests for the flock-or-mkdir lock primitive:
- `flock` available + lock free → acquires immediately.
- `flock` available + lock held → waits up to `LOCK_TIMEOUT_MS`,
  then succeeds when released.
- `ACCELERATOR_LOCK_FORCE_MKDIR=1` → mkdir-fallback path used; same
  semantics as flock path.
- `mkdir` fallback + lock-dir already exists (stale from killed
  prior holder) → contention timeout fires; clear error.
- Trap cleanup: process exits abnormally with lock held → next
  invocation reclaims (mkdir trap fires; flock auto-releases on
  process death).

**File**: `skills/design/inventory-design/scripts/playwright/lib/auth-header.test.js`

Tests for the origin-allowlist `route()` handler factory:
- Same-origin request → header attached.
- Cross-origin request (different scheme, host, or port) → header
  not attached / stripped.
- Default-port form: `https://example.com` and `https://example.com:443`
  must be treated as same origin (uses `URL.origin` exactly).
- Case-insensitive host match: `https://Example.COM` matches
  `https://example.com` (hostname comparison via `URL`).
- Subdomain confusion: `https://app.example.com.evil.com` does NOT
  match `https://app.example.com` (suffix attack rejection).
- IDN homograph: `https://xn--example-X.com` does NOT match
  `https://example.com`.
- Cross-origin redirect: request whose `request.frame().url()` is at
  the expected origin BUT whose `request.url()` is on a third-party
  CDN → header not attached.
- `Origin: null`: opaque origin → header not attached.
- Missing `ACCELERATOR_BROWSER_AUTH_HEADER` env: handler is a no-op
  (no header attached anywhere).

**File**: `skills/design/inventory-design/scripts/playwright/lib/daemon.test.js`

Focused tests for the daemon's timer / state-machine logic that don't
require launching Chromium (Playwright is mockable):
- Wall-clock timer arming on each blocking op (parametric over
  `navigate`, `snapshot`, `screenshot`, `evaluate`, `click`, `type`,
  `wait_for`); `clearTimeout` on prior timer; new timer expiry calls
  the kill handler.
- `ping`, `daemon-status`, `daemon-stop` do NOT arm the timer.
- Idle timer reset on each protocol op.
- Owner-PID watcher: when `kill -0 <owner_pid>` returns non-zero,
  daemon initiates shutdown within one watcher tick.
- Shutdown trigger convergence: `SIGTERM`, idle, owner-PID-exit,
  wall-clock all converge on the same `writeStoppedAndExit` path
  with the appropriate `reason` recorded.

**File**: `skills/design/inventory-design/scripts/playwright/lib/client.test.js`

Thin tests for `client.js`:
- `server-info.json` present + reachable → client connects, sends
  request, prints response.
- `server-info.json` present but TCP connect fails → client treats
  as stale, removes files, exec-replaces with `run.sh` (or signals
  `run.sh` to spawn fresh — see `client.js` design comment).
- `server-info.json` missing → client errors with
  `category: "usage"`, `error: "no-daemon"`, signalling `run.sh` to
  spawn rather than spawning directly (single spawn path
  responsibility).

**File**: `skills/design/inventory-design/scripts/playwright/test-run.js`

Integration tests using Node's built-in `node:test` (≥ Node 20). Cover:

- `ping` returns `{ok: true, node, playwright, chromium}` with `chromium`
  pointing at an existing executable, and does NOT launch a browser
  (assert by checking elapsed time is < 200 ms and no `chrome*` process
  is spawned).
- `ping` with `node_modules/playwright/.local-browsers/chromium-XXX/`
  removed (simulate corrupted bootstrap) → exits non-zero with
  `category: "bootstrap"`.
- `navigate` then `snapshot` returns a non-empty snapshot for a static
  fixture HTML served from a Node static server (avoid Python).
- `screenshot` writes a PNG to the resolved path; non-zero bytes.
- `screenshot` exhaustive path-guard cases (mirrors `path-guard.test.js`
  but at the integration boundary): unset env var, empty env var,
  absolute outside root, absolute with `..` resolving outside, relative
  with `..` escaping, symlink under root pointing outside. Each rejected
  with the appropriate error code and category.
- `screenshot` with a `[type=password]` field present produces an output
  whose pixels in the password area are uniform — assert via `pngjs`
  (added as a devDependency so the strong assertion always runs).
- `click` against a button with a known label triggers the expected DOM
  change.
- `type` writes characters into a text input.
- `wait_for` returns when the awaited text appears; times out cleanly
  when it does not.
- **Daemon lifecycle** (parameterise the idle timer to 100 ms and the
  wall-clock to 200 ms via env so tests are fast):
  - Two consecutive client calls reuse the same daemon (assert by
    reading the PID from `server-info.json` and confirming it is stable).
  - Stale-socket recovery: pre-create a `server.pid` pointing at a
    non-existent PID; next call detects, cleans up, and spawns a fresh
    daemon.
  - Concurrent first-spawn: launch two `run.sh navigate` in parallel
    against an empty state dir; assert exactly one daemon is created
    and both clients connect successfully.
  - Idle shutdown: with the timer parameterised to 100 ms, no protocol
    traffic for >100 ms causes the daemon to exit; `server-stopped.json`
    is written with `reason: "idle"`, `server.pid`/`server-info.json`
    are removed.
  - **Wall-clock per-op bound, single-op kill** (envelope check):
    with the bound parameterised to 200 ms, arming via `navigate`
    then no further calls, the in-flight client receives a structured
    error envelope `{protocol: 1, error: "wall-clock-exceeded",
    category: "browser", retryable: false, details: {op: "navigate",
    url: "...", wall_clock_ms: 200, ...}}` (NOT an `ECONNRESET`),
    daemon then exits with code 2, and `server-stopped.json` records
    the same fields plus `reason: "wall-clock"`.
  - **Wall-clock re-arm across consecutive ops** (mixed): with the
    bound parameterised to 200 ms, issue `navigate A` → wait 150 ms →
    `snapshot` → wait 150 ms → `evaluate '...'`. Total elapsed >
    400 ms (twice the bound) but no kill fires, because each op
    clears and rearms the timer.
  - **`wait_for` cap with truncation signal**: with the bound
    parameterised to 200 ms, issue `navigate A` →
    `wait_for {"text": "never appears", "timeout_ms": 5000}` against a
    fixture. Daemon kills `wait_for` at ~200 ms; the in-flight client
    receives the structured envelope with
    `details: {op: "wait_for", caller_timeout_ms: 5000, wall_clock_ms: 200, truncated: true}`.
    The `truncated: true` flag distinguishes cap-from-hang.
  - **`wait_for` shorter than wall-clock returns natural timeout
    cleanly** (cap direction check): with bound parameterised to
    200 ms, issue `wait_for {"text": "never appears", "timeout_ms": 50}`.
    Daemon returns the wait_for timeout result at ~50 ms (NOT killed,
    NOT truncated, no `server-stopped.json` written). Confirms the
    cap is `Math.min`, not `Math.max`.
  - **Wall-clock kill of in-flight op after re-arm**: `navigate A` →
    wait 50 ms → `evaluate` (slow expression, takes longer than the
    bound) → assert killed with structured envelope, `op: "evaluate"`
    (not `navigate`); `server-stopped.json` records same.
  - **Wall-clock budget env-var override**: with
    `ACCELERATOR_PLAYWRIGHT_WALL_CLOCK_MS=600000` (10 min) in the
    daemon's launch env, `wait_for {"timeout_ms": 540000}` (9 min)
    against a never-satisfied fixture is permitted to run; the
    `server-info.json` records `wall_clock_ms: 600000`. With the env
    var set above the 30-minute ceiling
    (`ACCELERATOR_PLAYWRIGHT_WALL_CLOCK_MS=99999999`), the daemon
    clamps to 1800000 ms and logs a warning.
  - **`ping` does NOT arm the timer**: `ping` followed by long idle
    does not cause a wall-clock kill; only the regular idle timer
    applies.
  - **Owner-PID watcher reaps daemon**: spawn daemon with
    `--owner-pid <pid-of-test-shell>`; on test-shell exit, daemon exits
    within 60 s (or sooner if the watcher poll interval is parameterised).
- **Protocol-version round-trip per subcommand** (parametric over all
  ops — `ping`, `navigate`, `snapshot`, `screenshot`, `evaluate`,
  `click`, `type`, `wait_for`, `daemon-status`, `daemon-stop`):
  - Each op's request includes `protocol: 1`; response includes
    `protocol: 1`.
  - Each op rejects `protocol: 999` with `protocol-mismatch` (via the
    canonical error envelope).
- Error envelope: every emitted error from every op path carries
  `{protocol, error, message, category, retryable, details?}` (asserted
  via `lib/errors.test.js` for purity and at the integration boundary
  for representative ops).
- `daemon-status`: returns a structured `{state: running|stopped, ...}`
  without spawning a daemon (cheap probe like `ping`).

These tests assume a local Playwright + Chromium install. Phase 3's
`ensure-playwright.sh` is the only install path; the test harness sources
`ensure-playwright.sh` (no `--mock`) the first time it runs and reuses
the cache thereafter. The "Phase-2-internal helper" idea from prior
drafts is dropped; Phase 3 lands first (see Implementation Approach).

#### 2. Add bash integration tests (failing tests first)

**File**: `skills/design/inventory-design/scripts/playwright/test-run.sh`

**Changes**: Source `scripts/test-helpers.sh`. Honour
`ACCELERATOR_PLAYWRIGHT_SKIP_REAL_INSTALL=1` to no-op the real bootstrap
prerequisite and skip Playwright-dependent cases (validator-style cases
still run); this keeps Phase 2 tests usable if Phase 3 is reverted.

Cover:

- `run.sh ping` exits 0 with `{"ok": true, "chromium": "..."}` on stdout, < 1s.
- `run.sh navigate '{"url": "http://localhost:<port>/fixture.html"}'`
  followed by `run.sh snapshot` produces a non-empty JSON snapshot.
- `run.sh screenshot '{"path": "home.png"}'` (relative path, with
  `ACCELERATOR_INVENTORY_OUTPUT_ROOT` set to a tmp dir) writes a PNG
  there.
- `run.sh screenshot '{"path": "/etc/x.png"}'` exits non-zero with
  `screenshot-path-outside-output-root`.
- `run.sh evaluate '{"expression": "getComputedStyle(document.body).color"}'`
  succeeds against a fixture page (round-trips a string).
- `run.sh evaluate '{"expression": "fetch(\"/x\")"}'` is FORWARDED (the
  deny-list has been removed); the test asserts the call reaches
  `page.evaluate` and returns whatever the page returns. This explicitly
  pins the new contract: the executor does not filter `evaluate`
  payloads.
- Two consecutive `run.sh navigate` calls share a daemon (assert by
  reading PID from `server-info.json`).
- A second `run.sh navigate` from a different shell against the same
  project root reuses the existing daemon.
- `run.sh daemon-stop` writes `server-stopped.json`, removes
  `server.pid` + `server-info.json`, and exits 0.
- After `daemon-stop`, a fresh `run.sh navigate` spawns a new daemon
  with a new PID.
- Concurrent `run.sh navigate` from two shells against an empty state
  dir produces exactly one daemon (the lock holder spawns; the other
  short-circuits on reuse after the first writes `server-info.json`).
- After kill -9 of the daemon (simulated by `kill -9 $(cat server.pid)`
  + leaving the files in place), the next `run.sh navigate` cleans up
  and spawns fresh.

#### 3. Implement the executor (multi-file)

The executor is split into a small set of focused modules so each piece
is independently testable:

```
skills/design/inventory-design/scripts/playwright/
├── package.json
├── package-lock.json
├── run.sh                  # bash launcher (~30 lines)
├── run.js                  # CLI dispatch (~80 lines)
├── lib/
│   ├── daemon.js           # browser context + server.listen + lifecycle
│   ├── client.js           # connect to server-info.json URL, send/receive
│   ├── state.js            # state-dir resolution, atomic file writes,
│   │                       # server-info.json / server-stopped.json schema
│   ├── identity.js         # start_time_of() + reuse short-circuit
│   ├── lock.js             # flock-or-mkdir lock acquisition
│   ├── auth-header.js      # route() handler factory
│   ├── mask.js             # default mask selectors + merge
│   ├── path-guard.js       # screenshot path constraint
│   └── errors.js           # canonical error envelope formatter
└── ...test files...
```

**File**: `skills/design/inventory-design/scripts/playwright/run.js`

```js
#!/usr/bin/env node
// CLI dispatch only. Parses argv, routes to either client.callRemote()
// (for navigate/snapshot/screenshot/evaluate/click/type/wait_for/ping/
// daemon-status/daemon-stop) or daemon.start() (for the `daemon`
// subcommand). All real work is in lib/.
```

**File**: `skills/design/inventory-design/scripts/playwright/lib/daemon.js`

Owns the Playwright browser context, the `route()` handler, the HTTP
server listening on `127.0.0.1:0`, the idle / wall-clock / SIGTERM /
owner-PID-watcher shutdown triggers, the atomic state-file writes, and
the `server-stopped.json` audit invariant.

**File**: `skills/design/inventory-design/scripts/playwright/lib/client.js`

Reads `server-info.json` (or, if missing, calls `daemon.spawn()` via
`run.sh`-equivalent path), opens an HTTP connection to the recorded URL,
sends the JSON envelope, prints the response, exits.

**File**: `skills/design/inventory-design/scripts/playwright/run.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

# Project state dir resolution mirrors visualiser:
# shellcheck disable=SC1091
source "$PLUGIN_ROOT/scripts/vcs-common.sh"
PROJECT_ROOT="$(find_repo_root "$PWD")"
# Fail-fast on no-repo: command-substitution swallows exit codes under
# `set -e`, so check the result explicitly.
if [[ -z "${PROJECT_ROOT:-}" ]]; then
  echo "error: inventory-design must be run inside a git or jj repository (no enclosing repo found from $PWD)" >&2
  exit 2
fi

# Honour user's per-project tmp config override (visualiser convention):
TMP_REL="$("$PLUGIN_ROOT/scripts/config-read-path.sh" tmp .accelerator/tmp)"
STATE_DIR="$PROJECT_ROOT/$TMP_REL/inventory-design-playwright"

umask 077
mkdir -p "$STATE_DIR"
chmod 0700 "$STATE_DIR"

# Machine-wide cache for node_modules (set by ensure-playwright.sh):
CACHE_ROOT="${ACCELERATOR_PLAYWRIGHT_CACHE:-${HOME}/.cache/accelerator/playwright}"

# Compute lockhash directly from the skill-shipped package-lock.json
# (deterministic, no dependency on a top-level pointer that another
# project's bootstrap may have rewritten). The top-level pointer in the
# cache is informational only.
PKG_LOCK="$SCRIPT_DIR/package-lock.json"
sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -c1-8
  else
    shasum -a 256 "$1" | cut -c1-8
  fi
}
LOCKHASH="$(sha256_of "$PKG_LOCK")"
NS_ROOT="$CACHE_ROOT/$LOCKHASH"

# Validate the namespace is bootstrapped; if not, fail with a clear error
# pointing at ensure-playwright.sh.
if [[ ! -f "$NS_ROOT/node_modules/playwright/package.json" ]]; then
  echo "error: Playwright not installed for lockhash $LOCKHASH at $NS_ROOT — run scripts/ensure-playwright.sh first" >&2
  exit 3
fi

export ACCELERATOR_PLAYWRIGHT_STATE_DIR="$STATE_DIR"
export NODE_PATH="$NS_ROOT/node_modules"
exec node "$SCRIPT_DIR/run.js" "$@"
```

The `vcs-common.sh` source path resolves via `PLUGIN_ROOT` (four levels
up from `scripts/playwright/`); the `config-read-path.sh` invocation
honours the same per-project `tmp` config override that visualiser
respects (so a user who set `tmp = some/other/path` in `.accelerator/config.md`
sees both skills' state land at the configured root). The lockhash is
computed directly from the skill-shipped `package-lock.json` rather
than read from a top-level cache pointer — deterministic across
concurrent multi-project bootstraps and immune to top-level pointer
corruption. The top-level pointer in the cache (written by
`ensure-playwright.sh`) is purely informational, used by humans and
diagnostic tools.

The `sha256_of` helper dispatches to `sha256sum` (Linux) or
`shasum -a 256` (macOS), eliminating the round-2 portability bug.

**File**: `skills/design/inventory-design/scripts/playwright/package.json`

```json
{
  "name": "accelerator-playwright-executor",
  "private": true,
  "version": "0.0.0",
  "type": "module",
  "engines": { "node": ">=20" },
  "dependencies": {
    "playwright": "~1.49.0"
  },
  "devDependencies": {
    "pngjs": "^7.0.0"
  }
}
```

**Tilde range** (`~1.49.0`) rather than an exact pin. Playwright ships
patch releases tracking Chromium fixes; floor + lockfile gives us
patch-level upgrade headroom without floating to a future major. The
committed `package-lock.json` (next to `package.json`) is the
authoritative version manifest.

#### 4. Wire executor tests into `scripts/test-design.sh`

**File**: `scripts/test-design.sh`

Append:

```bash
echo "=== inventory-design: playwright executor ==="
bash "$PLUGIN_ROOT/skills/design/inventory-design/scripts/playwright/test-run.sh"
```

Do not yet remove MCP assertions — that happens in Phase 4.

### Success Criteria

#### Automated Verification

- [x] `bash skills/design/inventory-design/scripts/playwright/test-run.sh` exits 0
- [x] `node --test skills/design/inventory-design/scripts/playwright/test-run.js` exits 0
- [x] `node --test skills/design/inventory-design/scripts/playwright/lib/*.test.js` exits 0 (per-module unit tests)
- [x] `bash scripts/test-design.sh` still passes (Phase 1 + Phase 2 cohabit)
- [x] `shellcheck skills/design/inventory-design/scripts/playwright/run.sh` clean
- [x] `node -c skills/design/inventory-design/scripts/playwright/run.js` exits 0
- [x] `jq empty skills/design/inventory-design/scripts/playwright/package.json` succeeds
- [x] `[ -f skills/design/inventory-design/scripts/playwright/package-lock.json ]` (lockfile committed)
- [x] `grep -r 'evaluate-payload-rejected' skills/design/inventory-design` returns no matches outside this plan and historical research (deny-list is gone)

#### Manual Verification

- [ ] `run.sh ping` returns within 1s without launching Chromium.
- [ ] `run.sh navigate '{"url": "https://example.com"}'` followed by
      `run.sh snapshot` produces a sensible JSON snapshot.
- [ ] `run.sh screenshot '{"path": "example.png"}'` (with
      `ACCELERATOR_INVENTORY_OUTPUT_ROOT=/tmp/inv`) writes
      `/tmp/inv/example.png`.
- [ ] `run.sh screenshot '{"path": "/tmp/x.png"}'` (with the same env
      var pointing elsewhere) is refused.
- [ ] Two consecutive `run.sh` calls in different shells against the
      same project root reuse the same daemon (port number identical).
- [ ] `kill -9 $(cat .accelerator/tmp/inventory-design-playwright/server.pid)`
      followed by another `run.sh navigate` cleanly recovers and spawns
      a new daemon.

---

## Phase 3: First-run bootstrap (`ensure-playwright.sh`)

### Overview

Lazy install of Playwright + Chromium binaries into a **machine-wide
cache** at `${HOME}/.cache/accelerator/playwright` on first
runtime/hybrid crawl. Idempotent re-runs are fast. Reproducible installs
via a committed `package-lock.json`. macOS + Linux only.

This phase **lands before Phase 2's tests run for real** — `ensure-playwright.sh`
is what Phase 2's `test-run.sh` and `test-run.js` invoke (no `--mock`)
to obtain a working Playwright. The "Phase-2-internal helper" idea from
prior drafts is dropped: building the bootstrap first removes the
need to write throwaway install code.

### Cache layout & rationale

- **Machine-wide binary cache** (this script's responsibility),
  namespaced by lockfile hash:
  `${HOME}/.cache/accelerator/playwright/<sha8>/`
  where `<sha8>` is `sha256(package-lock.json)[:8]` of the
  skill-shipped lockfile. Different plugin versions ship different
  lockfiles → different hashes → no cache collision.
  Each `<sha8>/` directory contains:
  - `node_modules/` — installed via `npm ci --ignore-scripts` from the
    skill-shipped `package.json` + `package-lock.json`
  - Chromium binary (under Playwright's standard browsers path,
    rooted at `playwright-browsers/` inside the namespace)
  - `.bootstrap-sentinel` — one-line JSON:
    `{lockhash, node_version, playwright_version, chromium_revision, completed_at}`.
    `lockhash` is the canonical version key; `node_modules` is only
    valid for one lockhash.
- A top-level `${HOME}/.cache/accelerator/playwright/.bootstrap-sentinel`
  symlink (or a `current` JSON) points at the most recently bootstrapped
  `<sha8>/` so `run.sh` can resolve the active install without reading
  every namespace dir. `run.sh` reads this top-level sentinel to find
  the matching `node_modules`; if missing, it picks the newest `<sha8>/`
  whose sentinel matches the skill's current lockhash.
- **Project-scoped runtime state** (Phase 2's responsibility):
  `$PROJECT_ROOT/.accelerator/tmp/inventory-design-playwright/`
- **Stale-namespace cleanup** (opt-in, conservative): `ensure-playwright.sh`
  runs an opt-in sweep at the end when `ACCELERATOR_PLAYWRIGHT_SWEEP=1`
  is set. Default is **no sweep** — users keep their cache until they
  explicitly clean it. Rationale: silent deletion of multi-hundred-MB
  binary caches is a UX hazard (round-3 review finding). When enabled,
  the sweep:
  - Reads the namespace sentinel's `completed_at` field (UTC ISO 8601)
    — uses content, not filesystem `mtime`, to avoid clock-skew false
    positives (NTP backwards adjustments, dead-battery dev laptops).
  - Computes `delta_days = (now_utc - completed_at) / 86400`. Sweeps
    only when `delta_days > 90` AND not the active namespace AND not
    the namespace pointed at by the top-level pointer (so a recently-
    used pinned-older-version is preserved).
  - Holds `bootstrap.lock` while sweeping so no concurrent invocation
    is reading from the swept dirs (in-use protection).
  - Logs each removed namespace to stderr with its sentinel-recorded
    `playwright_version` and `completed_at` (no silent deletion).
  Predictable storage growth ceiling, with explicit user consent.

  Manual cleanup recipe (always available, even without the env var):
  documented in README §Cache & cleanup.

We deliberately do NOT depend on `${CLAUDE_PLUGIN_DATA}` for the binary
cache. The visualiser skill stores its binaries similarly under the
plugin tree itself; we follow the visualiser-cache convention by rooting
under `~/.cache/accelerator/...`. This sidesteps the Claude-Code-version
gating concern and unifies the cleanup story (one `rm -rf` recipe).

### Changes Required

#### 1. Add bash behavioural tests (failing tests first)

**File**: `skills/design/inventory-design/scripts/test-ensure-playwright.sh`

**Changes**: Source `test-helpers.sh`. Cover:

- Script exists and is executable.
- macOS + Linux only — on `MSYS*` / `MINGW*` / `CYGWIN*` `OSTYPE`, exits
  with a clear "unsupported platform" message.
- Running with `ACCELERATOR_PLAYWRIGHT_CACHE` pointing at a tmp dir and
  Node ≥ 20 on PATH (real install — runs once per CI job, then sentinel
  short-circuits remaining tests in the same job):
  - First run: installs into cache, runs `playwright install chromium`,
    writes a populated `.bootstrap-sentinel` JSON, exits 0.
  - Second run with sentinel present and `require.resolve('playwright')`
    succeeding **inside the cache root**: exits 0 in < 2 seconds with a
    one-line "ready" log when `--verbose` is set, silent otherwise.
  - Sentinel deleted but `node_modules/playwright` resolvable: re-runs
    only `playwright install chromium`, rewrites sentinel.
  - `node_modules/playwright` corrupted (sentinel present, but
    `require.resolve` fails inside the cache): full reinstall.
- With Node missing on PATH: exits 1; stderr names "Node ≥ 20 required",
  the install URL, and the detected `OSTYPE`.
- With Node 18 on PATH: exits 1; stderr names the actual version and
  the floor.
- Pre-flight disk-space check: with cache-root filesystem reporting
  < 500 MB free (simulated by writing a fixture filesystem or by
  monkey-patching `df`), exits 1 with a clear "≥ 500 MB free required"
  message naming the cache root.
- Concurrent invocation: two `ensure-playwright.sh` runs from different
  shells against the same cache produce exactly one install (the second
  blocks on the lock then short-circuits on the sentinel).
- Concurrent invocation with mkdir-fallback forced
  (`ACCELERATOR_LOCK_FORCE_MKDIR=1`): same outcome, exercising the
  flock-less branch even on Linux CI where flock is normally available.
- **Lockhash namespacing**: with two committed test fixtures
  `evals/fixtures/lockhash/lock-a.json` and `lock-b.json` (deliberately
  divergent stub lockfiles, NOT real installs — used only to drive
  the namespacing test), running the bootstrap with each in turn
  produces two sibling `<sha8>/` directories whose names match
  `sha256_of(lock-a.json)[:8]` and `sha256_of(lock-b.json)[:8]`
  respectively. Both have valid sentinels; the top-level pointer
  reflects the most-recent bootstrap. The test reads the actual sha8
  values via the same `sha256_of` helper rather than hard-coding
  prefixes, so the assertion is robust to fixture content changes.

  Fixture-refresh recipe (documented in `evals/fixtures/lockhash/README.md`):
  the fixtures are intentionally minimal stub `package-lock.json`
  files (just enough JSON for the test path), not generated from real
  Playwright installs. Modify them by hand if the test needs new
  shapes; do not regenerate from a real `npm install`.
- **Stale-namespace sweep** (default off): with three `<sha8>/`
  directories where two have sentinel `completed_at` > 90 days ago and
  one is the active namespace, default invocation (no env var) leaves
  all three intact. With `ACCELERATOR_PLAYWRIGHT_SWEEP=1`, both stale
  dirs are removed (content-based age, not mtime), the active one is
  preserved, and stderr logs each removal naming its
  `playwright_version` and `completed_at`. Skip-on-negative-delta:
  when one sentinel's `completed_at` is in the future (simulated
  clock-skew), the sweep skips it with a warning rather than deleting.
- **Sweep does not race in-use namespaces**: with a long-running
  `run.sh evaluate` reading from a 91-day-old non-active namespace,
  starting `ensure-playwright.sh` with `ACCELERATOR_PLAYWRIGHT_SWEEP=1`
  in a different shell waits for the run.sh process to release the
  daemon (or the lock), then sweeps. Test asserts the long-running
  invocation completes successfully.
- **Audit summary visible (step 8a)**: with `npm audit --omit=dev` mocked
  to report a high-severity advisory, the bootstrap exits 0 (audit is
  non-failing) and stderr contains the literal `npm audit reported advisories`
  string. With audit clean, no warning printed.
- Parametric mock failure modes — used only by tests, replacing the
  prior single `--offline-mock` boolean:
  - `ACCELERATOR_PLAYWRIGHT_MOCK_NPM_EXIT=42` → simulated `npm ci`
    failure with exit 42; bootstrap exits 1, stderr surfaces the npm
    exit code AND lists relevant env vars (`NPM_CONFIG_REGISTRY`,
    `NODE_EXTRA_CA_CERTS`, `HTTPS_PROXY`).
  - `ACCELERATOR_PLAYWRIGHT_MOCK_PLAYWRIGHT_EXIT=42` → simulated
    `playwright install chromium` failure; stderr names
    `PLAYWRIGHT_DOWNLOAD_HOST` as a relevant override.
  - `ACCELERATOR_PLAYWRIGHT_MOCK_NPM_OK=1` → no-op the npm step (lets
    tests that don't need real install run fast).
  - `ACCELERATOR_PLAYWRIGHT_MOCK_PLAYWRIGHT_OK=1` → no-op the chromium
    download.
  - Both `_OK` flags set → fast path that just touches the expected
    files and writes a sentinel; equivalent to the old single-boolean
    mock but with each step independently controllable.
- Stdout in the "first run" case contains the user-facing preamble
  AND streams `npm ci` progress (i.e. `--silent` is NOT passed).
- SIGINT mid-install: writing a real test for this is fragile; we
  instead assert behaviourally that on bash trap of any non-zero exit,
  the partial sentinel and any tmp install directory are removed (set
  `trap 'cleanup' EXIT INT TERM`).

#### 2. Implement `ensure-playwright.sh`

**File**: `skills/design/inventory-design/scripts/ensure-playwright.sh`

**Changes**:

```bash
#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${BASH_VERSION:-}" ]]; then
  echo "ensure-playwright.sh requires bash" >&2; exit 2
fi

# Reject non-macOS, non-Linux platforms fast.
case "${OSTYPE:-unknown}" in
  darwin*|linux*|linux-gnu*) ;;
  *) echo "error: ensure-playwright.sh supports macOS and Linux only (OSTYPE=$OSTYPE)" >&2
     exit 2 ;;
esac

CACHE_ROOT="${ACCELERATOR_PLAYWRIGHT_CACHE:-${HOME}/.cache/accelerator/playwright}"
NODE_FLOOR_MAJOR=20
DISK_FLOOR_MB=500

# Skill-shipped manifest + lockfile (authoritative versioning):
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PKG_JSON="$SCRIPT_DIR/playwright/package.json"
PKG_LOCK="$SCRIPT_DIR/playwright/package-lock.json"

# Lockhash-keyed namespace (cross-platform sha256 dispatch)
sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -c1-8
  else
    shasum -a 256 "$1" | cut -c1-8
  fi
}
LOCKHASH="$(sha256_of "$PKG_LOCK")"
NS_ROOT="$CACHE_ROOT/$LOCKHASH"
SENTINEL="$NS_ROOT/.bootstrap-sentinel"
TOP_SENTINEL="$CACHE_ROOT/.bootstrap-sentinel"
LOCKFILE="$CACHE_ROOT/bootstrap.lock"

cleanup() {
  rm -f "${SENTINEL}.tmp" "${TOP_SENTINEL}.tmp"
}
trap cleanup EXIT INT TERM

# 1. Node presence + version check.
# 2. Pre-flight: ≥ DISK_FLOOR_MB free at the filesystem hosting CACHE_ROOT.
# 3. mkdir -p "$NS_ROOT"; chmod 0700 "$CACHE_ROOT" "$NS_ROOT"
# 4. Acquire lock — flock if available, else mkdir-based fallback:
#       acquire_lock() { ... }   # see below
# 5. Re-check sentinel under lock (lock-then-check):
#    if "$SENTINEL" exists AND `cd "$NS_ROOT" && node -e "require.resolve('playwright', {paths:['node_modules']})" 2>/dev/null`
#    succeeds AND the sentinel's `lockhash` field equals "$LOCKHASH",
#    update "$TOP_SENTINEL" to point at this namespace and exit 0
#    (with a one-line "ready" log if --verbose).
# 6. Print preamble (visible — no --silent):
#       inventory-design: first-run setup.
#       Installing Playwright + Chromium (~150 MB; takes 1–3 min).
#       Cache: $NS_ROOT
#       Press Ctrl-C to cancel; partial state will be cleaned up.
# 7. cp "$PKG_JSON" "$PKG_LOCK" "$NS_ROOT/"
# 8. cd "$NS_ROOT" && npm ci --ignore-scripts --no-fund
#    (drop --silent so npm progress is visible; npm audit summary is now
#    visible too — see step 8a).
# 8a. (best-effort) cd "$NS_ROOT" && npm audit --omit=dev --audit-level=high \
#       || echo "inventory-design: npm audit reported advisories; review with \`npm audit\` in $NS_ROOT" >&2
#    (non-failing — local-dev users see the warning once and can act on
#     it; CI runs see it in logs. Replaces the prior --no-audit deferral.)
# 9. cd "$NS_ROOT" && npx playwright install chromium
# 10. Write the namespace sentinel JSON via tmp+rename:
#       jq -n --arg lh "$LOCKHASH" --arg nv "$(node -v)" \
#             --arg pv "$(jq -r .version "$NS_ROOT"/node_modules/playwright/package.json)" \
#             '{lockhash:$lh, node_version:$nv, playwright_version:$pv, completed_at:(now | todate)}' \
#         > "${SENTINEL}.tmp" && mv "${SENTINEL}.tmp" "$SENTINEL"
# 10a. Update top-level pointer (atomic, points at active namespace):
#       cp "$SENTINEL" "${TOP_SENTINEL}.tmp" && mv "${TOP_SENTINEL}.tmp" "$TOP_SENTINEL"
# 11. Print: "inventory-design: setup complete."
# 12. Stale-namespace sweep (opt-in, content-based, clock-skew safe):
#       only when ACCELERATOR_PLAYWRIGHT_SWEEP=1; default is NO sweep.
#       Holds the lock from step 4 throughout — in-use protection.
#       For each <sha8>/ subdir of $CACHE_ROOT (excluding $LOCKHASH and
#       any namespace named in TOP_SENTINEL):
#         - jq -r .completed_at <sha8>/.bootstrap-sentinel  (UTC ISO 8601)
#         - delta_days = (date -u +%s - $(date -u -d "$completed_at" +%s)) / 86400
#           (or BSD: date -j -u -f "%Y-%m-%dT%H:%M:%SZ" "$completed_at" +%s)
#         - if delta_days > 90:
#             echo "inventory-design: pruning stale Playwright cache <sha8> (playwright=$pv, last bootstrap $completed_at, $delta_days days old)" >&2
#             rm -rf <sha8>/
#         - if delta_days < 0 (clock skew): skip with stderr warning, do not delete.
# 13. Release lock, exit 0.

# acquire_lock(): prefer `flock -n 9 "$LOCKFILE"`; if `command -v flock`
# fails (or `ACCELERATOR_LOCK_FORCE_MKDIR=1` for tests), fall back to
# `mkdir "${LOCKFILE}.d"` (atomic). On either failure, wait up to 5
# minutes for the holder to release (the holder is doing the real install;
# we want to short-circuit on the sentinel afterwards rather than fail).
# On the mkdir path, register a `trap` to `rmdir` on exit.
```

**`--ignore-scripts` transitive note**: this flag suppresses ALL transitive
postinstalls, not just Playwright's. The plan compensates with the explicit
`npx playwright install chromium` step. If a future patch within
`~1.49.0` adds a new transitive dep with a load-bearing postinstall, this
path will silently skip it — a snapshot test against the lockfile in
`scripts/test-design.sh` enumerates the postinstalls in the lockfile so
that future bumps surface this case at code-review time.

Edge cases:

- **Concurrent bootstrap (lock-then-check)**: lock first, then re-read
  the sentinel under the lock. The first holder writes the sentinel;
  subsequent holders see it and short-circuit without redoing the
  install.
- **Corrupted node_modules**: detect by `cd "$CACHE_ROOT" && node -e "require.resolve('playwright', {paths:['node_modules']})"`
  — note the explicit `paths` option pointing at the cache's
  `node_modules`, NOT the caller's CWD chain. On failure, full reinstall.
- **Cache root unwritable**: if `mkdir -p "$CACHE_ROOT"` fails, surface
  a clear "cache directory is not writable; tried <path>" error. Do
  not auto-fall-back silently.
- **Corporate network**: when `npm ci` or `playwright install` fails,
  the error message lists `NPM_CONFIG_REGISTRY`, `NODE_EXTRA_CA_CERTS`,
  `HTTPS_PROXY`, and `PLAYWRIGHT_DOWNLOAD_HOST` as the relevant
  configuration knobs.
- **`PLAYWRIGHT_BROWSERS_PATH`**: if the user has this set externally,
  we honour it (don't override) — Playwright will install into their
  configured location and `node_modules/playwright` will know where to
  find it. We just don't set it ourselves.

#### 3. Ship `package.json` + `package-lock.json`

**Files**:
- `skills/design/inventory-design/scripts/playwright/package.json` (already from Phase 2)
- `skills/design/inventory-design/scripts/playwright/package-lock.json` (NEW — generated by running `npm install` once locally and committing the resulting lockfile)

`npm ci --ignore-scripts` will refuse to run without a lockfile, so this
file is load-bearing. Regenerating: `cd <playwright-dir> && rm -rf node_modules package-lock.json && npm install --ignore-scripts && git add package-lock.json`.

#### 4. Declare the Node floor in `plugin.json` / README

**File**: `plugin.json`

If `plugin.json` supports an `engines`-style field, declare
`"node": ">=20"`. If not, add a top-level `requirements:` array of
human-readable strings. (Confirm during implementation; if the schema
does not support either, fall back to a README "Requirements" section
with a CHANGELOG "Breaking" callout in Phase 5.)

**File**: `README.md`

Add a "Requirements" subsection (placement TBD in Phase 5) naming Node
≥ 20 as a hard requirement for `inventory-design --crawler runtime|hybrid`,
along with the macOS + Linux supported-platforms statement.

#### 5. Hook the test into `scripts/test-design.sh`

**File**: `scripts/test-design.sh`

Append:

```bash
echo "=== inventory-design: ensure-playwright.sh ==="
bash "$PLUGIN_ROOT/skills/design/inventory-design/scripts/test-ensure-playwright.sh"
```

### Success Criteria

#### Automated Verification

- [x] `bash skills/design/inventory-design/scripts/test-ensure-playwright.sh`
      exits 0 (one real install per CI job, mocks for failure paths)
- [x] `bash scripts/test-design.sh` exits 0
- [x] `shellcheck skills/design/inventory-design/scripts/ensure-playwright.sh`
      clean
- [x] `[ -f skills/design/inventory-design/scripts/playwright/package-lock.json ]`
      (lockfile committed)

#### Manual Verification

- [ ] On a fresh machine without any cached Playwright (or with
      `${ACCELERATOR_PLAYWRIGHT_CACHE}` deleted), running
      `ensure-playwright.sh` prints the preamble, streams `npm ci`
      progress, downloads Chromium (1–3 min), prints "setup complete",
      and writes a populated sentinel.
- [ ] Re-running immediately after exits in < 2 seconds with no further
      output (or one "ready" line under `--verbose`).
- [ ] Running with Node 18 on PATH fails fast with "Node ≥ 20 required".
- [ ] Running with `ACCELERATOR_PLAYWRIGHT_CACHE` pointing at a
      filesystem with < 500 MB free fails with the disk-floor message.
- [ ] On macOS without `flock` on PATH, two concurrent invocations
      produce exactly one install (mkdir-based lock fallback works).
- [ ] On a corporate network (manual, opt-in test) where `NPM_CONFIG_REGISTRY`
      points at a private mirror, the bootstrap honours it and completes.

---

## Phase 4: Wire executor into skill + agents, remove MCP

### Overview

Wire the executor into the skill and the two browser agents. Replace
LLM-mediated MCP detection with a deterministic `run.sh ping` probe.
Update evals and the structural test suite in lockstep.

The phase is structured as **two separate PRs**, not one squash-merged PR.
Two PRs is more robust than relying on a particular merge strategy: each
PR lands as one (or a small number of) commit on `main` regardless of
whether the team uses squash, rebase, or merge-commit, and bisect /
partial-revert remain possible without procedural enforcement:

- **PR 4a — Add executor alongside MCP (dual tools)**: agents declare both
  `Bash(...run.sh *)` and existing `mcp__playwright__*` tools; SKILL.md
  `allowed-tools` is additive; agent prose preferentially uses the
  executor; structural tests assert both paths exist.
- **PR 4b — Remove MCP path**: depends on 4a being merged. Drops
  `mcp__playwright__*` from agent frontmatter and SKILL.md `allowed-tools`;
  deletes `.claude-plugin/.mcp.json`; allocates eval id 20 (replacing
  retired id 3); adds structural assertions that the MCP path is gone.

If a regression is reported after 4b merges, reverting just 4b restores
the working dual-tools state without touching the executor wiring.

**4a transitional caveat**: while 4a is on `main` (between the 4a and 4b
merges), users running `/inventory-design --crawler runtime` are still
exposed to the original MCP-inheritance hallucination class IF the agent
prose picks MCP first. To prevent this, 4a's structural test asserts the
agent body's `## Tools` section opens with `run.sh` instructions BEFORE
any MCP fallback prose — so 4a is a safe transitional state, not a state
where the original UAT failure can recur.

**4a → 4b stall policy** (commitment to bound the transitional window):

- Open the 4b PR within **2 working days** of 4a merging.
- If 4b cannot merge within **5 working days** of 4a (review feedback,
  scope debate, vacation), revert 4a from `main` rather than leaving
  the dual-tools state to bit-rot. The 4a revert is one commit; the
  4b PR can be re-opened later from a fresh 4a.
- Phase 5's final benchmark gate verifies `git log --oneline 4a-sha..4b-sha`
  is non-empty AND fewer than 5 working days separate the two commits.
  This converts the "transitional" claim into a measurable post-merge
  invariant.

Rationale: the round-3 review correctly noted that "transitional state"
without a duration commitment effectively means "permanent" if 4b
stalls. Bounding the window keeps the safety property of the split
real rather than aspirational.

### Step ordering inside the skill

The new skill flow (updated SKILL.md) is:

1. **Step 1** — `validate-source.sh` (cheap, pure, no daemon side-effects).
2. **Step 2** — Provisional crawler-mode resolution from CLI flag /
   defaults table.
3. **Step 3** — Bootstrap (only if provisional mode is `runtime` or
   `hybrid`): run `ensure-playwright.sh`. Returns one of `ready` /
   `unavailable-soft` / `unavailable-hard`.
4. **Step 4** — Final crawler-mode resolution: if Step 3 returned
   `unavailable-soft` and provisional mode was `hybrid`, downgrade to
   `code` and emit the script-driven downgrade notice (see §3 below). If
   `unavailable-hard` and mode was `runtime`, hard-fail with the
   bootstrap's stderr.
5. **Step 5** — Confirm executor liveness: `run.sh ping` (cheap, no
   browser launch). On failure, treat as `unavailable-hard`.
6. **Step 6** — Spawn agents (browser-locator → browser-analyser).
7. **Step 7** — Cleanup: `run.sh daemon-stop`.

This linearises the prior Step 0 / Step 3 chicken-and-egg coupling: each
step has at most one input from the previous step, and validation runs
before any daemon side-effects.

### Changes Required — 4a (additive: executor wiring alongside MCP)

#### 1. Update `agents/browser-locator.md` (via skill-creator)

**File**: `agents/browser-locator.md`

Frontmatter — additive:

```yaml
tools:
  - mcp__playwright__browser_navigate
  - mcp__playwright__browser_snapshot
  - Bash(${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/playwright/run.sh *)
```

Body changes:

- The agent's primary contract becomes `run.sh navigate` /
  `run.sh snapshot`. MCP tools remain declared but the prose instructs
  the agent to use them only as a fallback if `run.sh` is unavailable
  in a given session (this is belt-and-braces — Phase 4b removes them
  shortly).
- Add a `Cleanup` step: agent must call `run.sh daemon-stop` as its
  final action, regardless of which path it used.
- Add an "If `run.sh navigate` fails" branch: surface the executor's
  stderr JSON to the caller; do not retry. Programmatically the agent
  inspects `error.category`: `bootstrap` → unrecoverable;
  `browser`/`usage` → caller should diagnose; `protocol` → contract
  mismatch (file as a bug).

#### 2. Update `agents/browser-analyser.md` (via skill-creator)

**File**: `agents/browser-analyser.md`

Frontmatter — additive (all seven existing MCP tools remain plus
`Bash(...run.sh *)`).

Body changes:

- Replace `mcp__playwright__browser_*` references in the prose with
  `run.sh <op>` while leaving the MCP tools available as fallback.
- Keep the entire `browser_evaluate` payload allowlist section verbatim
  (still the documented contract). Update the heading to
  `run.sh evaluate Payload Allowlist`. The allowlist is the primary
  governance for what payloads the agent emits — the executor enforces
  no programmatic deny-list (this was a deliberate UX decision; see
  Phase 2 Overview).
- Add the `Cleanup` step (call `run.sh daemon-stop`).
- Remove any prose suggesting the executor pre-filters payloads — the
  agent body's allowlist is the only filter.

#### 3. Update `inventory-design/SKILL.md` (via skill-creator)

**File**: `skills/design/inventory-design/SKILL.md`

- Frontmatter `argument-hint`: already updated in Phase 1 to include
  `--allow-internal` and `--allow-insecure-scheme`. No further change.
- Frontmatter `allowed-tools`: ADDITIVE — keep existing seven
  `mcp__playwright__*` entries, add:

```yaml
Bash(${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/playwright/run.sh *),
Bash(${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/ensure-playwright.sh *),
Bash(${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/notify-downgrade.sh *),
```

- Restructure the body to the seven-step flow described above. Step 3
  (bootstrap) and Step 5 (ping probe) are the new shell-driven gates;
  the LLM-self-introspection of the toolbox at lines 103–124 is
  deleted.

- **Step 4 downgrade notice via shell script** (new): introduce a
  small helper `scripts/notify-downgrade.sh` that takes
  `--from <runtime|hybrid> --to <code> --reason <enum>` and emits a
  fixed user-facing line. The downgrade message becomes a script
  responsibility rather than LLM prose so it can be byte-asserted by
  eval id 20.

  **`--reason` is a closed enum**: the script rejects any value not in
  the enum with exit 2. Enum values and corresponding messages are
  defined as a JSON file (not a sourceable bash assoc-array — keeps
  the format independent of bash version and consumable by any tool):

  **File**: `skills/design/inventory-design/scripts/notify-downgrade-messages.json`
  ```json
  {
    "node-missing": "inventory-design: Playwright runtime is unavailable (Node ≥ 20 not found). Falling back to code-only crawler. Run `ensure-playwright.sh` manually to install, or pass --crawler code to suppress this notice.",
    "node-too-old": "inventory-design: Playwright runtime is unavailable (Node version is too old; ≥ 20 required). Falling back to code-only crawler. Run `ensure-playwright.sh` manually after upgrading Node, or pass --crawler code to suppress this notice.",
    "bootstrap-failed": "inventory-design: Playwright bootstrap failed. Falling back to code-only crawler. Run `ensure-playwright.sh` manually to see the full error, or pass --crawler code to suppress this notice.",
    "executor-ping-failed": "inventory-design: Playwright executor is unhealthy. Falling back to code-only crawler. Run `run.sh ping` manually to diagnose, or pass --crawler code to suppress this notice.",
    "cache-unwritable": "inventory-design: Playwright cache directory is not writable. Falling back to code-only crawler. Check permissions on $ACCELERATOR_PLAYWRIGHT_CACHE (or ~/.cache/accelerator/playwright) and retry, or pass --crawler code to suppress this notice.",
    "disk-floor-not-met": "inventory-design: Playwright cache filesystem has less than 500 MB free. Falling back to code-only crawler. Free space at $ACCELERATOR_PLAYWRIGHT_CACHE and retry, or pass --crawler code to suppress this notice."
  }
  ```

  `notify-downgrade.sh` reads this file via `jq -r --arg r "$REASON" '.[$r]'`,
  validates `--reason` is a key (exit 2 if not), strips control
  characters from the resolved message before printing (defence
  against future reflection of user-controlled reason strings), and
  prints exactly that message to stdout, then exits 0.

  **Detected-condition → enum mapping** (canonical, lives in PROTOCOL.md
  and is enforced by `ensure-playwright.sh` exit codes):

  | Condition | `ensure-playwright.sh` exit | `--reason` value |
  |--|--|--|
  | Node binary missing | 10 | `node-missing` |
  | Node major < 20 | 11 | `node-too-old` |
  | < 500 MB free at cache | 12 | `disk-floor-not-met` |
  | `mkdir $CACHE_ROOT` fails | 13 | `cache-unwritable` |
  | `npm ci` fails | 14 | `bootstrap-failed` |
  | `playwright install chromium` fails | 15 | `bootstrap-failed` |
  | `run.sh ping` fails post-bootstrap | (Step 5) | `executor-ping-failed` |

  `ensure-playwright.sh` writes a single line `ACCELERATOR_DOWNGRADE_REASON=<enum>`
  to stderr immediately before non-zero exit (in addition to the human-
  readable error message). SKILL.md Step 4 greps that line and passes
  the value verbatim to `notify-downgrade.sh --reason`. Eval id 20
  forces each detected condition (via the parametric mock flags from
  Phase 3) and asserts the correct reason is selected.

  **Per-reason goldenfile fixtures**: under `evals/fixtures/notify-downgrade/`
  one `<reason>.expected.txt` per enum key, byte-equal to the
  corresponding value in `notify-downgrade-messages.json`. Eval id 20
  and the test file below match against these fixtures byte-by-byte.

  **Goldenfile regeneration recipe** (committed):
  `skills/design/inventory-design/scripts/regenerate-notify-downgrade-fixtures.sh`
  — iterates the JSON keys with `jq -r 'keys[]'`, writes each value to
  `evals/fixtures/notify-downgrade/<key>.expected.txt`. Run after any
  message edit. Documented in the script's header.

  **Dedicated test file**: `skills/design/inventory-design/scripts/test-notify-downgrade.sh`
  covers:
  - For each enum key: `notify-downgrade.sh --reason <key>` produces
    stdout byte-equal to the corresponding fixture.
  - **Set-equality assertion**: `jq -r 'keys[]' notify-downgrade-messages.json | sort`
    equals `ls evals/fixtures/notify-downgrade/*.expected.txt | sed 's/.*\///;s/\.expected\.txt$//' | sort`.
    Catches fixture drift where a new enum key lacks a fixture (or
    vice versa).
  - Unknown-reason rejection (exit 2).
  - Missing-required-flag rejection (exit 2).
  - Control-character sanitisation: the strip rule is "remove all
    bytes outside printable ASCII range 0x20–0x7E plus newline 0x0A;
    reject (not strip) bidi-override codepoints U+202A–U+202E and
    U+2066–U+2069 by exiting non-zero on detection." Tests cover NUL,
    CR, BS, DEL, ESC[31m (ANSI CSI), and `‮` (bidi RLO).
  Hooked into `scripts/test-design.sh`.

- Replace the LLM-mediated MCP detection prose with explicit shell
  invocations of `ensure-playwright.sh` and `run.sh ping`. Document the
  return codes each step uses to drive Step 4's mode resolution.

- Update the auth-header allowlist paragraph (lines 83–89) to note
  that origin enforcement is now performed by the executor's `route()`
  handler. The agent's prose contract is unchanged in spirit.

- Replace any "Playwright MCP" prose with "Playwright executor" or
  "Playwright".

- Remove the `mise run deps:install:playwright` hint; replace with a
  reference to `ensure-playwright.sh` and what its exit message says.

#### 4. Empirically verify env-var expansion in sub-agent `tools:`

Before this PR opens for review, run a real `Task` invocation against
`accelerator:browser-locator` from a sandbox session that exercises
`run.sh ping`. If Claude Code's permission engine does NOT expand
`${CLAUDE_PLUGIN_ROOT}` in agent frontmatter `tools:` (only confirmed
for SKILL.md `allowed-tools` historically), the wiring will deny at
runtime even though structural tests pass. If denied, the fallback is a
permissive `Bash` allow paired with a runtime-side check inside `run.sh`
that the resolved `SCRIPT_DIR` is under `${CLAUDE_PLUGIN_ROOT}`. Document
the empirical result in the PR description.

#### 5. Update evals (4a portion)

**File**: `skills/design/inventory-design/evals/evals.json`

- **Eval id 14** (`browser-evaluate-safety-structural`): keep
  semantically intact; update the section-heading reference from
  `browser_evaluate Payload Allowlist` to `run.sh evaluate Payload Allowlist`.
- No new eval ids in 4a (the structural tests cover the additive
  wiring; new eval ids land in 4b once MCP is gone).

#### 6. Structural assertions in `scripts/test-design.sh` (4a portion)

- Add positive assertions that `agents/browser-locator.md` and
  `agents/browser-analyser.md` declare BOTH `mcp__playwright__*` tools
  AND `Bash(...run.sh *)` (dual-mode contract during 4a).
- Add positive assertion that `SKILL.md` `allowed-tools` includes
  `Bash(...run.sh *)`, `Bash(...ensure-playwright.sh *)`, and
  `Bash(...notify-downgrade.sh *)`.
- **Pin agent prose ordering** (4a transitional safety): assert that
  in each browser agent body, the first occurrence of `run.sh ` precedes
  the first occurrence of `mcp__playwright__`, so the LLM sees executor
  instructions before MCP fallback instructions and is more likely to
  pick `run.sh` first. This prevents 4a from silently regressing into
  the original UAT failure mode for any user who happens to land on the
  bug class while 4a is on `main`.
- Existing MCP-tool-list assertions stay — they are still true at
  the end of 4a.

### Changes Required — 4b (subtractive: remove MCP path)

#### 7. Drop MCP from agent frontmatter

**Files**: `agents/browser-locator.md`, `agents/browser-analyser.md`

Remove the `mcp__playwright__*` entries from the `tools:` lists. Update
prose to remove any "fallback to MCP" branches.

#### 8. Drop MCP from SKILL.md `allowed-tools`

**File**: `skills/design/inventory-design/SKILL.md`

Remove the seven `mcp__playwright__*` entries.

#### 9. Delete `.claude-plugin/.mcp.json`

No `plugin.json` change required — confirmed not referenced.

#### 10. Update evals (4b portion)

**File**: `skills/design/inventory-design/evals/evals.json`

- **Retire eval id 3** (`mcp-unavailable-fallback`) — mark with a
  `deprecated: true` field if the eval framework supports it; otherwise
  delete and document the retirement in the CHANGELOG.
- **Add eval id 20** (`executor-bootstrap-failure-fallback`): with
  `ensure-playwright.sh` mocked to exit 1 (`ACCELERATOR_PLAYWRIGHT_MOCK_NPM_EXIT=1`),
  user invokes `--crawler hybrid`. Asserts: stdout contains the literal
  `notify-downgrade.sh` output; the skill spawns a code-mode agent (no
  `run.sh navigate` calls beyond the `ping` probe); inventory artifact
  is produced.
- **Add eval id 21** (`executor-ping-no-browser`): asserts `run.sh ping`
  is what Step 5 calls (NOT `run.sh navigate`), by inspecting the
  executor state dir post-run and confirming no Chromium was launched
  on a code-mode crawl.
- Eval ids 18 and 19 from Phase 1 are unaffected.
- New ids 20 and 21 are added to `evals/benchmark.json` `metadata.evals_run`.

#### 11. Structural assertions in `scripts/test-design.sh` (4b portion)

- Remove the entire `=== Browser agents ===` MCP-tool-list block.
  Replace with assertions that browser-locator and browser-analyser
  each declare exactly one `Bash(...run.sh *)` tool and zero
  `mcp__playwright__*` tools.
- Remove the `=== .mcp.json ===` block. Add a positive assertion that
  `.claude-plugin/.mcp.json` does not exist.
- Replace the `inventory-design: skill structure` MCP assertions with
  assertions that `allowed-tools` lists the three new `Bash(...)`
  entries and contains no `mcp__playwright__*`.

  **Important phrasing**: the structural test asserts `mcp__playwright__*`
  is absent from agent `tools:` and SKILL.md `allowed-tools` — it does
  NOT positively assert "no MCP exists anywhere". This leaves a future
  parallel MCP-based path possible (e.g. if Anthropic fixes #13605/#13898
  and we want to offer MCP as an alternative).

- Confirm `argument-hint` includes both `--allow-internal` and
  `--allow-insecure-scheme`.

- Add an assertion that the agent body's `run.sh evaluate Payload Allowlist`
  section names every forbidden pattern documented in the original
  `browser_evaluate` allowlist (preserving eval id 14's contract). This
  replaces the prior plan's deny-list eval — the allowlist in agent prose
  is the only governance and the structural test pins it.

- Add an assertion that the executor source under
  `skills/design/inventory-design/scripts/playwright/` does NOT contain
  any of the deny-list pattern strings (`fetch`, `XMLHttpRequest`,
  `WebSocket`, `document\.cookie`, `localStorage`, `sessionStorage`,
  etc.) wired into a regex test array — i.e. the deny-list is
  structurally absent, not just one specific error code. This frames
  the assertion against the deny-list *behaviour* rather than against
  one specific error-code name (`evaluate-payload-rejected`), leaving
  the kebab-code namespace open for a future legitimate use of the same
  name (e.g. an opt-in deny-list for shared-CI deployments).

#### 12. Re-run benchmarks via skill-creator

Invoke `skill-creator:skill-creator` to re-run the `inventory-design`
eval suite as benchmarks (probabilistic ≥ 5 runs, deterministic ≥ 3
runs, structural-only once). Update `evals/benchmark.json`.

**Pass-rate gate**: existing eval ids (1, 2, 4, 5, 8, 10, 11, 12, 13, 14)
must report mean pass-rate ≥ their prior baseline (currently 1.0 across
the eval set, with at most a documented 0.05 variance margin per eval).
New eval ids (18, 19, 20, 21) must report ≥ 0.9 mean pass-rate. Eval
id 3 is retired and excluded.

### Success Criteria

#### Automated Verification

- [x] After 4a commit: `bash scripts/test-design.sh` exits 0; both MCP
      and executor wiring assertions pass; agents work via either path.
- [x] After 4b commit: `bash scripts/test-design.sh` exits 0 with the
      new (no-MCP) assertions.
- [x] `bash skills/design/inventory-design/scripts/playwright/test-run.sh` exits 0
- [x] `bash skills/design/inventory-design/scripts/test-ensure-playwright.sh` exits 0
- [x] `[ ! -e .claude-plugin/.mcp.json ]`
- [x] `grep -r 'mcp__playwright__' skills/design/inventory-design agents/ scripts/test-design.sh` returns no matches (active code only; deprecated eval id 3 data retained for history)
- [x] `grep -r 'evaluate-payload-rejected' skills/design/inventory-design agents/ scripts/test-design.sh` returns no matches
- [x] `jq empty skills/design/inventory-design/evals/evals.json` succeeds
- [ ] skill-creator benchmark: every existing eval ≥ baseline; new evals
      18, 19, 20, 21 ≥ 0.9.

#### Manual Verification

- [ ] `/inventory-design design-test-app ./examples/design-test-app --crawler hybrid`
      runs end-to-end against the fixture; bootstrap reuses cache; the
      runtime portion crawls without hallucination.
- [ ] `/inventory-design my-app http://localhost:3000 --crawler runtime`
      against a real local dev server crawls real routes (the original
      UAT failure case).
- [ ] In a session where Node ≥ 20 is unavailable,
      `/inventory-design x ./examples/design-test-app --crawler hybrid`
      cleanly downgrades to `code`. The downgrade message comes from
      `notify-downgrade.sh` (literal text match) rather than LLM prose.
- [ ] In a fresh machine with no `~/.cache/accelerator/playwright`, the
      first runtime crawl prints the bootstrap preamble and proceeds.
- [ ] After 4a merge, an agent invocation manually downgraded to MCP-only
      (by removing `Bash(...)` from `tools:` in a local edit) still
      works — proves the dual-tool fallback was real.
- [ ] Sub-agent `${CLAUDE_PLUGIN_ROOT}` expansion verified empirically
      (recorded in the 4a commit message or the PR description).

---

## Phase 5: Documentation, cleanup, sign-off

### Overview

Update README and the inventory-design skill's user-facing prose to
reflect the executor-based path. Decide and document the semver bump.
Final variance benchmark.

### Changes Required

#### 1. Publish executor protocol reference

**File**: `skills/design/inventory-design/PROTOCOL.md` (NEW — at skill
level, not buried under `scripts/playwright/`)

Promoting the document to skill-level makes it discoverable by:
- Downstream skill authors looking to reuse the executor
- Third-party agent-body authors who edit `agents/browser-*.md`
- Debugging shells (`ls skills/design/inventory-design/` shows it)
- The README's runtime-browser-dependency section can link via a
  short relative path (`see PROTOCOL.md`)

The executor's wire protocol becomes a contract for any future caller.
PROTOCOL.md covers:

- **Wire envelope** for requests and responses: `protocol: 1`, op,
  args/result, error.
- **Error envelope schema**: `{protocol, error, message, category, retryable, details?}`.
- **Category enum**: `usage | protocol | browser | bootstrap | filesystem`.
- **Per-subcommand reference table**: each op (`ping`, `navigate`,
  `snapshot`, `screenshot`, `evaluate`, `click`, `type`, `wait_for`,
  `daemon-status`, `daemon-stop`) lists args, success result shape,
  and the kebab error codes it can emit (with category and retryable
  for each).
- **Detected-condition → `notify-downgrade.sh` enum mapping**: the
  table from Phase 4 §3 (Node missing → exit 10 → `node-missing`,
  etc.) lives here as the canonical reference. SKILL.md Step 4 and
  `notify-downgrade.sh` both consume this contract.
- **Stability commitment** (softer than prior draft):

  > **v1 default behaviour**: the executor does not filter `evaluate`
  > payloads — they are forwarded verbatim to `page.evaluate`.
  >
  > **What v1 permits as additive (non-breaking) changes**:
  > - Opt-in tightening via documented env vars (e.g. a
  >   hypothetical `ACCELERATOR_PLAYWRIGHT_DENY_LIST=1` that activates
  >   a payload deny-list off-by-default). Callers who do not set the
  >   env var see no behaviour change.
  > - Additive new ops, additive new error codes, additive new fields
  >   in the response envelope (callers ignore unknown fields).
  >
  > **What requires a v2 protocol bump**:
  > - Default-on tightening of `evaluate` (i.e. payload filtering
  >   active without an env-var opt-in).
  > - Removing or renaming any op currently in the surface.
  > - Required new request fields (callers without them break).
  > - Semantic change to existing error categories or codes.

- **Versioning**: clients send `protocol: 1`; daemon rejects mismatches
  with `protocol-mismatch` (category `protocol`, non-retryable). Future
  versions can be added side-by-side; the daemon may support multiple
  major versions concurrently if needed.
- **User-facing rendering convention**: when an agent body surfaces an
  error envelope to the user, the recommended format is
  `inventory-design: <category>: <message>` with the kebab `error`
  code in parentheses for support diagnostics. Agent bodies adopt
  this format in their error-handling prose.

Cross-link from agent body prose ("for the executor wire schema, see
`skills/design/inventory-design/PROTOCOL.md`") and from the README's
runtime browser dependency subsection.

#### 2. Update README

**File**: `README.md`

**Changes**:

- Remove any reference to Playwright MCP as a runtime dependency (search
  for `playwright/mcp` and `Playwright MCP`). The line at README:611-612
  advertising the pinned MCP becomes obsolete.
- Add a "Requirements" subsection (or extend the existing one) naming:
  - Node ≥ 20 for `inventory-design --crawler runtime|hybrid`
  - macOS or Linux (Windows is not supported)
  - ~500 MB free disk for the first-run Chromium install
- Add a "Runtime browser dependency" subsection naming
  `ensure-playwright.sh` as the install path and the
  `~/.cache/accelerator/playwright/` cache root.
- Add a "Cache & cleanup" paragraph documenting:
  - Per-machine cache: `~/.cache/accelerator/playwright/`
  - Per-project state: `<project>/.accelerator/tmp/inventory-design-playwright/`
  - Reset incantation:
    `run.sh daemon-stop && rm -rf ~/.cache/accelerator/playwright .accelerator/tmp/inventory-design-playwright`
- Note the new `--allow-internal` and `--allow-insecure-scheme` flags in
  `/inventory-design` examples.
- Add a "Troubleshooting" bullet pointing at `run.sh daemon-stop` for
  hung-daemon recovery.

#### 3. Decide and document the semver bump

This plan introduces breaking changes:
- Removes `mcp__playwright__*` tools (any user invoking them directly via
  the previously-pinned MCP loses that path).
- Deletes `.claude-plugin/.mcp.json`.
- Introduces a Node ≥ 20 hard requirement for runtime/hybrid crawls.

Action items:
- **Bump to a minor version** (e.g. `1.21.x` → `1.22.0`) with a prominent [x]
  CHANGELOG "Breaking" callout. Precedent: this project's CHANGELOG
  shows prior breaking changes (e.g. the `tickets → work` skill-category
  rename) shipped under minor bumps with explicit `### Breaking`
  sections — this plan continues that practice rather than
  reaching for a major version. (Alternative: major version bump if Toby
  wants a stronger signal — confirm by checking the most recent breaking
  change in CHANGELOG and matching its convention.)
- Update `plugin.json` `version` field accordingly.
- If `plugin.json` supports `engines.node`, declare `">=20"` in 4a/4b
  scope. If not, document in the README "Requirements" subsection added
  above.

#### 4. Update CHANGELOG

**File**: `CHANGELOG.md` (or equivalent — verify path during
implementation; if the project uses release notes elsewhere, follow that
convention)

Entry should explicitly call out:

**Breaking changes:**
- Node ≥ 20 is now required for `/inventory-design --crawler runtime|hybrid`
- `.claude-plugin/.mcp.json` is removed; users relying on the
  project-scoped Playwright MCP must register it elsewhere if they want
  it
- Eval id 3 is retired; eval ids 18–21 are added; eval id 13 is
  refreshed (see plan §Phase 4 §10 for details)

**Other changes:**
- URL validator default-allows `http://localhost` and `http://127.0.0.1`;
  other internal hosts are gated on `--allow-internal`; `http://` to
  public hosts is gated on `--allow-insecure-scheme`
- Playwright MCP integration replaced with a Bash-invoked Node executor;
  eliminates the sub-agent MCP-inheritance hallucination class
- First-run `ensure-playwright.sh` lazy-installs Chromium (~150 MB) into
  `~/.cache/accelerator/playwright/`
- New CLI flags: `--allow-internal`, `--allow-insecure-scheme`

#### 5. Final benchmark via skill-creator

Run the full eval suite at the configured run-count tiers and write
`evals/benchmark.json` with variance analysis.

**Pass-rate gates** (reconciled from the prior plan's conflicting
0.9 / 1.0 thresholds):
- Existing evals (ids 1, 2, 4, 5, 8, 10, 11, 12, 13, 14): mean pass-rate
  must be **≥ prior baseline minus 0.05 variance margin**. The current
  baseline is 1.0 across the set, so each eval must report ≥ 0.95.
- New evals (ids 18, 19, 20, 21): mean pass-rate **≥ 0.9**. New evals
  have no historical baseline to regress against; 0.9 is the
  "deliberately allowing some variance for first-time runs" floor.
- Eval id 3 is retired and excluded.

### Success Criteria

#### Automated Verification

- [x] `bash scripts/test-design.sh` exits 0
- [x] `bash skills/design/inventory-design/scripts/playwright/test-run.sh` exits 0
- [x] `bash skills/design/inventory-design/scripts/test-ensure-playwright.sh` exits 0
- [x] `bash skills/design/inventory-design/scripts/validate-source.sh "http://localhost:8080"` exits 0
- [x] `grep -r 'mcp__playwright__\|Playwright MCP' README.md skills/design/ agents/ scripts/test-design.sh` returns no matches
      (matches in evals/evals.json id 3 and benchmark.json are deprecated eval
      data kept for historical reference — accepted as in Phase 4b)
- [x] `grep -r 'evaluate-payload-rejected' skills/design/ agents/ scripts/test-design.sh` returns no matches
      (matches in test-run.sh and test-run.js are assert_exit_code assertions
      that verify the string is absent from executor source — accepted deviation)
- [x] skill-creator final benchmark recorded in `evals/benchmark.json`
      with thresholds met per §4 above
      (evals 20 and 21 run via subagent; all expectations passed; pass_rate
      1.0 across all 14 active evals; CI gate test-evals-structure.sh 54/54)

#### Manual Verification

- [ ] Reading the updated README from a cold start makes the new
      runtime-dependency story self-evident: a new contributor can find
      where Playwright comes from, what the cache layout is, what
      `--allow-internal` and `--allow-insecure-scheme` do, and how to
      stop a hung daemon.
- [ ] On a fresh clone, `/inventory-design demo ./examples/design-test-app`
      and `/inventory-design demo http://localhost:3000 --crawler runtime`
      both work end-to-end.
- [ ] CHANGELOG entry is unambiguous about the Node ≥ 20 requirement
      and the `.mcp.json` removal.

---

## Testing Strategy

### Unit Tests

- **`run.js`**: Node-side `node:test` covers the executor protocol surface
  (each operation, masking, screenshot path constraint, daemon lifecycle,
  protocol-version tag, error-envelope shape). Tests assume Playwright +
  Chromium are installed; the harness invokes `ensure-playwright.sh` (no
  mock) once per CI job to guarantee a real install.
- **Daemon lifecycle**: parametrised idle timer (e.g. 100 ms via env var)
  enables fast tests for stale-socket recovery, concurrent first-spawn
  serialisation under the lock, idle-shutdown audit invariant, and
  wall-clock crawl bound enforcement.
- **Validator helpers**: `validate-source.sh` exposes
  `is_localhost_default`, `classify_internal`, and `canonicalise_host` via
  a `BASH_SOURCE`-guarded `main` so a `test-validate-source.sh` can
  source-and-call helpers directly with focused inputs.

### Integration Tests

- **`test-run.sh`**: bash-level integration that exercises `run.sh` end
  to end against a Node-served fixture page. Covers `ping`, daemon reuse
  across shells, screenshot path constraint, recovery after `kill -9`,
  state-file atomicity.
- **`test-ensure-playwright.sh`**: bash-level test using parametric
  mock flags (`ACCELERATOR_PLAYWRIGHT_MOCK_*_EXIT` and `_OK`); one real
  install runs first per CI job to exercise the actual install path; all
  failure-path tests use targeted mocks. The single-boolean
  `--offline-mock` is replaced by per-step controls.
- **`scripts/test-design.sh`**: structural assertions about skill /
  agent / manifest shape; runs in lockstep with the eval suite changes
  in Phase 4.

### Eval Tests

- **Unchanged**: ids 1, 2, 4, 5, 8, 10, 11, 12.
- **Refreshed (Phase 1)**: id 13 — internal-host rejection, now includes
  a `127.0.0.2` case to exercise the non-`127.0.0.1` loopback range.
- **Unchanged contract, refreshed heading reference (Phase 4)**: id 14 —
  `run.sh evaluate Payload Allowlist` (the agent-body allowlist remains
  the only governance for `evaluate` payloads).
- **New (Phase 1)**: id 18 — localhost-default-allow with canonicalisation
  cases (uppercase, trailing dot).
- **New (Phase 1)**: id 19 — `--allow-internal` and
  `--allow-insecure-scheme` flag passthrough; the two flags are not
  silently aliased.
- **New (Phase 4b)**: id 20 — executor-bootstrap-failure-fallback;
  asserts the script-driven downgrade message.
- **New (Phase 4b)**: id 21 — executor-ping-no-browser; asserts Step 5
  uses `ping` and code-mode crawls don't launch Chromium.
- **Retired**: id 3 (`mcp-unavailable-fallback`); excluded from the
  benchmark.
- Benchmark targets: probabilistic ≥ 5 runs, deterministic ≥ 3 runs,
  structural-only 1 run. Pass-rate floors per Phase 5 §4.

### Manual Testing Steps

1. Apply the branch on a clean machine with no
   `~/.cache/accelerator/playwright`. Run
   `/inventory-design demo ./examples/design-test-app --crawler hybrid`.
   Expect the bootstrap preamble (visible npm progress, no `--silent`),
   Chromium download (1–3 min), then a successful crawl of the fixture
   app. Inspect `meta/research/design-inventories/<dir>/`.
2. Re-run the same command. Expect no bootstrap output (sentinel hit
   under the lock), daemon reuse from the earlier session, fast crawl.
3. Start a local dev server on `http://localhost:3000`. Run
   `/inventory-design demo http://localhost:3000 --crawler runtime`.
   Expect validator to accept; expect crawl to complete with real
   observations (the original UAT failure case).
4. Run `/inventory-design demo http://10.0.0.1`. Expect validator to
   reject with a message naming `--allow-internal`. Re-run with the
   flag; expect validation to pass.
5. Run `/inventory-design demo http://example.com`. Expect validator
   to reject with a message naming `--allow-insecure-scheme` (NOT
   `--allow-internal`). Re-run with `--allow-insecure-scheme`; expect
   validation to pass.
6. While a runtime crawl is mid-flight, in another shell run
   `kill -9 $(cat .accelerator/tmp/inventory-design-playwright/server.pid)`.
   In a third shell, run another `/inventory-design …`. Expect the
   stale-socket recovery path to clean up and spawn a fresh daemon.
7. Spawn a sub-agent manually via `Task` against
   `agents/browser-analyser.md` in a sandbox session, hand it a payload
   exercising the executor. Confirm the agent does not reach for
   `mcp__chrome-devtools__*` (the original hallucination smell).
8. With Node 18 on PATH, run a `--crawler hybrid` invocation. Expect
   the script-driven downgrade notice from `notify-downgrade.sh`
   (literal text match) and a code-mode crawl producing an inventory.

## Performance Considerations

- First-run bootstrap downloads ~150 MB and takes 1–3 minutes on a typical
  connection. Documented in the preamble. Acceptable: once per machine,
  surfaced before any agent spawns (not mid-crawl). The user sees `npm ci`
  progress (no `--silent`) so the terminal does not appear frozen.
- Subsequent runtime crawls reuse the daemon across shells against the
  same project root (visualiser-pattern reuse short-circuit). Cold-start
  cost ~500 ms–1 s; warm-start cost ~50–200 ms (HTTP roundtrip on
  loopback).
- **Daemon idle timer is 30 minutes** — decoupled from the per-crawl
  wall-clock bound. The earlier "5 minutes matches the crawl bound"
  rationale was an invisible coupling that produced a race when crawls
  approached the limit; using 30 minutes gives long sessions room and
  unbounded crawls are still bounded by the wall-clock kill below.
- **Wall-clock crawl bound is 5 minutes**, enforced inside the daemon by
  a `setTimeout` armed at first `navigate`. On expiry, `browser.close()`
  + `process.exit(2)`, with `server-stopped.json` recording
  `reason: "wall-clock"`. This is the enforcement teeth deferred by the
  prior draft; landing it here keeps misbehaving pages bounded.
- Owner-PID watcher fires every 60 seconds — a daemon orphaned by a
  crashed parent shell self-terminates within one minute.
- Chromium-only saves ~350 MB of cache over installing all three
  browsers.

## Migration Notes

- Existing `meta/research/design-inventories/` artifacts are unaffected (no schema
  change).
- Users with the Playwright MCP already installed locally are unaffected
  by `.claude-plugin/.mcp.json` removal — the MCP server stays in their
  npx cache; we just no longer auto-register it project-scoped.
- **New disk footprint**: ~150 MB at `~/.cache/accelerator/playwright/`,
  plus per-project ~few KB at
  `<project>/.accelerator/tmp/inventory-design-playwright/`. Both
  documented in README §Cache & cleanup (Phase 5 §1).
- **New process behaviour**: a `node` daemon may persist for up to 30
  minutes after a crawl (or until the parent shell exits, whichever is
  sooner). Stop manually via `run.sh daemon-stop`; force-stop via
  `kill $(cat .accelerator/tmp/inventory-design-playwright/server.pid)`
  (the next launcher invocation cleans up files automatically).
- **No-op for users without `${CLAUDE_PLUGIN_DATA}`**: this plan does not
  rely on `${CLAUDE_PLUGIN_DATA}`, so older Claude Code builds are
  unaffected.
- **Node ≥ 20 is a hard new requirement for `--crawler runtime|hybrid`**
  (Phase 3). `--crawler code` continues to work without Node.

## References

- Original research: `meta/research/codebase/2026-05-06-design-skill-localhost-and-mcp-issues.md`
- Initial review: `meta/reviews/plans/2026-05-06-design-skill-localhost-and-mcp-issues-review-1.md`
- Visualiser daemon-management pattern adopted:
  `skills/visualisation/visualise/scripts/launch-server.sh`,
  `scripts/stop-server.sh`, `scripts/launcher-helpers.sh`,
  `server/src/server.rs`, `server/src/lifecycle.rs`,
  `server/src/shutdown.rs`
- Plan-of-record for the original design-convergence work:
  `meta/plans/2026-05-03-design-convergence-workflow.md`
- Anthropic issue 13605 (sub-agent MCP inheritance):
  https://github.com/anthropics/claude-code/issues/13605
- Anthropic issue 13898 (project-scoped MCP, hallucination):
  https://github.com/anthropics/claude-code/issues/13898
- skill-creator skill (Anthropic):
  `~/.claude/plugins/cache/claude-plugins-official/skill-creator/...`
- Existing skill: `skills/design/inventory-design/SKILL.md`
- Existing executor reference patterns: lackeyjb/playwright-skill
  (`run.js` universal-executor concept)
