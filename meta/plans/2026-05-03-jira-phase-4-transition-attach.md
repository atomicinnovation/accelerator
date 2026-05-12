---
date: "2026-05-03T00:00:00+01:00"
type: plan
skill: create-plan
work-item: ""
status: reviewed
---

# Jira Phase 4 — Workflow and Attachments Implementation Plan

## Overview

Add the two remaining Jira integration skills from Phase 4 of the research:
`transition-jira-issue` (walk an issue through its workflow by state name) and
`attach-jira-issue` (upload one or more files as attachments). Both follow
the write-skill pattern established in Phases 1–3: a bash flow script backed
by `jira-request.sh`, a `--describe` dry-run mode, a confirmation gate in the
SKILL.md, and a TDD test suite written before the implementation.

## Current State Analysis

Phases 1–3 are fully shipped. The following infrastructure is available and
must not be modified:

- **`jira-request.sh`** — handles `--multipart "file=@PATH"` with automatic
  `X-Atlassian-Token: no-check` injection (used by `POST .../attachments`).
  Multipart is already integration-tested in `test-jira-request.sh` Case 3.
- **`jira-common.sh`** — `jira_die`, `jira_warn`, `jira_state_dir`,
  `_jira_emit_generic_hint`, `_jira_uuid_v4`, `jira_with_lock`.
- **`jira-auth.sh`** — `jira_resolve_credentials`, sets `JIRA_SITE`,
  `JIRA_EMAIL`, `JIRA_TOKEN`.
- **`jira-body-input.sh`** — `jira_resolve_body`; will be reused for
  transition's optional `--comment` body.
- **`jira-custom-fields.sh`** — not needed for Phase 4.
- **Exit code namespace** — `EXIT_CODES.md` line 94 explicitly reserves
  `120+` for Phase 4 (transition / attach).
- **Test runner** — `test-jira-scripts.sh` sources all test files in order;
  two new entries need adding.
- **Mock server** — `test-helpers/mock-jira-server.py` with scenario files
  under `test-fixtures/scenarios/`; `consume: false` pattern guards dry-run
  paths.

### Key Discoveries

- `skills/integrations/jira/scripts/EXIT_CODES.md:94` — `120+` reserved for
  Phase 4.
- `test-jira-request.sh:134–142` — multipart test pattern (file= part + guard
  on `X-Atlassian-Token: no-check`).
- `test-fixtures/scenarios/comment-add-print-payload-guard.json` — the
  guard-scenario pattern: `"consume": false` paired with `"capture_url": true`
  prevents the test from silently passing if the dry-run path accidentally
  makes a real API call. Both fields must appear together in every guard fixture.
- `test-fixtures/scenarios/comment-delete-describe-guard.json` — same pattern
  for `--describe` ops.
- `jira-comment-flow.sh:37–58` — reference for the `--describe` dry-run
  pattern (outputs a JSON description object to stdout, exits 0).
- `jira-update-flow.sh:1–48` — reference for the standard flow-script header
  (comment block, sourcing chain, usage function, exit code table).
- `comment-jira-issue/SKILL.md:84–90` — `--describe` skill-side rendering
  reference (renders the `--describe` JSON output as prose and surfaces
  `irreversible: true`).

## Desired End State

After this plan is complete:

```
skills/integrations/jira/
  transition-jira-issue/
    SKILL.md                              # new
  attach-jira-issue/
    SKILL.md                              # new
  scripts/
    jira-transition-flow.sh               # new
    jira-attach-flow.sh                   # new
    test-jira-transition.sh               # new
    test-jira-attach.sh                   # new
    test-jira-scripts.sh                  # updated (2 new entries)
    EXIT_CODES.md                         # updated (120-139 range filled)
    test-fixtures/scenarios/
      transition-list-200.json            # new
      transition-list-ambiguous-200.json  # new
      transition-post-204.json            # new
      transition-post-204-capture.json    # new
      transition-post-204-direct.json     # new (--transition-id bypass, no GET)
      transition-post-400.json            # new
      transition-describe-guard.json      # new
      transition-list-401.json            # new
      transition-list-404.json            # new
      transition-post-204-no-notify.json  # new
      attach-post-200.json                # new
      attach-post-200-two-files.json      # new
      attach-post-403.json                # new
      attach-describe-guard.json          # new
      attach-post-401.json                # new
```

**Verification commands:**
```bash
bash skills/integrations/jira/scripts/test-jira-transition.sh
bash skills/integrations/jira/scripts/test-jira-attach.sh
bash skills/integrations/jira/scripts/test-jira-scripts.sh
```

All three must exit 0 with no FAIL lines.

## What We're NOT Doing

- No `delete-attachment` skill — Jira's REST API supports it but the research
  explicitly omitted a `delete-*` skill. VCS revert is the safety net, not in-app undo.
- No bulk-transition (transitioning multiple issues in one call).
- No `--transition-name` alias — disambiguation is handled via a separate
  `--transition-id` flag when multiple transitions share a state name.
- No attachment-download skill — read operations for attachments are out of
  scope for Phase 4.
- No changes to existing flow scripts, SKILL.md files, or test fixtures.

## Implementation Approach

Phase 4 is split into two independent tracks (transition and attach), each
following the same TDD sequence:

1. Write test fixture scenarios first (the shape of the API responses).
2. Write the test script against a not-yet-existing flow script — initially all
   cases will FAIL.
3. Write the flow script until all cases PASS.
4. Use `skill-creator:skill-creator` to create and validate the SKILL.md.
5. Update shared files (`EXIT_CODES.md`, `test-jira-scripts.sh`).

Transition is implemented first because its logic is richer (state-name lookup,
disambiguation, optional comment body). Attach is simpler and can proceed after
or in parallel once the exit-code range is agreed.

---

## Phase 1: Transition flow — TDD fixtures and test script

### Overview

Write the test fixtures and `test-jira-transition.sh` before the flow script
exists. The test cases document the intended behaviour in executable form.

### Changes Required

#### 1. Test fixture scenarios (all new files)

**File**: `skills/integrations/jira/scripts/test-fixtures/scenarios/transition-list-200.json`

A `GET /rest/api/3/issue/ENG-1/transitions` response with three distinct
transitions, one of which matches `"In Progress"`:

