---
id: "0048"
title: "Linear Integration"
date: "2026-05-06T17:49:44+00:00"
author: Toby Clemson
kind: story
status: ready
priority: medium
parent: "work-item:0045"
tags: [work-management, integrations, linear]
type: work-item
schema_version: 1
last_updated: "2026-06-15T00:00:00+00:00"
last_updated_by: Toby Clemson
blocked_by: ["work-item:0046"]
blocks: ["work-item:0051"]
---

# 0048: Linear Integration

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Implement eight skills under `skills/integrations/linear/` covering the issue
lifecycle against Linear's GraphQL API — `init-linear`, `search-linear-issues`,
`show-linear-issue`, `create-linear-issue`, `update-linear-issue`,
`transition-linear-issue`, `comment-linear-issue`, and `attach-linear-issue`.
Linear issue identifiers (e.g. `BLA-123`) become the `work_item_id` of the local
work item file the issue was created from.

## Context

Developers who track work in Linear need the same local-first work item
lifecycle the Jira integration already provides — managing items in `meta/work/`
while keeping them in sync with their team's Linear workspace. This story
delivers that integration.

Linear uses a single GraphQL endpoint with cursor-based pagination and
team-scoped `WorkflowState` UUIDs for state transitions. Rate-limit errors
arrive as HTTP `400` with `"code": "RATELIMITED"` in GraphQL error extensions
rather than the standard `429`. Linear accepts Markdown natively for issue
descriptions and comments — no conversion layer is required, unlike the Jira
ADF (Atlassian Document Format) integration. The pattern follows the established
Jira integration under `skills/integrations/jira/`.

## Requirements

- Eight skills:
  - `init-linear` — authenticate, persist team details (UUID, key, name) and
    WorkflowState catalogue to the linear subdirectory under the configured
    integrations path; scope to a single team
  - `search-linear-issues` — query issues by filter
  - `show-linear-issue` — display a single issue
  - `create-linear-issue` — create an issue from a local work item file; write
    the remote-allocated identifier (e.g. `BLA-123`) back as `work_item_id` in
    that same file
  - `update-linear-issue` — update fields on an existing issue (updatable
    fields: title, description, state, assignee, priority)
  - `transition-linear-issue` — transition state using WorkflowState UUIDs
    resolved from the cached catalogue
  - `comment-linear-issue` — add a Markdown comment to an issue
  - `attach-linear-issue` — attach a resource to an issue: link-based
    attachments (external URLs via `attachmentCreate`) and binary file uploads
    (two-step pre-signed URL flow)

## Acceptance Criteria

- [ ] Given `init-linear` is run with valid credentials, when authentication
  succeeds, then a catalogue file is persisted to the linear subdirectory under
  the configured integrations path containing the team's UUID, key, and name and
  a non-empty list of WorkflowStates, each mapping a state name to its UUID
- [ ] Given a user who is a member of teams `X` and `Y`, when `init-linear` is
  run and team `X` is selected, then the persisted catalogue contains exactly
  team `X`'s UUID, key, and name and only `X`'s WorkflowStates
- [ ] Given `init-linear` has been run and a local work item file with a numeric
  `work_item_id`, when `create-linear-issue` is invoked against that file, then a
  Linear issue is created whose title equals the work item's title and whose
  description equals the rendered Markdown body below the frontmatter, and its
  remote-allocated identifier (e.g. `BLA-123`) overwrites the `work_item_id`
  frontmatter field in that same file
- [ ] Given a local work item file whose `work_item_id` is already a
  remote-format identifier (already synced), when `create-linear-issue` is
  invoked against it, then no duplicate issue is created and the skill reports
  the file is already synced
- [ ] Given a token sent with a `Bearer` prefix, when `init-linear` runs, then it
  exits non-zero with an authentication-failure message (the Linear personal API
  key must be sent without a `Bearer` prefix)
- [ ] Given `init-linear` has been run and a fixture of 5 issues of which 2 are
  in state `S`, when `search-linear-issues` is invoked filtering on state `S`,
  then exactly those 2 issues (by identifier) are returned and the other 3 are
  absent
- [ ] Given a fixture issue with identifier `I`, title `T`, state `S`, assignee
  `A`, and description `D`, when `show-linear-issue I` is invoked, then the
  output reports exactly those values for each field
- [ ] Given an existing issue, when `update-linear-issue` is invoked setting the
  title to `T` and the state to `S`, then re-fetching the issue returns title `T`
  and state `S` (title and state are the representative cases — the latter
  exercises name→UUID resolution; the full updatable field set is defined in
  Requirements, and the remaining fields — description, assignee, priority — are
  verified by implementation-level tests rather than acceptance criteria)
- [ ] Given an existing issue, when `comment-linear-issue` is invoked with a
  Markdown body, then re-fetching the issue's comments includes a comment whose
  body equals the submitted Markdown
- [ ] Given `init-linear` has been run, when `transition-linear-issue` is
  invoked with a target state name, then re-fetching the issue reports its state
  as the target WorkflowState; and with the catalogue API endpoint stubbed to
  fail, the transition still succeeds — proving the WorkflowState UUID was
  resolved from the cached catalogue rather than a live lookup
