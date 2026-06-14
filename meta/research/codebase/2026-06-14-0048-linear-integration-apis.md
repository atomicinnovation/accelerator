---
type: codebase-research
id: "2026-06-14-0048-linear-integration-apis"
title: "Research: Linear Integration — Jira patterns to mirror and a deep understanding of the Linear GraphQL API"
date: "2026-06-14T21:19:22+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0048"
parent: "work-item:0048"
topic: "Linear Integration skills — Jira patterns and Linear GraphQL API (auth, rate limiting, pagination, mutations, attachments)"
tags: [research, codebase, integrations, linear, jira, graphql, authentication]
revision: "cefea6dd7267c4b4a5ebdaacf9ef1730d3827a7b"
repository: "ticket-management"
last_updated: "2026-06-14T21:19:22+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Linear Integration — Jira patterns to mirror and a deep understanding of the Linear GraphQL API

**Date**: 2026-06-14T21:19:22+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: cefea6dd7267c4b4a5ebdaacf9ef1730d3827a7b
**Branch**: ticket-management (jj workspace)
**Repository**: ticket-management

## Research Question

For the story at `meta/work/0048-linear-integration.md`: build a deep
understanding of (a) the existing Jira integration patterns this story must
mirror, and (b) the Linear GraphQL API — especially authentication, rate
limiting, pagination, the issue-lifecycle mutations, and attachments — through
web research against the official Linear developer documentation.

## Summary

The Linear integration (work item 0048) is an L-sized, intentionally
indivisible build of **eight skills** under `skills/integrations/linear/` plus a
shared `scripts/` library, structurally parallel to the existing
`skills/integrations/jira/` (which ships 8 skill dirs + ~30 shared scripts).
The mirroring is clean: three shared helpers (`linear-graphql.sh`,
`linear-auth.sh`, `linear-common.sh`), eight `*-flow.sh` orchestrators, eight
`SKILL.md` files in two archetypes (read = auto-invokable + scoped tools; write
= manual-only + preview/confirm/send gate), a dedicated `EXIT_CODES.md`, and one
new `plugin.json` directory registration. The repo-wide libraries
(`atomic-common.sh`, `vcs-common.sh`, `log-common.sh`, `config-*.sh`) are
integration-agnostic and reused unchanged. The Jira ADF
(Atlassian Document Format) markdown-conversion subsystem (awk + jq pipeline)
has **no Linear analogue** — Linear accepts Markdown natively — which is why the
story is sized lighter than Jira.

**The web research confirmed the story's central technical claims with two
material corrections you should fold back into the work item:**

1. **Rate limit is 5,000 requests/hour, NOT 2,500.** The work item states 2,500
   in two places (Dependencies and Technical Notes). Official Linear docs say
   **5,000 requests/hour** for personal-API-key auth. The 3,000,000 complexity
   points/hour figure **is** correct.
2. **There is no documented maximum page size (`first`).** The work item's
   acceptance test assumes pages of 50 (which is the documented *default*), but
   the real ceiling is emergent from the 10,000-point single-query complexity
   cap, not a fixed row count. Design the paginator to handle a 400 by halving
   `first` rather than assuming a fixed maximum.

Everything else in the story is confirmed: API key sent in `Authorization`
**without** a `Bearer` prefix; HTTP **400** + `extensions.code == "RATELIMITED"`
(not 429); `X-RateLimit-Requests-Reset` as epoch **milliseconds**; the
10,000-point single-query complexity cap; Relay cursor pagination
(`first`/`after` + `pageInfo.hasNextPage`/`endCursor`); team-scoped
`WorkflowState` UUIDs; native Markdown for descriptions/comments; and the
two attachment mechanisms (`attachmentCreate` for links; three-step `fileUpload`
pre-signed-URL flow for binaries).

## Detailed Findings

### Part A — The Jira integration to mirror (live codebase)

All paths are under the main repo root
`/Users/tobyclemson/Code/organisations/atomic/company/accelerator/`. (The
research deliberately ignored `workspaces/*/` jj checkouts.)

#### A.1 File inventory and structure

The Jira integration lives at `skills/integrations/jira/` with this shape:

- **8 skill dirs**, each a `SKILL.md`: `init-jira/`, `create-jira-issue/`,
  `show-jira-issue/`, `search-jira-issues/`, `update-jira-issue/`,
  `transition-jira-issue/`, `comment-jira-issue/`, `attach-jira-issue/`. Only
  `create-jira-issue/` and `comment-jira-issue/` carry an `evals/evals.json`.
