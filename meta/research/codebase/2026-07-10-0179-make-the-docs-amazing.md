---
type: codebase-research
id: "2026-07-10-0179-make-the-docs-amazing"
title: "Research: Make the Docs Amazing (0179) — docs site overhaul groundwork"
date: "2026-07-10T19:55:16+00:00"
author: Phil Helm
producer: research-codebase
status: complete
work_item_id: "0179"
parent: "work-item:0179"
relates_to: ["codebase-research:2026-07-10-0177-documentation-site-for-docs-tree"]
topic: "Docs site overhaul: generated per-skill reference, tutorials, splash landing, sidebar restructure, drift guards"
tags: [research, codebase, docs-site, starlight, skills, tasks, ci]
revision: "150f34d0b4e921942076d43ba726b137607ace1b"
repository: "barcelona"
last_updated: "2026-07-10T19:55:16+00:00"
last_updated_by: Phil Helm
schema_version: 1
---

# Research: Make the Docs Amazing (0179) — docs site overhaul groundwork

**Date**: 2026-07-10T19:55:16+00:00 (UTC)
**Author**: Phil Helm
**Git Commit**: 150f34d0b4e921942076d43ba726b137607ace1b
**Branch**: docs/0177-documentation-site
**Repository**: barcelona (accelerator)

## Research Question

What does the codebase look like today for implementing work item 0179 —
overhauling the Astro Starlight docs site with generated per-skill
reference pages, new tutorial/how-to content, a splash landing page, a
restructured sidebar, Starlight polish, and CI drift guards?

## Summary

The site the story builds on lives in **`docs-site/`** (not `docs/` — the
old tree was removed by 0177): 18 pages, ~1,517 lines, Astro 6 +
Starlight ^0.40, a fully **manual** sidebar in `astro.config.mjs`, mermaid
via `rehype-mermaid` (img-svg, dark variant, needs Playwright Chromium),
and `starlight-links-validator`. None of the polish items exist yet: no
`lastUpdated`, `editLink`, `social`, `favicon`, `logo`, or og `head`
config, and `index.md` uses the default doc template, not splash.

Key correction to the work item: there are **69 real skills, not ~82**
(stray SKILL.md files under `frontend/node_modules/` and migration test
fixtures inflate naive counts and must be excluded by any generator).
`plugin.json` registers skills as **14 directory globs**, not individual
entries, so both the generator and the drift guard must walk directories.
Of the 69, ~23 carry `user-invocable: false` (18 review lenses, 3
output-formats, `browser-executor`, `paths`) — the reference must decide
how to badge/segregate them.

The generation script has an obvious, cheap home: `tasks/docs.py`
(invoke picks new tasks up automatically), path constants in
`tasks/shared/paths.py`, wired as a mise `depends` of `docs:build`, which
is already exercised by both the PR gate (`check-docs`) and the deploy
job. The version-coherence check in `tasks/build.py` is a direct template
for the "every skill has a page" drift guard. One gap: **no Python YAML
parser exists in `tasks/`** — SKILL.md frontmatter parsing today is
shell-only, so a YAML dependency (or minimal parser) is needed.

## Detailed Findings

### Current docs site (`docs-site/`)

- **Config** — `docs-site/astro.config.mjs`: `site:
  'https://atomicinnovation.github.io'` (line 8), `base: '/accelerator'`
  (line 9), title (line 22), custom CSS (line 23),
  `starlight-links-validator` with `errorOnRelativeLinks: false`
  (line 24). Mermaid is at the Astro markdown level: `rehype-mermaid`
  `{ strategy: 'img-svg', dark: true }` (line 13) with Shiki excluding
  the `mermaid` lang (line 11); relative `.md` links are resolved by
  `astro-rehype-relative-markdown-links` `{ base: '/accelerator',
  collectionBase: false }` (lines 14–17).