```json
{
  "expectations": [
    {
      "method": "GET",
      "path": "/rest/api/3/issue/ENG-1/transitions",
      "response": {
        "status": 200,
        "body": "{\"transitions\":[{\"id\":\"11\",\"name\":\"To Do\",\"to\":{\"name\":\"To Do\"}},{\"id\":\"21\",\"name\":\"In Progress\",\"to\":{\"name\":\"In Progress\"}},{\"id\":\"31\",\"name\":\"Done\",\"to\":{\"name\":\"Done\"}}]}"
      }
    }
  ]
}
```

**File**: `skills/integrations/jira/scripts/test-fixtures/scenarios/transition-list-ambiguous-200.json`

Two transitions both leading to a state named `"In Review"` (different IDs,
different transition names):

```json
{
  "expectations": [
    {
      "method": "GET",
      "path": "/rest/api/3/issue/ENG-1/transitions",
      "response": {
        "status": 200,
        "body": "{\"transitions\":[{\"id\":\"41\",\"name\":\"Start Review\",\"to\":{\"name\":\"In Review\"}},{\"id\":\"42\",\"name\":\"Re-open for Review\",\"to\":{\"name\":\"In Review\"}}]}"
      }
    }
  ]
}
```

**File**: `skills/integrations/jira/scripts/test-fixtures/scenarios/transition-post-204.json`

Two-step scenario: GET transitions (success), then POST transition (204):

```json
{
  "expectations": [
    {
      "method": "GET",
      "path": "/rest/api/3/issue/ENG-1/transitions",
      "response": {
        "status": 200,
        "body": "{\"transitions\":[{\"id\":\"21\",\"name\":\"In Progress\",\"to\":{\"name\":\"In Progress\"}}]}"
      }
    },
    {
      "method": "POST",
      "path": "/rest/api/3/issue/ENG-1/transitions",
      "response": {
        "status": 204,
        "body": ""
      }
    }
  ]
}
```

**File**: `skills/integrations/jira/scripts/test-fixtures/scenarios/transition-post-204-capture.json`

Same as above but with `capture_body: true` on the POST so the test can assert
the request body contains the correct transition ID and any optional fields:

```json
{
  "expectations": [
    {
      "method": "GET",
      "path": "/rest/api/3/issue/ENG-1/transitions",
      "response": {
        "status": 200,
        "body": "{\"transitions\":[{\"id\":\"21\",\"name\":\"In Progress\",\"to\":{\"name\":\"In Progress\"}}]}"
      }
    },
    {
      "method": "POST",
      "path": "/rest/api/3/issue/ENG-1/transitions",
      "capture_body": true,
      "response": {
        "status": 204,
        "body": ""
      }
    }
  ]
}
```

**File**: `skills/integrations/jira/scripts/test-fixtures/scenarios/transition-post-204-direct.json`

Single-step scenario for the `--transition-id` bypass path: POST only, no
preceding GET. Used by Case 6 to verify the GET is skipped entirely:

```json
{
  "expectations": [
    {
      "method": "POST",
      "path": "/rest/api/3/issue/ENG-1/transitions",
      "capture_body": true,
      "response": {
        "status": 204,
        "body": ""
      }
    }
  ]
}
```

**File**: `skills/integrations/jira/scripts/test-fixtures/scenarios/transition-post-400.json`

GET succeeds, POST returns 400 (e.g. missing required resolution field):

```json
{
  "expectations": [
    {
      "method": "GET",
      "path": "/rest/api/3/issue/ENG-1/transitions",
      "response": {
        "status": 200,
        "body": "{\"transitions\":[{\"id\":\"31\",\"name\":\"Done\",\"to\":{\"name\":\"Done\"}}]}"
      }
    },
    {
      "method": "POST",
      "path": "/rest/api/3/issue/ENG-1/transitions",
      "response": {
        "status": 400,
        "body": "{\"errorMessages\":[\"A resolution must be set.\"],\"errors\":{}}"
      }
    }
  ]
}
```

**File**: `skills/integrations/jira/scripts/test-fixtures/scenarios/transition-describe-guard.json`

Guard: the eager GET (which `--describe` with `STATE_NAME` makes to resolve
the transition ID) is served by a consuming expectation first, then the POST
expectation uses `"consume": false` paired with `"capture_url": true` so that
if the dry-run path accidentally fires a real write the test detects it:

```json
{
  "expectations": [
    {
      "method": "GET",
      "path": "/rest/api/3/issue/ENG-1/transitions",
      "response": {
        "status": 200,
        "body": "{\"transitions\":[{\"id\":\"21\",\"name\":\"In Progress\",\"to\":{\"name\":\"In Progress\"}}]}"
      }
    },
    {
      "method": "POST",
      "path": "/rest/api/3/issue/ENG-1/transitions",
      "consume": false,
      "capture_url": true,
      "response": {
        "status": 204,
        "body": ""
      }
    }
  ]
}
```

**File**: `skills/integrations/jira/scripts/test-fixtures/scenarios/transition-list-401.json`

GET transitions → 401 Unauthorized:

```json
{
  "expectations": [
    {
      "method": "GET",
      "path": "/rest/api/3/issue/ENG-1/transitions",
      "response": {
        "status": 401,
        "body": "{\"errorMessages\":[\"You do not have permission to access this resource.\"],\"errors\":{}}"
      }
    }
  ]
}
```

**File**: `skills/integrations/jira/scripts/test-fixtures/scenarios/transition-list-404.json`

GET transitions → 404 (issue not found):

```json
{
  "expectations": [
    {
      "method": "GET",
      "path": "/rest/api/3/issue/ENG-1/transitions",
      "response": {
        "status": 404,
        "body": "{\"errorMessages\":[\"Issue does not exist or you do not have permission to see it.\"],\"errors\":{}}"
      }
    }
  ]
}
```

**File**: `skills/integrations/jira/scripts/test-fixtures/scenarios/transition-post-204-no-notify.json`

Two-step scenario with `capture_url: true` on the POST so the test can assert the
captured URL contains `?notifyUsers=false`:

```json
{
  "expectations": [
    {
      "method": "GET",
      "path": "/rest/api/3/issue/ENG-1/transitions",
      "response": {
        "status": 200,
        "body": "{\"transitions\":[{\"id\":\"21\",\"name\":\"In Progress\",\"to\":{\"name\":\"In Progress\"}}]}"
      }
    },
    {
      "method": "POST",
      "path": "/rest/api/3/issue/ENG-1/transitions",
      "capture_url": true,
      "response": {
        "status": 204,
        "body": ""
      }
    }
  ]
}
```

