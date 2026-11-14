---
type: "plan-validation"
id: "2026-11-13-0371-shortcut-integration-validation"
title: "Validation Report: Shortcut Integration Implementation Plan"
date: "2026-11-14T12:40:00+00:00"
author: "Tom Scott"
producer: "validate-plan"
status: "complete"
result: "partial"
parent: "plan:2026-11-13-0371-shortcut-integration"
target: "plan:2026-11-13-0371-shortcut-integration"
tags: ["shortcut", "integrations", "tracker"]
last_updated: "2026-11-14T12:40:00+00:00"
last_updated_by: "Tom Scott"
schema_version: 1
---

## Validation Report: Shortcut Integration Implementation Plan

The code implements all seven phases, and every automated check passes. I
mark the result `partial` for two reasons. Phase 5 leaves out the two
read-skill eval cases the plan asked for, and work item 0378 now holds them.
Sixteen manual checks for Phases 4–7 are still open. The plan stays at
`draft` until those checks pass.

### Implementation Status

✓ Phase 1: Make room for Shortcut - Fully implemented
✓ Phase 2: `shortcut-client` foundation - Fully implemented
✓ Phase 3: `shortcut-client` implements `RemoteTracker` - Fully implemented
✓ Phase 4: `work.integration: shortcut` syncs - Fully implemented; manual
  checks open
⚠️ Phase 5: `accelerator-shortcut` binary, read side - Implemented without
  the two eval cases; manual checks open
✓ Phase 6: Write side - Implemented; `attach` has no `--title`; manual
  checks open
✓ Phase 7: Documentation and follow-ups - Fully implemented; manual check
  open

### Automated Verification Results

✓ Full local CI mirror: `mise run` exited 0 on `cea2374c`. That run
  includes `docs:check`.
✓ Read-only CI set: `mise run check` exits 0 on `1bc700eb`, the PR head.
✓ Tests: `cargo nextest run -p shortcut-client -p shortcut-cli
  -p accelerator-work -p work-adapters -p accelerator-migrate --all-features`
  runs 1936 tests, and all 1936 pass.
✓ The plan has 44 ticked boxes and no open automated box.

nextest flags one test as leaky:
`accelerator-work::cli_create a_bad_author_environment_is_reported_clearly`.
That test predates this branch, and it passes.

### Code Review Findings

#### Matches Plan:

- `shortcut-client` parses `sc-1234` and `epic-56`, and it refuses a
  reference from a foreign workspace.
- The client follows the v4 shapes: `{"entity": ...}` for one entity,
  `{entities, next_page_url}` for a listing, and the page shape for
  `/members`.
- Each write picks a resend policy. Comment and attach use
  `OnlyWhenThrottled`, so a timed-out comment or attach never posts twice.
  Update uses `WhileRetryable`, because a repeated update has the same
  effect.
- `resolve_story_state` refuses a missing workflow, an unknown state name
  and an ambiguous state name.
- The `RemoteTracker` adapter sorts each failure into Retryable, Terminal,
  Unconfigured or Rejected. The sync engine tests check the 403 case.
- `accelerator-shortcut` follows the fourteen-point registration checklist.
  `tasks/public_api.py` lists `shortcut-client` as an adapter and
  `shortcut-cli` as a composition root.
- `work-cli` registers the Shortcut arm, and `/sync-work-items` resolves
  `shortcut`. The test scrub clears the Shortcut token variables.
- Three `case-shortcut-*` corpus fixtures pin the sync hashes. An
  independent Python script produced the expected values.
- The docs gain a Shortcut tab and a `shortcut.md` page. The CHANGELOG and
  the README mention Shortcut.

#### Deviations from Plan:

- Phase 5 adds no eval cases. The evals guard needs a real `benchmark.json`
  from live runs, and a fabricated one would defeat the guard. Work item
  0378 records them.
- `attach` has no `--title`. v4 stores an external link as a bare URL, so no
  field can carry a title.
- The 403 engine test pushes a linked story instead of creating one. Sync
  never sends `create` for a plain unlinked item, so the planned case could
  not run.
- No flow test covers a truncated listing or a timed-out create, because a
  flow test cannot shorten the deadline or the timeout. The client tests
  cover both.
- The exit codes take new values instead of reusing the Jira and Linear
  numbers. `exit_codes_parity` pins them against `captured-exit-codes.txt`.

#### Potential Issues:

- `shortcut-client` and `shortcut-cli` repeat much of the Jira and Linear
  crates. A fix to one tracker's transport or flow code will not reach the
  others. Work items 0376 and 0377 cover this.
- Sync maps title, body and kind only. A user who expects status, priority
  or parent links to sync will see them stay local. Work items 0373 and
  0375 cover this.
- Discovery finds stories only. An epic created in Shortcut does not appear
  locally until work item 0374 is done.
- The trigger phrases of `show-shortcut-story` and `search-shortcut-stories`
  have no eval yet. A wording change could stop a natural prompt from
  triggering them, and no test would fail.

### Manual Testing Required:

1. Sync (Phase 4), in a scratch repository with `work.integration: shortcut`:
  - [ ] Push a work item, and check that a story appears.
  - [ ] Push a work item of kind `epic`, and check that an epic appears.
  - [ ] Edit the story description in Shortcut. Run a pull, and check the
    local body.
  - [ ] Edit both sides. Check the conflict dossier, and resolve it with
    `adopt:sc-<id>`.
  - [ ] Create a story in Shortcut under the team. Check that discovery
    finds it.
  - [ ] Push with a read-only token. Check for a retryable failure on each
    item.

2. Read skills (Phase 5):
  - [ ] Run `/accelerator:init-shortcut` in a fresh repository.
  - [ ] Ask "what is the status of sc-<id>?". Check that
    `show-shortcut-story` runs.
  - [ ] Ask "show me open bugs in Shortcut". Check that
    `search-shortcut-stories` runs with `--type bug`.
  - [ ] Run `init verify` with a `sct_r_` token. Check that the output
    names the read-only scope.

3. Write skills (Phase 6), against a real story:
  - [ ] Run `/accelerator:create-shortcut-story` on a local work item.
  - [ ] Run `/accelerator:transition-shortcut-story sc-<id> "In Progress"`.
  - [ ] Run `/accelerator:update-shortcut-story sc-<id> --title "x"`.
  - [ ] Run `/accelerator:attach-shortcut-story sc-<id> --url
    https://example.com`.
  - [ ] Run `/accelerator:comment-shortcut-story`.

4. Docs (Phase 7):
  - [ ] Open the issue-trackers page. Check the Shortcut tab at desktop and
    phone widths.

### Recommendations:

- Run the sixteen manual checks before you merge PR #215.
- Tick each manual box in the plan as it passes. Set the plan `status` to
  `done` when all sixteen pass.
- Start work item 0378 next. The read-skill triggers are the part of this
  change that no automated test guards.