- **Sidebar** — fully manual (lines 25–51): eight top-level slugs plus a
  "Skills Reference" group of nine pages; label overrides at lines 28 and
  38–41 disambiguate the two "Development Loop" pages. 0179's Start Here
  / Guides / Reference restructure means rewriting this array; per-page
  `sidebar.*` frontmatter is used nowhere yet.
- **Pages** — 18 files. Top level: `index.md` (39 lines, doc template,
  `slug: ''`, hand-rolled light/dark `<img>` pairs), `philosophy.md`
  (24 lines — confirmed thin), `workflow.md` (103), `development-loop.md`
  (47), `visualiser.md` (95), `internals.md` (88), `configuration.md`
  (152), `migrations.md` (43), `releases-and-compatibility.md` (41).
  `skills/`: `index.md` "All Skills" (100) plus eight family pages;
  `issue-trackers.md` (280) is the longest. Frontmatter is minimal and
  uniform: `title:` everywhere, `description:` on three pages only, no
  `template`, `hero`, `sidebar.*`, or `head` anywhere.
- **Missing polish** — no `lastUpdated`, `editLink`, `social`, `favicon`,
  `logo`, or og metadata in the config; only one mermaid diagram exists
  site-wide; assets are 4 PNGs in `docs-site/public/` and there is no
  `src/components/` directory.
- **Content schema** — `docs-site/src/content.config.ts:5-7` is stock
  `docsLoader()` + `docsSchema()`, no extensions.

### Generation source material (skills, agents, plugin.json)

- **Registration** — `.claude-plugin/plugin.json:13-28` lists **14
  directory globs** (e.g. `./skills/vcs/`, `./skills/review/lenses/`),
  not individual skills. Agents are not registered there at all — they
  are auto-discovered from `agents/`.
- **Skill count** — **69 skills** across 14 categories: review/lenses 18,
  integrations/jira 8, integrations/linear 8, work 8, planning 5,
  config 5, github 3, research 3, decisions 3, review/output-formats 3,
  design 2, vcs 1, visualisation 1, notes 1. Stray SKILL.md files under
  `skills/visualisation/visualise/frontend/node_modules/**` and
  `skills/config/migrate/scripts/test-fixtures/**` **must be excluded**.
- **Frontmatter fields** — `name`, `description`, `argument-hint`,
  `allowed-tools`, plus optional `disable-model-invocation` and
  `user-invocable: false` (all 18 lenses, 3 output-formats,
  `skills/config/browser-executor/SKILL.md:7`,
  `skills/config/paths/SKILL.md:6`). Descriptions can be folded
  multi-line YAML — a naive line-based parser breaks; use a real YAML
  parser.
- **Rendering hazards for SKILL.md bodies** (a generator must
  strip/escape, never execute):
  1. `!` preprocessor lines — whole-line (`skills/vcs/commit/SKILL.md:
     12-15,66`) **and inline** mid-sentence forms
     (`skills/planning/create-plan/SKILL.md:22-23`).
  2. `${CLAUDE_PLUGIN_ROOT}` in frontmatter `allowed-tools` and bodies.
  3. Angle-bracket placeholders outside fences (`<url>`,
     `<migration-id>` — e.g. `skills/config/migrate/SKILL.md:35-261`) —
     raw HTML to MD, hard failure in MDX; needs escaping.
  4. Curly-brace placeholders like `{codebase locator agent}`
     (`create-plan/SKILL.md:78-80`) — MDX would treat as JS expressions.
     Together with (3), this argues for generating **`.md`, not `.mdx`**.
- **Agents** — 9 files in `agents/*.md` with `name`, `description`,
  `tools` (comma-separated string), optional `skills` list and `color`.
  The locator/analyser split is enforced via `tools`: locators have no
  Read (Grep/Glob/LS, or Bash for browser agents); analysers add Read.
  Pairs exist in three domains: codebase, documents, browser. This is
  exactly the material for the planned agents reference page.

### Build toolchain integration (`tasks/`, mise, CI)

- **Home for the generator** — `tasks/docs.py:1-17` (currently just
  `build` running `npm --prefix docs-site run build`). New `@task`
  functions are picked up by `Collection.from_module`
  (`tasks/__init__.py:59`) with no registration change.
