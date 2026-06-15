---
type: plan-validation
id: "2026-06-15-0048-linear-integration-validation"
title: "Validation Report: Linear Integration Implementation Plan"
date: "2026-06-15T12:31:26+00:00"
author: "Toby Clemson"
producer: validate-plan
status: complete
result: "pass"
parent: "plan:2026-06-15-0048-linear-integration"
target: "plan:2026-06-15-0048-linear-integration"
tags: [work-management, integrations, linear, graphql]
last_updated: "2026-06-15T12:31:26+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Linear Integration Implementation Plan

### Implementation Status

✓ Phase 1: Foundation + `init-linear` — Fully implemented
✓ Phase 2: Read skills (`search-linear-issues`, `show-linear-issue`) — Fully implemented
✓ Phase 3: Standard write skills (`create`, `update`, `comment`) — Fully implemented
✓ Phase 4: Divergent write skills (`transition`, `attach`) — Fully implemented

All eight skills, three shared helpers (`linear-graphql.sh`, `linear-auth.sh`,
`linear-common.sh`), the per-flow flow scripts, `EXIT_CODES.md`, the GraphQL
mock server, scenario fixtures, the shared `config_set_frontmatter_field`, and
the `plugin.json` registration are present. Four implementation commits map
one-to-one onto the four phases:

- `uwqzvtkx` Add Linear integration foundation and init-linear skill
- `txpnkvnu` Add Linear read skills: search-linear-issues and show-linear-issue
- `rykoktnt` Add Linear write skills: create, update, comment
- `ovxvnwqr` Add Linear divergent write skills: transition and attach
- `vvyuzkvt` Correct Linear API rate-limit figure to 5,000 requests/hr

### Automated Verification Results

✓ `test-linear-auth.sh` passes
✓ `test-linear-common.sh` passes
✓ `test-linear-graphql.sh` passes
✓ `test-linear-init-flow.sh` passes
✓ `test-linear-paths.sh` passes
✓ `test-linear-search.sh` passes
✓ `test-linear-show.sh` passes
✓ `test-linear-create.sh` passes
✓ `test-linear-update.sh` passes
✓ `test-linear-comment.sh` passes
✓ `test-linear-transition.sh` passes
✓ `test-linear-attach.sh` passes
✓ `scripts/test-config.sh` passes (covers `config_set_frontmatter_field`)
✓ `mise run scripts:check` passes (shfmt + ShellCheck + bashisms)
✓ `plugin.json` valid JSON and lists `./skills/integrations/linear/`
✓ Work item shows `5,000` and no longer contains `2,500`

All Automated Verification checkboxes across all four phases were exercised and
pass. The full `mise run` (frontend/Rust/Python mirror) was **not** run — the
change is shell-only (skills, `scripts/config-common.sh`, a Python mock server
under `skills/`, `plugin.json`, and a work-item edit); none of it touches the
frontend, server, or `tasks/` toolchains, so `scripts:check` plus the standalone
bash suites are the relevant gate. The plan's own per-phase Automated
Verification consistently specifies `mise run scripts:check`, which is green.

### Code Review Findings

#### Matches Plan:

- **GraphQL error dispatch** (`linear-graphql.sh`) — classification is isolated
  in `_linear_classify_gql_error` with the load-bearing order auth → complexity
  → ratelimited → bad-request, exactly as specified. The complexity
  discriminator lives in a single `LINEAR_COMPLEXITY_PATTERN="complexity"`
  constant (full word). 200-body `errors[]` are terminal and never routed into
  the retrying RATELIMITED path. 400 dispatch matches the four-way ordering and
  falls safe to `E_GQL_BAD_REQUEST` (34, non-retried).
- **bash 3.2 floor** — case-insensitive matching uses `tr '[:upper:]'
  '[:lower:]'`, never `${var,,}`; no associative arrays observed.
- **Pagination** (`_linear_paginate`) — `MAX_PAGES=20` cap, non-advancing-cursor
  break, `truncated: true` incompleteness signalling (never silently
  `hasNextPage: false`), and discard-on-partial-page-failure all present.
- **Auth malformed-token guard** (`linear-auth.sh`) — rejects control
  chars/newlines/quotes with `E_TOKEN_MALFORMED` (27) before any request, gated
  on the final resolved token across tiers.