#### 2. Test script

**File**: `skills/integrations/jira/scripts/test-jira-transition.sh`

Structure mirrors `test-jira-comment.sh`. Full list of cases:

| Case | Description | Key assertion | Fixture |
|------|-------------|---------------|---------|
| 1 | Happy path — state name match (exact case) → POST 204 → exit 0 | exit 0 | `transition-post-204.json` |
| 2 | Case-insensitive match (`"in progress"` → finds `"In Progress"`) | exit 0; body capture verifies `"id":"21"` in POST | `transition-post-204-capture.json` |
| 3 | `--describe` with STATE_NAME — GET resolves transition, no POST; stdout has `"key"`, `"state"` (non-null), `"transition_id"` (`"21"`) | exit 0; `jq -e '.state != null and .transition_id == "21"'` passes | `transition-list-200.json` |
| 3b | `--describe ENG-1 "Nonexistent"` — 0-match from describe branch → exit 122; no POST | exit 122 | `transition-list-200.json` |
| 4 | `--describe` guard — mock confirms no POST was made; call `stop_mock` before asserting captured URLs file | exit 0; `stop_mock` then `jq -c '.' "$CAPTURED_URLS"` equals `[]` | `transition-describe-guard.json` |
| 5 | Ambiguous state name → exit 123, stdout is a JSON array of matching transitions | exit 123; `jq 'type'` returns `"array"`; both IDs present | `transition-list-ambiguous-200.json` |
| 6 | `--transition-id` bypasses name lookup, POSTs directly (no GET); body contains correct transition ID | only POST in mock; body capture verifies ID | `transition-post-204-direct.json` |
| 7 | State name not found → exit 122 | exit 122 | `transition-list-200.json` |
| 8 | `--resolution NAME` included in POST body `fields.resolution.name` | body capture | `transition-post-204-capture.json` |
| 9 | `--comment TEXT` assembled as ADF in POST body `update.comment[].add.body` | body capture; `"type":"doc"` present | `transition-post-204-capture.json` |
| 10 | Missing issue key → exit 120 (no network call) | exit 120 | none |
| 11 | Missing both state name and `--transition-id` → exit 121 (no network call) | exit 121 | none |
| 12 | GET transitions 401 → exits 11, error on stderr | exit 11 | `transition-list-401.json` |
| 13 | GET transitions 404 → exits 13 | exit 13 | `transition-list-404.json` |
| 14 | POST transition 400 → exits 34, error body on stderr | exit 34 | `transition-post-400.json` |
| 15 | Unrecognised flag → exit 124 | exit 124 | none |
| 16 | `--comment-file PATH` where PATH does not exist → exit 125, no network call | exit 125 | none |
| 17 | `--resolution ''` (empty string) → exit 126, no network call | exit 126 | none |
| 18 | `--describe ENG-1 --transition-id 21` offline path — invoke via `transition_no_creds()` wrapper that unsets `ACCELERATOR_JIRA_TOKEN`, URL override, and site config before running the script | exit 0; `state` is `null`; `transition_id` is `"21"`; no network call attempted | none |
| 19 | `STATE_NAME` and `--transition-id` both supplied → exit 124; test both orderings: `ENG-1 "Done" --transition-id 21` and `ENG-1 --transition-id 21 "Done"` | exit 124 for both orderings | none |
| 20 | `--comment-file -` (dash-prefix rejection) → exit 125, no network call | exit 125 | none |
| 21 | `--transition-id abc` (non-numeric value) → exit 124, no network call | exit 124 | none |
| 22 | `--no-notify` appends `?notifyUsers=false` to POST URL | exit 0; captured POST URL contains `notifyUsers=false` | `transition-post-204-no-notify.json` |

Test file structure (mirrors `test-jira-request.sh`):
```bash
#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/jira-transition-flow.sh"
SCENARIOS="$SCRIPT_DIR/test-fixtures/scenarios"
MOCK_SERVER="$SCRIPT_DIR/test-helpers/mock-jira-server.py"

# ... fake-repo setup, start_mock/stop_mock helpers, req() wrapper ...

# Cases 1-22 (including 3b) as described above
```

### Success Criteria

#### Automated Verification

- [x] `bash skills/integrations/jira/scripts/test-jira-transition.sh` exits
  non-zero with `FAIL` lines (because `jira-transition-flow.sh` does not exist
  yet) — this proves the tests are actually testing something.

---

## Phase 2: Transition flow script implementation

### Overview

Write `jira-transition-flow.sh` until all 23 test cases pass (Cases 1–22
including 3b). Then update shared files.

### Changes Required

#### 1. Flow script

**File**: `skills/integrations/jira/scripts/jira-transition-flow.sh`

```
Usage: jira-transition-flow.sh [--describe] KEY (STATE_NAME | --transition-id ID)
         [--resolution NAME]
         [--comment TEXT | --comment-file PATH]
         [--no-notify]
         [--quiet]
         [--help | -h]
```

`STATE_NAME` and `--transition-id` are mutually exclusive but one is required.
`STATE_NAME` is the second positional when supplied; it is absent when
`--transition-id` is used as the sole means of identifying the transition.

**Exit codes (120–129):**

| Code | Name | Description |
|------|------|-------------|
| 120 | `E_TRANSITION_NO_KEY` | No issue key positional argument |
| 121 | `E_TRANSITION_NO_STATE` | No target state name (and no `--transition-id`) |
| 122 | `E_TRANSITION_NOT_FOUND` | No transition in the issue's current state matches the given state name |
| 123 | `E_TRANSITION_AMBIGUOUS` | Multiple transitions lead to the state name; disambiguation required |
| 124 | `E_TRANSITION_BAD_FLAG` | Unrecognised flag, conflicting argument combination, or non-numeric `--transition-id` value |
| 125 | `E_TRANSITION_NO_BODY` | `--comment-file` path invalid (dash-prefix, symlink to device, missing, or unreadable) |
| 126 | `E_TRANSITION_BAD_RESOLUTION` | Resolution value failed basic validation (empty or whitespace-only) |
| 127–129 | — | Reserved |

**Algorithm:**

Call `jira_require_dependencies` (exits `E_MISSING_DEP` if `jq` or `curl` is
absent) before argument parsing.

