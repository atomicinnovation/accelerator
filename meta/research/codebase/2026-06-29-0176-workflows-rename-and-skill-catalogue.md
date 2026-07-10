---
type: codebase-research
id: "2026-06-29-0176-workflows-rename-and-skill-catalogue"
title: "Research: Renaming skill-family pages to \"workflows\", a master skill catalogue, and per-skill pages"
date: "2026-06-29T15:01:34+00:00"
author: Phil Helm
producer: research-codebase
status: complete
work_item_id: "0176"
parent: "work-item:0176"
relates_to: ["codebase-research:2026-06-29-0175-slim-readme-split-docs-tree"]
topic: "Skill-family page naming, a master user-invokable skill list, and per-skill reference pages"
tags: [research, codebase, docs, skills, information-architecture, naming]
revision: "7c12519d1173e25c10c85e22dd1f60b4916d5434"
repository: "barcelona"
last_updated: "2026-06-29T15:01:34+00:00"
last_updated_by: Phil Helm
schema_version: 1
---

# Research: Renaming skill-family pages to "workflows", a master skill catalogue, and per-skill pages

**Date**: 2026-06-29T15:01:34+00:00
**Author**: Phil Helm
**Git Commit**: 7c12519d1173e25c10c85e22dd1f60b4916d5434
**Branch**: docs/0175-slim-readme-split-docs-tree
**Repository**: barcelona

## Research Question

In the context of work item 0176 (per-skill-family reference docs under `docs/`):

1. Should the current "skill family" pages be renamed to something like
   **"workflows"**?
2. Where should a **list of every user-invokable skill** live, and what does
   the complete set look like?
3. Should there be **a page for each skill** explaining how to use it, what it
   does, with room for skill-specific advice and guidelines?

## Summary

**The biggest finding up front:** the `docs/` and `docs/skills/` tree that
0176 describes building **already exists and is fully populated in this
checkout**. 0175 is marked done, and the seven per-family pages are on disk
with their skills tables. So all three questions are about *evolving* a
structure that is already in place, not designing from scratch. (0176's body
still references the pre-split README line ranges, which no longer exist — the
work item text is stale relative to the tree.)

Answers in brief:

1. **Renaming to "workflows" is not recommended as a blanket label.** The word
   "workflow" is *already* used throughout the docs for the phase *process*
   (the research→plan→implement loop) and in a page title (`VCS & PR
   Workflow`). Reusing it for the skill *groupings* would overload the term.
   Also, only some families are genuinely workflow-shaped (VCS/PR, design
   convergence, review, the planning loop); others are flat collections
   (work-items, issue-trackers, ADRs). "Workflows" fits the subset, not the
   whole. There is currently **no** user-facing collective noun — the README
   just uses a bold **Skills** heading; "skill family" is internal
   project-management vocabulary only.
2. **No master skill catalogue exists today.** Skills are listed only
   per-family across the seven `docs/skills/` pages. A single "All Skills"
   index page (or a table in the README) would be net-new and also directly
   serves 0177 (the docs-site navigation). The complete user-invokable set is
   **46 skills** across 12 families (full table below).
3. **Pages are currently per-family, not per-skill.** Going per-skill means ~46
   new pages — a large expansion with real maintenance cost and a drift risk
   against each skill's `SKILL.md` frontmatter (which already holds the
   canonical description + `argument-hint`). A hybrid (master index + richer
   per-skill subsections within family pages, reserved full pages only for the
   heaviest skills) is the lower-risk middle path.

## Detailed Findings

### Current documentation structure (already post-split)

- `README.md` is now an 85-line thin index. Only three H2s remain: `Getting
  Started`, `Documentation`, `License`. The `Documentation` section carries two
  bulleted link lists under bold labels **Concepts** and **Skills**
  (`README.md:48`, `README.md:63`). No skills tables remain in the README.
- `docs/` (7 concept pages): `philosophy.md`, `development-loop.md`,
  `configuration.md`, `visualiser.md`, `internals.md`, `migrations.md`,
  `releases-and-compatibility.md`.
- `docs/skills/` (7 family pages, already populated):
  - `planning.md` — "Planning" — companion + plan-support skills (prose bullets,
    no table)
  - `work-items.md` — "Work Items" — 6 skills, table + ASCII lifecycle
  - `issue-trackers.md` — "Issue Trackers (Jira & Linear)" — biggest page; two
    8-skill tables (~144 lines)
  - `adrs.md` — "Architecture Decision Records (ADRs)" — 3 skills, table + diagram
  - `vcs-and-pr.md` — "VCS & PR Workflow" — 4 skills, table only (~12 lines)
  - `review-system.md` — "Review System" — no skills table; two lens tables
  - `design-convergence.md` — "Design Convergence" — 2 skills, table + diagram +
    runtime subsections (~129 lines)