- **Create writeback** (`linear-create-flow.sh`) — already-synced guard
  (`E_CREATE_ALREADY_SYNCED` 102) against `^[A-Z][A-Z0-9]*-[0-9]+$`, returned-
  identifier validation (`E_CREATE_BAD_IDENTIFIER` 106), loud
  `E_CREATE_WRITEBACK_FAILED` (107) on create-succeeded/writeback-failed, and
  the writeback routed through shared `config_set_frontmatter_field`.
- **Attach** (`linear-attach-flow.sh`) — `_linear_upload_asset` is a separate
  direct-curl path sending no `Authorization`, with `--max-redirs 0` /
  `--proto =https`, label-boundary host allow-listing (`*.linear.app` /
  `uploads.linear.app`, rejecting look-alikes), `x-amz-*` header allow-list with
  CR/LF-value drop, and signed-URL redaction in failure messages.
- **Transition** (`linear-transition-flow.sh`) — resolves state name → `stateId`
  from `catalogue.json` directly (no live lookup), with
  `E_TRANSITION_STATE_AMBIGUOUS` for duplicate display names and
  `E_TRANSITION_STATE_NOT_IN_CATALOGUE` / `_NO_CATALOGUE` paths.
- **Registration + work-item patch** — both applied as specified.

#### Deviations from Plan:

- **Added `linear-auth-cli.sh`** (not in the plan) — a thin executable wrapper
  that sources `linear-auth.sh` (a library) and prints `token=<value>` so a
  SKILL can resolve a token via a bare-path `Bash(...)` permission. It guards
  against token leakage (`--debug` substitutes `***`, never propagates `-v`).
  This is a sensible, benign addition that bridges the sourced-library/skill gap;
  it does not contradict the plan.
- **Extra scenario fixtures** beyond the plan's "at minimum" list
  (`team-no-states-200`, `paginate-nonadvancing`, `viewer-slow-200`,
  `team-states-y-200`, plus several `.json.tmpl` templated fixtures). All
  additive; improve coverage.

#### Potential Issues:

- **Complexity-message heuristic (known, documented risk).** The
  `LINEAR_COMPLEXITY_PATTERN="complexity"` substring is correct against the
  authored fixtures but Linear's verbatim rejection wording is undocumented. The
  blast radius is contained by design (single constant, single classifier
  function, fail-safe to terminal bad-request on drift) — this is called out in
  the plan's Migration Notes and should be re-verified against a real complexity
  rejection during manual testing.
- **Create writeback is unlocked across files.** `linear_with_lock` guards only
  the state directory; the create-time `meta/work/` writeback is outside it.
  Accepted in the plan as appropriate for a single-developer interactive tool;
  noted, not a defect.

### Manual Testing Required:

These require a real Linear workspace and credentials and were not exercised:

1. init / scoping:
  - [ ] `/init-linear` lists teams, persists `catalogue.json` (team UUID/key/name
        + non-empty WorkflowStates); `viewer.json` gitignored, `catalogue.json`
        tracked
  - [ ] Selecting team X (member of X and Y) persists only X's states

2. read:
  - [ ] `/search-linear-issues --state "In Progress"` renders a correct table
  - [ ] `/show-linear-issue <ID>` renders the description as Markdown

3. write:
  - [ ] `/create-linear-issue meta/work/<file>.md` creates the issue, rewrites
        `work_item_id`, and re-run reports already synced
  - [ ] `/update-linear-issue <ID> --title … --state …` reflected via show
  - [ ] `/comment-linear-issue <ID> --body "…"` appears in Linear

4. divergent write:
  - [ ] `/transition-linear-issue <ID> "<state>"` reflected via show
  - [ ] `/attach-linear-issue <ID> --url https://…` adds a link attachment
  - [ ] `/attach-linear-issue <ID> --file ./screenshot.png` uploads and registers

5. Re-verify the real Linear complexity-rejection wording against
   `LINEAR_COMPLEXITY_PATTERN` (Migration Notes risk).

### Recommendations:

- Proceed to PR. The implementation is complete and fully matches the plan's
  design at all four divergence points.
- Before relying on `/create-linear-issue` in anger, confirm 0047 has shipped
  the `work_item_id` frontmatter field — the create skill fails closed against
  pre-0047 files by design (hard sequencing dependency noted in the plan).
- During the manual Linear pass, capture a real complexity-rejection body and
  confirm/adjust `LINEAR_COMPLEXITY_PATTERN` if the wording differs.
- Consider running the full `mise run` once before merge if CI does not already
  mirror the shell-only path, purely as belt-and-braces.