1. Parse arguments.
   - `KEY` is the first positional (exit 120 if missing).
   - `STATE_NAME` is the second positional — absent when `--transition-id` is
     used instead. Exit 121 if neither `STATE_NAME` nor `--transition-id` is
     present after full argument parsing.
   - Exit 124 on unrecognised flags, or if — after full argument parsing —
     both `STATE_NAME` and `--transition-id` are set (check
     `[[ -n "$STATE_NAME" && -n "$TRANSITION_ID" ]]` after the parsing loop;
     this post-parse check catches both argument orderings).
   - Exit 124 if `--transition-id ID` is given but `ID` does not match
     `^[0-9]+$` (non-numeric transition ID).
   - Exit 125 if `--comment-file PATH` is given and any of the following
     apply: PATH begins with `-` (e.g. `-` reads from stdin); PATH is a
     symlink resolving to a path under `/dev`, `/proc`, or `/sys`; PATH does
     not exist or is not readable (`[ -f path ] && [ -r path ]`).
   - Exit 126 if `--resolution` is given with an empty or whitespace-only
     value.
**Private lookup helper `_jira_transition_lookup`:**
Both step 2's `--describe STATE_NAME` branch and step 4's live path use a
private helper function that eliminates logic duplication:
- `GET /rest/api/3/issue/{KEY}/transitions` via `jira-request.sh`.
- Filter: `jq --arg s "$STATE_NAME" '[.transitions[] | select(.to.name | ascii_downcase == ($s | ascii_downcase))]'`
- On empty array: emit `E_TRANSITION_NOT_FOUND`, exit 122.
- On 2+ entries: emit the JSON array to stdout, exit 123.
- On 1 entry: emit the single-element array to stdout, exit 0.
  Caller extracts the resolved ID: `transition_id=$(printf '%s' "$matches" | jq -r '.[0].id')`

2. If `--describe`:
   - If `--transition-id` given: output describe JSON with the known ID and
     `state: null` (no network call needed — ID is already resolved):
     ```json
     {"key":"<KEY>","state":null,"transition_id":"<ID>","resolution":<VALUE-OR-NULL>,"comment":<bool>}
     ```
     Exit 0.
   - If `STATE_NAME` given: call `_jira_transition_lookup KEY STATE_NAME`. On
     exit 122: propagate. On exit 123: emit the array from the helper's stdout
     and exit 123. On exit 0: extract the resolved ID via
     `transition_id=$(printf '%s' "$matches" | jq -r '.[0].id')` and output
     describe JSON with resolved values:
     ```json
     {"key":"<KEY>","state":"<STATE_NAME>","transition_id":"<resolved-ID>","resolution":<VALUE-OR-NULL>,"comment":<bool>}
     ```
     Exit 0.
   - In all cases the describe output reflects the actual parsed flag values:
     `resolution` is the supplied `--resolution` value (or `null`); `comment`
     is `true` if `--comment` or `--comment-file` is present.
3. If `--transition-id` given: skip to step 5 with that ID.
4. Call `_jira_transition_lookup KEY STATE_NAME`. On exit 122/123: propagate.
   On exit 0: extract the resolved ID:
   `transition_id=$(printf '%s' "$matches" | jq -r '.[0].id')`
5. Build POST body using `jq -n --argjson` composition, following the
   incremental-merge pattern from `jira-update-flow.sh` lines 361–366:
   - Always include `"transition": {"id": "<ID>"}`.
   - Add `"fields": {"resolution": {"name": "<NAME>"}}` only if `--resolution`
     was given. Pass the value via `jq --arg r "$RESOLUTION"` — never
     string-interpolated.
   - Add `"update": {"comment": [{"add": {"body": <ADF>}}]}` only if
     `--comment`/`--comment-file` was given. First obtain the body text via
     `jira_resolve_body`: for `--comment TEXT`, pass `--body "$COMMENT_TEXT"`;
     for `--comment-file PATH`, pass `--body-file "$PATH"` (path validated in
     step 1). Then convert to ADF via `jira-md-to-adf.sh` (invoked as a
     subprocess — `bash .../jira-md-to-adf.sh` — not sourced).
6. `POST /rest/api/3/issue/{key}/transitions` with `?notifyUsers=false` if
   `--no-notify`.
7. On 204: exit 0. On non-2xx: propagate exit code from `jira-request.sh`.

**Sourcing (unconditional at script load time):**
```bash
source "$_JIRA_TRANSITION_SCRIPT_DIR/jira-common.sh"
source "$_JIRA_TRANSITION_SCRIPT_DIR/jira-body-input.sh"
```

#### 2. EXIT_CODES.md update

**File**: `skills/integrations/jira/scripts/EXIT_CODES.md`

Add rows for 120–129 in the table and update the namespace summary section:

```markdown
| 120 | `E_TRANSITION_NO_KEY`       | `jira-transition-flow.sh` | No issue key positional argument |
| 121 | `E_TRANSITION_NO_STATE`     | `jira-transition-flow.sh` | No target state name |
| 122 | `E_TRANSITION_NOT_FOUND`    | `jira-transition-flow.sh` | No transition matches the given state name |
| 123 | `E_TRANSITION_AMBIGUOUS`    | `jira-transition-flow.sh` | Multiple transitions share the state name |
| 124 | `E_TRANSITION_BAD_FLAG`     | `jira-transition-flow.sh` | Unrecognised flag |
| 125 | `E_TRANSITION_NO_BODY`      | `jira-transition-flow.sh` | `--comment-file` not found |
| 126 | `E_TRANSITION_BAD_RESOLUTION` | `jira-transition-flow.sh` | Empty resolution value |
| 127–129 | —                       | reserved                  | Reserved |
```

Update the Phase summary to add Phase 4 rows and rename the section heading
from `## Phase 3 namespace summary` to `## Phase 4 namespace summary`:
```markdown
| 120–126 | `jira-transition-flow.sh` | |
| 127–129 | reserved                   | Reserved for follow-up |
```

#### 3. Test runner update

**File**: `skills/integrations/jira/scripts/test-jira-scripts.sh`

Add before `exit "$EXIT_CODE"`:
```bash
bash "$SCRIPT_DIR/test-jira-transition.sh" || EXIT_CODE=$?
```

### Success Criteria

#### Automated Verification