- **Shared scripts** at `skills/integrations/jira/scripts/` (a sibling of the
  skill dirs, NOT per-skill):
  - Core: `jira-request.sh`, `jira-auth.sh`, `jira-auth-cli.sh`,
    `jira-common.sh`, `EXIT_CODES.md`.
  - Per-operation orchestrators: `jira-{init,create,show,search,update,
    transition,comment,attach}-flow.sh`.
  - Field/JQL helpers: `jira-fields.sh`, `jira-custom-fields.sh`,
    `jira-jql.sh`, `jira-jql-cli.sh`, `jira-body-input.sh`.
  - ADF pipeline (Jira-specific, **no Linear analogue**):
    `jira-md-to-adf.sh`, `jira-adf-to-md.sh`, `jira-render-adf-fields.sh`,
    `jira-md-tokenise.awk`, `jira-md-assemble.jq`, `jira-adf-render.jq`.
- **Tests** (standalone bash, run directly per CLAUDE.md): `test-jira-*.sh` for
  every script, plus a Python mock server at
  `scripts/test-helpers/mock-jira-server.py`.
- **Fixtures** at `scripts/test-fixtures/`: `adf-samples/` (paired md/adf.json),
  `api-responses/` (canned `error-{401,403,404,410,429,500}.json`, etc.),
  `scenarios/` (multi-step mock-server scenarios).
- **Registration**: `.claude-plugin/plugin.json:16` — `"./skills/integrations/jira/",`
  (one directory entry; the loader discovers all 8 SKILL.md files beneath it).
  Add `"./skills/integrations/linear/",` alongside.

#### A.2 Configurable integrations path

The state-dir resolution chain to mirror:
- `paths.integrations` key defined in `scripts/config-defaults.sh:39` with
  default `.accelerator/state/integrations` (`config-defaults.sh:59`).
- Read via `scripts/config-read-path.sh integrations`.
- Consumed in `jira-common.sh` to build `<root>/.accelerator/state/integrations/jira/`.
- Linear's analogue: `<root>/.accelerator/state/integrations/linear/`. **Note:**
  `meta/integrations/` is legacy and guard-banned (per the work item).

#### A.3 Transport helper — `jira-request.sh` (the shape for `linear-graphql.sh`)

`skills/integrations/jira/scripts/jira-request.sh` is an **executable** (not
sourced), `set -euo pipefail`. Contract: response **body → stdout**, errors →
stderr, outcome signalled purely via **exit codes** (enumerated at
`jira-request.sh:11-25`).

- **Credentials never in argv** (`jira-request.sh:330-333`): the load-bearing
  trick is `curl --config -` reading a config directive from stdin:
  ```bash
  printf 'user = "%s:%s"\n' "$JIRA_EMAIL" "$JIRA_TOKEN" |
    curl --config - "${curl_flags[@]}" -D "$hdr_file" -o "$body_file" "$URL"
  ```
  For Linear (header token, not Basic auth) the equivalent is
  `printf 'header = "Authorization: %s"\n' "$LINEAR_TOKEN" | curl --config - ...`
  — same anti-argv property (token invisible to `ps`/`/proc/<pid>/cmdline`).
- **Retry loop** (`jira-request.sh:322-432`): `max_attempts=4`. Only `429|5xx`
  are retried. `Retry-After` is parsed (numeric delta-seconds OR HTTP-date via
  a cross-platform `_jira_parse_http_date` at `:167-186`), clamped to **[1,60]s**.
  Absent/garbled `Retry-After` → **exponential backoff with ±30% jitter**
  (`base = 1<<(attempt-1)`, capped 60, `:405-420`). Exhaustion → `exit 19` (429)
  / `exit 20` (5xx).
- **Status→exit-code map** (`:348-428`): `2xx`→0 (after `jq -e .` body
  validation; non-JSON 2xx → `exit 16`); `400`→34, `401`→11, `403`→12, `404`→13,
  `410`→14; connect failure → `exit 21`; no-creds → `exit 22`.
- **Test seam** (`:37-63, 214-220`): `JIRA_RETRY_SLEEP_FN` overrides `sleep`,
  honoured only under `ACCELERATOR_TEST_MODE=1` and matching `^_?test_[a-z_]+$`.
  Mirror this so retry-timing tests don't actually sleep.
