---
type: work-item
id: "0177"
title: "Stand up a documentation site for the docs/ tree"
date: "2026-06-29T10:28:21+00:00"
author: Phil Helm
producer: refine-work-item
status: draft
kind: story
priority: medium
parent: "work-item:0178"
tags: []
last_updated: "2026-06-29T14:46:59+00:00"
last_updated_by: Phil Helm
schema_version: 1
external_id: PP-699
---

# 0177: Stand up a documentation site for the docs/ tree

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Phil Helm

## Summary

Select a documentation-site generator and wire publishing so the `docs/` tree
produced by 0175 and 0176 is built and served as a browsable site.

## Context

Child of 0145 — Documentation Improvements, satisfying the epic's second
requirement ("Build a documentation site"). No docs-site tooling exists today:
there is no mkdocs, Docusaurus, VitePress, or mdBook config, and no
docs-publishing job in `.github/workflows/main.yml`. This story adds the build
+ publish pipeline and resolves the epic's open tooling question. It serves
readers of the documentation — primarily new users and contributors browsing
the published `docs/` tree rather than reading raw Markdown in the repo.

The tooling-selection, hosting, and publishing-trigger decisions were resolved
at pickup (2026-07-10): Astro Starlight, recorded in ADR-0056, publishing to
GitHub Pages on push to `main`. See Technical Notes for the candidate
comparison and integration details.

## Requirements

- Build the site with Astro Starlight, per ADR-0056 (candidates evaluated:
  Starlight, Docusaurus, Nextra, VitePress, mkdocs-material, mdBook).
- Add the Starlight configuration and any required dependencies, including
  `starlight-links-validator` for build-time link checking.
- Adapt the `docs/` tree to Starlight: `title:` frontmatter on every page,
  remove the hand-rolled prev/next footers, rework links that escape `docs/`
  to the root README (see Technical Notes).
- Wire a GitHub Actions job publishing the built site to GitHub Pages on
  push to `main`.
- Ensure the site's navigation reflects the `docs/` tree structure (including
  the `docs/skills/` reference layer).

## Acceptance Criteria

- [ ] The generator selection (Astro Starlight) is recorded in an accepted
      ADR whose rationale weighs Starlight against Docusaurus, Nextra,
      VitePress, mkdocs-material, and mdBook under the project's constraints.
- [ ] `mise run docs:build` builds the site from the `docs/` tree with zero
      errors and zero broken-link findings (via `starlight-links-validator`).
- [ ] A CI job in `.github/workflows/main.yml` publishes the built site to
      GitHub Pages on push to `main`, and the published URL is documented in
      the README.
- [ ] The site navigation exposes every page produced by 0175 (its narrative
      pages and any sections it relocates) and 0176 (the `docs/skills/` reference
      pages), grouped to reflect the `docs/` tree structure — verified against
      those siblings' final Technical Notes mappings.

## Open Questions

All resolved at pickup (2026-07-10):

- Generator: Astro Starlight — rationale in Technical Notes and the
  accompanying ADR (per AC1).
- Hosting/trigger: GitHub Pages, publishing on push to `main`.
- No spike needed — selection was resolved by research during refinement.

## Dependencies

- Blocked by: 0175, 0176 (the `docs/` tree must exist to publish). 0177's
  navigation scope tracks the *final* page set those siblings produce —
  including the resolution of 0175's two unassigned README sections — not merely
  the tree's existence.
- External: GitHub Pages via GitHub Actions (confirmed per ADR-0056) — needs
  a one-time Pages-enablement repo setting and `pages: write` +
  `id-token: write` deploy permissions in the workflow.
- Related: 0145.

## Assumptions

- A static-site generator over a markdown source tree is sufficient; no
  dynamic/server-rendered docs platform is required.

## Technical Notes

### Generator decision: Astro Starlight (2026-07-10)

Candidates evaluated: Starlight, Docusaurus, Nextra, VitePress,
mkdocs-material, mdBook. Starlight selected because the team values
React + MDX authoring (React already in use for the visualiser
frontend) and accepts the frontmatter cost:

- **VitePress** — best plain-Markdown fit but Vue-based; custom work
  would sit oddly beside a React codebase.
- **mkdocs-material** — in maintenance mode since Nov 2025 (team moved
  to Zensical); poor bet for fresh adoption.
- **Docusaurus** — natively React but heaviest toolchain, no built-in
  search (Algolia/plugin), v4 migration imminent; full-React advantage
  buys little for ~17 pages.
- **Nextra 4** — React + MDX but couples to Next.js majors, has an open
  Pagefind-under-static-export bug (shuding/nextra#3987), no build-time
  link checking, and MDX-strict parsing of `.md` (raw `<img>`/`<br>`
  would break). Effectively single-maintainer.
- **mdBook** — hand-written SUMMARY.md nav, stale link-check plugin.

Starlight gives Pagefind search, dark mode, and an official GitHub
Pages action out of the box; React components embed in `.mdx` pages via
`@astrojs/react` islands. Accepted risks: pre-1.0 (breaking changes
between minors), link validation via third-party
`starlight-links-validator`, content conventionally under
`src/content/docs/`.

### Adaptation work on the docs/ tree

- Add `title:` frontmatter to all 17 pages (docs/*.md, docs/skills/*.md)
  — currently none have frontmatter; H1 is the title.
- Footer prev/next lines (added in 79bfc108) duplicate Starlight's
  built-in prev/next links — remove them and drop the `---` rule.
- Links escaping docs/ to `../README.md#documentation` (every footer,
  plus body links) must be rewritten: the site needs its own index page
  since there is no docs/index.md today.
- Sidebar order is defined by the existing footer-nav chain: philosophy
  → workflow → development-loop → visualiser → internals →
  configuration → migrations → releases-and-compatibility → skills/*
  (README first). `docs/skills/README.md` name-collides with Starlight
  index conventions — likely becomes skills/index.md.
- `docs/skills/development-loop.md` duplicates the name of
  `docs/development-loop.md` — fine for slugs, but nav labels must
  disambiguate.

### Build-system integration

- Site scaffold location TBD at planning (e.g. `docs-site/` alongside
  the tree, pointing a content loader at `docs/`, or relocating pages).
- mise tasks follow the wrapper pattern (mise.toml, e.g.
  `build:frontend` at mise.toml:98) — add `docs:build` / `docs:serve`;
  invoke module `tasks/docs.py` wired in `tasks/__init__.py` (strict
  pyrefly + ruff `ALL` apply; untyped deps need a
  `replace-imports-with-any` entry, pyproject.toml:154).
- A read-only `docs:check` (strict build incl. link validation) joins
  the `check` roll-up depends (mise.toml:451) and CI.
- CI: single workflow `.github/workflows/main.yml`; jobs follow
  checkout → jdx/mise-action → `mise run <task>` (main.yml:104). A
  Pages deploy needs `pages: write` + `id-token: write` and
  actions/deploy-pages — new plumbing; must pass actionlint
  (`lint:workflows:check`) and the workflow-topology test
  (`tests/unit/tasks/test_workflows.py`).
- No markdown lint/link checking exists anywhere today — the strict
  site build becomes the first.

## Drafting Notes

- Left as kind `story`; consider re-kinding the tooling-selection portion to a
  spike if the decision needs a time-boxed investigation.
- Author inherited from parent 0145.

## References

- Parent: 0145 — Documentation Improvements
- Related: 0175, 0176
- Decision: `meta/decisions/ADR-0056-astro-starlight-for-documentation-site.md`
  — generator selection (Astro Starlight)