- [x] `bash skills/integrations/jira/scripts/test-jira-transition.sh` exits 0,
  all 23 cases PASS (Cases 1–22 including 3b), no FAIL lines.
- [x] `bash skills/integrations/jira/scripts/test-jira-scripts.sh` exits 0.

#### Manual Verification

- [ ] Running `jira-transition-flow.sh --describe ENG-1 "In Progress"` (in a
  fake repo with credentials) makes a GET to resolve the transition, outputs a
  JSON object with `key`, `state`, and `transition_id` (not null), and exits 0
  without making any POST call.
- [ ] Running `jira-transition-flow.sh --describe ENG-1 --transition-id 21`
  outputs a JSON object with `transition_id: "21"` and `state: null` (no GET
  made), and exits 0.
- [ ] Running with an invalid flag produces a clear usage error and exits 124.

---

## Phase 3: `transition-jira-issue` SKILL.md

### Overview

Create the SKILL.md for `transition-jira-issue` using the `skill-creator`
skill, then validate it matches the codebase's write-skill conventions.

### Changes Required

#### 1. Skill directory and SKILL.md

**Directory**: `skills/integrations/jira/transition-jira-issue/`
**File**: `skills/integrations/jira/transition-jira-issue/SKILL.md`

**Invoke via `skill-creator:skill-creator`** with the full spec below, so
it creates and evaluates the SKILL.md.

**Frontmatter spec:**
```yaml
---
name: transition-jira-issue
description: >
  Use this skill only when the user explicitly invokes /transition-jira-issue
  to move a Jira issue through its workflow by state name. This is a write skill
  with irreversible side effects — it must never be auto-invoked from
  conversational context. Accepts an issue key and target state name
  (case-insensitive). Shows a transition preview and requires explicit
  confirmation before posting.
argument-hint: "ISSUE-KEY (STATE-NAME | --transition-id ID) [--resolution NAME] [--comment TEXT | --comment-file PATH] [--no-notify] [--quiet]"
disable-model-invocation: true
allowed-tools: Bash, Read, Write
---
```

**Prose spec (key requirements for skill-creator):**

- **Config-read include lines**: The first two body lines of the generated
  SKILL.md (immediately after the frontmatter) must be the config-read shell
  includes, following `comment-jira-issue/SKILL.md` lines 18–19:
  `` !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh` ``
  `` !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh transition-jira-issue` ``
- Opening: "Transition a Jira issue through its workflow by state name.
  Work through the steps below in order. This skill never auto-invokes — it
  only runs when the user explicitly types `/transition-jira-issue`."
- No trust-boundary enforcement paragraph (no description/body content from
  issue fetches). Include it only if `--comment` is used.
- Step 1: Parse flags.
  - Required positional: `KEY`.
  - `STATE_NAME` (second positional) OR `--transition-id ID` — at least one
    is required; both may not be supplied together.
  - Optional: `--resolution NAME`, `--comment TEXT`, `--comment-file PATH`,
    `--no-notify`, `--quiet`.
- Step 2: Trust-boundary note for `--comment` only (if `--comment` or
  `--comment-file` is present, body content must come from the user's current
  turn or a named file — not from previously fetched issue content).
- Step 3: Generate the describe preview with `--describe`:
  ```
  ${CLAUDE_PLUGIN_ROOT}/skills/integrations/jira/scripts/jira-transition-flow.sh \
    --describe <KEY> [STATE_NAME] [--transition-id ID] [--resolution NAME] \
    [--comment TEXT | --comment-file PATH] [--no-notify]
  ```
  When `STATE_NAME` is given (without `--transition-id`), `--describe` makes
  a read-only GET to Jira to resolve the transition eagerly — auth errors
  (exit 11/22) or network errors (exit 21) can surface here, not only during
  the POST. When `--transition-id` is given, `--describe` exits without any
  network call.
- Step 4: Handle the describe result:
  - Exit 0: proceed to Step 5.
  - Exit 122 (`E_TRANSITION_NOT_FOUND`): "No transition to `'<STATE_NAME>'`
    is available from the current state. Check the issue's workflow in Jira."
    Stop — do not proceed to confirm.
  - Exit 123 (`E_TRANSITION_AMBIGUOUS`): parse the JSON array from stdout
    (list of `{id, name, to.name}` objects). If stdout is not parseable as a
    JSON array, surface it verbatim and ask the user to re-invoke with
    `--transition-id` directly. Otherwise, present a table:
    | ID | Transition name | Target state |
    Ask: "Multiple transitions lead to `'<STATE_NAME>'`. Which would you like
    to use? Reply with the transition ID (e.g. `41`) to proceed, or `cancel`
    to abort."
    After 3 failed disambiguation attempts, abort: "Aborted — could not
    resolve transition unambiguously. No Jira write was made."
    Validate the chosen reply as digits only (`^[0-9]+$`) before constructing
    the re-invocation command; if not purely numeric, count the attempt as
    failed. On a valid numeric ID choice: re-invoke with `--describe
    --transition-id <chosen_id>` (omit STATE_NAME — STATE_NAME and
    `--transition-id` are mutually exclusive positional/flag slots; passing
    both triggers exit 124); loop back to Step 3.
  - Other non-zero: "Preview failed — no API call was made." Stop.
- Step 5: Render the preview under `> **Proposed Jira write — review before
  sending**`:
  - `POST /rest/api/3/issue/<KEY>/transitions`
  - Moving `<KEY>` → `"<STATE_NAME>"` via transition `<ID>` (both are now
    always known from the describe output — if `state` is null because only
    `--transition-id` was supplied, render as "via transition `<ID>`" without
    a target state name)
  - Resolution: `<NAME>` if `--resolution` given
  - Comment preview (truncate at 500 chars) if `--comment`/`--comment-file`
    given
  - ⚠️ Notifications suppressed if `--no-notify`
