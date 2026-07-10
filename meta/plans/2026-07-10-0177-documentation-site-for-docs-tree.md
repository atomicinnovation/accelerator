---
type: plan
id: "2026-07-10-0177-documentation-site-for-docs-tree"
title: "Documentation Site for the docs/ Tree Implementation Plan"
date: "2026-07-10T14:08:31+00:00"
author: Phil Helm
producer: create-plan
status: ready
work_item_id: "work-item:0177"
parent: "work-item:0177"
derived_from: ["codebase-research:2026-07-10-0177-documentation-site-for-docs-tree"]
relates_to: ["adr:ADR-0056"]
tags: [docs, starlight, github-pages, mise, ci]
revision: "012ec6fb8934c494ab3b9ecbc606df342273189a"
repository: "barcelona"
last_updated: "2026-07-10T14:08:31+00:00"
last_updated_by: Phil Helm
schema_version: 1
---

# Documentation Site for the docs/ Tree Implementation Plan

## Overview

Stand up an Astro Starlight documentation site in `docs-site/`, relocating
the 17 pages of the `docs/` tree into `docs-site/src/content/docs/`, wire it
into the mise/invoke build system as `docs:*` tasks, and publish it to
GitHub Pages on push to `main` from `.github/workflows/main.yml`.
Implements work item 0177 per ADR-0056.

## Current State Analysis

- `docs/` holds 17 plain-CommonMark pages (8 in `docs/`, 9 in
  `docs/skills/`) with **no frontmatter**; the H1 is the title.
- Every page ends with a hand-rolled prev/next footer whose "Docs home"
  link escapes to `../README.md#documentation` (footer pattern at
  `docs/philosophy.md:24-26`).
- All internal cross-links use relative `.md` paths with heading anchors
  (e.g. `docs/workflow.md:75`); two links point at repo source files
  outside `docs/` (`docs/skills/work-items.md:14`,
  `docs/skills/design-convergence.md:79`).
- One mermaid fenced block at `docs/workflow.md:14-50`; pervasive inline
  HTML (`<img>` iconify icons, `<br>` in the `docs/skills/README.md`
  table, `<a id="jira-integration">` at `docs/skills/issue-trackers.md:16`)
  — all Starlight-safe in `.md` pages.
- No docs-site tooling, no markdown link checking, no Pages plumbing in CI
  (permissions blocks exist only on `prerelease`/`release`,
  `main.yml:371-374`).
- Build system: mise leaf tasks wrap `invoke <ns>.<task>`
  (`mise.toml:98-101`); npm is invoked via `npm --prefix <dir>`
  (`tasks/deps.py:106-109`); Node pinned to 22.22.2 (`mise.toml:9`),
  which satisfies Astro 6's ≥22.12 floor.

### Decisions taken at planning (with the user, 2026-07-10)

- **Scaffold**: relocate pages into `docs-site/src/content/docs/` (no
  content loader pointed at `docs/`; `docs/` ceases to exist).
- **Mermaid**: `rehype-mermaid` at build time (`img-svg` + `dark`
  strategies; requires Playwright Chromium in the docs build).
- **Links**: keep `.md`-style relative links in source, resolved at build
  by `astro-rehype-relative-markdown-links` — pages stay GitHub-browsable
  at their new location.
- **Gates**: `docs:check` joins `_CHECK_GATES` in
  `tests/unit/tasks/test_mise.py`; the two source-file links become
  absolute GitHub blob URLs.
- **Title disambiguation**: sidebar labels "Development Loop" (concept)
  and "Development Loop (skills)" (reference), matching the old footer
  convention.
- **CI shape**: build via `mise run docs:build` in the standard
  checkout → mise-action job shape, then
  `actions/upload-pages-artifact` + `actions/deploy-pages` — not
  `withastro/action`, keeping the repo's uniform job pattern and staying
  within the actionlint/topology guardrails.

### Verified stack versions (July 2026)

- `astro` 6.x + `@astrojs/starlight` ^0.41 (0.41.1+ supports Astro 6.4)
- `starlight-links-validator` ^0.25 — validates hash anchors
  (`errorOnInvalidHashes` default true); `errorOnRelativeLinks` must be
  `false` because our source links are relative (the rehype plugin
  resolves them before the validator sees rendered output — verify
  interaction at build)
- `rehype-mermaid` ^3 + `playwright`; Astro shiki needs
  `excludeLangs: ['mermaid']`
