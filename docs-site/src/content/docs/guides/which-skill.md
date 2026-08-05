---
title: Which Skill Do I Need?
description: A decision index mapping common intents to the Accelerator
  skill that handles them, with links to each skill's reference page.
---

Accelerator ships 69 skills, but day to day you reach for a handful.
This page maps common intents to the skill to invoke. Every skill links
to its generated reference page, which documents its arguments and
behaviour in full.

Invoke any skill as `/accelerator:<name>` (or just `/<name>` when it is
unambiguous).

## Understanding a codebase

| I want to…                                     | Use                                                              |
| ---------------------------------------------- | ---------------------------------------------------------------- |
| Understand how part of the codebase works      | [`research-codebase`](../reference/skills/research/research-codebase.md) |
| Investigate a bug or production issue          | [`research-issue`](../reference/skills/research/research-issue.md)       |
| Answer a question with a time-boxed experiment | [`conduct-spike`](../reference/skills/research/conduct-spike.md)         |

## Planning and implementing

| I want to…                                    | Use                                                                |
| --------------------------------------------- | ------------------------------------------------------------------ |
| Plan a feature, refactor, or fix              | [`create-plan`](../reference/skills/planning/create-plan.md)               |
| Get a plan reviewed through quality lenses    | [`review-plan`](../reference/skills/planning/review-plan.md)               |
| Have my plan's assumptions interrogated       | [`stress-test-plan`](../reference/skills/planning/stress-test-plan.md)     |
| Execute an approved plan phase by phase       | [`implement-plan`](../reference/skills/planning/implement-plan.md)         |
| Check the implementation matched the plan     | [`validate-plan`](../reference/skills/planning/validate-plan.md)           |

## Tracking work

| I want to…                                     | Use                                                                      |
| ---------------------------------------------- | ------------------------------------------------------------------------ |
| Capture a feature, bug, task, spike, or epic   | [`create-work-item`](../reference/skills/work/create-work-item.md)               |
| Pull work items out of a spec or meeting notes | [`extract-work-items`](../reference/skills/work/extract-work-items.md)           |
| See what work items exist                      | [`list-work-items`](../reference/skills/work/list-work-items.md)                 |
| Break a work item down or sharpen its criteria | [`refine-work-item`](../reference/skills/work/refine-work-item.md)               |
| Change a work item's status or fields          | [`update-work-item`](../reference/skills/work/update-work-item.md)               |
| Review a work item before planning             | [`review-work-item`](../reference/skills/work/review-work-item.md)               |
| Stress-test a work item's scope and assumptions | [`stress-test-work-item`](../reference/skills/work/stress-test-work-item.md)    |
| Sync work items with Jira or Linear            | [`sync-work-items`](../reference/skills/work/sync-work-items.md)                 |

See [Sync work items with Jira or Linear](sync-work-items.md) for the
end-to-end setup.

## Recording decisions

| I want to…                                        | Use                                                                  |
| ------------------------------------------------- | -------------------------------------------------------------------- |
| Record an architectural decision                  | [`create-adr`](../reference/skills/decisions/create-adr.md)                  |
| Pull decisions out of existing research or plans  | [`extract-adrs`](../reference/skills/decisions/extract-adrs.md)              |
| Review, accept, or supersede an ADR               | [`review-adr`](../reference/skills/decisions/review-adr.md)                  |
| Jot down a quick observation or insight           | [`create-note`](../reference/skills/notes/create-note.md)                    |

See [Capture a decision](capture-a-decision.md) for the ADR lifecycle.

## Shipping

| I want to…                                | Use                                                                |
| ----------------------------------------- | ------------------------------------------------------------------ |
| Commit my session's changes               | [`commit`](../reference/skills/vcs/commit.md)                              |
| Write or update a PR description          | [`describe-pr`](../reference/skills/github/describe-pr.md)                 |
| Review a pull request                     | [`review-pr`](../reference/skills/github/review-pr.md)                     |
| Work through PR review feedback           | [`respond-to-pr`](../reference/skills/github/respond-to-pr.md)             |

See [Review a pull request](review-a-pr.md) for the review flow.

## Issue trackers

| I want to…                          | Use                                                                                   |
| ----------------------------------- | ------------------------------------------------------------------------------------- |
| Connect a repo to Jira              | [`init-jira`](../reference/skills/integrations/jira/init-jira.md)                             |
| Connect a repo to Linear            | [`init-linear`](../reference/skills/integrations/linear/init-linear.md)                       |
| Look up a Jira issue                | [`show-jira-issue`](../reference/skills/integrations/jira/show-jira-issue.md)                 |
| Search Jira issues                  | [`search-jira-issues`](../reference/skills/integrations/jira/search-jira-issues.md)           |
| Look up a Linear issue              | [`show-linear-issue`](../reference/skills/integrations/linear/show-linear-issue.md)           |
| Search Linear issues                | [`search-linear-issues`](../reference/skills/integrations/linear/search-linear-issues.md)     |

## Design and visualisation

| I want to…                                      | Use                                                                              |
| ----------------------------------------------- | --------------------------------------------------------------------------------- |
| Inventory the design system in a frontend       | [`inventory-design`](../reference/skills/design/inventory-design.md)                      |
| Find gaps between design intent and the code    | [`analyse-design-gaps`](../reference/skills/design/analyse-design-gaps.md)                |
| Browse my `meta/` documents in a local web view | [`visualise`](../reference/skills/visualisation/visualise.md)                             |

## Configuring the plugin

| I want to…                                    | Use                                                          |
| --------------------------------------------- | ------------------------------------------------------------ |
| Prepare a repo for Accelerator                | [`init`](../reference/skills/config/init.md)                         |
| Create or edit project configuration          | [`configure`](../reference/skills/config/configure.md)               |
| Apply pending schema migrations after upgrade | [`migrate`](../reference/skills/config/migrate.md)                   |

See the [Configuration cookbook](configuration-cookbook.md) for ready-made
recipes and the [Configuration](../configuration.md) reference for the
full key list.