- **The one new branch for GraphQL**: GraphQL returns **HTTP 200 with an
  `errors[]` array** in the body. `linear-graphql.sh` must inspect the JSON body
  for top-level `errors` and map those (RATELIMITED, complexity-cap, auth) to
  dedicated exit codes — Jira's pure-HTTP-status dispatch is insufficient.
- Linear simplifications: single fixed endpoint `/graphql` collapses Jira's
  elaborate path validator (`_req_validate_path`, `:69-149`) and site/subdomain
  validation to near-nothing; request is always `POST {query, variables}`.

#### A.4 Auth helper — `jira-auth.sh` (the shape for `linear-auth.sh`)

`skills/integrations/jira/scripts/jira-auth.sh` is a **sourced library**. Single
public entrypoint `jira_resolve_credentials` (`:132-250`) sets vars in the
caller's scope (`JIRA_SITE/EMAIL/TOKEN` + provenance vars); it does **not** print
the token.

- **4-tier token precedence** (`jira-auth.sh:165-233`), first hit wins:
  1. `ACCELERATOR_JIRA_TOKEN` env (`:169-173`)
  2. `ACCELERATOR_JIRA_TOKEN_CMD` env → run via `_jira_run_token_cmd` (`:175-181`)
  3. `config.local.md` `token` or `token_cmd` (`:183-219`, behind the perm gate)
  4. shared `config.md` — **`token` only** (`:221-233`)
- **Shared-config `token_cmd` ban** (`:221-233`): a `token_cmd` in shared
  `config.md` is **never executed** — it warns `E_TOKEN_CMD_FROM_SHARED_CONFIG`
  and reads only the static `token`. Rationale (`:23-30`): team config must not
  inject credential-access commands that run on a teammate's machine.
- **0600 fail-closed perm check** (`:184-208`): before reading any token from
  `config.local.md` — symlink → reject `exit 29`; mode must have last two octal
  digits `00` (≤0600) via `_jira_mode_is_secure` (`:82-87`); insecure → fail
  closed unless `ACCELERATOR_ALLOW_INSECURE_LOCAL=1` **AND** a VCS-tracked
  `.claude/insecure-local-ok` marker (`:194-208`).
- **No Authorization header is built here, and no "Bearer" anywhere.** The
  header is assembled in `jira-request.sh:331` (HTTP Basic). For Linear,
  `linear-auth.sh` resolves **only `LINEAR_TOKEN`** (no email/site), keeping the
  identical 4-tier precedence + perm gate + ban; the header (no Bearer prefix
  for personal keys) is built in `linear-graphql.sh`, not the auth helper.

#### A.5 Common helper — `jira-common.sh` (the shape for `linear-common.sh`)

`skills/integrations/jira/scripts/jira-common.sh`, sourced; sources the
repo-wide `atomic-common.sh`, `vcs-common.sh`, `log-common.sh`, `work-common.sh`
(`:44-61`).

- **`jira_state_dir`** (`:69-87`): `find_repo_root` → `config-read-path.sh
  integrations` → append `/jira` → `mkdir -p` → print. Linear:
  `linear_state_dir` returning `.../integrations/linear`.
- **`JIRA_INNER_GITIGNORE_RULES`** (`:53-57`) = `(site.json .refresh-meta.json
  .lock/)` — per-developer / transient state kept out of git; byte-pinned to a
  migration-script copy by `test-jira-paths.sh`. Linear needs its own
  `LINEAR_INNER_GITIGNORE_RULES`.
- **`jira_atomic_write_json`** (`:98-113`): reads JSON from stdin, validates with
  `jq empty`, pipes to `atomic_write` (`scripts/atomic-common.sh:16-32` —
  mktemp + atomic rename).
- **`jira_with_lock <fn>`** (`:140-221`): mkdir-based exclusive lock on
  `$state_dir/.lock`, with PID+process-start-time stale detection (PID-recycle
  guard, `:183-189`) and atomic `mv`-then-`rm` reclaim. Timeout 60s →
  `return 53`. This is the model for `linear_with_lock` guarding refresh state.
- `jira_require_dependencies` (`:226-249`) asserts `jq`/`curl`/`awk` + jq ≥ 1.6.
- Drop `_jira_uuid_v4` (`:254-277`) and all ADF bits for Linear.

