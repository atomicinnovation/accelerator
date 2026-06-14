---
type: plan
id: "2026-06-15-0048-linear-integration"
title: "Linear Integration Implementation Plan"
date: "2026-06-14T23:37:32+00:00"
author: "Toby Clemson"
producer: create-plan
status: ready
work_item_id: "work-item:0048"
parent: "work-item:0048"
derived_from: ["codebase-research:2026-06-14-0048-linear-integration-apis"]
tags: [work-management, integrations, linear, graphql]
revision: "0df1144f5279aa68e84685405ea57e0f0de984e5"
repository: "ticket-management"
last_updated: "2026-06-15T08:51:06+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Linear Integration Implementation Plan

## Overview

Implement eight skills under `skills/integrations/linear/` covering the Linear
issue lifecycle against Linear's GraphQL API, structurally mirroring the
existing `skills/integrations/jira/` integration. The skills are
`init-linear`, `search-linear-issues`, `show-linear-issue`,
`create-linear-issue`, `update-linear-issue`, `transition-linear-issue`,
`comment-linear-issue`, and `attach-linear-issue`, backed by three shared
helpers (`linear-graphql.sh`, `linear-auth.sh`, `linear-common.sh`), a
dedicated `EXIT_CODES.md` namespace, a GraphQL mock-server test harness, and
one `plugin.json` directory registration.

The build follows the Jira four-phase shape, each phase independently
mergeable, with tests written before implementation (TDD).

## Current State Analysis

The Jira integration is the proven template and was read end-to-end for this
plan. Concrete anchors:

- **Transport** — `skills/integrations/jira/scripts/jira-request.sh` is an
  executable (`set -euo pipefail`) whose contract is *body → stdout, errors →
  stderr, outcome via exit code*. Credentials never reach argv: a
  `curl --config -` directive is piped from stdin (`jira-request.sh:330-333`).
  A 4-attempt retry loop handles `429|5xx` with `Retry-After`/backoff clamped
  to `[1,60]s` and ±30% jitter (`jira-request.sh:322-432`). HTTP status maps to
  exit codes at `jira-request.sh:348-428`. A sleep test-seam
  (`JIRA_RETRY_SLEEP_FN`) is gated on `ACCELERATOR_TEST_MODE=1`
  (`jira-request.sh:37-63`).
- **Auth** — `jira-auth.sh` is a sourced library with a single entrypoint
  `jira_resolve_credentials` (`:132-250`). 4-tier token precedence (env →
  env_cmd → `config.local.md` → shared `config.md` token-only)
  (`:165-233`), a shared-config `token_cmd` ban (`:221-233`), and a 0600
  fail-closed permission gate with symlink rejection (`:184-208`).
- **Common** — `jira-common.sh` provides `jira_state_dir` (config-driven
  integrations path → `<root>/.accelerator/state/integrations/jira`,
  `:69-87`), `jira_atomic_write_json` (`:98-113`), `jira_with_lock`
  (mkdir lock + PID-start-time stale reclaim, `:140-221`),
  `jira_require_dependencies`, and `_jira_emit_generic_hint` (`:285-296`).
  `JIRA_INNER_GITIGNORE_RULES=(site.json .refresh-meta.json .lock/)`
  (`:53-57`) keeps per-developer/transient files out of git.
- **Flows** — eight uniform `jira-*-flow.sh` orchestrators: one `_<verb>`
  function, `[[ "${BASH_SOURCE[0]}" == "${0}" ]]` source-guard, flag parse →
  validate with stable `E_*` codes → `jq -n` conditional-merge body → call
  transport as subprocess → propagate exit code. Dry-run via
  `--print-payload` (create/update/comment) or `--describe` (transition/attach).
- **SKILL.md archetypes** — read (`search`, `show`):
  `disable-model-invocation: false`, narrowly-scoped `allowed-tools`
  (`search-jira-issues/SKILL.md:13-18`). Write (the rest):
  `disable-model-invocation: true`, `allowed-tools: [Bash, Read, Write]`, a
  "write skill with irreversible side effects" description, and a preview →
  confirm → send gate (`create-jira-issue/SKILL.md`). Every SKILL injects live
  context via the `!` preprocessor (`config-read-context.sh`,
  `config-read-skill-context.sh <name>`, `config-read-skill-instructions.sh
  <name>`).
- **Exit codes** — `skills/integrations/jira/scripts/EXIT_CODES.md` owns the
  Jira namespace. There is **no** top-level shared `scripts/EXIT_CODES.md`;
  each integration owns its own.
- **Test harness** — `test-helpers/mock-jira-server.py` is a sequenced
  expectation server: it pops one expectation per request, matches method+path
  (query string ignored), optionally captures request bodies/URLs, validates
  auth + custom headers, and shuts down after the last expectation. Scenario
  fixtures live under `test-fixtures/scenarios/*.json`. Tests are standalone
  bash, sourcing `scripts/test-helpers.sh`, launching the mock and pointing the
  transport at it via `ACCELERATOR_<X>_BASE_URL_OVERRIDE_TEST` under
  `ACCELERATOR_TEST_MODE=1` (`test-jira-request.sh:84-90`). A file-based sleep
  counter records backoff durations without sleeping (`:96-114`).
- **Registration** — `.claude-plugin/plugin.json:16` lists
  `"./skills/integrations/jira/"`; the loader discovers all SKILL.md files
  beneath it.

There is **no Linear integration today**, and **no generic frontmatter
field-writer** in `scripts/` — only readers (`config_extract_frontmatter` in
`config-common.sh`). The `work_item_id` writeback therefore needs a small new
atomic single-field replacer.

## Desired End State

`skills/integrations/linear/` ships eight working skills plus shared helpers,
mirroring the Jira footprint minus the ADF subsystem (Linear is Markdown-native).
`/init-linear` authenticates, lets the user pick one team, and persists a
team + WorkflowState catalogue. The seven verb skills query and mutate Linear
issues against that catalogue, with `create-linear-issue` writing the
remote-allocated identifier back into the local work-item file. Every Linear
skill handles Linear's HTTP-400 RATELIMITED and complexity-cap responses with
distinct exit codes, and `linear-graphql.sh` transparently paginates Relay
cursor connections. `mise run` (full local CI mirror) exits 0.

Verification: every acceptance criterion in `meta/work/0048-linear-integration.md`
is exercised by an automated bash test against the mock server, and a manual
end-to-end pass against a real Linear workspace succeeds.

### Key Discoveries