- `astro-rehype-relative-markdown-links` ^0.19 — preserves anchors;
  needs `base` and `collectionBase: false`; `README.md` gets **no**
  special slug treatment, subdirectory `index.md` does; a collection-root
  index needs frontmatter `slug: ''`
- `withastro/action` not used; `actions/configure-pages` /
  `upload-pages-artifact` / `deploy-pages` current majors, pinned by SHA
  per repo convention

## Desired End State

- `docs-site/` builds with `mise run docs:build`: zero errors, zero
  broken-link findings, mermaid pre-rendered to SVG.
- `mise run check` includes `docs:check`; `mise run` (default) includes
  `docs:build`.
- Push to `main` publishes the site to
  `https://atomicinnovation.github.io/accelerator/`; the URL is in the
  README, whose Documentation section points at the site.
- ADR-0056 is accepted.
- Sidebar order matches the old footer chain: philosophy → workflow →
  development-loop → visualiser → internals → configuration → migrations
  → releases-and-compatibility → skills index → skills/development-loop →
  investigation → work-items → issue-trackers → adrs → vcs-and-pr →
  review-system → design-convergence.

## What We're NOT Doing

- No content rewrites beyond mechanical adaptation (frontmatter, footers,
  the two source-file links, the skills-index rename).
- No custom Starlight theming, no MDX conversion of existing pages, no
  React islands yet (the capability is the point of ADR-0056; using it is
  future work).
- No markdown linting beyond the strict site build.
- No npm caching work in CI beyond what mise-action already provides.
- No versioned docs, i18n, or search tuning (Pagefind defaults).
- Not extending actionlint beyond `main.yml` — the deploy job lives in
  `main.yml` (`tasks/lint/workflows.py:7` hardcodes it).

## Implementation Approach

Three phases, each independently mergeable: (1) the site exists and
builds standalone; (2) it is wired into the local build system and
guarded by tests; (3) CI checks it on PRs and publishes it on push to
`main`. TDD applies where there are testable seams — the topology tests
in phases 2 and 3 are written first; phase 1 is content + configuration
whose test *is* the strict build with the links validator.

---

## Phase 1: Site scaffold and content migration

### Overview

Create the `docs-site/` Astro Starlight project, move the 17 pages into
`docs-site/src/content/docs/`, and adapt them. Done when
`npm --prefix docs-site run build` is green with zero link findings.

Also in this phase (pre-work): run `/accelerator:review-adr` on ADR-0056
to move it `proposed → accepted` (AC1).

### Changes Required:

#### 1. Scaffold

**File**: `docs-site/package.json` (new)
Dependencies: `astro` ^6, `@astrojs/starlight` ^0.41,
`starlight-links-validator` ^0.25, `rehype-mermaid` ^3, `playwright`,
`astro-rehype-relative-markdown-links` ^0.19. Scripts: `dev`, `build`,
`preview`. Commit `package-lock.json` (repo consumes lockfiles via
`npm ci`).

**File**: `docs-site/astro.config.mjs` (new)

```js
import { defineConfig } from 'astro/config'
import starlight from '@astrojs/starlight'
import starlightLinksValidator from 'starlight-links-validator'
import rehypeMermaid from 'rehype-mermaid'
import rehypeAstroRelativeMarkdownLinks from 'astro-rehype-relative-markdown-links'

export default defineConfig({
  site: 'https://atomicinnovation.github.io',
  base: '/accelerator',
  markdown: {
    syntaxHighlight: { type: 'shiki', excludeLangs: ['mermaid'] },
    rehypePlugins: [
      [rehypeMermaid, { strategy: 'img-svg', dark: true }],
      [rehypeAstroRelativeMarkdownLinks, {
        base: '/accelerator',
        collectionBase: false,
      }],
    ],
  },
  integrations: [
    starlight({
      title: 'Accelerator',
      plugins: [
        starlightLinksValidator({ errorOnRelativeLinks: false }),
      ],
      sidebar: [
        'philosophy',
        'workflow',
        { slug: 'development-loop', label: 'Development Loop' },
        'visualiser',
        'internals',
        'configuration',
        'migrations',
        'releases-and-compatibility',
        {
          label: 'Skills Reference',
          items: [
            'skills',
            { slug: 'skills/development-loop',
              label: 'Development Loop (skills)' },
            'skills/investigation',
            'skills/work-items',
            'skills/issue-trackers',
            'skills/adrs',
            'skills/vcs-and-pr',
            'skills/review-system',
            'skills/design-convergence',
          ],
        },
      ],
    }),
  ],
})
```

