---
type: "pr-description"
id: "215"
title: "[0371] Add a Shortcut issue-tracker connector"
date: "2026-11-14T12:25:13+00:00"
author: "Tom Scott"
producer: "describe-pr"
status: "complete"
work_item_id: "0371"
parent: "work-item:0371"
relates_to: ["work-item:0372", "work-item:0373", "work-item:0374", "work-item:0375", "work-item:0376", "work-item:0377", "work-item:0378"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/215"
pr_number: 215
tags: ["shortcut", "integrations", "sync"]
revision: "ba5dd5390bd7e05f73ece2192e7cee6a8c42e3cd"
repository: "accelerator"
last_updated: "2026-11-14T12:25:13+00:00"
last_updated_by: "Tom Scott"
schema_version: 1
---

# [0371] Add a Shortcut issue-tracker connector

## Summary

This PR adds Shortcut as a third issue tracker beside Jira and Linear. A
developer can now sync the work items in `meta/work/` with Shortcut stories
and epics, and use eight new skills to read and write Shortcut from Claude
Code. The connector targets the Shortcut REST API v4 and follows the shape of
the Jira and Linear connectors.

## Changes

- **`shortcut-client` crate.** It wraps the v4 API: references (`sc-1234`,
  `epic-56`), the search filter, the workflow-state catalogue, and the write
  calls (create, update, transition, comment, attach a link). Each write
  chooses a resend policy, so a timed-out non-idempotent write never repeats.
- **`shortcut-cli` crate and the `accelerator-shortcut` sub-binary.** It
  provides `init`, `show`, `search`, `create`, `update`, `transition`,
  `comment` and `attach`, with a pinned exit-code table. The binary follows
  the fourteen-point registration checklist.
- **Eight skills** under `skills/integrations/shortcut/`: `init-shortcut`,
  `show-shortcut-story`, `search-shortcut-stories`, and the five write skills
  for create, update, transition, comment and attach.
- **Sync wiring.** `work-cli` registers a Shortcut arm in the tracker
  registry, and `/sync-work-items` resolves `work.integration: shortcut`. Sync
  maps a work item's title and body to the story, and its `kind` to the
  story type. An `epic` item becomes a Shortcut epic.
- **Docs.** The issue-trackers page gains a Shortcut tab, and a new
  `shortcut.md` page covers setup. The sync guide, the cookbook,
  `which-skill`, `configuration.md`, the README and the CHANGELOG mention
  Shortcut.
- **Follow-up work items 0372–0378.** They record the work this PR leaves
  out.

## Context

- Work item: `meta/work/0371-shortcut-integration.md`
- Plan: `meta/plans/2026-11-13-0371-shortcut-integration.md`. Each phase has
  implementation notes that record where the code departs from the plan.

## Testing

- [x] The full `mise run` exits 0 on `cea2374c`.
- [x] Client tests cover references, filters, state resolution and the 16
      write cases, including a truncated listing and a timed-out create.
- [x] CLI flow tests run each subcommand against 21 recorded scenarios. Pins
      hold the CLI surface, the keywords, the exit codes, the stdout goldens
      and the stderr diagnostics.
- [x] Nine `sync_run_real_client` cases drive the sync engine through the
      real Shortcut client against a mock server. They cover push, pull,
      conflict, discovery and a 403 on push.
- [x] Three corpus fixtures pin the sync hashes. An independent Python script
      produced the expected values.
- [ ] Manual: push a work item and an epic item to a live workspace, pull an
      edit made in Shortcut, resolve a conflict with `adopt:`, and discover a
      story created in Shortcut.
- [ ] Manual: confirm that a read-only token gets a retryable failure, and
      that `init verify` names the read-only scope.
- [ ] Manual: run `/accelerator:init-shortcut` in a fresh repository, and
      check that natural prompts trigger the show and search skills.
- [ ] Manual: run each write skill once against a real story.
- [ ] Manual: view the Shortcut tab on the issue-trackers page at desktop and
      phone widths.

## Notes for Reviewers

- **The client and CLI crates repeat much of the Jira and Linear crates.** I
  kept the duplication so this PR changes no existing tracker. Work items
  0376 and 0377 propose one exhaustive tracker registry and a review of the
  shared code.
- **`attach` takes a URL and no title.** v4 stores an external link as a
  bare URL, so no field can carry a title. File upload is work item 0372.
- **The read-skill evals are missing.** The evals guard needs a real
  `benchmark.json` from live runs. Work item 0378 tracks them.
- **Sync does not yet map parent links, epics in discovery, or status and
  priority.** Work items 0373, 0374 and 0375 cover these.
- **The exit codes are new values.** They do not reuse the Jira or Linear
  numbers. `exit_codes_parity` pins them against
  `captured-exit-codes.txt`.
- Live tests ran against a scratch Shortcut workspace with a personal token.
  No token appears in the repository or the fixtures.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