- **mise wiring** — `mise.toml:79-91`: `docs:build` depends on
  `deps:install:docs-playwright` → `deps:install:docs`; `docs:check`
  just depends on `docs:build` (the strict build incl. link validation
  is the gate) and is in the top-level `check` roll-up
  (`mise.toml:474-476`). A prebuild generation task slots in as a new
  `depends` of `docs:build`, automatically exercised by both the PR gate
  and deploy.
- **Paths** — `tasks/shared/paths.py:5-27` already has `REPO_ROOT`,
  `DOCS_SITE`, `PLUGIN_JSON`; add `SKILLS_DIR` and the generated-content
  dir there.
- **Drift-guard template** — `tasks/build.py:184-210`
  (`validate_version_coherence`): collects a `found` dict, raises a
  domain error listing mismatches, takes `repo_root: Path | None` for
  test injection. Check-task conventions:
  `tasks/lint/vendor_shims.py:8-42` (`raise Exit("actionable message
  naming the fix command", code=1)`), read-only tasks end `:check`, one
  CI job = one mise task (`tasks/README.md:144-152`).
- **YAML gap** — SKILL.md frontmatter parsing exists only in shell
  (`scripts/frontmatter-emission-rules.sh`, `scripts/parse-frontmatter*`,
  tested by `scripts/test-skill-frontmatter-conformance.sh`); no yaml
  library is imported anywhere in `tasks/` — check `pyproject.toml`
  before adding one. Atomic writes via
  `tasks/shared/files.py:4-11` (`atomic_write_text`).
- **Tests** — `tests/conftest.py:11-17` `fake_repo_tree` builds a minimal
  repo (extend with `skills/*/SKILL.md`); `tests/unit/tasks/
  test_version.py:18-63` shows the shape (MagicMock Context,
  `mocker.patch.object` on module path constants, assert written files).
  Toolchain is pyrefly-strict + ruff-ALL; tests are relaxed.
- **CI** — single `.github/workflows/main.yml`: `check-docs` (line 343,
  `mise run docs:check`) and `deploy-docs` (line 361, `needs:
  check-docs`, `if: push`, uploads `docs-site/dist`); `force-deploy-docs`
  branch push is the manual redeploy escape hatch (lines 7–9). **Both
  checkouts are default shallow (depth 1)** — enabling `lastUpdated`
  requires adding `fetch-depth: 0`. CI topology is guarded by
  `tests/unit/tasks/test_workflows.py` (name-agnostic invariants; never
  touch the `accelerator-release` concurrency group); actionlint
  hardcodes `main.yml` (`tasks/lint/workflows.py:7`).

## Code References

- `docs-site/astro.config.mjs:8-51` — site/base, plugins, manual sidebar
- `docs-site/src/content/docs/index.md:1-6` — landing page (doc
  template, `slug: ''`)
- `docs-site/src/content.config.ts:5-7` — stock content collection
- `docs-site/package.json:6-18` — scripts (no prebuild) and deps
- `.claude-plugin/plugin.json:13-28` — skills as directory globs
- `skills/vcs/commit/SKILL.md:12-15` — whole-line `!` preprocessor
- `skills/planning/create-plan/SKILL.md:22-23,78-80` — inline `!` and
  curly-brace placeholders
- `agents/*.md` — 9 agents; locator/analyser split via `tools`
- `tasks/docs.py:1-17` — existing docs tasks (generator home)
- `tasks/__init__.py:59` — collection registration
- `tasks/shared/paths.py:5-27` — path constants
- `tasks/build.py:184-210` — coherence-check template for drift guard
- `tasks/lint/vendor_shims.py:8-42` — check-task failure-message pattern
- `mise.toml:79-91,474-476` — docs task wiring and check roll-up
- `.github/workflows/main.yml:7-9,343-398` — docs CI jobs, shallow
  checkouts, force-deploy branch
- `tests/conftest.py:11-17`, `tests/unit/tasks/test_version.py:18-63` —
  test fixtures/shape

