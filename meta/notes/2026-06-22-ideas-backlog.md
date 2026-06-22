---
type: note
id: "2026-06-22-ideas-backlog"
title: "Ideas Backlog"
date: "2026-06-22T23:49:56+00:00"
author: "Toby Clemson"
producer: create-note
status: captured
topic: "Ideas backlog"
tags: [backlog, ideas]
revision: "59c765f5533a34285f086d31f5190fbb2689082e"
repository: "miscellaneous"
last_updated: "2026-06-22T23:49:56+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Ideas Backlog

Running backlog of feature, skill, and infrastructure ideas for the
Accelerator plugin. Captured from `stories.md`. Many of these have been
extracted into structured work items in `meta/work/`.

* Make the visualiser UI responsive and more appropriate for larger screens
* Migrate shell scripts into Rust CLI
* Graph based representation of knowledge base
* Spike incorporating graphify with an agent similar to codebase-locator
* Add configuration to visualiser frontend
* Allow selecting jj workspace / git worktree in visualiser
* Spike how to track changes to artefacts so that we can highlight change as it is happening / scan through artifact history
* Add `/review-codebase` command
* Consider adding a refactoring workflow, driven by static analyses and code quality metrics
* Improve documentation
  * Break up README.md into smaller docs
  * Build documentation site
* Add per-user configuration of additional skill context and instructions
* Add auto-mode to all relevant skills
* Add auto-lens selection to all review skills (all, auto, manual, confirm, list)
* Consider adding effort control to review skills
* Determine a caching strategy for skills that use `token_cmd` type configuration
* Convert all artefact references to links (both in frontmatter and markdown) before rendering in visualiser
* Allow sorting by column headers in visualiser lists
* Consider adding simplicity / elegance lenses for story / plan / pr review
* Investigate how to retain edit deltas for artefacts
* Investigate how to allow comments against artefacts
* Add status mapping to work item synchronisation
* Add kind mapping to work item synchronisation
* Add priority mapping to work item synchronisation
* Establish parent-child relationships between work items on synchronisation
* Restrict work item sync based on configured labels or projects
* Use same logic as `/update-work-item N status <status>` in kanban board
* Get skills to prompt to update artefact statuses (e.g., work item status, review status, plan status) upon completion
* Normalise statuses and status formats across artefacts
* Rename the GitHub skill groups to collaboration
* Move GitHub specific interactions into an integration and decouple collaboration skills
* Add more collaboration skills
  * Add `/open-pr` skill
  * Add `/merge-pr` skill
  * Add `/list-prs` skill, with `--mine`
  * Add `/open-pr-stack` skill (or maybe `/open-prs --stacked`, maybe with `--describe` to bundle describing on open)
  * Add `/describe-pr-stack` skill (or maybe `/describe-prs --stacked`)
  * Add `/respond-to-pr-stack` skill (or maybe `/respond-to-prs --stacked`)
  * Add `/review-prs-stack` skill (or maybe `/review-prs --stacked`)
* Add more VCS skills
  * Add `/resolve-conflicts` skill to VCS skill group
* Add more work management skills
  * Add `/create-milestone-plan` skill
  * Add `/create-release-plan` skill
* Add product management / strategy skills
  * Add `/capture-product-vision` skill
  * Add `/create-strategy-document` skill
  * Add `/define-user-personas` skill
  * Add `/analyse-market` skill (maybe include customer segmentation here)
  * Add `/analyse-competitor` skill
  * Add `/analyse-jobs-to-be-done` skill
  * Add `/create-lean-canvas` skill
  * Add `/map-customer-journey` skill
  * Add `/create-problem-statement` skill
  * Add `/review-problem-statement` skill
  * Add `/create-hypothesis` skill
  * Add `/explore-solutions` / `generate-ideas` skill
  * Metrics definition skills? Metrics analysis skills?
* Add prototyping skills
  * Add `/create-prototype-brief` skill
  * Add `/build-prototype` skill (with `--mode=wireframes|interactive`)