#### A.6 Flow orchestrators (the shape for the eight `linear-*-flow.sh`)

Uniform pattern: one `_<verb>` function, guarded by
`[[ "${BASH_SOURCE[0]}" == "${0}" ]]` (sourceable for tests), parses flags →
validates with stable `E_*` codes → assembles body with `jq -n` conditional
merge → calls the transport as a subprocess → propagates the exit code
unchanged. Key per-flow facts:

- **Search** (`jira-search-flow.sh`): single-shot, **pagination is the caller's
  job**. Body assembled at `:274-285` (`maxResults` = page size; `nextPageToken`
  merged only if `--page-token` given). The flow prints the **raw response
  JSON** including any `nextPageToken`; the *model* loops by re-invoking with
  `--page-token` (`search-jira-issues/SKILL.md:90-97, 129-137`). **For Linear,
  the work item chooses the opposite split: fold cursor pagination *into*
  `linear-graphql.sh`** so a single call returns all pages (AC: "all 150 issues
  in a single result set"). (`jira-comment-flow.sh list` *does* loop internally
  — `:213-326`, `MAX_PAGES=20` — proving internal pagination is an established
  in-repo pattern.)
- **Create** (`jira-create-flow.sh`): **only prints the allocated key**
  (`:393` `printf '%s\n' "$response"` of Jira's raw `{id,key,self}`); there is
  **no writeback** into any local work-item file. The model renders
  `> Issue created: **<KEY>**` (`create-jira-issue/SKILL.md:150-157`).
  **`create-linear-issue`'s `work_item_id` writeback is net-new** — no Jira
  precedent — and must Read+Write the local file frontmatter.
- **Attach** (`jira-attach-flow.sh`): **binary-multipart only, no link branch**
  (`:164-172`, curl `-F "file=@path"` + `X-Atlassian-Token: no-check`).
  **`attach-linear-issue`'s dual link/binary split is net-new.**
- **Transition** (`jira-transition-flow.sh`): resolves a state **name → ID via a
  live GET** of `/transitions` (`:79-116`, case-insensitive `.to.name` match;
  ambiguous → `return 123`). **Linear diverges:** it resolves state names → UUIDs
  from the **`init-linear`-cached catalogue**, not a live call (an AC stubs the
  catalogue endpoint to fail and still expects the transition to succeed).
- **Init** (`jira-init-flow.sh`): `verify` (`GET /myself` → persist `site.json`),
  `discover` (wrapped in `jira_with_lock`; `GET /project` → `projects.json`
  catalogue + field refresh), `prompt-default`. Linear: `verify` via
  `viewer`/`teams`, persist the team + WorkflowState catalogue.
- **Dry-run convention**: write flows accept `--print-payload` (create/update/
  comment) or `--describe` (attach/transition) returning the would-be payload
  with no API call — drives the SKILL's confirmation gate.
- **Disjoint exit-code ranges per skill** (search 70-73, show 80-82, comment
  91-99, create 100-107, update 110-117, transition 120-126, attach 130-133,
  init 60-61); transport codes 11-23,34 reserved and propagated unchanged.

#### A.7 SKILL.md archetypes

Two shapes (the Linear skills map 1:1):

- **Read archetype** (search, show): `disable-model-invocation: false`; long
  natural-language trigger `description` enumerating phrasings; **narrowly scoped**
  `allowed-tools` —
  `Bash(${CLAUDE_PLUGIN_ROOT}/skills/integrations/jira/scripts/*)`,
  `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)`, `Bash(jq)`, `Bash(curl)`
  (`search-jira-issues/SKILL.md:1-19`). For Linear, swap the glob to
  `.../linear/scripts/*`.
- **Write archetype** (init, create, update, transition, comment, attach):
  `disable-model-invocation: true`; `allowed-tools: [Bash, Read, Write]`;
  description always contains "This is a write skill with irreversible side
  effects — it must never be auto-invoked from conversational context"
  (`create-jira-issue/SKILL.md:1-17`).
- **`!` preprocessor injection** (every skill): opens after the H1 with
  ```
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh <skill-name>`
  ```
  and closes with
  `!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh <skill-name>``
  (`search-jira-issues/SKILL.md:21-24, 139`). The `<skill-name>` always equals
  the `name:` frontmatter.
- **Bare-path execution caveat** (copy verbatim,
  `create-jira-issue/SKILL.md:61-64`): "Run the bare path **directly** as an
  executable; never prefix it with `bash`/`sh`/`env` (a wrapper prefix escapes
  the skill's `allowed-tools` permission and forces an unnecessary prompt)."
- **Write gate**: preview via `--print-payload`/`--describe` →
  `> **Proposed Jira write — review before sending**` →
  confirmation phrase "Send this to Jira? Reply **y** to confirm, **n** to
  revise, anything else to abort." → identical command minus the dry-run flag →
  render response → exit-code table (`create-jira-issue/SKILL.md` Steps 4-10).

### Part B — The Linear GraphQL API (web research)

> Doc-host note: the canonical developer docs now live at `linear.app/developers/*`;
> old `developers.linear.app/docs/*` deep links 301-redirect there. Point code
> comments at the new URLs.

#### B.1 Authentication — CONFIRMED: no `Bearer` prefix for personal keys

- Endpoint: **`https://api.linear.app/graphql`**, `POST`,
  `Content-Type: application/json`, body `{"query": ..., "variables": ...}`.
- Personal API key prefix **`lin_api_`**, created at Settings → Account →
  Security & Access. **User-scoped, long-lived (no expiry unless revoked)**,
  inherits the creating user's permissions across all their teams. Multiple keys
  for one user **share one rate-limit quota**.
- **Sent as `Authorization: <API_KEY>` WITHOUT a `Bearer` prefix.** Official docs
  give the literal `-H "Authorization: <Replace this with your API Key>"`.
  Sending `Bearer lin_api_...` **fails authentication** — this is the work item's
  Bearer-prefix acceptance criterion, confirmed.
- OAuth2 access tokens are the **opposite**: `Authorization: Bearer <ACCESS_TOKEN>`,
  and they expire (~24h standard flow with refresh; ~30d client-credentials) and
  carry scopes. So: build the `Bearer` decision in `linear-graphql.sh`, defaulting
  to no-prefix for personal keys.
- Source: <https://linear.app/developers/graphql>,
  <https://linear.app/developers/oauth-2-0-authentication>,
  <https://unified.to/blog/linear_api_key_how_to_generate_and_use_it_graphql_guide_for_developers>

#### B.2 Team discovery & issue identifiers

- `query Teams { teams { nodes { id name key } } }` — `key` is the short prefix
  (e.g. `BLA`) used to form identifiers. (`key` is a well-attested Team field;
  the official getting-started snippet shows only `id name`, so the three-field
  selection is a safe composition.)
- Issue identifier = team key + sequential number, e.g. `BLA-123`. The Issue type
  exposes both `id` (UUID) **and** `identifier` (the `BLA-123` string). Issues can
  be queried by the shorthand: `issue(id: "BLA-123") { id identifier title }`.
  This `identifier` is what `create-linear-issue` writes back as `work_item_id`.
- Source: <https://linear.app/developers/graphql>

#### B.3 Rate limiting — CONFIRMED with one CORRECTION

- **Requests/hour: 5,000 (NOT 2,500)** for personal-API-key auth.
  **Complexity points/hour: 3,000,000 (correct).** Leaky-bucket algorithm
  (continuous refill). Quota is per **authenticated user** (shared across that
  user's keys).
- **Rate-limited response: HTTP 400** (not 429) with
  `errors[].extensions.code == "RATELIMITED"`. HTTP 400 is also Linear's generic
  GraphQL-error status, so the `extensions.code` check is what disambiguates a
  rate-limit from an ordinary bad query.
  ```json
  { "errors": [ { "message": "...", "extensions": { "code": "RATELIMITED" } } ] }
  ```
- **Headers** (sent on every request): `X-RateLimit-Requests-Limit`,
  `-Remaining`, **`-Reset` (UTC epoch in MILLISECONDS)**, plus a parallel
  complexity set (`X-Complexity`, `X-RateLimit-Complexity-{Limit,Remaining,Reset}`).
- **Backoff** `delay_seconds = (reset_epoch_ms − now_epoch_ms) / 1000` is correct
  (the work item's ±2s/≈30s test holds). Hardening: clamp to ≥ 0 (skew +
  leaky-bucket can go slightly negative); when rate-limited, back off until the
  **later** of the requests-reset and complexity-reset headers.
- Source: <https://linear.app/developers/rate-limiting>

#### B.4 Query complexity — CONFIRMED

- **Single-query cap 10,000 points**, verbatim: "We also enforce a maximum
  complexity of a single query at any time to 10,000 points. Your query will
  always get rejected if it exceeds that." Rejection is **HTTP 400** + GraphQL
  error (same path as rate-limiting; parse `extensions`/`message` defensively
  rather than string-matching).
- Calculation: "Each property is 0.1 point, each object is 1 point and any
  connection multiplies its children's points based on the given pagination
  argument, or the default 50." The **multiplier is your `first` value**, not
  rows returned; nested connections multiply multiplicatively. Keep selections
  lean and `first` modest.
- Source: <https://linear.app/developers/rate-limiting>

#### B.5 Pagination — CONFIRMED (Relay cursor), one CORRECTION (no max page size)

- Relay-style `first`/`after` (forward), `last`/`before` (backward). **Default
  page size 50** ("The first 50 results are returned by default without query
  arguments"). Loop while `pageInfo.hasNextPage`, feeding `pageInfo.endCursor` to
  the next `after`.
- Connections expose **both** `edges { node cursor }` and a convenience
  `nodes { ... }`. Use `nodes` + `pageInfo { hasNextPage endCursor }` for
  paginating.
  ```graphql
  query IssuesPage($cursor: String) {
    issues(first: 50, after: $cursor) {
      nodes { id identifier title }
      pageInfo { hasNextPage endCursor }
    }
  }
  ```
- **No documented hard maximum for `first`.** Docs state only the default of 50;
  the real ceiling is the 10,000-point complexity cap. Community convention is
  `first: 250` for lean selections, but it's not guaranteed — **design the
  paginator to handle a 400 by halving `first` and retrying** rather than
  assuming a fixed maximum. (The work item's "3 pages of 50" fixture is fine as a
  test; just don't bake 50 in as a hard limit.)
- Source: <https://linear.app/developers/pagination>

#### B.6 WorkflowStates — CONFIRMED (team-scoped UUIDs)

- Both query shapes valid:
  `team(id:$id){ states { nodes { id name type position } } }` and top-level
  `workflowStates(filter:{ team:{ key:{ eq:"ENG" } } }){ nodes { id name type } }`.
- **`type` enum**: `backlog`, `unstarted`, `started`, `completed`, `canceled`,
  `triage` (only when the team's Triage feature is on).
- **State `id`s are UUIDs, team-scoped** — each team has its own set; `issueUpdate`/
  `issueCreate` require a `stateId` belonging to that issue's team. This is exactly
  the catalogue `init-linear` caches (name → UUID), and the cache is what
  `transition-linear-issue` resolves against (no live lookup).
- Default state if `issueCreate` omits `stateId`: the team's first `backlog`
  state (or Triage state if enabled).
- Source: <https://linear.app/developers/graphql>,
  <https://linear.app/developers/filtering>,
  <https://linear.app/docs/configuring-workflows>

#### B.7 Issue & comment mutations — CONFIRMED (native Markdown)

- **`issueCreate(input: IssueCreateInput)`**: required `teamId`, `title`;
  optional `description`, `stateId`, `assigneeId`, `priority`, `labelIds`, etc.
  Returns `{ success, issue { id identifier title ... } }` — select `identifier`
  to get the `BLA-123` key for writeback.
- **`issueUpdate(id, input: IssueUpdateInput)`**: `id` accepts the `BLA-123`
  identifier OR UUID; updates `title`, `description`, `stateId`, `assigneeId`,
  `priority`. Returns `{ success, issue { ... state { id name } } }`.
- **`commentCreate(input: CommentCreateInput)`**: `issueId` + `body` (Markdown).
  `bodyData` is internal Prosemirror — **do not use it; send `body`**.
- **Markdown is native** — confirmed: the `body`/`description` fields take
  Markdown directly, no ADF/Prosemirror conversion. (This is why Linear is
  lighter than Jira — the whole ADF subsystem is dropped.)
- Source: <https://linear.app/developers/graphql>,
  <https://linear.app/developers/sdk-fetching-and-modifying-data>

#### B.8 Attachments — CONFIRMED (two mechanisms)

- **Link** — `attachmentCreate(input: AttachmentCreateInput)`: required
  `issueId`, `title`, `url`; optional `subtitle`, `iconUrl`, `metadata`. **The
  `url` is an idempotent key per issue** — re-creating with the same URL on the
  same issue updates the original instead of duplicating. Returns
  `{ success, attachment { id } }`.
- **Binary** — three-step pre-signed-URL flow:
  1. `fileUpload(contentType, filename, size)` → `uploadFile { uploadUrl,
     assetUrl, headers { key value } }`.
  2. HTTP **PUT** the raw bytes to `uploadUrl`, sending `Content-Type:
     <contentType>`, `Cache-Control: public, max-age=31536000`, **plus every
     header from the returned `headers` array** (these carry the signed-upload
     authorization).
  3. Use the returned `assetUrl` — embed in Markdown or register via
     `attachmentCreate` as `url`.
  - **Cannot be done client-side** (CSP blocks it) — fine for a CLI. The AC's
    "issue's `attachments` connection gains exactly one new attachment" is
    satisfiable by step 3's `attachmentCreate`.
- Source: <https://linear.app/developers/attachments>,
  <https://linear.app/developers/how-to-upload-a-file-to-linear>

#### B.9 schpet/linear-cli — UX patterns to mirror

[github.com/schpet/linear-cli](https://github.com/schpet/linear-cli) (v2.0.0,
2026-04-03, TypeScript/Deno):
- Auth via **`LINEAR_API_KEY`** env var (the de-facto convention; the work item's
  `ACCELERATOR_LINEAR_TOKEN` indirection layers on top).
- **`--json`** flag for structured/agent output.
- **VCS-aware context detection**: parses issue IDs from git branch names
  (`eng-123-...`) **and** from jj commit-description trailers
  (`Linear-issue:`). Directly relevant since this repo supports both git and jj.

## Code References

- `skills/integrations/jira/scripts/jira-request.sh:330-333` — `curl --config -`
  token-out-of-argv trick (mirror for `linear-graphql.sh`).
- `skills/integrations/jira/scripts/jira-request.sh:322-432` — 4-attempt retry +
  Retry-After/backoff clamp [1,60]s.
- `skills/integrations/jira/scripts/jira-request.sh:348-428` — HTTP status →
  exit-code dispatch.
- `skills/integrations/jira/scripts/jira-auth.sh:165-233` — 4-tier token
  precedence + shared-config `token_cmd` ban.
- `skills/integrations/jira/scripts/jira-auth.sh:184-208` — 0600 fail-closed perm
  + symlink reject.
- `skills/integrations/jira/scripts/jira-common.sh:69-87` — `jira_state_dir`
  (config-driven integrations path).
- `skills/integrations/jira/scripts/jira-common.sh:140-221` — `jira_with_lock`
  mkdir lock + PID-start stale reclaim.
- `skills/integrations/jira/scripts/jira-common.sh:98-113` —
  `jira_atomic_write_json`.
- `skills/integrations/jira/scripts/jira-search-flow.sh:274-285` — search body;
  caller-side pagination (Linear folds this into the transport).
- `skills/integrations/jira/scripts/jira-create-flow.sh:393` — create prints key
  only; no writeback (Linear writeback is net-new).
- `skills/integrations/jira/scripts/jira-attach-flow.sh:164-172` — binary
  multipart only (Linear adds a link branch).
- `skills/integrations/jira/scripts/jira-transition-flow.sh:79-116` — live
  name→ID lookup (Linear resolves from cache).
- `skills/integrations/jira/scripts/jira-init-flow.sh:73-158` — verify/discover +
  catalogue persistence under `jira_with_lock`.
- `scripts/config-defaults.sh:39,59` — `paths.integrations` default.
- `.claude-plugin/plugin.json:16` — Jira registration (add `linear` sibling).
- `skills/integrations/jira/search-jira-issues/SKILL.md:1-19,21-24,139` — read
  archetype frontmatter + `!` injection.
- `skills/integrations/jira/create-jira-issue/SKILL.md:61-64,150-157` — bare-path
  caveat + key-surfacing response render.

## Architecture Insights

- **Transport contract = stdout body / stderr error / exit code.** Every flow is
  a thin wrapper that propagates the transport's exit code unchanged; the SKILL
  maps codes to user guidance. The one structural addition for GraphQL is that
  errors arrive in an HTTP-200 `errors[]` body, so `linear-graphql.sh` must
  inspect the body, not just the status line.
- **Security posture is reusable verbatim**: token never in argv (curl
  `--config -`), 4-tier precedence, shared-config `token_cmd` ban, 0600
  fail-closed local config. Only the header construction differs (Linear: single
  `Authorization` header, no Bearer prefix).
- **State catalogue + mkdir lock** is the persistence backbone. Linear's catalogue
  (team UUID/key/name + WorkflowState name→UUID map) is richer than Jira's but
  uses the identical `*_state_dir` / `*_atomic_write_json` / `*_with_lock` shape.
- **Three deliberate divergences from Jira** (net-new design, not copied):
  1. Pagination folded *into* the transport (vs caller-loop).
  2. `work_item_id` writeback into the local file (no Jira precedent).
  3. Dual attach (link via `attachmentCreate` + binary via `fileUpload`); and
     transition resolves names from the cached catalogue, not a live API call.
- **Markdown-native means a whole subsystem disappears**: no ADF tokeniser/
  assembler/renderer (`jira-md-*.{awk,jq,sh}`), shrinking the integration's
  footprint relative to Jira.

## Historical Context

- `meta/research/codebase/2026-04-29-jira-cloud-integration-skills.md` — the
  closest existing-provider precedent; foundational for the mirror.
- `meta/plans/2026-04-29-jira-integration-phase-1-foundation.md` +
  `.../2026-05-02-...-phase-2-read-skills.md` +
  `.../2026-05-03-...-phase-3-write-skills.md` +
  `.../2026-05-03-jira-phase-4-transition-attach.md` — the four-phase Jira build
  the Linear work will parallel (transport/auth/exit-codes live in Phase 1).
- `meta/plans/2026-05-08-0046-work-management-system-configuration.md` +
  `meta/research/codebase/2026-05-08-0046-work-management-system-configuration.md`
  — the configurable integrations path (0048's `blocked_by`).
- `meta/plans/2026-04-28-configurable-work-item-id-pattern.md` +
  `meta/research/codebase/2026-04-28-configurable-work-item-id-pattern.md` — the
  `work_item_id` semantics consumed by `create-linear-issue`'s writeback.
- `meta/decisions/ADR-0022-work-item-terminology.md`,
  `meta/decisions/ADR-0017-configuration-extension-points.md` — terminology and
  config extension-point ADRs.
- `meta/validations/2026-04-29-jira-integration-phase-1-foundation-validation.md`
  — Phase 1 validation (transport/exit-code coverage reference).
- `meta/notes/2026-04-29-accelerator-config-state-reorg.md` — config/state layout
  reorganisation touching integration paths.

## Related Research

- `meta/research/codebase/2026-04-29-jira-cloud-integration-skills.md`
- `meta/research/codebase/2026-05-08-0046-work-management-system-configuration.md`
- `meta/research/codebase/2026-04-28-configurable-work-item-id-pattern.md`
- `meta/research/codebase/2026-04-08-ticket-management-skills.md`

## Open Questions

1. **Rate-limit figure in the work item is wrong (2,500 → 5,000 req/hr).** Update
   `meta/work/0048-linear-integration.md` lines 154 and 214. (The complexity
   figure 3,000,000/hr and the 10,000/query cap are correct.)
2. **Page-size assumption.** The work item's pagination AC fixture ("3 pages of
   50") is fine, but the implementation must not hard-code 50 as a max — there is
   no documented max `first`; handle complexity-cap 400s by halving. Decide
   whether to encode a halving-retry in `linear-graphql.sh`.
3. **Complexity-cap vs RATELIMITED disambiguation.** Both arrive as HTTP 400 with
   a GraphQL `errors[]`. RATELIMITED carries `extensions.code == "RATELIMITED"`;
   the complexity rejection's exact `code`/`message` is not quoted verbatim in
   the docs. The work item already mandates two **distinct** exit codes — confirm
   the discriminator empirically (introspect or probe) so the two 400s don't
   collapse into one handler.
4. **OAuth vs personal-key header.** The story scopes to personal keys
   (no Bearer). Confirm `linear-graphql.sh` hard-codes the no-prefix header for
   now (OAuth/Bearer is out of scope) rather than adding a token-type sniff.
5. **`fileUpload` returned headers.** The signed-PUT requires echoing **all**
   `headers[]` from the `fileUpload` response plus `Content-Type` and
   `Cache-Control` — verify the test fixtures/mock server reproduce that header
   set so the binary-attach AC is genuinely exercised.