## Architecture Insights

- **Generate `.md`, not `.mdx`.** ADR-0056 chose Starlight partly so
  plain `.md` stays real CommonMark; SKILL.md bodies contain
  angle-bracket and curly-brace placeholders that would hard-fail MDX.
  MDX/React islands remain sanctioned-but-unused, available for the
  splash page and tutorial polish.
- **Prebuild generation in `tasks/` is the confirmed fit** (matches the
  work item's assumption): the strict `docs:build` is both the PR gate
  and the deploy build, so a generator wired as a mise dependency is
  automatically drift-checked in CI. The drift guard can be either a
  determinism check (regenerate + diff) or a coverage check modelled on
  `validate_version_coherence`.
- **The family pages decision reverses 0176.** 0176 deliberately chose
  "no standalone per-skill pages"; 0179 keeps the eight family pages as
  curated overviews linking into generated pages — expect to rework the
  `skills/index.md` "All Skills" relationship, not just add pages.
- **Base-path discipline**: markdown links are handled by
  `astro-rehype-relative-markdown-links`, but `hero.actions` links and
  any hand-written absolute links must be manually prefixed with
  `/accelerator`.
- **Sidebar for ~69 generated pages** should use per-page
  `sidebar.label`/`sidebar.order`/`sidebar.badge` frontmatter plus
  autogenerated collapsed groups, keeping `astro.config.mjs` small —
  nothing in the current site uses these yet, so there is no convention
  to conflict with.
- **Non-invokable skills need a policy**: 23 of 69 skills are
  `user-invocable: false` internals (lenses, output-formats,
  browser-executor, paths). The 0178 epic AC says "every skill in
  plugin.json has a page" — badge them (e.g. `sidebar.badge:
  Internal`) or group them separately rather than omitting them.
- **Starlight is pre-1.0** — minor releases can break; the site is on
  ^0.40 while the 0177 plan mentions ^0.41. Pin-and-watch on upgrade.

## Historical Context

- `meta/decisions/ADR-0056-astro-starlight-for-documentation-site.md` —
  Starlight rationale and rejected alternatives (don't relitigate).
- `meta/plans/2026-07-10-0177-documentation-site-for-docs-tree.md` — what
  0177 built; its "What We're NOT Doing" list is effectively 0179's
  menu (no content rewrites, no theming, no MDX, no CI caching).
- `meta/research/codebase/2026-07-10-0177-documentation-site-for-docs-tree.md`
  — anchor fragility in headings with raw `<img>`, Chromium ~150MB per
  CI run uncached (optimisation candidate as builds grow).
- `meta/work/0176-per-skill-family-reference-docs.md` — the family-pages
  model 0179 supersedes; its deferred open question (generate the index
  from frontmatter with a CI consistency check) is 0179's drift-guard
  mandate.
- `meta/work/0178-documentation.md` — the audience framing; Diátaxis is
  an influence, not a mandate; prefers generation + drift guards over
  hand-maintained duplication. Unresolved: whether 0145's in-progress
  content is absorbed into 0178's children.
- `meta/work/0145-documentation-improvements.md` — earlier docs epic;
  its IA questions are effectively superseded by 0178/0179.

## Related Research

- `meta/research/codebase/2026-07-10-0177-documentation-site-for-docs-tree.md`
- `meta/research/codebase/2026-06-29-0176-workflows-rename-and-skill-catalogue.md`

## Open Questions

All resolved with the user on 2026-07-10 (recorded in work item 0179's
Assumptions):

- **Generated pages are build-time only** (gitignored output in the
  content collection); the drift guard reduces to a coverage check in
  the generator.
- **`user-invocable: false` skills** get pages in a separate collapsed
  "Internal" sidebar group with an Internal badge.
- **Family pages become curated overviews**; the hand-written "All
  Skills" index is replaced by a generated index.
- **PyYAML** (`safe_load`, version-pinned) for frontmatter parsing.
- The work item's "~82 skills" figure was corrected to 69.
