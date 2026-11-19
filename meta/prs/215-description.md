---
type: "pr-description"
id: "215"
title: "[PP-990] Add a Shortcut issue-tracker connector"
date: "2026-11-14T12:25:13+00:00"
author: "Tom Scott"
producer: "describe-pr"
status: "complete"
work_item_id: "PP-990"
parent: "work-item:PP-990"
relates_to: ["work-item:PP-984", "work-item:PP-988", "work-item:PP-985", "work-item:PP-989", "work-item:PP-987", "work-item:PP-986", "work-item:PP-983"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/215"
pr_number: 215
tags: ["shortcut", "integrations", "sync"]
revision: "68a98275305a49cdf8e22d974678fee0dc7f53d9"
repository: "accelerator"
last_updated: "2026-11-19T13:46:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [PP-990] Add a Shortcut issue-tracker connector

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
- **Shortcut keys can own work item IDs.** Under `work.id_pattern:
  "{tracker}"`, a pushed story takes its `sc-<n>` key as its `id` and an
  epic takes `epic-<n>`. ADR-0076 records the decision. ADR-0070's rules
  for drafts, promotion and ID retirement apply unchanged.
- **`ACCELERATOR_LOG` reaches the work and Shortcut binaries**, so a sync
  can trace each Shortcut request. The trace never includes the token.
- **Docs.** The issue-trackers page gains a Shortcut tab, and a new
  `shortcut.md` page covers setup. The sync guide, the cookbook,
  `which-skill`, `configuration.md`, the README and the CHANGELOG mention
  Shortcut.
- **Follow-up work items PP-983 to PP-989.** They record the work this PR
  leaves out, and are synced to Linear.

## Context

- Work item: `meta/work/PP-990-shortcut-integration.md`
- Plan: `meta/plans/2026-11-13-0391-shortcut-integration.md`. Each phase has
  implementation notes that record where the code departs from the plan.
- Decision: `meta/decisions/ADR-0076-shortcut-keys-as-tracker-owned-work-item-ids.md`

## Testing

- [x] The full `mise run` passes on the branch rebased onto main, which is
      behind only main's latest version bump.
- [x] Client tests cover references, filters, state resolution and the 16
      write cases, including a truncated listing and a timed-out create.
- [x] CLI flow tests run each subcommand against 21 recorded scenarios. Pins
      hold the CLI surface, the keywords, the exit codes, the stdout goldens
      and the stderr diagnostics.
- [x] Eleven `sync_run_real_client` cases drive the sync engine through
      the real Shortcut client against a mock server, and the shared pull
      test runs for Shortcut beside Jira and Linear. They check how sync
      classifies push, pull, conflict and discovery, and they apply a push
      to a story and an epic, a pull, and a push refused with 403. One case
      checks that a search across several teams stops at one deadline.
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
  PP-987 and PP-986 propose one exhaustive tracker registry and a review of
  the shared code.
- **`attach` takes a URL and no title.** v4 stores an external link as a
  bare URL, so no field can carry a title. File upload is work item PP-984.
- **The read-skill evals are missing.** The evals guard needs a real
  `benchmark.json` from live runs. Work item PP-983 tracks them.
- **Sync does not yet map parent links, epics in discovery, or status and
  priority.** Work items PP-988, PP-985 and PP-989 cover these.
- **The exit codes are new values.** They do not reuse the Jira or Linear
  numbers. `exit_codes_parity` pins them against
  `captured-exit-codes.txt`.
- **The Shortcut work items and ADR were renumbered after the rebase.** Main
  moved work items onto Linear keys, so the items numbered 0391 to 0398 are
  now PP-990 and PP-983 to PP-989, with their old IDs kept in `aliases`. The
  plan's filename still carries 0391. Main also took ADR-0075, so the
  Shortcut ADR is now ADR-0076.
- **The branch carries a Linear sync of unrelated work items.** The sync
  that promoted the Shortcut items also pulled nine items edited in Linear:
  PP-22, PP-87, PP-865, PP-868, PP-886, PP-893, PP-916, PP-926 and PP-968.
  PP-87 is held at its content from before the pull, because Linear's copy
  lists a topic-research report template that main does not ship yet, which
  fails the template-shape cross-check. A full sync from this branch pulls it
  again, so leave PP-87 out until that template lands.
- Live tests ran against a scratch Shortcut workspace with a personal token.
  No token appears in the repository or the fixtures.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