- Step 6: Confirm — use the exact canonical phrase: "Send this to Jira? Reply
  **y** to confirm, **n** to revise, anything else to abort."
  Interpret the reply with three branches:
  - **Clear yes** (`y` / `yes`): proceed to Step 7.
  - **Clear no** (`n` / `no` or an explicit revision request): ask "What would
    you like to change?" Re-apply the trust-boundary check if `--comment`
    content is revised (it must come from the user's current turn). Rebuild
    the preview from Step 3 with the updated parameters. Allow up to 3
    revision cycles; after the third rejection abort with "Aborted — no Jira
    write was made."
  - **Ambiguous or off-topic**: abort immediately with "Aborted — no Jira
    write was made."
- Step 7: Send — invoke without `--describe`.
- Step 8: Render response:
  - 204 with state name known → `✓ **<KEY>** transitioned to "<STATE_NAME>".`
  - 204 with only `--transition-id` (state null) → `✓ **<KEY>** transition ID <ID> applied.`
  - After a disambiguation flow: the original STATE_NAME from the user's
    invocation is known in the skill's context even though the re-invoke used
    `--transition-id` (which returns `state: null`). Prefer rendering
    `✓ **<KEY>** transitioned to "<ORIGINAL_STATE_NAME>" (via transition ID <ID>)`
    when the original STATE_NAME is available.
- Step 9: Exit-code table (120–126, plus inherited 11, 12, 13, 19, 20, 21,
  22, 34; group 11, 12, and 22 together as auth errors with "Check credentials
  with /init-jira"; override the exit-12 entry specifically with: "HTTP 403 —
  you do not have the `TRANSITION_ISSUES` permission on this project. Check
  your Jira role with your project admin.").
- Examples:
  - Example 1: `/transition-jira-issue ENG-42 "In Progress"`
  - Example 2: `/transition-jira-issue ENG-42 "Done" --resolution "Fixed"`
  - Example 3: disambiguation flow — ambiguous state name, user picks by ID.

### Success Criteria

#### Automated Verification

- [x] `ls skills/integrations/jira/transition-jira-issue/SKILL.md` exits 0.
- [x] The SKILL.md frontmatter parses cleanly (valid YAML between `---` delimiters).
- [x] `skill-creator` eval passes (all eval cases PASS).

#### Manual Verification

- [ ] The SKILL.md contains the disambiguation step (exit 123 path) in the describe-result handler (Step 4), not post-confirmation.
- [ ] The SKILL.md contains the `--transition-id` bypass path (no GET, describe shows known ID).
- [ ] The SKILL.md contains the confirmation gate with the exact canonical phrase.
- [ ] The SKILL.md renders success differently for STATE_NAME vs `--transition-id`-only invocations.

---

## Phase 4: Attach flow — TDD fixtures and test script

### Overview

Same TDD-first sequence for the attachment skill. Simpler than transition
(no name lookup, no disambiguation), but multipart upload has its own
subtleties.

### Changes Required

#### 1. Test fixture scenarios

**File**: `skills/integrations/jira/scripts/test-fixtures/scenarios/attach-post-200.json`

Single-file upload; `expect_headers` validates `X-Atlassian-Token: no-check`:

```json
{
  "expectations": [
    {
      "method": "POST",
      "path": "/rest/api/3/issue/ENG-1/attachments",
      "expect_headers": {"X-Atlassian-Token": "no-check"},
      "response": {
        "status": 200,
        "body": "[{\"id\":10001,\"filename\":\"screenshot.png\",\"size\":12345}]"
      }
    }
  ]
}
```

**File**: `skills/integrations/jira/scripts/test-fixtures/scenarios/attach-post-200-two-files.json`

Two-file upload. `capture_body: true` is added so the test can assert that
both `filename=` parts are present in the captured multipart body (verifying
the implementation sends all files, not just the first):

```json
{
  "expectations": [
    {
      "method": "POST",
      "path": "/rest/api/3/issue/ENG-1/attachments",
      "expect_headers": {"X-Atlassian-Token": "no-check"},
      "capture_body": true,
      "response": {
        "status": 200,
        "body": "[{\"id\":10001,\"filename\":\"a.txt\"},{\"id\":10002,\"filename\":\"b.txt\"}]"
      }
    }
  ]
}
```

The test asserts `grep -c 'filename=' <captured-body-file>` returns `2`,
confirming both file parts were sent.

**File**: `skills/integrations/jira/scripts/test-fixtures/scenarios/attach-post-403.json`

403 Forbidden (missing `CreateAttachments` project permission). Note: the
`X-Atlassian-Token: no-check` header is already injected by `jira-request.sh`
for all multipart requests, so this 403 simulates a permissions error only —
not a missing-token scenario:

```json
{
  "expectations": [
    {
      "method": "POST",
      "path": "/rest/api/3/issue/ENG-1/attachments",
      "response": {
        "status": 403,
        "body": "{\"errorMessages\":[\"You do not have permission to attach files to this issue.\"],\"errors\":{}}"
      }
    }
  ]
}
```

**File**: `skills/integrations/jira/scripts/test-fixtures/scenarios/attach-describe-guard.json`

Guard for `--describe` mode — `"consume": false` paired with `"capture_url":
true` (matching the pattern in all existing guard fixtures) so that if the
dry-run path accidentally makes a POST the test detects it:

```json
{
  "expectations": [
    {
      "method": "POST",
      "path": "/rest/api/3/issue/ENG-1/attachments",
      "consume": false,
      "capture_url": true,
      "response": {
        "status": 200,
        "body": "[]"
      }
    }
  ]
}
```

**File**: `skills/integrations/jira/scripts/test-fixtures/scenarios/attach-post-401.json`

POST attachments → 401 Unauthorized:

```json
{
  "expectations": [
    {
      "method": "POST",
      "path": "/rest/api/3/issue/ENG-1/attachments",
      "response": {
        "status": 401,
        "body": "{\"errorMessages\":[\"You do not have permission to access this resource.\"],\"errors\":{}}"
      }
    }
  ]
}
```

#### 2. Test script

**File**: `skills/integrations/jira/scripts/test-jira-attach.sh`

| Case | Description | Key assertion | Fixture |
|------|-------------|---------------|---------|
| 1 | Happy path — single file → POST 200 → exit 0, attachment JSON on stdout | exit 0; stdout contains `"id"` | `attach-post-200.json` |
| 2 | Two files → POST 200 → exit 0; captured body contains both `filename=` parts | exit 0; `grep -c 'filename=' body` == 2 | `attach-post-200-two-files.json` |
| 3 | `--describe` dry-run — no POST made, outputs description JSON; supply a real file created via `mktemp` so file validation passes | no POST; stdout has `"key"`, `"files"`; `jq '.files[0]'` matches the temp file path | none (no mock needed) |
| 4 | `--describe` guard — `consume: false` + `capture_url: true` confirms no POST; call `stop_mock` before asserting captured URLs file | exit 0; `stop_mock` then `jq -c '.' "$CAPTURED_URLS"` equals `[]` | `attach-describe-guard.json` |
| 5a | Single file not found → exit 132, no network call | exit 132 | none |
| 5b | Two files supplied, second one missing → exit 132 (fail-fast), no network call | exit 132 | none |
| 6 | No files supplied → exit 131, no network call | exit 131 | none |
| 7 | No issue key supplied → exit 130, no network call | exit 130 | none |
| 8 | POST 403 → exits 12 | exit 12 | `attach-post-403.json` |
| 9 | POST 401 → exits 11 | exit 11 | `attach-post-401.json` |
| 10 | Unrecognised flag → exit 133 | exit 133 | none |

### Success Criteria

#### Automated Verification

- [x] `bash skills/integrations/jira/scripts/test-jira-attach.sh` exits
  non-zero (FAIL lines) before `jira-attach-flow.sh` exists — proves the
  tests are live.

---

## Phase 5: Attach flow script implementation

### Overview

Write `jira-attach-flow.sh` until all 10 test cases pass.

### Changes Required

#### 1. Flow script

**File**: `skills/integrations/jira/scripts/jira-attach-flow.sh`

```
Usage: jira-attach-flow.sh [--describe] KEY FILE [FILE...]
         [--quiet]
         [--help | -h]
```

**Exit codes (130–139):**

| Code | Name | Description |
|------|------|-------------|
| 130 | `E_ATTACH_NO_KEY` | No issue key positional argument |
| 131 | `E_ATTACH_NO_FILES` | No file paths supplied |
| 132 | `E_ATTACH_FILE_MISSING` | At least one named file does not exist or is not readable |
| 133 | `E_ATTACH_BAD_FLAG` | Unrecognised flag |
| 134–139 | — | Reserved |

**Algorithm:**

Call `jira_require_dependencies` (exits `E_MISSING_DEP` if `jq` or `curl` is
absent) before argument parsing.

1. Parse arguments. `KEY` is the first positional; all remaining positionals
   are file paths (after `--`-terminated option parsing). Exit 130 if KEY is
   missing; exit 131 if no file paths are supplied; exit 133 for unrecognised
   flags.
2. Validate files (before `--describe` short-circuit so that describe mode
   also exits 132 on invalid paths, making the preview accurate):
   - For each path, in order:
     - Reject paths beginning with `-` (e.g. `-` itself reads from stdin);
       exit 132 with a clear error.
     - Reject symlinks that resolve to paths under `/dev`, `/proc`, or `/sys`;
       exit 132 with a clear error.
     - Check existence and readability (`[ -f path ] && [ -r path ]`). On
       first failing file: emit `E_ATTACH_FILE_MISSING` and exit 132.
   - All file paths must be quoted when assembled as `--multipart` arguments
     to prevent word-splitting.
3. If `--describe`:
   - Output JSON `{"key":"<KEY>","files":["<path1>","<path2>",...]}` and exit 0.
     No network call or credentials needed.
4. Build `jira-request.sh` arguments: one `--multipart "file=@<path>"` per
   file (paths already validated and quoted in step 2). `jira-request.sh`
   resolves credentials internally.
5. `POST /rest/api/3/issue/{key}/attachments` via `jira-request.sh`.
6. On 200: print response JSON to stdout, exit 0.
7. On non-2xx: propagate exit code from `jira-request.sh`.

**Sourcing (unconditional at script load time):**
```bash
source "$_JIRA_ATTACH_SCRIPT_DIR/jira-common.sh"
```

**Note**: `jira-request.sh` already injects `X-Atlassian-Token: no-check`
for multipart requests — no explicit handling needed in the flow script.

#### 2. EXIT_CODES.md update

Add rows for 130–139:

```markdown
| 130 | `E_ATTACH_NO_KEY`          | `jira-attach-flow.sh` | No issue key positional argument |
| 131 | `E_ATTACH_NO_FILES`        | `jira-attach-flow.sh` | No file paths supplied |
| 132 | `E_ATTACH_FILE_MISSING`    | `jira-attach-flow.sh` | A named file does not exist or is not readable |
| 133 | `E_ATTACH_BAD_FLAG`        | `jira-attach-flow.sh` | Unrecognised flag |
| 134–139 | —                      | reserved              | Reserved |
```

Update namespace summary:
```markdown
| 130–133 | `jira-attach-flow.sh` | |
| 134–139 | reserved               | Reserved for follow-up |
```

#### 3. Test runner update

**File**: `skills/integrations/jira/scripts/test-jira-scripts.sh`

Add before `exit "$EXIT_CODE"`:
```bash
bash "$SCRIPT_DIR/test-jira-attach.sh" || EXIT_CODE=$?
```

### Success Criteria

#### Automated Verification

- [x] `bash skills/integrations/jira/scripts/test-jira-attach.sh` exits 0,
  all 11 cases PASS (Cases 1–4, 5a, 5b, 6–10), no FAIL lines. (Cases 5a and
  5b are two separate assert blocks within Case 5's test block — the total
  test table entry count is 10.)
- [x] `bash skills/integrations/jira/scripts/test-jira-scripts.sh` exits 0.

#### Manual Verification

- [ ] `jira-attach-flow.sh --describe ENG-1 /nonexistent` exits 132 with a
  clear error (file validation runs before `--describe` exits).
- [ ] `jira-attach-flow.sh --describe ENG-1 /tmp/test.txt` outputs
  `{"key":"ENG-1","files":["/tmp/test.txt"]}` and exits 0 without a network
  call.
- [ ] `jira-attach-flow.sh ENG-1 -` exits 132 with a clear error (stdin path
  rejected).
- [ ] `jira-attach-flow.sh ENG-1 /nonexistent` exits 132 with a clear error.

---

## Phase 6: `attach-jira-issue` SKILL.md

### Overview

Create the SKILL.md for `attach-jira-issue` using the `skill-creator` skill.
Simpler than `transition-jira-issue` — no disambiguation, no body content, no
trust-boundary enforcement.

### Changes Required

#### 1. Skill directory and SKILL.md

**Directory**: `skills/integrations/jira/attach-jira-issue/`
**File**: `skills/integrations/jira/attach-jira-issue/SKILL.md`

**Invoke via `skill-creator:skill-creator`** with the spec below.

**Frontmatter spec:**
```yaml
---
name: attach-jira-issue
description: >
  Use this skill only when the user explicitly invokes /attach-jira-issue to
  upload one or more local files as attachments to a Jira issue. This is a
  write skill with irreversible side effects — it must never be auto-invoked
  from conversational context. Shows a preview of what will be uploaded and
  requires explicit confirmation before POSTing.
argument-hint: "ISSUE-KEY FILE [FILE...] [--quiet]"
disable-model-invocation: true
allowed-tools: Bash, Read, Write
---
```

**Prose spec (key requirements for skill-creator):**

- **Config-read include lines**: The first two body lines of the generated
  SKILL.md (immediately after the frontmatter) must be the config-read shell
  includes, following `comment-jira-issue/SKILL.md` lines 18–19:
  `` !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh` ``
  `` !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh attach-jira-issue` ``
- Opening: "Upload one or more local files as attachments to a Jira issue.
  Work through the steps below in order. This skill never auto-invokes — it
  only runs when the user explicitly types `/attach-jira-issue`."
- No trust-boundary paragraph (no body content involved).
- No body-input chain (files are identified by the user-supplied paths).
- Step 1: Parse flags. Required: `KEY` (issue key), one or more `FILE` paths.
  Optional: `--quiet`.
- Step 2: Generate the describe preview with `--describe`:
  ```
  ${CLAUDE_PLUGIN_ROOT}/skills/integrations/jira/scripts/jira-attach-flow.sh \
    --describe <KEY> <FILE> [FILE...]
  ```
- Step 3: Handle preview failures:
  - Exit 132 (`E_ATTACH_FILE_MISSING`) → "File not found: `<path>`". Stop —
    file validation runs before `--describe`, so this means a named file does
    not exist or is not readable.
  - Exit 133 (`E_ATTACH_BAD_FLAG`) → "Unrecognised flag. Usage:
    `/attach-jira-issue ISSUE-KEY FILE [FILE...] [--quiet]`". Stop.
  - Other non-zero → "Preview failed — no API call was made." Stop.
- Step 4: Render the preview under `> **Proposed Jira write — review before
  sending**`:
  - `POST /rest/api/3/issue/<KEY>/attachments`
  - List of files to be uploaded. For each file, run `wc -c <path>` via Bash
    and show basename + size (humanised: display as KB if ≥ 1 024 bytes, MB
    if ≥ 1 048 576 bytes, otherwise bytes).
  - ⚠️ "Attachments cannot be removed by this skill once uploaded."
- Step 5: Confirm — use the exact canonical phrase: "Send this to Jira? Reply
  **y** to confirm, **n** to revise, anything else to abort."
- Step 6: Send — invoke without `--describe`.
- Step 7: Render response — parse the JSON array returned by the API; for
  each attachment show filename, ID, and humanised size (KB/MB as above). Show:
  ```
  ✓ Attached to **<KEY>**:
  - <filename> (ID: <id>, <humanised-size>)
  ```
- Step 8: Exit-code table (130–133, plus inherited 11–12, 13, 19, 20, 21, 34).
- Examples:
  - Example 1: `/attach-jira-issue ENG-42 ./screenshot.png`
  - Example 2: `/attach-jira-issue ENG-42 ./logs.txt ./debug.json`

### Success Criteria

#### Automated Verification

- [x] `ls skills/integrations/jira/attach-jira-issue/SKILL.md` exits 0.
- [x] SKILL.md frontmatter is valid YAML.
- [x] `skill-creator` eval passes.

#### Manual Verification

- [ ] The SKILL.md contains the irreversibility warning in the preview step.
- [ ] The SKILL.md contains the confirmation gate with standard phrasing.
- [ ] The SKILL.md renders the attachment list from the JSON array response.

---

## Testing Strategy

### TDD sequencing (strictly enforced)

For each flow script, the sequence is:

1. Write fixtures → 2. Write test script → 3. Run tests (all FAIL) →
4. Write flow script → 5. Run tests (all PASS)

Never write the flow script before confirming the test script fails on a
missing implementation.

### Unit-level tests (within each test script)

- **Argument validation** (no network): missing key, missing state/files,
  bad flags — all verified before any mock server starts.
- **Dry-run guard** (`--describe` / `consume: false`): confirms the
  dry-run path makes zero API calls.
- **Happy path** (mock server): exercises the full call chain.
- **Error propagation**: verifies each HTTP error (401, 403, 404, 400)
  produces the correct exit code and surfaces the error body on stderr.

### Integration (umbrella runner)

`test-jira-scripts.sh` is the final gate. It must exit 0 for the phase to
be considered complete.

---

## Performance Considerations

No performance concerns beyond those already addressed by `jira-request.sh`'s
retry logic. Multipart uploads of large files (>10 MB) are possible; the flow
script validates file existence and readability but does not validate size. When
a file exceeds 10 MB (Jira Cloud's default attachment limit), the flow script
should emit a warning to stderr before attempting the upload — `curl`'s 30-second
timeout (exit 21) is otherwise the only backstop, and it produces a confusing
failure rather than an actionable message. Size is checked via `wc -c` on each
file during step 2 validation.

---

## Migration Notes

No migration required. The two new flow scripts and SKILL.md directories are
purely additive. No existing files change except:

- `EXIT_CODES.md` — additive rows only.
- `test-jira-scripts.sh` — two new `bash` invocations appended.

---

## References

- Research: `meta/research/codebase/2026-04-29-jira-cloud-integration-skills.md` §§2.5,
  2.7, 4.1 (Phase 4), 4.10 (per-skill design notes)
- Exit code namespace: `skills/integrations/jira/scripts/EXIT_CODES.md`
- Multipart test reference: `skills/integrations/jira/scripts/test-jira-request.sh:134–142`
- Describe pattern reference: `skills/integrations/jira/comment-jira-issue/SKILL.md:84–90`
- Guard fixture pattern: `skills/integrations/jira/scripts/test-fixtures/scenarios/comment-add-print-payload-guard.json`
- Flow script pattern: `skills/integrations/jira/scripts/jira-update-flow.sh:1–48`
- Write skill SKILL.md pattern: `skills/integrations/jira/update-jira-issue/SKILL.md`
- `skill-creator` skill for SKILL.md creation and eval