- [ ] Given a valid external URL, when `attach-linear-issue` is invoked, then
  a link-based attachment is created on the issue via `attachmentCreate`
- [ ] Given a local file path, when `attach-linear-issue` is invoked, then the
  file is uploaded via the pre-signed URL flow and the resulting asset URL is
  registered as an attachment on the issue — the issue's `attachments` connection
  gains exactly one new attachment whose title/URL corresponds to the uploaded
  file
- [ ] Given the API responds with HTTP `400` carrying `"code": "RATELIMITED"` in
  its GraphQL error extensions, when any Linear skill handles the response, then
  the skill exits with the dedicated RATELIMITED transport exit code (per
  `EXIT_CODES.md`), prints a message naming the rate limit rather than a generic
  failure, and computes a backoff delay in seconds from the
  `X-RateLimit-Requests-Reset` HTTP response header (epoch ms) — e.g. a reset
  header 30000 ms after the current time yields a backoff within ±2 seconds of
  `(reset_epoch_ms − now_epoch_ms) / 1000` (≈30 seconds, not 30000)
- [ ] Given a query whose results span multiple pages (test fixture: 3 pages of
  50 issues), when a Linear skill executes the query via `linear-graphql.sh`,
  then all 150 issues are returned in a single result set and pagination
  continues until `pageInfo.hasNextPage` is `false`
- [ ] Given a query whose response carries Linear's complexity-limit GraphQL
  error (a single query exceeding the 10,000-point cap), when the skill handles
  it, then it exits with the dedicated complexity-cap exit code (per
  `EXIT_CODES.md`), prints a message naming the 10,000-point limit, and emits no
  partial result set

## Open Questions

- —

## Dependencies

- Blocked by: 0046 — Work Management System Configuration; provides the
  configurable integrations path (resolved via `config-read-path.sh
  integrations`) that the Linear team/WorkflowState catalogue and state
  persistence write under.
- Blocks: 0051 — Sync Work Items Skill. This is a **contributory** relationship,
  not a hard gate: 0051's prerequisite (at least one integration complete) is
  already satisfied by the completed Jira integration, so 0048 adds Linear
  support to 0051 rather than unblocking it.
- External system: Linear GraphQL API (`https://api.linear.app/graphql`) — hard
  rate limits (5,000 requests/hr, 3,000,000 complexity points/hr, 10,000 points
  per single query) and the non-standard `400 RATELIMITED` response are
  availability and throughput couplings every skill depends on.
- Credential prerequisite: a user-provisioned Linear personal API key
  (`lin_api_...`), resolved via the `ACCELERATOR_LINEAR_TOKEN`/
  `ACCELERATOR_LINEAR_TOKEN_CMD` indirection, must exist before `init-linear` and
  every downstream skill can authenticate (see Assumptions).
- Shared convention: the `work_item_id` writeback consumes the
  numeric-`work_item_id`-means-unsynced contract owned by 0047 (Core Skills Sync
  Integration). `create-linear-issue` consumes this contract — it does not
  redefine it; the push-then-write semantics live in 0047. This is a
  **definitional reference, not an ordering blocker**: `create-linear-issue` can
  allocate and write the remote ID independently of 0047's completion, so 0047 is
  intentionally absent from `blocked_by`.
- Shared artefact: this integration claims a new transport-code range in the
  shared `scripts/EXIT_CODES.md` (already populated by the Jira integration);
  the range must be coordinated to avoid colliding with existing codes.
- Internal ordering: `init-linear` must complete (catalogue populated) before the
  other seven skills are functional or independently verifiable.

## Assumptions

- Auth follows the `ACCELERATOR_LINEAR_TOKEN_CMD` indirection pattern
  established by the Jira integration. The resolved token is a Linear personal
  API key (`lin_api_...`) passed in the `Authorization` header without a
  `Bearer` prefix — using `Bearer` causes authentication failure.
- A single team is configured at `init-linear` time and stored in the
  catalogue; no per-invocation `--team` flag is required.
- Linear accepts Markdown natively for issue descriptions and comments;
  no conversion layer is needed.
- WorkflowState UUIDs are team-scoped and cached by `init-linear`; state
  transitions resolve names to UUIDs from the cache, not via live API calls.
- Binary file uploads via `attach-linear-issue` use a two-step server-side
  pre-signed URL flow (`fileUpload` mutation → HTTP PUT); this cannot be done
  client-side.

## Technical Notes

**Size**: L — eight skills under `skills/integrations/linear/` plus a dedicated
`linear-graphql.sh` transport helper, `linear-auth.sh`, `linear-common.sh`,
per-skill `*-flow.sh` orchestrators, and mirrored test suites + fixtures,
paralleling the Jira integration's ~42-file footprint; lighter than Jira only
because Linear's native Markdown removes the ADF↔Markdown conversion subsystem.
The L sizing is intentionally indivisible: every skill depends on the shared
`linear-graphql.sh` transport, `linear-auth.sh`, and the `init-linear`-populated
team/WorkflowState catalogue, so splitting the verb skills from that foundation
would leave non-deliverable fragments rather than independently shippable
stories.