(Exact sidebar item forms to be adjusted to the installed Starlight
version's schema; explicit `items` array order is the display order.)

**File**: `docs-site/src/content.config.ts` (new) — standard
`docsLoader()` + `docsSchema()` collection definition.

**File**: `.gitignore`
**Changes**: add `docs-site/dist/` and `docs-site/.astro/` (the existing
root-anchored `/dist/` at line 22 does not cover it; `node_modules/` at
line 30 already does).

#### 2. Content relocation and adaptation

**Files**: `docs/*.md` → `docs-site/src/content/docs/*.md` (git mv);
`docs/skills/*.md` → `docs-site/src/content/docs/skills/*.md`;
`docs/skills/README.md` → `docs-site/src/content/docs/skills/index.md`.

Per page:
- Add frontmatter: `title:` from the H1; remove the H1 line (Starlight
  renders the title). The two "Development Loop" pages get `sidebar`
  labels via the config above, keeping distinct-enough `description:`
  values for search.
- Delete the trailing prev/next footer and its preceding `---` rule
  (Starlight's built-in pagination replaces them).
- Rewrite links that reference `skills/README.md` to `skills/index.md`
  (`workflow.md:12,46,52,101` and any others found by grep).
- `releases-and-compatibility.md:3`: repoint the `../README.md` body link
  at the GitHub blob URL
  `https://github.com/atomicinnovation/accelerator/blob/main/README.md`.
- `skills/work-items.md:14` →
  `https://github.com/atomicinnovation/accelerator/blob/main/skills/config/configure/SKILL.md#work`;
  `skills/design-convergence.md:79` →
  `https://github.com/atomicinnovation/accelerator/blob/main/skills/design/inventory-design/PROTOCOL.md`.
- All other relative `.md` links and anchors stay as-is (resolved by the
  rehype plugin, validated by the links validator).

**File**: `docs-site/src/content/docs/index.md` (new) — site landing
page ("Docs home"), frontmatter `slug: ''` (required for a
collection-root index under the rehype plugin), `title: Accelerator`,
brief orientation + links into the tree, link back to the GitHub repo.

#### 3. Known risks to verify at build

- Anchors into headings containing raw `<img>` HTML: the links validator
  errors if Astro's slugger produces different IDs than the source links
  assume — fix the affected links (or headings) as findings appear.
- The rehype plugins' interaction with the links validator
  (relative-link resolution order) — if resolved links still trip the
  validator, adjust `errorOnRelativeLinks` / plugin ordering.

### Success Criteria:

#### Automated Verification:

- [x] `npm --prefix docs-site ci` succeeds
- [x] `npx --prefix docs-site playwright install --with-deps chromium` succeeds
- [x] `npm --prefix docs-site run build` exits 0 with zero
      `starlight-links-validator` findings
- [x] `git ls-files docs/` is empty (tree fully relocated)
- [x] `rg -n 'Docs home|README.md#documentation' docs-site/src/content/docs/` finds nothing

#### Manual Verification:

- [ ] `npm --prefix docs-site run preview`: sidebar order matches the old
      footer chain; both "Development Loop" entries disambiguated
- [ ] Mermaid diagram on the workflow page renders as SVG in light and
      dark themes
- [ ] Iconify `<img>` icons and the skills index table render correctly
- [ ] Prev/next pagination footer appears and follows sidebar order
- [ ] Pagefind search returns results (in `preview`, not `dev`)
- [x] ADR-0056 status is `accepted`

---

## Phase 2: Build-system integration

### Overview

Wire the site into mise/invoke: `docs:build`, `docs:check`, `docs:serve`,
`deps:install:docs`, `deps:install:docs-playwright`; join the `check` and
`default` roll-ups; guard with tests. TDD: extend
`tests/unit/tasks/test_mise.py` first.

### Changes Required:

#### 1. Tests first

**File**: `tests/unit/tasks/test_mise.py`
**Changes**: append `"docs:check"` to `_CHECK_GATES` (line 18). Run —
red. Optionally assert `docs:build` in `default.depends` alongside.

#### 2. Paths and deps

**File**: `tasks/shared/paths.py`
**Changes**: add `DOCS_SITE = REPO_ROOT / "docs-site"`.

**File**: `tasks/deps.py`
**Changes**: following `install_node` / `install_playwright`
(`tasks/deps.py:106-117`):

```python
@task
def install_docs(context: Context) -> None:
    """Install Node.js dependencies for the documentation site."""
    context.run(f"npm --prefix {DOCS_SITE} ci")


@task
def install_docs_playwright(context: Context) -> None:
    """Install the Chromium binary rehype-mermaid renders with."""
    context.run(
        f"npx --prefix {DOCS_SITE} playwright install --with-deps chromium"
    )
```

#### 3. Docs tasks

**File**: `tasks/docs.py` (new)

```python
"""Documentation-site tasks (Astro Starlight in docs-site/)."""

from invoke import Context, task

from .shared.paths import DOCS_SITE


@task
def build(context: Context) -> None:
    """Build the documentation site (strict: link validation fails the build)."""
    context.run(f"npm --prefix {DOCS_SITE} run build")


@task
def serve(context: Context) -> None:
    """Serve the documentation site with live reload."""
    context.run(f"npm --prefix {DOCS_SITE} run dev", pty=True)
```

(`docs:check` is the same strict build — the links validator is the
check; it is a mise-level alias rather than a second invoke task.)

**File**: `tasks/__init__.py`
**Changes**: add `docs` to the `from . import (...)` block and
`ns.add_collection(Collection.from_module(docs))` alongside the others
(lines 3–19, 55–63).

#### 4. mise wiring

**File**: `mise.toml`
**Changes**: new leaf tasks following the `deps:install:node` /
`build:frontend` patterns:

```toml
[tasks."deps:install:docs"]
description = "Install Node.js dependencies for the documentation site"
run = "invoke deps.install-docs"

[tasks."deps:install:docs-playwright"]
description = "Install the Chromium binary the docs mermaid render needs"
depends = ["deps:install:docs"]
run = "invoke deps.install-docs-playwright"

[tasks."docs:build"]
description = "Build the documentation site (strict: broken links fail the build)"
depends = ["deps:install:docs-playwright"]
run = "invoke docs.build"

[tasks."docs:check"]
description = "Read-only docs gate: the strict site build incl. link validation"
depends = ["docs:build"]

[tasks."docs:serve"]
description = "Serve the documentation site locally with live reload"
depends = ["deps:install:docs-playwright"]
run = "invoke docs.serve"
```

Append `"docs:check"` to `check.depends` (`mise.toml:451-453`) and
`"docs:build"` to `default.depends` (`mise.toml:455-457`).

Note: `docs:check` writes `docs-site/dist/` (gitignored) — same class of
side effect as compilation caches in other checks; acceptable.

### Success Criteria:

#### Automated Verification:

- [x] `uv run pytest tests/unit/tasks/test_mise.py -v` passes (was red
      before wiring)
- [x] `uv run pytest tests/unit/tasks/test_python_coverage.py -v` passes
      (new `tasks/docs.py` picked up)
- [x] `mise run docs:build` exits 0 from a clean checkout
- [x] `mise run docs:check` exits 0
- [x] `mise run build-system:check` passes (pyrefly strict + ruff ALL on
      the new module)
- [x] `mise run check` exits 0

#### Manual Verification:

- [ ] `mise run docs:serve` serves the site with live reload

---

## Phase 3: CI and publishing

### Overview

Add a `check-docs` job (runs on PRs and pushes) and a push-gated
`deploy-docs` job publishing to GitHub Pages. Update the README with the
published URL. TDD: extend `tests/unit/tasks/test_workflows.py` first.

### Changes Required:

#### 1. Tests first

**File**: `tests/unit/tasks/test_workflows.py`
**Changes**: add assertions (red first) that:
- a `check-docs` job exists and runs `mise run docs:check`;
- a `deploy-docs` job exists, is gated `if: github.event_name == 'push'`,
  declares `environment` `github-pages`, has exactly
  `pages: write` + `id-token: write` (+ `contents: read`) permissions,
  `needs: check-docs`, and does **not** carry the `accelerator-release`
  concurrency group (the existing invariant at lines 84–90 must keep
  holding: exactly 2 jobs on that lock).

Respect existing invariants: no `Sign*`/`Prepare*` step names near
secrets (lines 115–144); no `needs: check-architecture` outside the
release lane (lines 313–341).

#### 2. Workflow jobs

**File**: `.github/workflows/main.yml`
**Changes**: after `check-visualiser-frontend` (`main.yml:322-338`),
mirroring its shape:

```yaml
  check-docs:
    name: Check documentation site
    runs-on: ubuntu-latest
    steps:
      - name: Checkout code
        uses: actions/checkout@<pinned-sha>
      - name: Install dependencies
        uses: jdx/mise-action@<pinned-sha>
        with:
          install: true
          cache: true
          experimental: true
      - name: Run documentation site checks
        run: mise run docs:check

  deploy-docs:
    name: Deploy documentation site
    runs-on: ubuntu-latest
    needs: check-docs
    if: github.event_name == 'push'
    permissions:
      contents: read
      pages: write
      id-token: write
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - name: Checkout code
        uses: actions/checkout@<pinned-sha>
      - name: Install dependencies
        uses: jdx/mise-action@<pinned-sha>
        with:
          install: true
          cache: true
          experimental: true
      - name: Build documentation site
        run: mise run docs:build
      - name: Configure Pages
        uses: actions/configure-pages@<pinned-sha>
      - name: Upload site artifact
        uses: actions/upload-pages-artifact@<pinned-sha>
        with:
          path: docs-site/dist
      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@<pinned-sha>
```

Pin all new actions by full commit SHA (repo convention; resolve current
SHAs for the pinned major at implementation time). `docs:check` /
`docs:build` self-provision Chromium via the `deps:install:docs-playwright`
task dependency — no separate workflow step needed. Note `check-docs` is
deliberately **not** added to `prerelease.needs` — the release lane's
gate set is a product decision; `deploy-docs` failing does not block
releases and vice versa. (Add it to `prerelease.needs` instead if docs
must gate releases — decide at review.)

**One-time repo setting** (manual, before first deploy): Settings →
Pages → Source: "GitHub Actions".

#### 3. README

**File**: `README.md`
**Changes**: document the published URL
`https://atomicinnovation.github.io/accelerator/` at the top of the
Documentation section (lines 50–82); repoint the per-page links from
`docs/*.md` paths to the corresponding site URLs (extensionless slugs,
e.g. `.../accelerator/skills/work-items/`); fix `README.md:44`
(`docs/releases-and-compatibility.md`) likewise.

### Success Criteria:

#### Automated Verification:

- [ ] `uv run pytest tests/unit/tasks/test_workflows.py -v` passes (was
      red before the workflow change)
- [ ] `mise run lint:workflows:check` (actionlint) passes
- [ ] `mise run check` exits 0
- [ ] `mise run` (bare default) exits 0 end-to-end

#### Manual Verification:

- [ ] After merge to `main`: the `deploy-docs` job runs and the site is
      live at https://atomicinnovation.github.io/accelerator/
- [ ] On a PR: `check-docs` runs, `deploy-docs` is skipped
- [ ] All README documentation links resolve on the live site
- [ ] Search, dark mode, and the mermaid diagram work on the live site

---

## Testing Strategy

### Unit Tests:

- `test_mise.py`: `docs:check` in `_CHECK_GATES` (phase 2, written first)
- `test_workflows.py`: `check-docs` / `deploy-docs` topology invariants
  (phase 3, written first)
- `test_python_coverage.py`: covers `tasks/docs.py` automatically

### Integration Tests:

- The strict Starlight build itself: `starlight-links-validator` with
  anchor checking is the end-to-end content test, run by
  `mise run docs:check` locally and in CI on every PR.

### Manual Testing Steps:

1. `mise run docs:serve` — click through every page; verify sidebar
   order, pagination, icons, tables, the mermaid SVG in both themes.
2. `npm --prefix docs-site run build && npm --prefix docs-site run preview`
   — verify Pagefind search.
3. After the first `main` deploy, spot-check the live site and every
   README link.

## Performance Considerations

- Chromium download (~150 MB) per CI run for rehype-mermaid; mise-action
  caches tools but not Playwright browsers. Accept initially; add a
  `~/.cache/ms-playwright` actions/cache step later if job time hurts.
- `docs:check` in the `check` roll-up adds an npm ci + Astro build to
  every full local check — comparable to the existing frontend gate.

## Migration Notes

- `docs/` disappears; editors author under `docs-site/src/content/docs/`,
  which remains GitHub-browsable (relative `.md` links intact by design).
- Any external deep links to `docs/*.md` on GitHub break at relocation —
  accepted; the published site becomes the canonical reading surface.
- One-time GitHub Pages enablement (Source: GitHub Actions) must be done
  by a repo admin before the first deploy; until then `deploy-docs` fails
  — enable it before merging phase 3.

## References

- Original work item: `meta/work/0177-documentation-site-for-docs-tree.md`
- Decision: `meta/decisions/ADR-0056-astro-starlight-for-documentation-site.md`
- Research: `meta/research/codebase/2026-07-10-0177-documentation-site-for-docs-tree.md`
- Job shape to mirror: `.github/workflows/main.yml:322-338`
- npm task pattern: `tasks/deps.py:106-117`
- mise wrapper pattern: `mise.toml:98-101`