**Implication for 0176:** the work item reads as "not started" (status draft,
references stale README line ranges), but the deliverable largely exists. Worth
reconciling the work item against the tree before treating this as greenfield.

### Cross-linking convention

Consistent rule: **link text equals the destination page's H1 title.** The
README "Concepts" and "Skills" lists both follow it
(`README.md:50-61`, `README.md:65-77`), as do inter-doc links
(`development-loop.md:25` → `[Planning](skills/planning.md)`;
`configuration.md:100` → `[Review System](skills/review-system.md)`, which
already satisfies 0176's custom-lens linkage AC). Any new index/per-skill pages
should keep this convention.

### Question 1 — "skill family" → "workflows"?

- `"skill family"` / `"skill-family"` appears **nowhere** in the README or any
  `docs/` page. It lives only in `meta/` work items and plans
  (`meta/work/0176-...md`, `meta/work/0175-...md`, the 0145 epic). It is
  internal vocabulary.
- `"workflow"` is already in user-facing use for **processes**, not groupings:
  - `development-loop.md:3` — "The primary workflow is a three-phase loop"
  - `philosophy.md:10` — "a development workflow where"
  - `vcs-and-pr.md:1` / `:4` — H1 "VCS & PR Workflow"
  - `design-convergence.md:6` — "the workflow plugs straight into…"
- `"category"` appears once, unrelated (`visualiser.md:15`, a visualiser doc
  category).

**Assessment.** Renaming all seven pages to "workflows" collides with the
established process-meaning and with the `VCS & PR Workflow` title. The families
are not uniformly workflow-shaped: VCS/PR, design convergence, review, and the
planning loop are sequenced processes; work-items, issue-trackers, and ADRs are
flat skill collections. A single label that fits both does not exist in current
usage. Options, in rough order of recommendation: (a) keep the bold **Skills**
grouping and give the section an explicit collective noun like "Skill
reference" / "Skills by area"; (b) call them "skill areas" or "skill
categories"; (c) reserve "workflow" for the genuinely sequenced pages only.

### Question 2 — a master list of every user-invokable skill

**No master catalogue exists.** Skills are enumerated only per-family. Closest
cross-cutting views: the README "Skills" link list (7 pages, not skills) and the
`docs/internals.md:14-27` meta-directory table (by output path, not a skill
list). `.claude-plugin/plugin.json` is the machine-readable manifest but
registers **14 directory roots**, not individual skills — Claude Code discovers
every `SKILL.md` beneath each root.

**The complete user-invokable set: 46 skills** (visible in the `/` menu, i.e.
not carrying `user-invocable: false`). Note the two invocation-control signals:

- `user-invocable: false` → hidden from the `/` menu (preloaded-by-agent only).
  **Excludes** a skill from the user-invokable set.
- `disable-model-invocation: true` → stays in the `/` menu (a human can type it)
  but Claude won't auto-trigger it. **Still user-invokable.**

| Family | Count | Skills |
|---|---|---|
| vcs | 1 | commit |
| github | 3 | describe-pr, review-pr, respond-to-pr |
| planning | 5 | create-plan, implement-plan, validate-plan, review-plan, stress-test-plan |
| research | 3 | research-codebase, research-issue, conduct-spike |
| decisions | 3 | create-adr, review-adr, extract-adrs |
| work | 8 | create-work-item, update-work-item, refine-work-item, review-work-item, stress-test-work-item, extract-work-items, sync-work-items, list-work-items |
| design | 2 | inventory-design, analyse-design-gaps *(both `disable-model-invocation: true`)* |
| notes | 1 | create-note |
| visualisation | 1 | visualise *(`disable-model-invocation: true`)* |
| config | 3 | configure, init, migrate *(configure & init are `disable-model-invocation: true`; migrate has no flag)* |
| integrations/jira | 8 | init-jira, create-jira-issue, update-jira-issue, transition-jira-issue, comment-jira-issue, attach-jira-issue, show-jira-issue, search-jira-issues |
| integrations/linear | 8 | init-linear, create-linear-issue, update-linear-issue, transition-linear-issue, comment-linear-issue, attach-linear-issue, show-linear-issue, search-linear-issues |
| **Total** | **46** | |

**Not user-invokable (23, `user-invocable: false`)** — excluded from the list a
user would browse:

- config 2: `browser-executor`, `paths` (preloaded by agent definitions)
- review/lenses 18: every `*-lens` (lens specifications loaded by review
  orchestrators)
- review/output-formats 3: `plan-review-output-format`,
  `pr-review-output-format`, `work-item-review-output-format`

Total shipped Accelerator skills: **69** (= 46 user-invokable + 23 internal).

**Where a catalogue should live.** A `docs/skills/index.md` titled e.g. "All
Skills" (or a "Skill index" table appended to the README "Skills" section) is
the natural home, grouped by the same 12 families, each row linking to its
family page (and later to a per-skill anchor/page). This also seeds 0177's
navigation directly.

### Question 3 — a page per skill

Current pages are per-**family** (7 pages, multiple skills each). Per-skill
would be ~46 pages.

- **For:** maximum discoverability; a stable URL per skill; a dedicated home for
  skill-specific advice/guidelines that doesn't fit in a frontmatter
  description.
- **Against:** ~46 pages to maintain; high duplication risk against each
  `SKILL.md`'s `name` / `description` / `argument-hint` (the canonical source),
  which will drift; many skills are thin (e.g. `create-note`, `list-work-items`)
  and don't warrant a full page; the integration families are near-identical
  Jira/Linear pairs that read better side-by-side than as 16 separate pages.
- **Hybrid (recommended):** keep family pages; add the master index; within each
  family page give each skill a consistent subsection (What it does / How to use
  it / Advice & guidelines) anchored for deep-linking; reserve standalone pages
  only for the heaviest skills (e.g. `sync-work-items`, `inventory-design`,
  `review-pr`). This delivers the per-skill guidance the question wants without
  46-page maintenance or SKILL.md duplication.

## Code References

- `README.md:46-80` — the thin `Documentation` index (Concepts + Skills link lists)
- `.claude-plugin/plugin.json:13-28` — 14 registered skill directory roots
- `docs/skills/` — the seven already-populated per-family pages
- `docs/configuration.md:100` — existing `[Review System](skills/review-system.md)` link (satisfies 0176 custom-lens AC)
- `docs/internals.md:14-27` — meta-directory table (closest existing cross-cutting view)
- `skills/config/browser-executor/SKILL.md:7`, `skills/config/paths/SKILL.md:6` — maintainer notes explaining `user-invocable: false` vs `disable-model-invocation`

## Architecture Insights

- **`SKILL.md` frontmatter is the canonical skill metadata** (`name`,
  `description`, `argument-hint`, `allowed-tools`). Any catalogue or per-skill
  page should treat it as the single source of truth — ideally generated from
  it rather than hand-duplicated, to avoid drift (there is no automated
  consistency check today, mirroring the line-width duplication gotcha in
  CLAUDE.md).
- **"Workflow" is a load-bearing process term** in the docs already; reusing it
  for groupings would dilute the phase-model vocabulary the project deliberately
  established (see `docs/philosophy.md`, `docs/development-loop.md`).
- **Two-axis invocability** (`user-invocable` × `disable-model-invocation`)
  means "user-invokable" is a precise, derivable set — a generated catalogue can
  filter on `user-invocable != false` reliably.

## Historical Context

- `meta/work/0145-documentation-improvements.md` — parent epic; holds the
  authoritative "Target information architecture (agreed 2026-06-29)" with the
  full `docs/` + `docs/skills/` layout. The IA decision lives here inline; there
  is **no** standalone ADR for docs IA or skill-grouping terminology.
- `meta/work/0175-slim-readme-and-split-into-docs-tree.md` — **done**;
  established the `docs/` tree and README index.
- `meta/work/0177-documentation-site-for-docs-tree.md` — **draft**; the docs-site
  nav must expose every page 0176 produces. A master skill index would directly
  feed it.
- `meta/research/codebase/2026-06-29-0175-slim-readme-split-docs-tree.md` —
  0175 research; section→destination line-range mapping (now stale vs the tree).
- `meta/plans/2026-03-15-readme-restructure.md` and
  `meta/work/0019-readme-structure.md` — earlier README-structure prior art.
- `meta/notes/2026-06-22-ideas-backlog.md` — seed entries "Break up README" and
  "Build documentation site" (→ 0175, 0177).

## Related Research

- `meta/research/codebase/2026-06-29-0175-slim-readme-split-docs-tree.md`

## Open Questions

- **Reconcile 0176 with reality:** the per-family pages already exist. Is 0176's
  remaining scope now "enrich the existing family pages + add a master index"
  rather than "create the pages"? The work item body needs updating (stale line
  ranges) regardless.
- **Collective noun decision:** if not "workflows", what is the user-facing term
  for a skill grouping — "skill areas", "categories", or just the bold "Skills"
  with no noun? This is a small naming ADR candidate.
- **Generate vs hand-write the catalogue:** given drift risk against
  `SKILL.md`, should the index (and per-skill stubs) be generated by a
  `tasks/` invoke task from frontmatter, with a CI check? This affects 0177 too.
- **Per-skill depth threshold:** which skills genuinely warrant standalone pages
  vs an anchored subsection? Candidates for full pages: `sync-work-items`,
  `inventory-design`, `analyse-design-gaps`, `review-pr`.
- **Integration pairing:** keep Jira/Linear as paired family pages, or split
  into 16 per-skill pages? The near-identical surface argues for pairing.