- A dedicated `linear-graphql.sh` request helper is required; `jira-request.sh`
  cannot be reused due to the differing rate-limit response format and GraphQL
  construction. It should handle: query/mutation construction, Relay-style
  cursor pagination (`pageInfo.hasNextPage` + `endCursor`), and `400 RATELIMITED`
  detection with backoff using `X-RateLimit-Requests-Reset` (epoch ms).
- The `schpet/linear-cli` (TypeScript, v2.0.0 April 2026, actively maintained)
  is a useful reference for auth patterns, the `LINEAR_API_KEY` env var
  convention, and agent-friendly UX (`--json` output, VCS-aware context
  detection).
- Linear personal API keys are long-lived and user-scoped, providing access to
  all teams the user is a member of. `init-linear` should call
  `teams { nodes { id name key } }` to discover and select the target team.
- Rate limits: 5,000 requests/hr and 3,000,000 complexity points/hr per user
  for API keys. Single query maximum: 10,000 complexity points.

### Patterns to mirror from the Jira integration

The Linear integration parallels `skills/integrations/jira/` (8 skill dirs + a
shared `scripts/` library). Concrete anchors:

- **Transport helper**: `linear-graphql.sh` plays the role of
  `jira-request.sh` (`skills/integrations/jira/scripts/jira-request.sh`). Mirror
  its shape: credentials piped via `curl --config -` so the token never appears
  in argv (`jira-request.sh:331-332`); a single retry loop (max 4 attempts) with
  `Retry-After`/backoff clamping (`jira-request.sh:322-432`). Note: pagination is
  the *caller's* job in Jira (`jira-search-flow.sh:278-285`), not the request
  helper — Linear's cursor pagination can either follow that split or be folded
  into `linear-graphql.sh` (the work item assumes the latter).
- **Auth**: add `linear-auth.sh` mirroring `jira-auth.sh:165-233` precedence
  (`ACCELERATOR_LINEAR_TOKEN` env → `ACCELERATOR_LINEAR_TOKEN_CMD` env →
  `config.local.md` → shared `config.md` token only). Reuse the shared-config
  `token_cmd` ban and the 0600 fail-closed check (`jira-auth.sh:184-208,221-233`).
- **State/catalogue**: persist the team + WorkflowState catalogue via the
  `jira_state_dir`/`jira_atomic_write_json`/`jira_with_lock` shape
  (`jira-common.sh:69-221`). Real default path is
  `.accelerator/state/integrations/linear/` (resolved via
  `config-read-path.sh integrations`), **not** `meta/integrations/` (legacy,
  banned by a guard test).
- **Exit codes**: claim a transport-code range in an `EXIT_CODES.md` equivalent
  (`scripts/EXIT_CODES.md`) and propagate it unchanged through flow scripts.
- **Registration**: add `"./skills/integrations/linear/"` to
  `.claude-plugin/plugin.json` (registration is per-integration-dir, not per-skill).

### Two divergences from the Jira pattern (net-new, not copied)

- **`work_item_id` writeback is net-new.** No Jira skill writes the remote key
  back into a local work item file — `create-jira-issue` only prints the
  allocated key (`jira-create-flow.sh:393`, `create-jira-issue/SKILL.md:150-157`).
  `create-linear-issue`'s requirement to write `work_item_id` into the local file
  is a new capability to design, not a Jira pattern to mirror.
- **Dual attach diverges.** `attach-jira-issue` is binary-multipart only, with no
  link branch (`jira-attach-flow.sh`). `attach-linear-issue`'s link-based
  (`attachmentCreate`) + binary (pre-signed URL) split has no Jira analogue.

## Drafting Notes

- `linear-graphql.sh` helper kept in Technical Notes only — implementation
  detail, not user-facing concern.
- Storage references use "the linear subdirectory under the configured
  integrations path" because the integrations path is being made configurable in
  work item 0046 (Work Management System Configuration), on which this story is
  blocked. This configured path (default `.accelerator/state/integrations/linear/`)
  supersedes the literal `meta/integrations/<system>/` paths used illustratively
  in the parent epic 0045, which predates 0046; `meta/integrations/` is legacy
  and guard-banned.
- `attach-linear-issue` keeps both attachment branches (link via
  `attachmentCreate`; binary via the pre-signed `fileUpload` → HTTP PUT flow) in
  one skill for a single attach entrypoint. The branches share no API surface
  and may be delivered and verified incrementally (link first, binary second).
- Eight skills (not seven as in the epic) — `attach-linear-issue` added since
  both link-based and binary file upload attachments are confirmed supported by
  the API.
- Linear uses Markdown natively — confirmed via research; no ADF-equivalent
  conversion needed.
- Single-team scoping at `init-linear` time confirmed by user; team UUID and
  key stored in catalogue.

## References

- Source: `meta/work/0045-work-management-integration.md`
- Research: Linear Developers documentation (auth, rate limiting, pagination,
  attachments, WorkflowStates)
- Research: [schpet/linear-cli](https://github.com/schpet/linear-cli)
