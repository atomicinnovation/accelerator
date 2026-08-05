---
type: adr
id: "ADR-0056"
title: "Astro Starlight for the Documentation Site"
date: "2026-07-10T13:05:52+00:00"
author: Phil Helm
producer: create-adr
status: accepted
parent: "work-item:0177"
tags: [documentation, tooling]
last_updated: "2026-07-10T13:05:52+00:00"
last_updated_by: Phil Helm
schema_version: 1
---

# ADR-0056: Astro Starlight for the Documentation Site

**Date**: 2026-07-10
**Status**: Accepted
**Author**: Phil Helm

## Context

Work item 0177 publishes the `docs/` tree (17 Markdown pages produced by
0175/0176) as a browsable site on GitHub Pages via GitHub Actions. No
docs-site tooling exists. The pages are plain CommonMark with no
frontmatter, relative `.md` cross-links (some escaping `docs/` to the
root README), and raw inline HTML (`<img>`, `<br>` in tables). The repo
already carries Node (React 19 + Vite visualiser frontend) and Python
(uv + invoke build system) toolchains pinned via mise; the team values
React + MDX authoring for future interactive docs content. No
link-checking exists anywhere in the repo today.

## Decision Drivers

- React + MDX authoring capability (React expertise already in-house)
- Build-time broken-link validation (currently unenforced)
- First-class static output for GitHub Pages, with search and dark mode
  working without external services
- Low maintenance burden and healthy upstream (avoid frameworks in
  maintenance mode or facing imminent major migrations)
- Minimal new moving parts beside the existing mise/Node/Python setup

## Considered Options

1. **Astro Starlight** — Astro's docs theme; MDX with React via islands
2. **Docusaurus** — Meta's natively-React MDX docs framework
3. **Nextra 4** — Next.js App Router docs framework
4. **VitePress** — Vue team's Vite-based docs generator
5. **mkdocs-material** — Python; the historical default for md trees
6. **mdBook** — Rust project's book generator

## Decision

We will use **Astro Starlight**, deployed to GitHub Pages on push to
`main` via GitHub Actions.

Starlight satisfies the React + MDX driver through `@astrojs/react`
islands while keeping plain `.md` pages as real CommonMark (raw inline
HTML keeps working). It ships Pagefind search, dark mode, and
directory-derived sidebar groups out of the box, and static output to
Pages is its default happy path. Its adoption costs are accepted and
small at this scale: adding `title:` frontmatter to 17 files, adopting
the community `starlight-links-validator` plugin for build-time link
checking, and hosting content under Starlight's content-collection
conventions.

The alternatives each failed a driver: **Docusaurus** is the heaviest
toolchain, has no built-in search (Algolia or plugins), and faces an
imminent v3→v4 migration — its full-React advantage buys little for ~17
pages. **Nextra 4** couples the repo to annual Next.js majors, compiles
`.md` as strict MDX (breaking existing inline HTML), lacks build-time
link checking, has an open Pagefind-under-static-export bug
(shuding/nextra#3987), and is effectively single-maintainer.
**VitePress** is the best plain-Markdown fit but is Vue-based, clashing
with the React driver. **mkdocs-material** entered maintenance mode in
November 2025 (critical/security fixes only until ~November 2026; the
maintainers moved to Zensical) — a poor bet for fresh adoption, and it
has no React/MDX story. **mdBook** requires hand-maintained
`SUMMARY.md` navigation (no tree-derived nav), and its link-check
plugin `mdbook-linkcheck` is semi-dormant (last release October 2024).

## Consequences

### Positive

- MDX pages can embed React components when interactive content is
  wanted, without committing the whole site to a React runtime.
- Build-time link validation becomes the repo's first enforced
  cross-link check for `docs/`.
- Search (Pagefind), dark mode, and sidebar navigation work with no
  external services or heavy theming.
- Static output deploys with GitHub's standard Pages actions, keeping
  CI plumbing thin.

### Negative

- Starlight is pre-1.0: minor releases can contain breaking changes,
  so upgrades need changelog attention.
- Link validation depends on a third-party plugin
  (`starlight-links-validator`), not core.
- All 17 pages need `title:` frontmatter, and footer prev/next lines
  plus `../README.md` links must be reworked to fit the site.

### Neutral

- Adds an Astro toolchain beside the existing Vite app — same
  ecosystem (Node/Vite-based), but a separate build.
- Content conventionally lives under `src/content/docs/`; the scaffold
  location and whether to point a loader at `docs/` is an
  implementation-planning decision for 0177.

## References

- `meta/work/0177-documentation-site-for-docs-tree.md` — owning work
  item; its Technical Notes carry the full candidate research
- `meta/work/0145-documentation-improvements.md` — parent epic
