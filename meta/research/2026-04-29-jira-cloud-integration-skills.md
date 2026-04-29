---
date: "2026-04-29T22:31:35+01:00"
researcher: Toby Clemson
git_commit: 77cbbb0e4288c49347d3258fffbcf3d83b396999
branch: (no bookmark — change lwoqvyopkmsu)
repository: accelerator
topic: "Jira Cloud integration skills as the first integration under skills/integrations/"
tags: [research, integrations, jira, skills, work-management, adf, jql, configuration]
status: complete
last_updated: "2026-04-29"
last_updated_by: Toby Clemson
---

# Research: Jira Cloud Integration Skills

**Date**: 2026-04-29 22:31:35 BST
**Researcher**: Toby Clemson
**Git Commit**: 77cbbb0e4288c49347d3258fffbcf3d83b396999
**Branch**: (no bookmark — change lwoqvyopkmsu)
**Repository**: accelerator

## Research Question

The Accelerator plugin needs an integrations layer so users can synchronise
work items to and from remote work-management systems. As a precursor to a
work-management synchronisation skill, the plugin should ship one or more
focused skills for interacting with Jira directly — searching for and reading
ticket content and fields, and writing back ticket content and fields. These
skills should work via the Jira Cloud REST API directly (maximally flexible
and dependency-free), with `ankitpokhrel/jira-cli` taken as a source of
inspiration for authentication, custom-field handling, and search ergonomics.
The eventual `sync-work-items` skill is **out of scope** for this research —
the design here covers only the Jira primitives.

## Summary

Eight verb-decomposed skills will be added under a new `skills/integrations/`
category: `init-jira`, `search-jira-issues`, `show-jira-issue`,
`create-jira-issue`, `update-jira-issue`, `transition-jira-issue`,
`comment-jira-issue`, `attach-jira-issue`. They share a small set of bash
helper scripts under `skills/integrations/jira/scripts/` that handle auth
resolution, signed HTTP requests, JQL composition and quoting, custom-field
discovery, and Markdown ↔ Atlassian Document Format (ADF) conversion. Helpers
emit raw API JSON by default, with an opt-in `--render-adf` flag that
walks known ADF-bearing fields and replaces them with rendered Markdown
strings. The Markdown subset supported by the bidirectional ADF converter is
deliberately constrained: paragraphs, headings, fenced code blocks (with
language), single-level bullet/ordered lists, GitHub-style checklists, hard
breaks, and the inline marks for bold, italic, code, and links — anything
richer is deferred until the planned Rust CLI replaces the bash scripts.
Authentication uses email + API token via HTTP Basic, with a token-resolution
chain of `ACCELERATOR_JIRA_TOKEN` env > `ACCELERATOR_JIRA_TOKEN_CMD`
indirection > `accelerator.local.md` > `accelerator.md`. The
`work.default_project_code` config key already used to scope local work-item
filenames doubles as the default Jira project key, eliminating a separate
`jira.default_project_key` knob. Per-instance custom-field discovery is
performed once by `init-jira` and persisted to `meta/integrations/jira/` so
subsequent skills can resolve friendly field names (e.g. `story-points`,
`sprint`) to the instance-specific `customfield_NNNNN` IDs without an extra
round-trip on every call. The implementation phases naturally into four:
foundation (config, auth, request helper, ADF converters, `init-jira`); read
skills; write skills; workflow and attachments.

## Detailed Findings

### 1. Existing landscape

#### 1.1 Local work-item skills

The `skills/work/` family is complete and provides the local source of truth
that integration skills will eventually synchronise with. Seven skills:
`create-work-item`, `extract-work-items`, `list-work-items`,
`refine-work-item`, `review-work-item`, `stress-test-work-item`,
`update-work-item`. Each follows the canonical SKILL.md structure documented
in `meta/research/2026-04-08-ticket-management-skills.md`.

The 2026-04-28 ID-pattern research delivered two configuration keys that
matter for Jira integration:

- **`work.id_pattern`** — defaults to `{number:04d}`. Users sync'ing with
  Jira will set this to `{project}-{number:04d}` so local work-item filenames
  carry the Jira-style `PROJ-NNNN` prefix.
- **`work.default_project_code`** — string used to substitute `{project}`
  in the pattern when no `--project` is passed. **This research treats
  `work.default_project_code` as also being the default Jira project key**:
  the same identifier serves both local filename prefixing and remote API
  scoping. No separate `jira.default_project_key` is introduced.

The pattern compiler at `skills/work/scripts/work-item-pattern.sh` and
resolver at `skills/work/scripts/work-item-resolve-id.sh` are the precedent
for the bash + awk style we will use in the Jira helpers.

References:

- `meta/research/2026-04-08-ticket-management-skills.md` — original
  ticket-management research; Resolved Question 5 explicitly defers external
  sync as a future enhancement.
- `meta/research/2026-04-28-configurable-work-item-id-pattern.md` — the
  ID-pattern design that paved the way for matching local filenames to
  external tracker keys.
- `skills/work/scripts/work-item-pattern.sh` — bash + awk style guide for
  regex-driven pattern compilation.
- `meta/decisions/ADR-0017-configuration-extension-points.md` — framework
  under which a new `jira.*` config section is added.

#### 1.2 GitHub skills as integration precedent

`skills/github/` is the existing precedent for "skills that talk to a remote
system". It contains three skills (`describe-pr`, `respond-to-pr`,
`review-pr`) that all delegate to the `gh` CLI rather than calling the GitHub
API directly. This gives us a pattern for a category structure but **not**
a transport pattern — Jira has no first-party CLI we want to depend on, and
the user has explicitly chosen to minimise external CLI dependencies. The
Jira skills will therefore be the first in this codebase to call an external
HTTP API directly.

Reference: `skills/github/describe-pr/SKILL.md` for the SKILL.md structure
and the convention of dispatching to a Bash-based external command.

#### 1.3 Personal `~/.claude/skills/jira` as UX prior art

The user maintains a personal `jira` skill at `~/.claude/skills/jira/SKILL.md`
that wraps `jira-cli`. It is the most direct evidence of which Jira
operations actually matter for day-to-day developer use. Patterns worth
preserving from it:

- Filter flags `-s`/`-a`/`-l`/`-t`/`-y`/`-C`/`-r`/`-w`/`-P` for status,
  assignee, label, type, priority, component, reporter, watching, parent.
- The `~`-negation prefix (`-s~Done` means "status is not Done"; `-l~bug`
  excludes the `bug` label).
- A positional argument as free-text search.
- `--current` for "the current sprint".
- `--columns key,summary,status,assignee` for selecting output fields.
- The "what's assigned to me" / "what did I do this week" / "show me open
  bugs" common patterns.

Output conventions in the personal skill: `--plain` for tab-separated rows,
`--raw` for unmodified API JSON, `--csv` for structured escaping. We will
not match these exactly because we are JSON-first uniformly (see §4.6), but
the muscle memory around filter flags and the `~`-negation prefix will be
preserved.

### 2. Jira Cloud REST API v3 — implementation reference

#### 2.1 Authentication

Jira Cloud accepts HTTP Basic with `email:api_token` credentials,
Base64-encoded, sent as `Authorization: Basic <base64>`. With curl, the
shorthand `-u "$EMAIL:$TOKEN"` performs the encoding correctly. API tokens
are obtained at <https://id.atlassian.com/manage-profile/security/api-tokens>.

