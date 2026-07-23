---
title: "Case Study: Shipping the Docs Site"
description: >-
  A real feature taken end to end with Accelerator — work item, research,
  decision record, plan, and implementation, with genuine meta/ excerpts.
---

The best way to understand the [phase model](philosophy.md) is to watch
it ship something real. This page follows one feature from
Accelerator's own history — **work item 0177, standing up this very
documentation site** — through the workflow, with genuine (trimmed)
excerpts from the artefacts each phase left in the repository's `meta/`
directory. Nothing below is mocked up; every excerpt is taken from a
file the workflow actually produced.

## 1. Capture: a work item

Work begins as a document, not a conversation. `create-work-item`
(here via a refinement pass, hence `producer: refine-work-item`) wrote
`meta/work/0177-documentation-site-for-docs-tree.md`:

```markdown
---
type: work-item
id: "0177"
title: "Stand up a documentation site for the docs/ tree"
producer: refine-work-item
status: draft
kind: story
parent: "work-item:0178"
external_id: PP-699
---

## Summary

Select a documentation-site generator and wire publishing so the
`docs/` tree produced by 0175 and 0176 is built and served as a
browsable site.

## Acceptance Criteria

- [ ] `mise run docs:build` builds the site from the `docs/` tree with
      zero errors and zero broken-link findings (via
      `starlight-links-validator`).
- [ ] A CI job in `.github/workflows/main.yml` publishes the built site
      to GitHub Pages on push to `main` …
```

The frontmatter is doing real work: `parent` ties the story to its
epic, `external_id` ties it to a synced Linear issue, and `status`
drives the visualiser's Kanban board. Acceptance criteria are written
as verifiable statements — they become the plan's success criteria
later.

## 2. Research: understand before planning

`research-codebase` spawned locator and analyser agents across the
build system, CI workflow, and docs tree, then distilled what came
back into
`meta/research/codebase/2026-07-10-0177-documentation-site-for-docs-tree.md`:

```markdown
---
type: codebase-research
id: "2026-07-10-0177-documentation-site-for-docs-tree"
producer: research-codebase
status: complete
work_item_id: "0177"
revision: "012ec6fb8934c494ab3b9ecbc606df342273189a"
---

## Summary

The `docs/` tree is 17 plain-CommonMark pages (8 in `docs/`, 9 in
`docs/skills/`) with **no frontmatter anywhere**; every page carries a
hand-rolled prev/next footer ending in a `../README.md#documentation`
link, and **all internal cross-links use `.md` extensions with
anchors**, which break under Starlight's extensionless slugs unless
rewritten. …

The build system is uniform and welcoming: mise tasks are thin
`invoke <ns>.<task>` wrappers; a new `tasks/docs.py` is one import +
one `add_collection` line … CI is a single workflow whose push trigger
only fires on `main` … **No Pages plumbing exists**.
```

Note what the document is: conclusions with file-and-line evidence,
pinned to a git `revision` so its claims can be re-verified. The
fifty-odd files read to produce it never reach the next phase — this
summary does. That is the [context-rot defence](philosophy.md) in
practice.

## 3. Decide: an ADR for the reversible-until-it-isn't choice

Choosing the site generator was an architectural decision worth
recording, so `create-adr` captured it as
`meta/decisions/ADR-0056-astro-starlight-for-documentation-site.md`:

```markdown
---
type: adr
id: "ADR-0056"
title: "Astro Starlight for the Documentation Site"
status: accepted
parent: "work-item:0177"
---

## Decision Drivers

- React + MDX authoring capability (React expertise already in-house)
- Build-time broken-link validation (currently unenforced)
- First-class static output for GitHub Pages, with search and dark
  mode working without external services
- Low maintenance burden and healthy upstream …

## Considered Options

1. **Astro Starlight** — Astro's docs theme; MDX with React via islands
2. **Docusaurus** — Meta's natively-React MDX docs framework
3. **Nextra 4** — Next.js App Router docs framework
4. **VitePress** — Vue team's Vite-based docs generator
5. **mkdocs-material** — Python; the historical default for md trees
```

Accepted ADRs are immutable — they can only be superseded or
deprecated — so the rationale survives even if the decision is later
reversed.

## 4. Plan: phased, reviewed, and grounded in the research

`create-plan` read the work item, the research document, and the ADR —
its `derived_from` frontmatter records exactly which — and produced
`meta/plans/2026-07-10-0177-documentation-site-for-docs-tree.md`:

```markdown
---
type: plan
id: "2026-07-10-0177-documentation-site-for-docs-tree"
producer: create-plan
status: ready
work_item_id: "work-item:0177"
derived_from:
  ["codebase-research:2026-07-10-0177-documentation-site-for-docs-tree"]
relates_to: ["adr:ADR-0056"]
---

## Overview

Stand up an Astro Starlight documentation site in `docs-site/`,
relocating the 17 pages of the `docs/` tree into
`docs-site/src/content/docs/`, wire it into the mise/invoke build
system as `docs:*` tasks, and publish it to GitHub Pages on push to
`main` from `.github/workflows/main.yml`.

### Decisions taken at planning (with the user, 2026-07-10)

- **Scaffold**: relocate pages into `docs-site/src/content/docs/` …
- **Mermaid**: `rehype-mermaid` at build time (`img-svg` + `dark`
  strategies; requires Playwright Chromium in the docs build).
- **Links**: keep `.md`-style relative links in source, resolved at
  build by `astro-rehype-relative-markdown-links` …
```

The plan is interactive by design — the "Decisions taken at planning"
section records choices made *with* the user before a line of code was
written, which is the cheapest possible moment to change course. The
plan's three phases (site scaffold and content migration → build-system
integration → CI and publishing) each end in explicit success criteria
with checkboxes.

## 5. Implement, review, land

`implement-plan` executed the plan phase by phase, ticking success
criteria as each was verified. Because the plan is a checklist on
disk, the implementation could have been interrupted at any point and
resumed in a fresh session from the first unchecked item.

The resulting commit trail maps almost one-to-one onto the plan's
phases:

```text
ad4cf3e7 Scaffold Astro Starlight docs site and relocate the docs/ tree
84ade459 Wire the docs site into the mise/invoke build system
718986da Repoint shell drift guards at the relocated docs tree
afc50c0c Publish the docs site to GitHub Pages from CI
b6a4e0a3 Allow forcing a docs deploy by pushing force-deploy-docs
```

From there the [VCS & PR skills](skills/vcs-and-pr.md) close the loop:
`commit` groups the working tree into atomic commits (proposing the
grouping and messages for approval first), `describe-pr` generates the
pull-request description from the plan and diff, and `review-pr` /
`respond-to-pr` handle the review cycle.

## What the trail is worth

Six months later, every question about this feature has a findable
answer: *why Starlight and not Docusaurus?* — ADR-0056. *What was true
of the codebase when this was designed?* — the research document,
pinned to a commit. *What was the intended scope?* — the work item's
acceptance criteria. *What was actually decided during planning?* —
the plan's decision log. None of it lives in anyone's chat history.

That trail is also machine-readable: the frontmatter links
(`parent`, `derived_from`, `relates_to`) form a graph the
[visualiser](visualiser.md) renders as clustered lifecycle timelines —
the 0177 cluster shows this exact chain, from work item to plan, as a
single connected story.