- Token-out-of-argv via `curl --config -` (`jira-request.sh:330-333`) — for
  Linear the directive becomes
  `printf 'header = "Authorization: %s"\n' "$LINEAR_TOKEN"` (no Bearer prefix
  for personal `lin_api_` keys). Because the token is embedded inside a
  **quoted** config directive (unlike Jira's curl-encoded `user =`), a token
  containing `"`, `\`, or a newline would terminate or inject the directive;
  `linear_resolve_credentials` therefore **rejects** tokens containing control
  characters, quotes, or newlines before use (a malformed-token guard with no
  Jira precedent).
- GraphQL errors arrive in an **HTTP-200 `errors[]` body** (or HTTP 400 for
  rate/complexity/validation), so `linear-graphql.sh` must inspect the body,
  not just the status line — the one structural addition over Jira's pure
  status dispatch.
- **RATELIMITED and complexity-cap are both HTTP 400** and the complexity
  rejection most likely carries the **same** `extensions.type == "ratelimited"`
  as a throttle — Linear publishes **no distinct machine-readable code** for the
  10,000-point cap (confirmed against Linear's `@linear/sdk` `error.ts` error
  map and the official rate-limiting docs). The only reliable discriminator is
  **message-substring** (`"complexity"` / `"10,000"`), and it must be checked
  **before** the RATELIMITED code check or the two collapse.
- `X-RateLimit-Requests-Reset` is **epoch milliseconds**; backoff seconds =
  `reset_ms/1000 − now_s` where `now_s = $(date +%s)` (**second** granularity —
  BSD/macOS `date` has no GNU `%N`, so millisecond `now` is non-portable, and
  the ±2s test tolerance permits seconds). The result is clamped to
  **`[1,60]s`** (mirroring Jira's `jira-request.sh`), **not** the raw
  requests-reset and **never** the hourly complexity-reset — an unclamped or
  "later-of-two-resets" backoff could otherwise sleep for minutes-to-an-hour
  per attempt.
- WorkflowState `id`s are **team-scoped UUIDs** — exactly what `init-linear`
  caches and `transition-linear-issue` resolves against (no live lookup).
- `work_item_id` writeback (`create-linear-issue`) and dual attach
  (`attach-linear-issue`) have **no Jira precedent** — net-new design.
- **bash 3.2 floor** (macOS): no associative arrays, no `${var,,}`. The
  case-insensitive discriminator matching must use `[[ … =~ ]]` with explicit
  alternation or `tr`, never `${var,,}`.

## What We're NOT Doing

- **OAuth2 / `Bearer` tokens.** `linear-graphql.sh` hard-codes the no-prefix
  `Authorization` header for personal `lin_api_` keys. OAuth is out of scope.
- **Automatic complexity-cap halving / `first`-downshift retry.** The
  complexity cap is treated as a **terminal** error (dedicated exit code, no
  partial result set), per the work item. The paginator uses a fixed default
  `first: 50`. **Invariant**: paginated queries must keep their selection well
  under the 10,000-point cap at `first: 50` — the lean selections in this plan
  satisfy this, but a future richer selection (nested comments/attachments
  connections) could breach it and turn a working query into a terminal
  failure. The designated evolution path if selections grow is to add a
  `--first` downshift seam; this door is closed deliberately, not by oversight.
- **The Jira ADF subsystem.** Linear is Markdown-native; no
  tokeniser/assembler/renderer.
- **Per-invocation `--team` flag / multi-team scoping.** A single team is
  chosen at `init-linear` time and stored in the catalogue.
- **VCS-aware issue-id detection** (branch-name / jj-trailer parsing from
  `schpet/linear-cli`) — a nice-to-have, not in this work item.
- **Comment edit/delete and `bodyData` (Prosemirror).** Only `commentCreate`
  with Markdown `body` is in scope.
- **0047 (Core Skills Sync Integration).** This plan *consumes* the
  numeric-`work_item_id`-means-unsynced contract; it does not redefine it.
  **Hard sequencing dependency**: current work-item files carry `id:`, not
  `work_item_id:` — the latter field is introduced by 0047. `create-linear-issue`
  reads and writes `work_item_id` and `config_set_frontmatter_field` fails
  closed on an absent field, so the create skill cannot function against
  existing files until 0047 has shipped the `work_item_id` field. The create
  flow's fixtures therefore use post-0047 files that already carry the field,
  and the create skill must not be relied upon before 0047 lands.

## Implementation Approach

Mirror the Jira integration's proven shapes verbatim wherever the API allows,
diverging only at the four points the research identified: (1) GraphQL error
bodies + RATELIMITED/complexity discrimination, (2) pagination folded into the
transport, (3) `work_item_id` writeback, (4) cache-resolved transitions + dual
attach. Each phase is test-first: author scenario fixtures and a `test-*.sh`
suite that encodes the relevant acceptance criteria, watch them fail, then
implement the helper/flow/SKILL until green. Each phase ends with
`mise run scripts:check` green and is independently mergeable.

---

## Phase 1: Foundation + `init-linear`

### Overview

Build the shared transport/auth/common helpers, the exit-code namespace, the
GraphQL test harness, the plugin registration, and the bootstrap `init-linear`
skill. Delivers a working `/init-linear` and a fully tested transport that
every later phase depends on. Also patches the work item's rate-limit figure.

### Changes Required

#### 1. Exit-code namespace

**File**: `skills/integrations/linear/scripts/EXIT_CODES.md` (new)
**Changes**: Define the Linear namespace, mirroring Jira's transport-code
shape and adding two Linear-specific transport codes.

- Transport (`linear-graphql.sh`):
  - `11` `E_GQL_UNAUTHORIZED` — auth failure (HTTP 401 or GraphQL
    `extensions.type == "authentication error"`)
  - `16` `E_GQL_BAD_RESPONSE` — non-JSON body on HTTP 200
  - `18` `E_TEST_OVERRIDE_REJECTED` — base-URL override refused (gate
    `ACCELERATOR_TEST_MODE=1`)
  - `20` `E_GQL_SERVER_ERROR` — HTTP 5xx (retries exhausted)
  - `21` `E_GQL_CONNECT` — connection/DNS/timeout
  - `22` `E_GQL_NO_CREDS` — no resolvable token
  - `23` `E_TEST_HOOK_REJECTED` — `LINEAR_RETRY_SLEEP_FN` refused
  - `34` `E_GQL_BAD_REQUEST` — HTTP 400 GraphQL error that is *neither* rate
    limit, complexity, **nor** authentication (validation / bad query). Note:
    Jira's per-status codes for 403/404/410 are deliberately **not** mirrored —
    GraphQL returns these as HTTP-200/400 `errors[]` bodies, so they collapse
    into `E_GQL_BAD_REQUEST` / `E_GQL_UNAUTHORIZED`; this divergence is recorded
    in `EXIT_CODES.md`.
  - `35` `E_GQL_RATELIMITED` — HTTP 400 + `extensions.code == "RATELIMITED"`,
    retries exhausted
  - `36` `E_GQL_COMPLEXITY` — single-query complexity cap (10,000 points)
    exceeded
- Auth (`linear-auth.sh`): `24` `E_NO_TOKEN`, `25` `E_TOKEN_CMD_FAILED`,
  `26` `E_TOKEN_CMD_FROM_SHARED_CONFIG` (stderr warning only), `27`
  `E_TOKEN_MALFORMED` (token contains control chars / quotes / newlines —
  would corrupt the `curl --config -` directive), `29`
  `E_LOCAL_PERMS_INSECURE`. (No site/email codes — Linear resolves token only.)
- Common (`linear-common.sh`): `53` `E_REFRESH_LOCKED`.
- Flow ranges (disjoint, mirroring Jira): init `60-62`, search `70-73`,
  show `80-82`, comment `90-96`, create `100-109`, update `110-117`,
  transition `120-126`, attach `130-139`. Each flow declares its codes as
  `readonly E_*=NN` constants near the top so `return $E_*` sites read
  symbolically — the **constants are the source of truth** and `EXIT_CODES.md`
  is derived documentation (this is an intentional idiom upgrade over Jira's
  bare-literal `return 100`; recorded so the divergence is deliberate, and the
  Jira flows may adopt it later). `EXIT_CODES.md` notes the deliberate
  per-namespace divergences from Jira: the transport band (11-23) keeps
  positional parity for cross-integration reading, but `27` is
  `E_TOKEN_MALFORMED` (Jira's `27`/`28` are site/email, which Linear lacks), and
  the flow bands (create/update/transition/attach) reuse Jira's range
  boundaries while assigning code *meanings* independently per the differing
  skill semantics — so readers do not assume per-number parity outside the
  transport band. To keep the derived doc honest, a lightweight check in
  `test-linear-*.sh` greps each flow's `readonly E_*=NN` declarations and
  asserts each appears with the same value in `EXIT_CODES.md` (mirroring the
  gitignore-rules equality assertion). `EXIT_CODES.md` also carries Jira's
  "gaps within ranges are reserved" note (the vacated transport slots
  12-15/17/19 correspond to Jira-only HTTP-status codes) and annotates `26`
  `E_TOKEN_CMD_FROM_SHARED_CONFIG` as "stderr warning only; not a fatal exit
  code", matching the Jira table.
- A test-seam table mirroring Jira's: `ACCELERATOR_TEST_MODE`,
  `ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST`, `LINEAR_RETRY_SLEEP_FN`,
  `LINEAR_LOCK_TIMEOUT_SECS`, `LINEAR_LOCK_SLEEP_SECS`.

#### 2. Common helper

**File**: `skills/integrations/linear/scripts/linear-common.sh` (new)
**Changes**: Port `jira-common.sh`, dropping UUID/ADF. Provide:
- `linear_state_dir` → `<root>/.accelerator/state/integrations/linear` (via
  `config-read-path.sh integrations`, append `/linear`, `mkdir -p`).
- `LINEAR_INNER_GITIGNORE_RULES=(viewer.json .refresh-meta.json .lock/)` —
  `viewer.json` (per-developer viewer id) is gitignored; `catalogue.json`
  (team + states, team-scoped not user-scoped) is **committed**.
- `linear_atomic_write_json`, `linear_with_lock` (mkdir + PID-start reclaim,
  `LINEAR_LOCK_*` seams), `linear_require_dependencies` (jq ≥ 1.6, curl, awk),
  `linear_jq_field`, `_linear_emit_generic_hint`.

The `work_item_id` writeback is **not** placed here — it is a cross-cutting
frontmatter-mutation concern (0047 and a future Jira-side writeback want the
identical operation) and is added to shared `scripts/` instead (see item 8).
Note also that `linear_with_lock` guards only the state directory; the
create-time writeback mutates a `meta/work/` file outside that lock. This is
acceptable for an interactive single-developer tool — the read→transform→
validate→atomic-rename window is kept minimal — but the limitation is recorded
rather than papered over with heavyweight cross-file locking.

#### 3. Auth helper

**File**: `skills/integrations/linear/scripts/linear-auth.sh` (new)
**Changes**: Port `jira-auth.sh`'s `_jira_read_field_from_file`, perm-gate,
`token_cmd` ban, and runner verbatim. Public entrypoint
`linear_resolve_credentials` sets **only** `LINEAR_TOKEN` (+
`LINEAR_RESOLUTION_SOURCE_TOKEN`) in the caller's scope, reading a `linear:`
config section. 4-tier precedence: `ACCELERATOR_LINEAR_TOKEN` →
`ACCELERATOR_LINEAR_TOKEN_CMD` → `config.local.md` token/token_cmd (0600 gate)
→ shared `config.md` token only (token_cmd banned). No site/email. The
malformed-token guard runs on the **final resolved `LINEAR_TOKEN` for every
tier** (including `token_cmd` output, **after** the runner's trailing-whitespace
trim) immediately before it can reach the `curl --config -` directive: reject
any value containing control characters, double-quotes, backslashes, or
newlines with `E_TOKEN_MALFORMED` (27), since the token sits inside a quoted
config string. (Test both an env token and a `token_cmd` emitting an embedded
newline.)

#### 4. Transport

**File**: `skills/integrations/linear/scripts/linear-graphql.sh` (new,
executable)
**Changes**: Port `jira-request.sh`'s structure (temp files, retry loop, sleep
seam, base-URL override gate) with these Linear adaptations:

```
# Usage:
#   linear-graphql.sh --query @file|<inline> [--variables @file|<inline>]
#                     [--paginate <jq-connection-path>]
```

- Fixed endpoint `https://api.linear.app/graphql`, always
  `POST {query, variables}`, `Content-Type: application/json`.
- Header built here: `Authorization: <LINEAR_TOKEN>` (no Bearer), via
  `curl --config -`.
- **Error dispatch** (the divergence) — classification lives in a single
  isolated function `_linear_classify_gql_error` (one edit point; the
  load-bearing ordering rationale documented in its header comment) that
  returns a symbolic class. The complexity discriminator substring lives in
  **one labelled constant** (`LINEAR_COMPLEXITY_PATTERN`) so the known-fragile
  heuristic has a single, test-pinned home.
  - 2xx: parse body. No top-level `errors[]` → emit body, exit 0. With
    `errors[]`, run the classifier (auth → complexity → bad-request). A
    200-body error is **terminal** — it is **not** routed into the retrying
    RATELIMITED path (RATELIMITED is documented as HTTP 400 only; retrying a
    200 risks re-issuing a non-idempotent mutation).
  - 401 (status line) → `E_GQL_UNAUTHORIZED` (11).
  - HTTP 400: run the classifier in this order (the order is load-bearing):
    1. **Auth** (first, so a body-borne auth error never falls through to
       bad-request): if any `errors[].extensions.type` is
       `authentication error` (or `.code` is `AUTHENTICATION_ERROR`) →
       `E_GQL_UNAUTHORIZED` (11).
    2. **Complexity**: else if any `errors[].message` matches
       `LINEAR_COMPLEXITY_PATTERN` (requires the full word `complexity` — not
       the bare `complex` stem, to avoid matching an unrelated "complex query"
       phrasing; the bare `10,?000` numeric counts only as corroboration
       alongside it, so a *rate-limit* message that happens to contain `10,000`
       does not false-positive into the terminal path; case-insensitive via
       `[[ =~ ]]` —
       never `${var,,}`) → `E_GQL_COMPLEXITY` (36); print a message naming the
       10,000-point limit; emit **no** partial result. Pinned by fixtures in
       both directions (a rate-limit message containing `10,000` must classify
       as RATELIMITED, not complexity).
    3. **Rate limit**: else if any `errors[].extensions.code` (or `.type`)
       equals `RATELIMITED`/`ratelimited` → rate-limit path: compute backoff
       from `X-RateLimit-Requests-Reset` (`reset_ms/1000 − $(date +%s)`, clamped
       to **`[1,60]s`**). **Fallback**: if the reset header is absent, empty, or
       non-numeric, fall back to exponential backoff with jitter (also clamped
       `[1,60]s`, mirroring `jira-request.sh`) — never the bare subtraction,
       which would otherwise clamp up to a 1s tight-loop retry. Retry up to 4
       attempts; on exhaustion `E_GQL_RATELIMITED` (35).
    4. **Else** → `E_GQL_BAD_REQUEST` (34). **Fail-safe**: any 400 error
       matching neither a confirmed auth/RATELIMITED code nor the complexity
       heuristic takes this **non-retried** path, so an unrecognised error (or
       a complexity-message wording drift) never spins the retry loop.
  - 5xx → retry → `E_GQL_SERVER_ERROR` (20); connect failure → 21;
    no creds → 22.
  - On any classified error, emit only `errors[].message` /
    `errors[].extensions.code` to stderr rather than the whole raw body, so a
    verbose backend error or request-echoed input is not blanket-surfaced (the
    token is header-only, never in the query/variables, so it cannot leak this
    way).
- **Pagination** (`--paginate <path>`): isolated in its own function
  `_linear_paginate` — the per-request retry loop lives strictly *inside* each
  single request, and the pagination loop wraps it, keeping the single-request
  transport contract intact. The query must select the named connection with
  `nodes`, `pageInfo { hasNextPage endCursor }`, and accept a `$cursor`
  variable. The loop issues the query with the current cursor, appends
  `<path>.nodes`, and follows `<path>.pageInfo.endCursor` while `hasNextPage`,
  with these safety bounds:
  - **`MAX_PAGES`** cap (mirroring `jira-comment-flow.sh`'s `MAX_PAGES=20`) so a
    pathological connection cannot loop unboundedly.
  - **Break** when `hasNextPage` is true but `endCursor` is null/empty or equal
    to the previous cursor (non-advancing cursor → no infinite loop / no
    unbounded node accumulation). The previous-cursor variable starts unset/
    sentinel; the equality check applies only from the second iteration, so the
    first request relies solely on the null/empty-`endCursor` guard.
  - **Incompleteness signalling**: if the loop terminates while `hasNextPage`
    is still true — i.e. the `MAX_PAGES` cap was hit, or the non-advancing-cursor
    guard fired — the synthesized response carries `truncated: true` (and a
    `WARN:` to stderr naming the page count), **not** `hasNextPage: false`, so a
    truncated result is never indistinguishable from a complete one (mirroring
    `jira-comment-flow.sh`'s `truncated` flag). Silent truncation-as-complete is
    a correctness defect this guard exists to prevent.
  - Emit a single synthesized response with all nodes merged under `<path>` —
    **always a JSON array**, defaulting to `[]` for an empty connection — with
    `hasNextPage: false` and `truncated: false` **only** when the connection was
    genuinely exhausted. A first page that already has `hasNextPage: false`
    (including a zero-result page) is emitted as-is.
  - **Partial-page failure**: if any page after the first fails (rate-limit
    exhausted, bad request, etc.), the whole paginate **fails with that page's
    exit code and emits no partial result** — accumulated nodes are discarded,
    matching the complexity path's no-partial-result invariant.
  Default page size lives in the caller's query (`first: 50`).
- **Test seams**: `ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST` (loopback-only),
  `LINEAR_RETRY_SLEEP_FN` (`^_?test_[a-z_]+$`, gated).

#### 5. init-linear flow + skill

**File**: `skills/integrations/linear/scripts/linear-init-flow.sh` (new)
**Changes**: Port `jira-init-flow.sh` structure. Subcommands:
- `verify` — `linear_resolve_credentials`, query `viewer { id name }`, persist
  `viewer.json` (`{id, name}`); fail `E_INIT_VERIFY_FAILED` (61) on no id.
  Detect a `Bearer`-prefixed token failing auth and surface it as an
  authentication failure (AC: Bearer prefix → non-zero + auth message).
- `list-teams` — query `teams { nodes { id name key } }`, print as JSON (for
  selection).
- `discover` (inside `linear_with_lock`) — given the selected team, query
  `team(id:$id){ id name key states { nodes { id name type position } } }` and
  persist `catalogue.json` = `{team:{id,key,name}, workflowStates:[{id,name,
  type,position}]}`. Fail `E_INIT_NO_TEAM` (62) if the team is not found or has
  no states.
- Full flow: `verify` → team selection → `discover`. Writes the inner
  `.gitignore` (`LINEAR_INNER_GITIGNORE_RULES`) and `.gitkeep` like Jira.
  `--non-interactive` fails fast with `E_INIT_NEEDS_CONFIG` (60).

**File**: `skills/integrations/linear/init-linear/SKILL.md` (new)
**Changes**: `disable-model-invocation: true`, with **narrowly-scoped**
`allowed-tools` matching `init-jira/SKILL.md` (its direct analogue):
`Bash(${CLAUDE_PLUGIN_ROOT}/skills/integrations/linear/scripts/*)`,
`Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)`, `Bash(jq)`, `Bash(curl)` — not
the broad `[Bash, Read, Write]` archetype (that is reserved for the verb
write-skills that genuinely Read/Write local work-item files). Steps: resolve
creds → `verify` →
present teams from `list-teams` → user picks one → `discover` → confirm
catalogue persisted. Standard `!` preprocessor injection + bare-path caveat.

#### 6. Registration + work-item patch

**File**: `.claude-plugin/plugin.json`
**Changes**: Add `"./skills/integrations/linear/"` immediately after the jira
entry (line 16).

**File**: `meta/work/0048-linear-integration.md`
**Changes**: (a) Patch the rate-limit figure `2,500` → `5,000` requests/hr at
lines 154 and 214 (documentation correction; the figure is server-enforced and
never appears in code). (b) Correct the stale references to a shared
`scripts/EXIT_CODES.md` (work item "Patterns to mirror" / "Shared artefact"
sections, ~lines 168 and 241) to point at the per-integration
`skills/integrations/linear/scripts/EXIT_CODES.md` — no top-level shared
exit-code file exists; each integration owns its own namespace.

#### 7. Test harness

**Files** (new):
- `skills/integrations/linear/scripts/test-helpers/mock-linear-server.py` —
  port `mock-jira-server.py`. **Extension**: since every request is
  `POST /graphql`, add `expect_body_contains` matching (a substring or list of
  substrings the request body must contain) so sequenced expectations can
  target specific queries/mutations. A mismatch is a **hard failure** — it
  pushes to `server.errors` (which already forces a non-zero exit at shutdown)
  *and* the request is answered with a non-2xx marker — so a flow that sends
  the wrong operation, in the wrong order, or omits a step cannot pass merely
  by positional consumption. **Captured-errors readback**: the inherited
  `stop_mock` SIGTERMs the mock and `wait … || true` *discards* its exit code,
  so the shutdown `sys.exit(1)` is never observed by the suite. The extension
  therefore has the mock write `server.errors` to a `--captured-errors-file` on
  shutdown, and each scenario asserts that file is empty after the flow — not
  the discarded process exit status. **Header capture**: add `captured_headers`
  (a sibling to the existing `captured_bodies`/`captured_urls`) so a test can
  assert which request headers a step actually sent — required for the binary
  PUT's no-`Authorization` / dropped-disallowed-header assertions (Phase 4).
  Keep ordered consumption for pagination sequences.
- `skills/integrations/linear/scripts/test-fixtures/scenarios/*.json` — at
  minimum: `viewer-200`, `teams-200`, `team-states-200`, `bearer-401`,
  `graphql-auth-error-200` (`extensions.type == "authentication error"` in a
  200 body → exit 11), `ratelimited-400-then-200` (+ reset header),
  `ratelimited-exhausted`, `complexity-400`, `bad-request-400` (a generic 400
  with **no** complexity/ratelimited wording → exit 34, the discriminator's
  negative control), `bad-request-mentions-10000` (a RATELIMITED-coded 400
  whose message contains `10,000` → must classify as RATELIMITED 35, **not**
  complexity 36 — the discriminator's other-direction control),
  `ratelimited-400-no-reset-header` (missing reset header → exponential-backoff
  fallback, not a tight loop), `paginate-3x50` (3 sequenced pages, 150 nodes
  total), `paginate-zero` (empty connection → `nodes: []`), `paginate-runaway`
  (perpetual `hasNextPage:true` / non-advancing cursor → terminates at
  `MAX_PAGES` with `truncated: true` and a bounded node count),
  `graphql-errors-200` (a non-auth 200-body error → terminal, no retry, message
  on stderr).
- `skills/integrations/linear/scripts/test-linear-common.sh` (incl. a
  stale-lock-reclaim case and a lock-timeout → `E_REFRESH_LOCKED` (53) case),
  `test-linear-auth.sh` (incl. a malformed-token → `E_TOKEN_MALFORMED` (27)
  case for a token containing `"` and a newline), `test-linear-graphql.sh`,
  `test-linear-init-flow.sh`, `test-linear-paths.sh` (gitignore-rules
  assertion), and `test-config.sh` coverage for the new shared
  `config_set_frontmatter_field` (see item 8). Reuse `scripts/test-helpers.sh`
  and the file-based sleep counter.

#### 8. Shared frontmatter field setter

**File**: `scripts/config-common.sh` (extend)
**Changes**: Add a generic `config_set_frontmatter_field <file> <key> <value>`
as a sibling to the existing reader `config_extract_frontmatter`. This is the
cross-cutting writeback primitive (0047 and any future Jira-side writeback want
the identical operation, so it lives in shared `scripts/`, not in
`linear-common.sh`). Contract:

- Operates **only** inside the frontmatter block (between the first two `---`
  delimiters) via awk anchored to that range — never the Markdown body.
- Replaces exactly the named field's line; **fails closed** (non-zero, file
  untouched) if the field is absent or matched more than once.
- Pipes through `atomic_write`, and **before** the rename re-extracts the
  frontmatter from the candidate output and verifies it still parses and now
  carries the expected value — the integrity check `atomic_write` itself does
  not perform (analogous to `jira_atomic_write_json`'s `jq empty`).
- Treats `<value>` as literal data (no awk/regex metacharacter interpretation),
  so a value containing `&`, `/`, or `\` cannot corrupt the substitution.

Covered by new cases in `scripts/test-config.sh`: happy overwrite, missing
field (fails closed), special-character value, quoted-value, and a
byte-identical-remainder assertion (only the target line changes).

### Success Criteria

#### Automated Verification

- [ ] Transport tests pass: `bash skills/integrations/linear/scripts/test-linear-graphql.sh`
- [ ] Auth tests pass: `bash skills/integrations/linear/scripts/test-linear-auth.sh`
- [ ] Common tests pass: `bash skills/integrations/linear/scripts/test-linear-common.sh`
- [ ] init-flow tests pass: `bash skills/integrations/linear/scripts/test-linear-init-flow.sh`
- [ ] Path/gitignore tests pass: `bash skills/integrations/linear/scripts/test-linear-paths.sh`
- [ ] `paginate-3x50` scenario returns all 150 nodes in a single result set and
      stops when `hasNextPage` is `false` (asserted in `test-linear-graphql.sh`)
- [ ] `paginate-zero` (empty connection) returns `nodes: []` (a JSON array, not
      null) and exits 0 (asserted in `test-linear-graphql.sh`)
- [ ] `paginate-runaway` terminates at `MAX_PAGES` with `truncated: true`, a
      bounded node count, and a `WARN:` on stderr (never `hasNextPage: false`)
- [ ] RATELIMITED 400 yields exit `35`, a message naming the rate limit, and a
      backoff within ±2s of `reset_ms/1000 − now_s` for a reset 30000 ms ahead,
      **clamped to `[1,60]s`** (asserted via the sleep-seam counter)
- [ ] `ratelimited-400-then-200` resumes: exit `0`, the 200 body on stdout, and
      exactly **one** recorded sleep (the happy retry path)
- [ ] `ratelimited-400-no-reset-header` falls back to exponential backoff
      (clamped `[1,60]s`), not a sub-1s tight loop (sleep-seam assertion)
- [ ] `bad-request-mentions-10000` (RATELIMITED-coded 400 whose message
      contains `10,000`) classifies as RATELIMITED (35), **not** complexity (36)
- [ ] complexity 400 yields exit `36`, a message naming the 10,000-point limit,
      and **no** stdout result (asserted in `test-linear-graphql.sh`)
- [ ] `bad-request-400` (no complexity/ratelimited wording) yields exit `34`
      and is **not** retried — the discriminator's negative control
- [ ] `graphql-auth-error-200` (`extensions.type == "authentication error"` in
      a 200 body) yields exit `11`, complementing the 401 path
- [ ] malformed token (containing `"` / newline) yields exit `27`
      (`E_TOKEN_MALFORMED`) before any request (`test-linear-auth.sh`)
- [ ] stale-lock reclaim and lock-timeout → `E_REFRESH_LOCKED` (53) both
      asserted in `test-linear-common.sh`
- [ ] `config_set_frontmatter_field` integrity: happy overwrite, missing-field
      fails closed, special-char value, and byte-identical remainder
      (`bash scripts/test-config.sh`)
- [ ] Bearer-prefixed token → non-zero exit with an auth-failure message
      (`test-linear-init-flow.sh`)
- [ ] `plugin.json` is valid JSON and lists the linear dir:
      `jq -e '.skills | index("./skills/integrations/linear/")' .claude-plugin/plugin.json`
- [ ] Shell checks pass: `mise run scripts:check`
- [ ] Work item shows `5,000` (not `2,500`):
      `! grep -q "2,500" meta/work/0048-linear-integration.md`

#### Manual Verification

- [ ] `/init-linear` against a real workspace lists the user's teams, persists
      a `catalogue.json` containing the chosen team's UUID/key/name and a
      non-empty WorkflowState list, and `viewer.json` is gitignored while
      `catalogue.json` is tracked
- [ ] Selecting team `X` when a member of `X` and `Y` persists only `X`'s
      states (AC: single-team scoping)

---

## Phase 2: Read skills (`search-linear-issues`, `show-linear-issue`)

### Overview

The two read-archetype skills. Both depend on the transport and the catalogue.
`search` exercises the folded-in pagination; `show` exercises single-issue
field extraction.

### Changes Required

#### 1. Search flow + skill

**File**: `skills/integrations/linear/scripts/linear-search-flow.sh` (new)
**Changes**: Port `jira-search-flow.sh`'s flag-parse + body-assembly shape.
Compose a Linear `issues(filter:{…}, first: 50, after: $cursor)` query from
structured flags (`--state`, `--assignee`, `--label`, free-text), run it
through `linear-graphql.sh --paginate .data.issues`, and emit the merged
result. There is **no** `--team` flag — the team is implied by the catalogue
(consistent with the single-team scoping in "What We're NOT Doing"). Echo the
composed filter to stderr (`INFO:`) for auditability. State filter resolves
names against the catalogue where Linear's filter expects them. Exit codes
`70-73`.

**File**: `skills/integrations/linear/search-linear-issues/SKILL.md` (new)
**Changes**: Read archetype (`disable-model-invocation: false`; `allowed-tools`
scoped to `.../linear/scripts/*`, `config-*`, `jq`, `curl`). Render a Markdown
table (identifier, title, state, assignee). Standard `!` injection.

#### 2. Show flow + skill

**File**: `skills/integrations/linear/scripts/linear-show-flow.sh` (new)
**Changes**: `issue(id:"BLA-123"){ id identifier title state{name} assignee{name}
description comments{nodes{body}} }`. Print the structured result; exit codes
`80-82` (`E_SHOW_NO_KEY`, bad flag, etc.).

**File**: `skills/integrations/linear/show-linear-issue/SKILL.md` (new)
**Changes**: Read archetype. Render the issue's fields; Markdown description is
already native (no ADF rendering).

#### 3. Tests + fixtures

**Files** (new): `test-linear-search.sh`, `test-linear-show.sh`, and scenarios
`search-filter-state-200` (5 issues, 2 in state S), `search-paginate-200`,
`show-issue-200` (identifier I, title T, state S, assignee A, description D),
`show-issue-404`.

### Success Criteria

#### Automated Verification

- [ ] `bash skills/integrations/linear/scripts/test-linear-search.sh` passes
- [ ] `bash skills/integrations/linear/scripts/test-linear-show.sh` passes
- [ ] Search filtered on state `S` over a 5-issue fixture (2 in `S`) returns
      exactly those 2 identifiers and none of the other 3
- [ ] The composed GraphQL filter the search actually sends is body-captured
      and asserted to carry the catalogue-resolved state value (not just that
      the canned response renders) — the mock answers positionally, so the
      filter must be verified directly
- [ ] A multi-page search (3×50) returns all 150 issues in one result set
- [ ] `show-linear-issue I` reports exactly `I/T/S/A/D` for each field
- [ ] `mise run scripts:check` passes

#### Manual Verification

- [ ] `/search-linear-issues --state "In Progress"` against a real workspace
      renders a correct table
- [ ] `/show-linear-issue <ID>` renders the description as readable Markdown

---

## Phase 3: Standard write skills (`create`, `update`, `comment`)

### Overview

The three write-archetype mutation skills sharing the preview → confirm → send
gate. `create-linear-issue` carries the net-new `work_item_id` writeback.

### Changes Required

#### 1. Create flow + skill (writeback)

**File**: `skills/integrations/linear/scripts/linear-create-flow.sh` (new)
**Changes**:
- Accept a local work-item file path. Read its `work_item_id` and `title` via
  `config_extract_frontmatter`; read the Markdown description via the existing
  shared `config_extract_body` (which already handles the no-frontmatter and
  unclosed-frontmatter edge cases) — **not** a bespoke second-`---` awk.
- **Already-synced guard**: trim surrounding quotes/whitespace from the
  extracted `work_item_id`, then if it matches a remote-format identifier
  (`^[A-Z][A-Z0-9]*-[0-9]+$`), create nothing and report already synced
  (`E_CREATE_ALREADY_SYNCED`, 102). A numeric `work_item_id` means unsynced
  (0047's contract). (Trimming matters: a quoted `"BLA-123"` must still fire
  the guard or a re-run would create a duplicate.)
- `issueCreate(input:{teamId: <catalogue>, title, description, …})`, selecting
  `issue { id identifier }` in the response.
- `--print-payload` dry-run (no API call, no writeback).
- On a real successful create:
  1. **Validate** the returned `identifier` against `^[A-Z][A-Z0-9]*-[0-9]+$`
     before it is written anywhere (a tampered response must not inject
     newlines / YAML into a tracked file); fail with a distinct code if it does
     not match.
  2. Call the shared `config_set_frontmatter_field <file> work_item_id
     <identifier>` (see Phase 1 item 8) to overwrite the field; its built-in
     integrity check (re-parse + fail-closed, anchored to the frontmatter
     block, literal value) guards against corrupting the human-authored file.
  3. Print the new identifier.
- **Create-succeeded / writeback-failed ordering**: the remote create is not
  idempotent and the local guard only arms once the writeback lands, so a
  writeback failure after a successful create is surfaced **loudly** (distinct
  exit code + message stating the issue was created as `<identifier>` but the
  local file was not updated, so the user must not blindly re-run) rather than
  collapsed into a generic error.
- Exit codes `100-109` (`E_CREATE_NO_FILE`, `E_CREATE_ALREADY_SYNCED`,
  `E_CREATE_BAD_FRONTMATTER`, `E_CREATE_NO_TITLE`, `E_CREATE_BAD_IDENTIFIER`,
  `E_CREATE_WRITEBACK_FAILED`, bad flag, …).

**File**: `skills/integrations/linear/create-linear-issue/SKILL.md` (new)
**Changes**: Write archetype with the full preview → confirm → send gate and
trust-boundary language ported from `create-jira-issue/SKILL.md`. The response
render announces both the created identifier **and** that the local file's
`work_item_id` was updated.

#### 2. Update flow + skill

**File**: `skills/integrations/linear/scripts/linear-update-flow.sh` (new)
**Changes**: `issueUpdate(id, input:{…})` over updatable fields title,
description, state (name → UUID via catalogue), assignee, priority. Incremental
`jq -n` merge; `--print-payload` dry-run; exit codes `110-117`.

**File**: `skills/integrations/linear/update-linear-issue/SKILL.md` (new)
**Changes**: Write archetype + gate.

#### 3. Comment flow + skill

**File**: `skills/integrations/linear/scripts/linear-comment-flow.sh` (new)
**Changes**: `commentCreate(input:{issueId, body})` (Markdown native, not
`bodyData`). `--print-payload` dry-run; exit codes `90-96`.

**File**: `skills/integrations/linear/comment-linear-issue/SKILL.md` (new)
**Changes**: Write archetype + gate.

#### 4. Tests + fixtures

**Files** (new): `test-linear-create.sh`, `test-linear-update.sh`,
`test-linear-comment.sh`, with scenarios that **capture** request bodies
(`create-201-capture`, `update-200-capture`, `comment-201-capture`), a
`create-malformed-identifier-201` scenario (returns an identifier with
newlines/YAML), plus local-file fixtures for writeback (including one whose
`work_item_id` is quoted). `test-linear-create.sh` asserts the issue
title/description equal the work item's, the file's `work_item_id` is rewritten
to the returned identifier, the already-synced guard short-circuits a file
whose `work_item_id` is already remote-format (quoted form included), a
malformed returned identifier is rejected before any write, and on a successful
create the rest of the file (body + other frontmatter) is left **byte-identical**
apart from the `work_item_id` line.

### Success Criteria

#### Automated Verification

- [ ] `bash skills/integrations/linear/scripts/test-linear-create.sh` passes
- [ ] `bash skills/integrations/linear/scripts/test-linear-update.sh` passes
- [ ] `bash skills/integrations/linear/scripts/test-linear-comment.sh` passes
- [ ] create from a numeric-`work_item_id` file: issue title == work-item title,
      description == rendered body, and the file's `work_item_id` is overwritten
      with the returned identifier (e.g. `BLA-123`)
- [ ] create against an already-remote `work_item_id` creates no issue and
      reports already synced (exit 102) — including a quoted `"BLA-123"` value
- [ ] a malformed returned identifier (newline/YAML) is rejected with
      `E_CREATE_BAD_IDENTIFIER` and the local file is left untouched
- [ ] a successful remote create whose writeback then fails (e.g. read-only
      target file) yields `E_CREATE_WRITEBACK_FAILED` with a message naming the
      created identifier and warning against a blind re-run
- [ ] on a successful create the file is byte-identical apart from the
      `work_item_id` line (diff the remainder)
- [ ] update setting title `T` + state `S` produces a payload with the
      catalogue-resolved `stateId` and title `T` (body-capture assertion)
- [ ] comment with a Markdown body sends `commentCreate.input.body` equal to the
      submitted Markdown (body-capture assertion)
- [ ] `mise run scripts:check` passes

#### Manual Verification

- [ ] `/create-linear-issue meta/work/<file>.md` creates the issue, the local
      file's `work_item_id` becomes the Linear identifier, and re-running
      reports already synced
- [ ] `/update-linear-issue <ID> --title … --state …` then `/show-linear-issue
      <ID>` reflects the change
- [ ] `/comment-linear-issue <ID> --body "…"` appears on the issue in Linear

---

## Phase 4: Divergent write skills (`transition`, `attach`)

### Overview

The two most-divergent skills: transition resolves state names from the cached
catalogue (no live lookup), and attach has dual link/binary branches.

### Changes Required

#### 1. Transition flow + skill (cache resolution)

**File**: `skills/integrations/linear/scripts/linear-transition-flow.sh` (new)
**Changes**: Read the target state name → UUID from `catalogue.json`
**directly** (no API call). Then `issueUpdate(id, input:{stateId})`. This is the
divergence from Jira's live `/transitions` GET. State-name matching is
**case-insensitive and trimmed** (bash 3.2-safe — via `tr` or `[[ =~ ]]`, never
`${var,,}`), mirroring Jira's case-insensitive `.to.name` match, so a user
typing `"in progress"` resolves `"In Progress"`. If two catalogue states share
a display name (Linear permits this across categories), the match is
**rejected as ambiguous** rather than silently picking one. Exit codes
`120-126` (`E_TRANSITION_NO_KEY`, `E_TRANSITION_NO_STATE`,
`E_TRANSITION_STATE_NOT_IN_CATALOGUE`, `E_TRANSITION_STATE_AMBIGUOUS`,
`E_TRANSITION_NO_CATALOGUE` → run init, bad flag). `--describe` dry-run.

**File**: `skills/integrations/linear/transition-linear-issue/SKILL.md` (new)
**Changes**: Write archetype + gate.

#### 2. Attach flow + skill (dual link/binary)

**File**: `skills/integrations/linear/scripts/linear-attach-flow.sh` (new)
**Changes**: Two mutually-exclusive sub-modes (exactly one required per
invocation; deliverable incrementally — link first, then binary):
- **Link**: `--url URL [--title T]` → `attachmentCreate(input:{issueId, title,
  url})`.
- **Binary**: `--file PATH [--title T]` → three steps:
  1. `fileUpload(contentType, filename, size)` → `uploadFile{ uploadUrl,
     assetUrl, headers{key value} }`.
  2. HTTP **PUT** the raw bytes to `uploadUrl` via a **separate direct-curl
     code path** (not the GraphQL transport, and crucially **sending NO
     `Authorization` header** — the `lin_api_` token must never leave the
     `api.linear.app` GraphQL endpoint). Before contacting it, **validate**
     that `uploadUrl` (and `assetUrl`) are `https://` on an allow-listed host.
     The check is **anchored to a parsed host component** (full-host equality
     against the documented Linear uploads host, or a `.linear.app` suffix
     verified at a label boundary), never a substring/glob — so a look-alike
     such as `uploads.linear.app.evil.com` or `evil-linear.app` is rejected
     (a fixture asserts this). A server-supplied URL is untrusted and an
     unvalidated PUT is an SSRF / token-and-file exfiltration primitive. The
     PUT is issued with **no redirect-following** (`--max-redirs 0`,
     `--proto =https`, never `-L`), so a 30x from the allow-listed host to an
     attacker location cannot bypass the post-validation host check (a fixture
     asserts a redirect to a non-allow-listed host is not followed). Any
     loopback admission for the test base-URL override is gated on
     `ACCELERATOR_TEST_MODE=1` (mirroring the transport's loopback-only gate),
     so the guard cannot be disabled outside test mode via an env var. Send
     `Content-Type:
     <contentType>`, `Cache-Control: public, max-age=31536000`, **plus the
     returned `headers[]` entries filtered to an allow-list** (the documented
     signed-upload set, e.g. `x-amz-*`); reject any header whose name is
     outside the allow-list or whose value contains CR/LF, and never echo
     `Authorization`/`Host`. The PUT carries an explicit (pinned) timeout and a
     small bounded retry, mapped to `E_ATTACH_UPLOAD_FAILED`. This whole step —
     URL validation, header filtering, timeout, retry — lives in its own named
     helper (e.g. `_linear_upload_asset`), distinct from the orchestrating
     `_attach` function and from the GraphQL transport, mirroring how
     `_linear_classify_gql_error` / `_linear_paginate` were extracted so each
     concern is independently testable.
  3. `attachmentCreate(input:{issueId, title, url: assetUrl})`.
- **Partial-failure semantics** (no Jira precedent): the three steps are not
  atomic. A PUT-succeeds / `attachmentCreate`-fails outcome leaves an orphaned
  uploaded asset; the flow surfaces **which step failed and the resulting
  remote state** (distinct messaging, not a single collapsed code), states that
  the operation is **not idempotent across steps** (a re-run re-uploads), and
  accepts orphaning as recoverable per the destructive-op-safety convention
  rather than attempting compensating deletes. Failure messages name the host
  and step but **redact the signed query string** of `uploadUrl`/`assetUrl`
  (those are short-TTL bearer-style capabilities that should not be logged
  verbatim).
- File validation mirrors `jira-attach-flow.sh` (dash-prefix reject, device
  symlink reject, exists+readable, 10 MB warn) — keep its `readlink -f || true`
  BSD guard (noting it is effectively a no-op on stock macOS where `readlink`
  lacks `-f`, so the `[[ -f && -r ]]` check is the real cross-platform gate) and
  introduce no new GNU-only coreutils flags in the PUT path. Format the MB
  figure with **awk** (an already-declared dependency), **not** `bc` (which
  `linear_require_dependencies` does not require), to avoid an undeclared tool
  dependency on the attach path.
  `--describe` dry-run. Exit codes `130-139` (`E_ATTACH_NO_KEY`,
  `E_ATTACH_NO_TARGET`, `E_ATTACH_BOTH_TARGETS`, `E_ATTACH_FILE_MISSING`,
  `E_ATTACH_BAD_URL`, `E_ATTACH_BAD_UPLOAD_URL` (host/scheme not allow-listed),
  `E_ATTACH_UPLOAD_FAILED`, `E_ATTACH_REGISTER_FAILED` (step 3 after a
  successful PUT), bad flag).

**File**: `skills/integrations/linear/attach-linear-issue/SKILL.md` (new)
**Changes**: Write archetype + gate; documents both branches. The
preview→confirm→send gate must show the **exact** target issue, the file path
or link URL, and (for binary) the resolved `uploadUrl` host before the confirm,
and the gating language must state the file path / URL comes only from the
user's current turn (not synthesised from prior tool output). `disable-model-invocation: true` retained.

#### 3. Tests + fixtures

**Files** (new): `test-linear-transition.sh`, `test-linear-attach.sh`, with
scenarios:
- `transition-update-200`; a **catalogue-stubbed-fail** case (catalogue/team
  endpoint stubbed to fail or omitted, transition still succeeds → proves cache
  resolution); a **case-insensitive** match case (`"in progress"` resolves
  `"In Progress"`); and a **duplicate-name** catalogue → `E_TRANSITION_STATE_AMBIGUOUS`.
- `attach-link-200` (`attachmentCreate`).
- `attach-binary` 3-step sequence: `fileUpload-200` (returns loopback
  `uploadUrl`/`assetUrl` + a `headers[]` set including an allow-listed and a
  non-allow-listed header), a PUT expectation that **captures** the actual
  request headers, then `attachmentCreate-200`. The bash test asserts directly
  (not solely via `expect_headers` teardown semantics) that the PUT carried
  every allow-listed signed header + `Content-Type` + `Cache-Control`, carried
  **no** `Authorization` header, and **dropped** the non-allow-listed header;
  the suite also asserts the mock's captured-errors/exit status after the
  scenario. Assert the issue's attachments connection gains exactly one
  attachment.
- `attach-binary-bad-upload-url` (fileUpload returns a non-`https`/off-host
  `uploadUrl`, plus a look-alike host such as `uploads.linear.app.evil.com`)
  → `E_ATTACH_BAD_UPLOAD_URL`, no PUT issued.
- `attach-binary-redirect` (allow-listed `uploadUrl` 30x's to a non-allow-listed
  host) → the PUT does **not** follow the redirect (asserts `--max-redirs 0`).
- `attach-binary-upload-fail` (PUT returns 5xx / connection error) → bounded
  retry then `E_ATTACH_UPLOAD_FAILED`, retry count asserted via the sleep seam,
  distinct from the register-fail case.
- `attach-binary-crlf-header` (fileUpload returns an allow-listed-name header
  whose **value** contains CR/LF) → the PUT (via `captured_headers`) does not
  forward it (the value-level CR/LF guard fires independently of name allow-listing).
- `attach-binary-register-fail` (PUT 200 then `attachmentCreate` errors) →
  `E_ATTACH_REGISTER_FAILED` with messaging naming the orphaned asset.

### Success Criteria

#### Automated Verification

- [ ] `bash skills/integrations/linear/scripts/test-linear-transition.sh` passes
- [ ] transition to a target state name sends `issueUpdate` with the
      catalogue-resolved `stateId`; with the catalogue/team API endpoint stubbed
      to fail, the transition still succeeds (proves cache resolution, no live
      lookup)
- [ ] `bash skills/integrations/linear/scripts/test-linear-attach.sh` passes
- [ ] link attach calls `attachmentCreate` with the supplied URL
- [ ] binary attach performs `fileUpload` → PUT → `attachmentCreate`, the
      attachment count increases by one, and the PUT (captured in the test)
      carried every allow-listed signed header + `Content-Type` +
      `Cache-Control`, **no** `Authorization` header, and **dropped** a
      non-allow-listed returned header
- [ ] a non-`https`/off-host/look-alike `uploadUrl` is rejected with
      `E_ATTACH_BAD_UPLOAD_URL` and **no** PUT is issued
- [ ] an allow-listed `uploadUrl` that 30x's to a non-allow-listed host is
      **not** followed by the PUT (`--max-redirs 0`)
- [ ] a failing PUT (5xx/connection) retries (bounded, sleep-seam asserted)
      then yields `E_ATTACH_UPLOAD_FAILED`, distinct from register-fail
- [ ] a returned header whose value contains CR/LF is dropped from the PUT
      (asserted via `captured_headers`)
- [ ] a step-3 failure after a successful PUT yields `E_ATTACH_REGISTER_FAILED`
      with messaging naming the orphaned asset (not a generic upload-failed)
- [ ] transition resolves a state name case-insensitively and rejects a
      duplicate-name catalogue with `E_TRANSITION_STATE_AMBIGUOUS`
- [ ] `mise run scripts:check` passes

#### Manual Verification

- [ ] `/transition-linear-issue <ID> "<state>"` then `/show-linear-issue <ID>`
      reports the target state
- [ ] `/attach-linear-issue <ID> --url https://…` adds a link attachment in
      Linear
- [ ] `/attach-linear-issue <ID> --file ./screenshot.png` uploads the file and
      registers it as an attachment visible in Linear

---

## Testing Strategy

### Unit / component tests (per phase, TDD)

- Standalone bash suites under `skills/integrations/linear/scripts/test-*.sh`,
  sourcing `scripts/test-helpers.sh`, run directly (per CLAUDE.md).
- Each suite drives `mock-linear-server.py` with sequenced scenario fixtures and
  points the transport at it via `ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST`
  under `ACCELERATOR_TEST_MODE=1`.
- Retry/backoff timing is asserted via the file-based sleep counter
  (`LINEAR_RETRY_SLEEP_FN`), never real sleeps.

### Key edge cases

- Auth → complexity → RATELIMITED → bad-request disambiguation, with auth
  **first** and complexity before RATELIMITED; the bad-request negative control
  proves the discriminator's boundaries, not just its positive fixtures.
- 200-body errors are terminal (never routed into the retry path); auth error
  in a 200 body → exit 11.
- Backoff arithmetic from epoch-ms reset header at second granularity, clamped
  to `[1,60]s` (±2s tolerance); RATELIMITED-then-200 resumes with one sleep.
- Pagination terminating when `hasNextPage` is false; 150-node merge; empty
  connection → `[]`; non-advancing cursor and `MAX_PAGES` both bound the loop;
  a mid-pagination failure emits no partial result.
- `work_item_id` writeback: numeric → rewritten; already-remote (incl. quoted)
  → no-op + report; malformed returned identifier rejected; remainder
  byte-identical; create-succeeded/writeback-failed surfaced loudly.
- Malformed token (quote/newline) → `E_TOKEN_MALFORMED` before any request.
- Binary attach: PUT carries no `Authorization`, validates the `uploadUrl`
  host/scheme, allow-lists echoed headers; step-3 failure → orphaned-asset
  messaging.
- Transition: case-insensitive trimmed state match; duplicate-name → ambiguous.
- bash 3.2: no `${var,,}`/associative arrays, no GNU-only `date +%N` /
  `readlink -f` without a BSD guard, anywhere.

### Integration / full mirror

- `mise run` (default task) exits 0 end-to-end as the definition of done for the
  final phase.

## Migration Notes

- The Linear state path (`.accelerator/state/integrations/linear/`) is net-new —
  no relocation migration is required. `init-linear` writes the inner
  `.gitignore` directly (mirroring `_jira_ensure_inner_gitignore`). Unlike Jira,
  the gitignore rules are **not** pinned to a migration-script copy, because no
  migration writes them; `test-linear-paths.sh` asserts the rules in
  `linear-common.sh` directly.
- **Known risk (bounded)**: Linear's verbatim complexity-rejection message is
  undocumented. The message-substring discriminator (`complexity` / `10,000`)
  is correct against the authored fixtures and the work item's ACs, but should
  be re-verified against a real complexity rejection during manual testing and
  adjusted if Linear's wording differs. The blast radius is contained by
  design: the discriminator lives in one labelled constant
  (`LINEAR_COMPLEXITY_PATTERN`) inside the single `_linear_classify_gql_error`
  function (one edit point), and the classifier **fails safe** — a 400 that
  matches neither a confirmed code nor the complexity heuristic takes the
  non-retried `E_GQL_BAD_REQUEST` path, so a wording drift degrades to a
  terminal bad-request rather than a futile retry loop.

## References

- Original work item: `meta/work/0048-linear-integration.md`
- Research: `meta/research/codebase/2026-06-14-0048-linear-integration-apis.md`
- Transport model: `skills/integrations/jira/scripts/jira-request.sh:322-432`
  (retry/backoff), `:330-333` (token-out-of-argv), `:348-428` (status dispatch)
- Auth model: `skills/integrations/jira/scripts/jira-auth.sh:165-233`
- Common model: `skills/integrations/jira/scripts/jira-common.sh:69-87`
  (`state_dir`), `:140-221` (`with_lock`)
- Flow models: `jira-init-flow.sh:73-158`, `jira-create-flow.sh` (create;
  `:393` prints key only — Linear adds writeback), `jira-search-flow.sh:274-285`
  (Linear folds pagination into the transport),
  `jira-transition-flow.sh:79-116` (Linear resolves from cache, not live),
  `jira-attach-flow.sh:164-172` (Linear adds a link branch)
- SKILL.md archetypes: `search-jira-issues/SKILL.md` (read),
  `create-jira-issue/SKILL.md` (write + gate + bare-path caveat at `:61-64`)
- Test harness: `skills/integrations/jira/scripts/test-helpers/mock-jira-server.py`,
  `test-jira-request.sh:84-114`
- Registration: `.claude-plugin/plugin.json:16`
- Complexity/RATELIMITED discriminator evidence: Linear `@linear/sdk`
  `error.ts` error map; <https://linear.app/developers/rate-limiting>