Two flavours of API token now exist:

- **Classic tokens** — full account permissions; use against
  `https://your-site.atlassian.net/rest/api/3/...`.
- **Scoped tokens** — restricted to selected OAuth-style scopes, expire
  after 1–365 days, and require a different base URL
  (`https://api.atlassian.com/ex/jira/{cloudId}/...`).

For a single-user CLI tool a classic token against the site URL is the
simplest path. Scoped tokens are forward-compatible but add a Cloud-ID
lookup and a different base URL — they are noted here for future support
but are explicitly out of scope for v1.

OAuth 3LO, Forge, and Connect are all targeted at apps acting on behalf of
other users or distributed via the Marketplace; none is relevant for this
use case.

References:

- [Basic auth for REST APIs — Jira Cloud platform](https://developer.atlassian.com/cloud/jira/platform/basic-auth-for-rest-apis/)
- [Manage API tokens for your Atlassian account](https://support.atlassian.com/atlassian-account/docs/manage-api-tokens-for-your-atlassian-account/)
- [Scoped API Tokens in Confluence Cloud](https://support.atlassian.com/confluence/kb/scoped-api-tokens-in-confluence-cloud/)

#### 2.2 Base URL and API version

Base URL: `https://${SITE}.atlassian.net/rest/api/3/` for platform endpoints,
`https://${SITE}.atlassian.net/rest/agile/1.0/` for Software/board endpoints.
The `${SITE}` is the Cloud subdomain (e.g. `atomic-innovation` for
`https://atomic-innovation.atlassian.net`).

We use **only v3**. Per Atlassian, "v2 and v3 offer the same collection of
operations; v3 supports the Atlassian Document Format (ADF)". The single
operational difference is that rich-text fields (`description`,
`environment`, `comment.body`, multi-line custom fields) are ADF JSON in v3
versus wiki markup in v2. All other endpoint shapes are identical.

References:

- [v3 REST API intro](https://developer.atlassian.com/cloud/jira/platform/rest/v3/intro/)
- [v2 REST API intro](https://developer.atlassian.com/cloud/jira/platform/rest/v2/intro/)

#### 2.3 Search via JQL

Search is `POST /rest/api/3/search/jql`. The legacy `GET /search` was
deprecated October 2024 and removed; calls return HTTP 410 Gone.

Request body:

```json
{
  "jql": "project = ENG AND status = 'In Progress'",
  "fields": ["summary","status","assignee"],
  "expand": "names,schema",
  "fieldsByKeys": false,
  "maxResults": 100,
  "nextPageToken": null
}
```

- **`jql`** — JQL string. See §2.10 for safe quoting.
- **`fields`** — array of field IDs/names. Supports `*all`, `*navigable`
  (default-ish set used in the UI), and subtractive `-fieldname`. Custom
  fields by ID (`customfield_10016`) or by name when `fieldsByKeys=true`.
- **`expand`** — comma-separated string. See §2.11.
- **`nextPageToken`** — opaque pagination cursor. Omit on first call.
- **`maxResults`** — page size, server-capped (commonly 100).

**Pagination is token-based**. The response contains `issues` and (when more
pages exist) `nextPageToken`. Crucially, `total` is no longer returned and
`startAt` is no longer accepted. Continue requesting while a `nextPageToken`
is present; absence means last page.

Approximate counts come from a separate `POST /rest/api/3/search/approximate-count`
with body `{"jql": "..."}`. Useful for "X+ matches" UX.

References:

- [Atlassian REST API Search Endpoints Deprecation summary](https://docs.adaptavist.com/sr4jc/latest/release-notes/breaking-changes/atlassian-rest-api-search-endpoints-deprecation)
- [Run JQL search query using Jira Cloud REST API — KB](https://confluence.atlassian.com/jirakb/run-jql-search-query-using-jira-cloud-rest-api-1289424308.html)
- [Avoiding Pitfalls: A Guide to Smooth Migration to Enhanced JQL APIs](https://community.atlassian.com/forums/Jira-articles/Avoiding-Pitfalls-A-Guide-to-Smooth-Migration-to-Enhanced-JQL/ba-p/2985433)

#### 2.4 Issue CRUD

- **GET issue** — `GET /rest/api/3/issue/{issueIdOrKey}` with
  `fields`, `fieldsByKeys`, `expand`, `properties` query parameters.
- **Create issue** — `POST /rest/api/3/issue` with body
  `{"fields": {"project":{"key":"ENG"}, "summary":"...", "issuetype":{"name":"Task"}, "description":<ADF>, ...}}`.
  Required at minimum: `project`, `summary`, `issuetype`. Other required
  fields depend on the project's create screen — discoverable via
  `createmeta`.
- **Edit issue** — `PUT /rest/api/3/issue/{issueIdOrKey}` accepts
  two top-level keys, combinable in one body:
  - `fields` — direct value replacement (set semantics)
  - `update` — operation list per field. Supported ops: `set`, `add`,
    `remove`, `edit`, `copy`. Most useful for multi-value fields like
    `labels`, `components`, `fixVersions`.
- **Delete issue** — `DELETE /rest/api/3/issue/{issueIdOrKey}` exists
  but is intentionally out of scope; we do not write a `delete-jira-issue`
  skill.

Notes:

- `issuetype` accepts `{"name":"Task"}` or `{"id":"10001"}`. ID is more
  reliable across rename.
- Assignee/reporter use `accountId` only post-GDPR (not username/email).
- `notifyUsers=false` query parameter on PUT suppresses email notifications.

Reference: [Issues group — REST v3](https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issues/).

#### 2.5 Transitions

A workflow is a state machine: from the current state, only specific
transitions are available, each identified by an ID specific to the
workflow. The same target status may have different transition IDs in
different projects.

- **List** — `GET /rest/api/3/issue/{key}/transitions` returns transitions
  available *from the issue's current state*. With `expand=transitions.fields`,
  also returns required/optional fields per transition (e.g. `resolution`).
- **Apply** — `POST /rest/api/3/issue/{key}/transitions` with
  `{"transition":{"id":"31"}, "fields":{"resolution":{"name":"Done"}}, "update":{"comment":[{"add":{"body":<ADF>}}]}}`.

We must look up transitions per-issue because we cannot know which are valid
without asking. Caching transition IDs per project+workflow+source-state is
possible but invalidates on workflow edits — we will not cache.

`transition-jira-issue` will accept the **state name** (case-insensitive)
rather than the transition ID, mirroring jira-cli's UX (`jira issue move
<key> "In Review"`).

#### 2.6 Comments

- `GET /rest/api/3/issue/{key}/comment` — list. **This endpoint still uses
  offset-based pagination** (`startAt`/`maxResults`/`total`); the search
  migration did not affect it.
- `POST /rest/api/3/issue/{key}/comment` — body in ADF.
- `PUT /rest/api/3/issue/{key}/comment/{id}` — edit.
- `DELETE /rest/api/3/issue/{key}/comment/{id}` — remove.

Body shape: `{"body": <ADF>, "visibility": {"type":"role|group", "value":"..."}}`.
Omit visibility for public comments. `expand=renderedBody` returns HTML
alongside.

Reference: [Issue comments group — REST v3](https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issue-comments/).

#### 2.7 Attachments

`POST /rest/api/3/issue/{key}/attachments` is multipart/form-data with the
file part named `file`. Two non-obvious requirements:

- Header `X-Atlassian-Token: no-check` (CSRF bypass; without this, Jira may
  reject with 403).
- Multiple files allowed by repeating the `file` part.

Curl pattern: `curl -u "$EMAIL:$TOKEN" -H "X-Atlassian-Token: no-check" -F "file=@./path" "$BASE/issue/$KEY/attachments"`.

Permissions required on the project: Browse Projects + Create Attachments.

Reference: [How to add an attachment KB](https://support.atlassian.com/jira/kb/how-to-add-an-attachment-to-a-jira-cloud-issue-using-rest-api/).

#### 2.8 Custom fields

Custom fields appear as `customfield_NNNNN`-style keys in issue payloads.
The numeric suffix is allocated when the field is first created on the
instance — **IDs are not stable across instances**.

Discovery endpoints:

- `GET /rest/api/3/field` — non-paginated; returns every field with `id`,
  `key`, `name`, `custom`, `schema.type`, `schema.custom`, `schema.customId`,
  `clauseNames` (JQL-usable names). One-shot lookup table. Used by
  `init-jira`.
- `GET /rest/api/3/field/search` — paginated and filterable; useful for
  large instances where the unpaginated endpoint is too heavy.

Common defaults to *expect but not rely on*:

- Story Points — typically `customfield_10016`.
- Sprint — typically `customfield_10020`. Returned as an array of objects.
- Epic Link (legacy) — typically `customfield_10014`. **Deprecated**;
  see below.
- Epic Name (legacy) — typically `customfield_10011`. **Deprecated**.
- Rank — typically `customfield_10019` or `_10020`.

#### 2.9 Epic Link → `parent`

Atlassian replaced Epic Link and Parent Link with a single standard `parent`
field. After migration, set/read parent via `fields.parent =
{"key":"ENG-123"}`. Team-managed projects always used `parent`. Both old
and new are returned during the transition period.

**We use `parent` exclusively** in this design and do not write to legacy
Epic Link. If a user is on a still-classic project that has not been
migrated, the parent assignment may need to fall back; this is an edge case
we will surface as a clear error from `update-jira-issue` rather than try
to handle silently.

References:

- [Upcoming changes: epic-link replaced with parent](https://support.atlassian.com/jira-software-cloud/docs/upcoming-changes-epic-link-replaced-with-parent/)
- [Deprecation of Epic Link, Parent Link in REST APIs](https://community.developer.atlassian.com/t/deprecation-of-the-epic-link-parent-link-and-other-related-fields-in-rest-apis-and-webhooks/54048)

#### 2.10 JQL safe quoting

JQL has its own quoting rules independent of JSON.

- String literals in `'single'` or `"double"` quotes.
- Embedded same-kind quotes escaped with `\"` or `\'`.
- Reserved words (`AND`, `OR`, `NOT`, `IN`, `EMPTY`, `NULL`, `ORDER`, `BY`)
  must be quoted if used as values.
- Special characters needing quoting: `+ . , ; ? | * / % ^ $ # @ [ ]` and
  whitespace.
- The index does not store some special characters in text fields, so
  searching for them inside text may return nothing regardless of quoting.

There is no parameterised-query mechanism — string construction is the only
path. **`jira-jql.sh` will provide quoting helpers** that wrap user-supplied
values in single quotes and escape internal single quotes by doubling
(`'don''t'`). The `--jql` escape hatch on `search-jira-issues` lets users
write raw JQL when the helpers are insufficient — at their own risk.

References:

- [JQL escaping community thread](https://community.atlassian.com/forums/Jira-questions/Help-with-JQL-query-quotation-marks/qaq-p/1769883)
- [JRA-23235 escaping special characters](https://jira.atlassian.com/browse/JRASERVER-23235)

#### 2.11 Useful expansions and field shapes

`expand` supports comma-separated values:

- `names` — adds a top-level `names` map of fieldId → human name. Useful so
  consumers don't need a separate `/field` call.
- `schema` — adds a `schema` map describing each field's type. Combined
  with `names` produces a self-describing payload.
- `renderedFields` — returns rich-text fields pre-rendered to HTML alongside
  the ADF originals. We will **not** rely on this — we render ADF
  ourselves with the jq script (see §4.4) for consistency. Listed here for
  completeness.
- `transitions` — embeds the transitions list inside a `GET issue`
  response.
- `changelog` — issue history.
- `editmeta` — per-field editability metadata.

Field shapes worth noting:

- `summary` — string.
- `description` — ADF JSON in v3.
- `assignee` / `reporter` — `{"accountId":"...","displayName":"...","active":true}`.
  Write with `accountId` only.
- `priority` — `{"id":"3","name":"Medium"}`. Writeable with either.
- `status` — read-only on issue; change via transitions.
- `labels` — array of strings; no spaces inside a label.
- `components` / `fixVersions` / `versions` — array of `{"id":"...","name":"..."}`.
- `duedate` — `"YYYY-MM-DD"`.
- `parent` — `{"key":"ENG-1"}`; covers subtask-parent and epic-parent.

#### 2.12 Rate limits

Three concurrent rate-limit systems on Cloud:

1. **Points-based hourly quota** — request cost varies; default 65,000/hour
   global.
2. **Burst limit (per-second per endpoint)** — ~100 RPS GET/POST, ~50 RPS
   PUT/DELETE.
3. **Per-issue write limit** — 20 writes / 2s, 100 writes / 30s, single
   issue.

Headers on 429:

- `Retry-After` — seconds. **Always honour first.**
- `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`.
- `RateLimit-Reason` — one of `jira-quota-global-based`,
  `jira-quota-tenant-based`, `jira-burst-based`,
  `jira-per-issue-on-write`.

The `jira-request.sh` helper (see §4.3) handles 429 with capped exponential
backoff (4 retries, jitter ±30%), prefering `Retry-After` when present.

Reference: [Rate limiting — Jira Cloud](https://developer.atlassian.com/cloud/jira/platform/rate-limiting/).

#### 2.13 Error response shape

Non-2xx bodies (400, 401, 403, 404, 409, 410, 422, 429, 500):

```json
{
  "errorMessages": ["Top-level message"],
  "errors": {"summary": "Summary is required."},
  "status": 400
}
```

`errorMessages` carries general messages (auth, deprecation 410s);
`errors` is field-keyed validation errors. Either may be empty.

`jira-request.sh` surfaces the body to stderr alongside the status code so
calling skills can present a useful diagnostic.

Status code semantics worth handling explicitly:

- **401** — missing/invalid Authorization. Likely token expired or wrong
  email.
- **403** — authenticated but lacking project permission, **or** missing
  `X-Atlassian-Token: no-check` on attachment uploads.
- **404** — issue/project not found *or* user lacks permission to see it
  (Jira prefers 404 over 403 here).
- **410** — deprecated endpoint. Usually means we hit a legacy route by
  mistake.
- **429** — rate limited; respect `Retry-After`.

### 3. Recent breaking changes that constrain the design

#### 3.1 v2 API retired (October 2025)

The Jira Cloud REST API v2 was deprecated May 2025, progressively shut down
August–October 2025, and **fully retired October 31 2025** (returns 410
Gone). Older guidance ("send wiki markup to the v2 endpoint and let Jira
convert it to ADF server-side") **is no longer viable**.

This is the single most important constraint on the design. Every write
that touches a rich-text field must include valid ADF. We cannot fall back
to wiki markup. The legacy `pf-editor-service/convert` endpoint that some
integrations relied on for wiki↔ADF conversion was decommissioned in 2024
and has no public replacement.

References:

- [REST API migration timeline](https://developer.atlassian.com/cloud/jira/platform/deprecation-notice-user-privacy-api-migration-guide/)
- [JRACLOUD-77436 — Public REST API endpoint to convert between ADF and other formats](https://jira.atlassian.com/browse/JRACLOUD-77436)
- [Community thread — pf-editor-service decommissioning](https://community.developer.atlassian.com/t/post-html-issue-description-with-jira-rest-api-v3/38482)

#### 3.2 Search endpoint migration (October 2024)

`GET /rest/api/3/search` was deprecated October 2024, replaced by
`POST /rest/api/3/search/jql` plus
`POST /rest/api/3/search/approximate-count`. Pagination switched from
offset (`startAt`/`total`) to opaque-token (`nextPageToken`). `total` is
no longer returned.

Implications:

- We cannot show "1 of 247 results" without a separate approximate-count
  call.
- Pagination state must be threaded explicitly through helper scripts (a
  call passes `--page-token` and a response surfaces the next token).
- The bulk-fetch companion endpoint `POST /rest/api/3/issue/bulkfetch`
  is the recommended way to retrieve many issues by ID/key without a JQL
  query — useful for hydrating a list of work items by ID.

#### 3.3 Epic Link → `parent` migration

Use `parent` exclusively for both subtask-parent and epic-parent. See §2.9.

#### 3.4 Createmeta endpoint refactor

The legacy `GET /rest/api/3/issue/createmeta` is deprecated in favour of
two paginated replacements:

- `GET /rest/api/3/issue/createmeta/{projectIdOrKey}/issuetypes`
- `GET /rest/api/3/issue/createmeta/{projectIdOrKey}/issuetypes/{issueTypeId}`

`init-jira` and `create-jira-issue` use the new paths. Removal of the legacy
endpoint has been postponed indefinitely as of this research, but we use
the modern paths to avoid future breakage.

### 4. ADF round-trip strategy

The full Markdown ↔ ADF round-trip is the central design challenge for a
bash-only client. The strategy is **Strategy A only**: pure bash + jq + awk
for both directions, supporting a deliberately constrained Markdown subset.
No optional binary dependencies, no shell-out, no `--raw-adf` escape
hatch — power users can construct ADF by hand if they need richer
formatting, or wait for the Rust CLI.

#### 4.1 Supported Markdown subset

| Markdown | ADF node | Notes |
| --- | --- | --- |
| `# heading` … `###### heading` | `heading` with `attrs.level` 1–6 | ATX style only; no setext. |
| Blank-line-separated paragraphs | `paragraph` with `text` children | The default. |
| `**bold**` / `__bold__` | `text` with `marks: [{type: strong}]` | Inline. |
| `*italic*` / `_italic_` | `text` with `marks: [{type: em}]` | Inline. |
| `` `code` `` | `text` with `marks: [{type: code}]` | Inline. |
| `[text](url)` | `text` with `marks: [{type: link, attrs: {href}}]` | Inline. |
| Trailing two-space line wrap | `hardBreak` node | Inside a paragraph. |
| `- item` / `* item` / `+ item` | `bulletList` / `listItem` / `paragraph` | Single level only. |
| `1. item` | `orderedList` / `listItem` / `paragraph` | Single level only. |
| `- [ ] task` / `- [x] task` | `taskList` / `taskItem` with `attrs.state` `TODO`/`DONE`, `attrs.localId` | `localId` from `uuidgen`. |
| ``` ```lang\n…``` ``` | `codeBlock` with `attrs.language` | Triple-backtick fences only. |

**Out of scope** (will round-trip as `[unsupported: <type>]` placeholders
on read; rejected with a clear error on write):

- Tables, panels, expand, blockquote, mediaSingle, mediaGroup, status,
  date, mention, emoji, inlineCard, rule (`---`), strike, underline, sub/sup,
  text colour, nested lists.

This subset covers the overwhelming majority of work-item description
content (rule-of-thumb ~80%) and avoids the combinatorial explosion of
nested-list state management and table column-alignment that pure-bash
implementations always struggle with.

#### 4.2 Markdown → ADF compiler architecture

`jira-md-to-adf.sh`, ~250–400 lines, two-pass:

**Pass 1** (awk) — block tokeniser:

1. Read stdin line by line. Track state: `BODY`, `CODE_FENCE`, `LIST`.
2. Recognise fenced code (`^```\(\w*\)$`) — toggle `CODE_FENCE` state.
   Inside, every line is a literal until the matching close fence.
3. Recognise blank lines as block separators (outside `CODE_FENCE`).
4. Recognise headings (`^#{1,6} `).
5. Recognise list items (`^[-*+] `, `^\d+\. `, `^[-*+] \[[ x]\] `).
6. Emit a stream of `RECORD-TYPE\tDATA` lines, one per logical block:
   `H{level}\t<text>`, `P\t<text>`, `BUL\t<text>`, `ORD\t<text>`,
   `TASK_TODO\t<text>`, `TASK_DONE\t<text>`, `CODE_OPEN\t<lang>`,
   `CODE_LINE\t<text>`, `CODE_CLOSE\t`.

**Pass 2** (jq with `--raw-input --slurp`) — record-stream → ADF:

1. Split stdin on newlines, parse each line into `{type, data}`.
2. Group consecutive `BUL`/`ORD`/`TASK_*` records into list nodes.
3. Group `CODE_OPEN`/`CODE_LINE`*/`CODE_CLOSE` into `codeBlock`.
4. For each block, run the inline tokeniser (also in jq) on the text
   payload. Inline marks `**…**`, `*…*`, `` `…` ``, `[…](…)` are matched
   greedily-but-non-overlapping; ambiguous cases (`***bold-italic***`)
   are not supported.
5. Wrap the result in `{"version":1,"type":"doc","content":[…]}` and
   emit.

Inline tokeniser detail: walk the text left-to-right, at each position
attempt patterns in order: code (`` ` ``), link (`[`), bold (`**`),
italic (`*`). Emit `text` nodes with appropriate marks. Backslash
escapes (`\*`, `\``, `\[`) emit literal characters with no marks.

The `localId` for `taskItem` nodes is generated via `uuidgen` (BSD/GNU
both ship it; the helper script falls back to a `/dev/urandom`-based
hex generator if absent).

The compiler rejects unsupported nodes by name in pre-validation: if the
input contains `|` followed by another `|` on the same line, or `:::`
panel fences, or HTML, the script exits non-zero with a list of detected
unsupported features.

#### 4.3 ADF → Markdown renderer architecture

`jira-adf-to-md.sh`, ~150–250 lines, single-pass jq:

A recursive jq function `render(node)` matches on `.type`:

```
def render:
  if   .type == "doc"        then .content | map(render) | join("\n\n")
  elif .type == "paragraph"  then .content // [] | map(render_inline) | join("")
  elif .type == "heading"    then "#" * .attrs.level + " " + (.content | map(render_inline) | join(""))
  elif .type == "bulletList" then .content | map("- " + render) | join("\n")
  …
  end;
```

(Pseudo-jq; actual implementation uses jq's actual syntax.)

`render_inline` walks `text` nodes, looks at the `marks` array, and wraps
the text in the appropriate Markdown delimiters. Mark precedence is
fixed: link wraps everything; then bold; then italic; then code (innermost).

For unsupported node types, emit `[unsupported ADF node: <type>]` on its
own line. This is visible to the user (so they know something was lost)
and unambiguous to the compiler if round-tripped (the compiler rejects it
as a literal-unmodified placeholder).

**Performance** is a non-issue: even large issues are tens of KB; jq parses
and walks them in milliseconds.

#### 4.4 The `--render-adf` flag on data-fetching helpers

Helpers like `jira-show.sh` and `jira-search.sh` accept `--render-adf`. When
set, the helper post-processes its JSON response: for each known
ADF-bearing field path (`fields.description`, `fields.environment`, each
`comments.comments[].body`, multi-line custom fields with `schema.type ==
"string"` and `schema.system` matching the textarea pattern), pipe the
ADF subtree through `jira-adf-to-md.sh` and replace it with the rendered
Markdown string in the JSON tree (using `jq`'s `setpath`). The output
shape is otherwise unchanged.

The standalone `jira-adf-to-md.sh` filter is the building block; the
`--render-adf` flag is a convenience layered on top so callers don't need
to walk the JSON themselves.

### 5. jira-cli patterns we adopt

#### 5.1 Auth-cmd indirection

jira-cli supports keychain integration via OS-specific paths (Freedesktop
Secret Service on Linux, macOS Keychain on Mac). We replace this with a
single, more general primitive: `ACCELERATOR_JIRA_TOKEN_CMD`. If set, the
auth helper runs the command with `bash -c` and captures stdout as the
token. This subsumes:

- 1Password CLI: `op read op://Personal/Atlassian/credential`
- pass: `pass show personal/atlassian-api-token`
- macOS Keychain: `security find-generic-password -s "atlassian-api-token" -w`
- Freedesktop Secret Service: `secret-tool lookup service jira-cli account "$EMAIL"`
- AWS Secrets Manager: `aws secretsmanager get-secret-value --secret-id jira-token --query SecretString --output text`

…with no codebase-side knowledge of any of them.

#### 5.2 JQL builder + ~-negation

`jira-jql.sh` exposes shell-callable builder functions:

- `jql_filter <field> <value>` — single-value `field = "value"` or
  `field IS EMPTY`/`IS NOT EMPTY` for `EMPTY`/`~EMPTY` sentinels
- `jql_in <field> <value...>` — multi-value `field IN ("a","b")`
- `jql_not_in <field> <value...>` — `field NOT IN ("a","b")`
- `jql_split_neg <values>` — split `~`-prefixed entries from positives,
  emit two arrays for `IN`/`NOT IN`

The skills compose calls into an `AND`-joined JQL string. The `--jql`
escape hatch concatenates a user-supplied raw clause with `AND` to the
project-scoping clause if `--project` is also supplied.

#### 5.3 Per-issue transition lookup by state name

`transition-jira-issue` accepts a state name (`"In Progress"`,
`"Done"`, etc.), case-insensitive. Implementation: GET the transitions
list, find the entry whose `to.name` (lower-cased) matches, POST with
that transition ID. If multiple transitions lead to the same state name,
list them and ask the user to disambiguate by transition ID.

#### 5.4 Body-input precedence chain

For `create-jira-issue`, `update-jira-issue`, and `comment-jira-issue`:

1. `--body "<inline>"` — highest precedence
2. `--body-file <path>` — file contents
3. `--body-from-stdin` — stdin (or implicit if stdin is not a TTY)
4. `$EDITOR` on a tempfile — only if no other body source supplied AND
   stdout is a TTY AND `--no-input` is not set

This matches jira-cli's chain and gives both interactive (editor) and
scripted (stdin/file/inline) flows from the same skill prose.

#### 5.5 Field cache at init

`init-jira` calls `GET /rest/api/3/field` once and persists the resulting
catalogue. Subsequent skills resolve friendly names via the cache rather
than re-fetching. See §4.5 for cache layout and §4.9 for the init flow.

### 6. Output convention

**JSON-first uniformly**. Every helper script emits raw API JSON to stdout
by default. Status/error messages go to stderr.

The `--render-adf` flag (described in §4.4) is supported on read-side
helpers and walks known ADF-bearing field paths to replace them with
rendered Markdown strings — without changing the surrounding JSON shape.

There is **no `--plain` or `--csv`** at the helper level. Skills that want
tabular output let Claude render the JSON directly (which it does well).
This is a deliberate departure from jira-cli's three-format-per-command UX:
helpers are primitives for Claude to compose, not human-facing CLIs.

The standalone `jira-adf-to-md.sh` filter (stdin→stdout) is available for
ad-hoc piping when a script wants only the rendered description without
the surrounding JSON envelope.

## Design Proposal

### 4.1 Skill set

Eight verb-decomposed skills:

| Skill | Purpose | Read/Write |
| --- | --- | --- |
| `init-jira` | Verify auth; discover projects, issue types, custom fields; persist field catalogue and project list | Read |
| `search-jira-issues` | Search via JQL or composed flags; paginate; return matching issues | Read |
| `show-jira-issue` | Fetch one issue with description, comments, transitions; optionally render ADF | Read |
| `create-jira-issue` | Create a new issue (interactive or from args/stdin/template) | Write |
| `update-jira-issue` | Edit fields on an existing issue (summary, description, assignee, priority, labels, components, parent, custom fields) | Write |
| `transition-jira-issue` | Move an issue through workflow by state name | Write |
| `comment-jira-issue` | Add, edit, or delete comments | Write |
| `attach-jira-issue` | Upload one or more files as attachments | Write |

Skills NOT included in v1, but plausible follow-ups:

- `assign-jira-issue` — folded into `update-jira-issue` (`--assignee`).
- `link-jira-issues` — issue links are deferred; covered later by sync work.
- `list-jira-projects`, `list-jira-fields` — exposed as flags on
  `init-jira` (`--list-projects`, `--list-fields`) rather than separate
  skills, keeping the surface area small.

### 4.2 Directory layout

```
skills/
  integrations/
    jira/
      scripts/
        jira-common.sh             # shared helpers (path resolution, error formatting)
        jira-auth.sh               # token resolution chain
        jira-request.sh            # signed HTTP request with retries
        jira-jql.sh                # JQL builder + safe quoting
        jira-fields.sh             # custom-field discovery, name↔ID resolution
        jira-md-to-adf.sh          # Markdown → ADF compiler
        jira-adf-to-md.sh          # ADF → Markdown renderer
        test-jira-md-to-adf.sh
        test-jira-adf-to-md.sh
        test-jira-jql.sh
        test-fixtures/
          adf-samples/
          api-responses/
      init-jira/
        SKILL.md
      search-jira-issues/
        SKILL.md
      show-jira-issue/
        SKILL.md
      create-jira-issue/
        SKILL.md
      update-jira-issue/
        SKILL.md
      transition-jira-issue/
        SKILL.md
      comment-jira-issue/
        SKILL.md
      attach-jira-issue/
        SKILL.md
```

### 4.3 Helper scripts

**`jira-common.sh`** — sourceable. Defines `jira_die`, `jira_warn`,
`jira_log`, path resolution (`jira_find_repo_root`, `jira_state_dir`),
JSON utility wrappers around jq.

**`jira-auth.sh`** — sourceable. Exposes `jira_resolve_credentials` which
returns site, email, and token via the resolution chain (§4.8). Caller
sources this and reads from the resulting variables; the script itself
never prints the token.

**`jira-request.sh`** — executable. Wraps curl. Usage:
`jira-request.sh GET /rest/api/3/myself`,
`jira-request.sh POST /rest/api/3/issue --json @body.json`,
`jira-request.sh POST /rest/api/3/issue/KEY/attachments --multipart "file=@./path"`.
Responsibilities:

- Resolve credentials via `jira-auth.sh`.
- Compose the URL from the configured site.
- Sign the request with `Authorization: Basic`.
- Set `X-Atlassian-Token: no-check` for multipart.
- Honour `Retry-After` on 429 with capped exponential backoff (max 4
  retries, jitter ±30%).
- On non-2xx, surface the response body to stderr alongside the status
  code; exit non-zero with a code mapped to status (e.g. exit 11 for
  401, 12 for 403, 13 for 404, 19 for 429, 20 for 5xx).
- Emit response body to stdout.

**`jira-jql.sh`** — sourceable. Functions described in §5.2.

**`jira-fields.sh`** — executable + sourceable. As executable:
`jira-fields.sh refresh` re-fetches and persists the catalogue;
`jira-fields.sh resolve <friendly-name>` prints the `customfield_NNNNN`
ID; `jira-fields.sh list` prints the cached catalogue. As sourceable:
exposes `jira_field_id <name>` and `jira_field_schema <id>` for use
within other helpers.

**`jira-md-to-adf.sh`**, **`jira-adf-to-md.sh`** — executable filters
(stdin → stdout). Architecture in §4.2 and §4.3.

### 4.4 Configuration schema

New top-level YAML section in `accelerator.md` and `accelerator.local.md`:

```yaml
---
jira:
  site: atomic-innovation                       # the Cloud subdomain
  email: toby@go-atomic.io                      # account email
  token: ""                                     # rarely set in shared config
  token_cmd: "op read op://Work/Atlassian/credential"  # auth-cmd indirection
---
```

All four keys are optional. Resolution order documented in §4.8.

The `work.default_project_code` config key (already defined) doubles as the
default Jira project key. Skills that take a `--project` flag fall back to
this when omitted; if neither is set, the skill errors with a clear message
pointing to the configuration.

### 4.5 State layout

```
meta/
  integrations/
    jira/
      site.json              # {site, accountId, displayName, lastVerified}
      fields.json            # cached output of GET /field, with timestamp
      projects.json          # cached output of GET /project, with timestamp
      issuetypes/
        ${PROJECT}.json      # cached createmeta per project (lazy)
```

`fields.json` schema:

```json
{
  "lastUpdated": "2026-04-29T22:31:35+01:00",
  "site": "atomic-innovation",
  "fields": [
    {
      "id": "customfield_10016",
      "name": "Story Points",
      "slug": "story-points",
      "custom": true,
      "schema": {"type": "number", "customId": 10016},
      "clauseNames": ["cf[10016]", "Story Points", "Story point estimate"]
    },
    …
  ]
}
```

`slug` is generated client-side from `name` via lowercase + non-alphanumeric → dash
collapse, matching jira-cli's slug convention. Skills accept either the slug
(`story-points`) or the full ID (`customfield_10016`) in `--custom`-style
flags.

The `meta/integrations/jira/` directory is version-controlled and shared
across the team. This is deliberate: per-instance custom-field IDs are not
secrets and are useful to share. Per-developer auth lives in
`accelerator.local.md` (and env vars / token_cmd).

### 4.6 Output convention recap

Helpers emit raw API JSON to stdout. The `--render-adf` flag is supported
where rich-text fields appear in the response (`show-jira-issue`,
`search-jira-issues` only when `--fields description` or similar is
explicitly requested, `comment-jira-issue` on read paths). The standalone
`jira-adf-to-md.sh` is available for ad-hoc piping.

### 4.7 JQL safety

`jira-jql.sh`:

- Wraps every value in single quotes.
- Doubles internal single quotes (`'don''t'`).
- Rejects values containing characters not in `[A-Za-z0-9_\-. @+:/]` *unless*
  they are escaped via the function's own escape rules.
- The `--jql` escape hatch in `search-jira-issues` is logged with a clear
  "raw JQL passed through" note to stderr so users know they bypassed the
  builder.

Default scoping: every search prepends `project = "${PROJECT}" AND` when a
project is configured (matching jira-cli's behaviour). `--all-projects`
disables the prepended scope.

### 4.8 Auth resolution algorithm

`jira-auth.sh` resolves the token in this order, stopping at the first
non-empty value:

1. `ACCELERATOR_JIRA_TOKEN` env var (highest priority).
2. `ACCELERATOR_JIRA_TOKEN_CMD` env var — run as `bash -c "$cmd"`,
   capture stdout, trim trailing whitespace.
3. `accelerator.local.md` `jira.token`.
4. `accelerator.local.md` `jira.token_cmd` — run, capture stdout.
5. `accelerator.md` `jira.token`.
6. `accelerator.md` `jira.token_cmd` — run, capture stdout.

Site and email follow the same chain (without the `_CMD` indirection —
those are not secrets and don't warrant a shell escape).

If the resolved token is empty, the helper prints a message pointing to
`accelerator.md`'s `jira:` section with example syntax and exits non-zero.

The token is **never logged**, including under `--debug`. The `jira-request.sh`
debug output redacts it from the curl command line.

### 4.9 Init flow

`init-jira` walks the user through:

1. **Site** — accept `--site` arg; otherwise prompt. Validate form
   (subdomain only, no protocol or path).
2. **Email** — `--email` arg or prompt. Validate as email-shaped.
3. **Token** — try the resolution chain. If empty, print a message
   directing the user to `https://id.atlassian.com/manage-profile/security/api-tokens`
   and explaining how to set `ACCELERATOR_JIRA_TOKEN` /
   `jira.token` / `jira.token_cmd`. Exit non-zero.
4. **Verify** — call `GET /rest/api/3/myself`. Expect 200 with
   `accountId`, `displayName`, `emailAddress`, `timeZone`. Print confirmation.
5. **Discover projects** — `GET /rest/api/3/project`. Persist to
   `meta/integrations/jira/projects.json`.
6. **Discover fields** — `GET /rest/api/3/field`. Compute slugs.
   Persist to `meta/integrations/jira/fields.json`.
7. **Default project** — if `work.default_project_code` is unset, prompt
   the user to choose from the discovered projects. Update
   `accelerator.md` (or `accelerator.local.md` based on user choice) with
   `work.default_project_code`.
8. **Persist site metadata** — `meta/integrations/jira/site.json` with
   `{site, accountId, displayName, lastVerified}`.

Re-running `init-jira` re-verifies and refreshes the caches. Idempotent.

`init-jira --refresh-fields` is a fast path that only re-fetches the
field catalogue without re-prompting.

`init-jira --list-projects` and `init-jira --list-fields` are read-only
sub-modes that print the cached data; they are the answer to "I need to
see what's available" without spawning new skills.

### 4.10 Per-skill design notes

**`init-jira`** — flow in §4.9. SKILL.md primary content is the prompts and
the persistence steps; very little Claude reasoning required.

**`search-jira-issues`** — accepts `--project`, `--status` (multi),
`--assignee` (`@me` resolves to current user via the cached `accountId`,
`x` for unassigned), `--type` (multi), `--label` (multi), `--component`
(multi), `--reporter`, `--parent`, `--watching`, `--free-text` (positional),
`--jql` (escape hatch), `--order-by`, `--reverse`, `--limit`, `--page-token`,
`--fields` (comma-separated). All multi-value flags accept `~` prefix for
negation. Calls `jira-jql.sh` to compose, then `jira-request.sh POST
/rest/api/3/search/jql`. Response is the API's `{issues, nextPageToken}` JSON
verbatim. With `--render-adf`, walks each issue's description (if present
in `fields`).

**`show-jira-issue`** — single positional argument: issue key. Optional
`--expand` (default: `renderedFields,names,schema,transitions`),
`--fields` (default: `*all`), `--comments <N>` (default: 0; uses a separate
GET to the comments endpoint and embeds the `N` most recent), `--render-adf`.

**`create-jira-issue`** — required: `--project` (or default), `--type`,
`--summary`. Optional: `--body` / `--body-file` / stdin / editor (chain in
§5.4), `--assignee`, `--reporter`, `--priority`, `--label` (multi),
`--component` (multi), `--parent`, `--custom <slug>=<value>` (multi). Runs
the body through `jira-md-to-adf.sh` before POST. Returns the API response
JSON (which includes the new key in `key`).

**`update-jira-issue`** — required: positional issue key. Optional any of
the create fields except `--type` and `--project` (immutable on edit).
Body changes use `jira-md-to-adf.sh`. Multi-value fields support `--add-label`,
`--remove-label`, etc. for `update`-style operations; `--label` alone is a
set operation.

**`transition-jira-issue`** — required: positional issue key, then state
name. Optional: `--resolution`, `--comment` (multi-line, runs through
ADF). Looks up transitions, matches by lowercased state name, posts with
matching transition ID.

**`comment-jira-issue`** — sub-actions: `add KEY [body...]`, `list KEY`,
`edit KEY COMMENT_ID [body...]`, `delete KEY COMMENT_ID`. Body chain as
in create. Renders ADF on `list` when `--render-adf` is set.

**`attach-jira-issue`** — required: positional issue key, then one or more
file paths. Multipart upload via `jira-request.sh POST … --multipart`.

## Phased Implementation

Four phases, slicing for independent deliverable value at each cut.

### Phase 1 — Foundation

**Scope.** Configuration schema, auth resolution, the request helper, the
ADF round-trip pair, and `init-jira`. No user-visible work-related skills
yet, but everything downstream depends on this.

**Deliverables.**

- New `jira` config section in `skills/config/configure/SKILL.md`
  documentation.
- `scripts/test-config.sh` updates for `jira.*` keys.
- `skills/integrations/jira/scripts/jira-common.sh`
- `skills/integrations/jira/scripts/jira-auth.sh`
- `skills/integrations/jira/scripts/jira-request.sh` with retry logic
- `skills/integrations/jira/scripts/jira-jql.sh`
- `skills/integrations/jira/scripts/jira-fields.sh`
- `skills/integrations/jira/scripts/jira-md-to-adf.sh`
- `skills/integrations/jira/scripts/jira-adf-to-md.sh`
- `skills/integrations/jira/scripts/test-jira-*.sh` — unit tests for
  ADF round-trip (with hand-written fixtures) and JQL builder
- `skills/integrations/jira/init-jira/SKILL.md`
- `.claude-plugin/plugin.json` — add `"./skills/integrations/jira/"` entry
- `meta/integrations/jira/.gitkeep` (or seed contents to commit the path)

**Exit criteria.** A user with credentials in `accelerator.local.md` runs
`/init-jira`, sees their site verified, gets `meta/integrations/jira/`
populated with the field and project catalogues. ADF round-trip helpers
pass tests covering the supported subset and reject unsupported nodes
cleanly.

**Depends on.** Nothing.

### Phase 2 — Read skills

**Scope.** The two read skills that complete the day-one usability story.

**Deliverables.**

- `skills/integrations/jira/search-jira-issues/SKILL.md`
- `skills/integrations/jira/show-jira-issue/SKILL.md`
- Eval fixtures using a local replay server (jira-cli style) under
  `skills/integrations/jira/scripts/test-fixtures/api-responses/`
- README updates announcing the integration with example invocations

**Exit criteria.** End-to-end smoke test: `/search-jira-issues --jql
"assignee = currentUser() AND statusCategory != Done"` returns a usable
list. `/show-jira-issue PROJ-123 --comments 3 --render-adf` renders a
ticket cleanly with comments inline.

**Depends on.** Phase 1.

### Phase 3 — Write skills (excluding workflow)

**Scope.** Authoring workflows: create, update, comment.

**Deliverables.**

- `skills/integrations/jira/create-jira-issue/SKILL.md`
- `skills/integrations/jira/update-jira-issue/SKILL.md`
- `skills/integrations/jira/comment-jira-issue/SKILL.md`
- Body-input precedence chain shared via `jira-common.sh` helper
- Eval fixtures for the write paths

**Exit criteria.** `/create-jira-issue --project ENG --type Task --summary
"foo" --body-file plan.md` round-trips a Markdown plan as ADF and creates
an issue. `/update-jira-issue ENG-1 --summary "bar"` patches it.
`/comment-jira-issue add ENG-1 "ack"` adds a comment.

**Depends on.** Phase 1 (specifically the ADF compiler and the request
helper). Phase 2 is not strictly required but `show-jira-issue` is useful
for verifying writes.

### Phase 4 — Workflow and attachments

**Scope.** The remaining skills, both of which add their own complexity
(transition lookup, multipart upload).

**Deliverables.**

- `skills/integrations/jira/transition-jira-issue/SKILL.md`
- `skills/integrations/jira/attach-jira-issue/SKILL.md`
- Test coverage for multi-state transition disambiguation
- Test coverage for multipart upload signing

**Exit criteria.** `/transition-jira-issue ENG-1 "In Progress"` walks the
state machine. `/attach-jira-issue ENG-1 ./screenshot.png ./logs.txt`
uploads attachments.

**Depends on.** Phases 1 and 3.

### Phase Summary

| Phase | Focus | Skills | New Scripts | Complexity |
| --- | --- | --- | --- | --- |
| 1 | Foundation | `init-jira` | 7 helpers + tests | High (ADF compilers, retry logic, auth chain) |
| 2 | Read | `search-jira-issues`, `show-jira-issue` | — | Medium |
| 3 | Write | `create-jira-issue`, `update-jira-issue`, `comment-jira-issue` | — | Medium-High (body chain, update ops) |
| 4 | Workflow & files | `transition-jira-issue`, `attach-jira-issue` | — | Medium |

Phase 1 is the largest. Phases 2–4 are independent of each other except
for the soft Phase 2 → 3 ordering for verification ergonomics.

## Code References

- `skills/decisions/create-adr/SKILL.md` — pattern for interactive skills
  with prompts and persistence.
- `skills/work/scripts/work-item-pattern.sh` — bash + awk style guide for
  pattern-based scripts.
- `skills/work/scripts/work-item-resolve-id.sh` — pattern for ID
  resolution with multiple input forms.
- `skills/github/describe-pr/SKILL.md` — pattern for skills delegating to
  external commands; SKILL.md preamble structure.
- `skills/config/configure/SKILL.md` — where new `jira.*` config docs land.
- `scripts/config-read-value.sh` — the generic value reader; `jira.*`
  keys are read with no changes to the reader itself.
- `scripts/config-read-path.sh` — paths config (no changes; we use a fixed
  `meta/integrations/jira/` location).
- `~/.claude/skills/jira/SKILL.md` — UX prior art for filter flags and
  `~`-negation.
- `meta/decisions/ADR-0017-configuration-extension-points.md` — framework
  for adding `jira.*`.

## Architecture Insights

### Pattern: First true API client in the codebase

The `skills/github/` family delegates to `gh`, never to HTTP APIs directly.
The Jira integration is the first time the codebase signs and dispatches
HTTP requests itself, retries on rate limits, and parses error bodies.
The new `jira-request.sh` helper is the natural seed of a more general
`http-request.sh` if a future integration (Linear, Trello, Shortcut) wants
to share retry and auth-resolution machinery. We deliberately do not lift
this generality in v1 — wait for the second concrete consumer before
abstracting.

### Pattern: Cached integration state under meta/

`meta/integrations/jira/` is the first persistent integration state in the
codebase. The choice to version-control it (rather than gitignore) reflects
the principle that field IDs and project lists are team-shared, not
secrets. Per-developer state (auth) goes elsewhere. Future integrations
(Linear's team UUIDs, Trello's board IDs, Shortcut's workflow IDs) follow
the same split: shared identifiers committed, personal credentials kept
out.

### Pattern: Auth-cmd indirection as keychain abstraction

A single `_TOKEN_CMD` env var (or config key) that runs an arbitrary shell
command and uses stdout as the token is strictly more general than coding
keychain backends. We deliberately avoid platform-specific paths
(Freedesktop Secret Service / macOS Keychain / Windows Credential Manager)
and rely on the user's existing CLI tooling. This is the same shape jira-cli
arrives at via the `JIRA_API_TOKEN_AUTH_CMD` discussion threads but
generalised consistently.

### Pattern: Constrained Markdown for round-trip safety

ADF ↔ Markdown is a hard problem. Choosing a deliberately small subset
(paragraphs, headings, code, lists, checklists, inline marks, links) lets
us implement both directions in pure bash + awk + jq with reasonable
effort. The cost is honest: tables, panels, mentions, media, nested lists
are not supported. The supported subset covers the realistic majority of
work-item content. Richer support waits for the planned Rust CLI, where
a real Markdown parser and ADF emitter are tractable.

### Pattern: JSON-first helpers, formatted skills

Helper scripts emit raw API JSON; skill prose describes how Claude should
present that JSON to the user. This is consistent with the codebase's
existing bash + jq habit and with the principle that skills are prose
read by Claude rather than human-facing CLIs. The `--render-adf` flag is
the one concession to formatted output, because ADF is not human-readable
even for Claude — it is the one place where the helper does its own
rendering.

## Historical Context

- `meta/research/2026-04-08-ticket-management-skills.md:916-920` (Resolved
  Question 5) — Sync with external systems was explicitly deferred as a
  future enhancement. This research is one half of that follow-up; the
  sync skill itself remains deferred.
- `meta/research/2026-04-28-configurable-work-item-id-pattern.md` — The
  ID-pattern work that established `{project}-{number:04d}` filenames and
  `work.default_project_code`. This research reuses that field as the
  default Jira project key without introducing a new config knob.
- `meta/decisions/ADR-0017-configuration-extension-points.md` — The
  framework under which `jira.*` is added.
- `meta/decisions/ADR-0023-meta-directory-migration-framework.md` — The
  migration framework that would handle a future
  `meta/integrations/` → `.accelerator/state/integrations/` move (see
  Open Questions).
- `~/.claude/skills/jira/SKILL.md` — The user's existing personal Jira
  skill, which supplied UX prior art for filter flags, `~`-negation, and
  common usage patterns.

## Resolved Questions

1. **Transport** — RESOLVED: bash + curl directly to the Jira Cloud REST
   API v3. No `jira-cli` dependency. No language other than bash + awk +
   jq + sed. A future Rust CLI will replace these scripts; they should
   transliterate cleanly.

2. **Skill decomposition** — RESOLVED: verb-decomposed (eight skills) for
   progressive disclosure. Justified by the codebase's existing pattern in
   `skills/work/` and by Claude's context-management costs of monolithic
   skills.

3. **Skill location** — RESOLVED: `skills/integrations/jira/` as a new
   top-level category. Future integrations (Linear, Trello, etc.) live as
   sibling directories under `skills/integrations/`.

4. **Jira deployment scope** — RESOLVED: Cloud only for v1.
   Server/Data Center support is a future enhancement, hampered by Server
   nearing end-of-life anyway. mTLS, scoped tokens, and OAuth 3LO are all
   out of scope.

5. **Read vs write coverage** — RESOLVED: full lifecycle in scope (search,
   read, create, update, transition, comment, attach). Issue links and
   bulk operations are deferred; assignment is folded into update.

6. **Sync skill** — RESOLVED: explicitly out of scope for this research.
   Once we have at least one integration shipped (and ideally a second,
   e.g. Linear), the sync skill becomes a separate research item that can
   reason about the actual integrations rather than hypothetical ones.

7. **Auth model** — RESOLVED: env var > token_cmd indirection >
   `accelerator.local.md` > `accelerator.md`. The
   `ACCELERATOR_JIRA_TOKEN_CMD` indirection subsumes 1Password, pass,
   keychain, AWS Secrets Manager, etc. without codebase-side knowledge.

8. **State storage location** — RESOLVED for v1: `meta/integrations/jira/`,
   version-controlled. The longer-term reorg to a top-level
   `.accelerator/{config,state,tmp}/` tree (covering accelerator-specific
   markdown config, migration state files, integration caches, and tmp)
   is captured as a separate note for future work.

9. **Default project key** — RESOLVED: reuse `work.default_project_code`.
   No separate `jira.default_project_key`. A user's local work-item
   project code IS their Jira project key.

10. **Output format** — RESOLVED: JSON-first uniformly. The `--render-adf`
    flag walks known ADF-bearing fields and replaces them in-place with
    rendered Markdown strings. Standalone `jira-adf-to-md.sh` available
    for ad-hoc piping. No `--plain` or `--csv` at the helper level —
    Claude handles formatting from JSON.

11. **Markdown subset for ADF round-trip** — RESOLVED: paragraphs,
    headings, fenced code blocks (with language), single-level bullet/
    ordered lists, GitHub-style checklists, inline bold/italic/code/links,
    hard breaks. Tables, panels, mentions, media, nested lists, status
    badges, dates, and rich-text marks beyond the basics are deferred to
    the future Rust CLI.

12. **Search interface** — RESOLVED: minimal flags (status, assignee,
    type, label, component, reporter, parent, watching, free-text) plus a
    `--jql` escape hatch. JQL composition with `~`-prefix negation
    matches the user's existing personal jira skill UX.

## Open Questions

None blocking. Two longer-term items captured separately:

- **Top-level `.accelerator/` reorg** — captured in
  `meta/notes/2026-04-29-accelerator-config-state-reorg.md`. Affects
  config, migration framework, integration state, and tmp paths. Worth
  doing before too many integrations bake the `meta/integrations/`
  location into prose.
- **Linear and Trello integrations** — implicit follow-up. The structure
  of `skills/integrations/jira/` is the template; the auth-cmd
  indirection, JSON-first helpers, and `meta/integrations/<tool>/` state
  pattern all generalise. Once two integrations exist, the
  `sync-work-items` skill becomes the natural next research.
