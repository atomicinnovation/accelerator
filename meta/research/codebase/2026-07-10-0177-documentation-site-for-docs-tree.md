---
type: codebase-research
id: "2026-07-10-0177-documentation-site-for-docs-tree"
title: "Research: Documentation site for the docs/ tree (0177)"
date: "2026-07-10T13:16:28+00:00"
author: Phil Helm
producer: research-codebase
status: complete
work_item_id: "0177"
parent: "work-item:0177"
relates_to: ["codebase-research:2026-06-29-0175-slim-readme-split-docs-tree", "codebase-research:2026-06-29-0176-workflows-rename-and-skill-catalogue"]
topic: "What must change in the docs/ tree, build system, and CI to stand up an Astro Starlight documentation site published to GitHub Pages"
tags: [research, codebase, docs, starlight, github-pages, mise, ci]
revision: "012ec6fb8934c494ab3b9ecbc606df342273189a"
repository: "barcelona"
last_updated: "2026-07-10T13:16:28+00:00"
last_updated_by: Phil Helm
schema_version: 1
---

# Research: Documentation site for the docs/ tree (0177)

**Date**: 2026-07-10T13:16:28+00:00
**Author**: Phil Helm
**Git Commit**: 012ec6fb8934c494ab3b9ecbc606df342273189a
**Branch**: docs/0176-update-skill-docs
**Repository**: barcelona (accelerator workspace)

## Research Question

What must change across the `docs/` tree, the mise/invoke build system, and
`.github/workflows/main.yml` to implement work item 0177 — building the
`docs/` tree with Astro Starlight (per ADR-0056) and publishing it to GitHub
Pages on push to `main`?

## Summary

The `docs/` tree is 17 plain-CommonMark pages (8 in `docs/`, 9 in
`docs/skills/`) with **no frontmatter anywhere**; every page carries a
hand-rolled prev/next footer ending in a `../README.md#documentation` (or
`../../…`) link, and **all internal cross-links use `.md` extensions with
anchors**, which break under Starlight's extensionless slugs unless
rewritten. Inline HTML is pervasive (`<img>` iconify icons before headings,
`<br>` in the skills index table, one manual `<a id>` anchor) but is
Starlight-safe for `.md` pages; one mermaid block in `docs/workflow.md`
needs a rendering strategy. No local image assets exist — all icons are
remote iconify URLs.

The build system is uniform and welcoming: mise tasks are thin
`invoke <ns>.<task>` wrappers; a new `tasks/docs.py` is one import + one
`add_collection` line, is automatically under pyrefly strict + ruff ALL, and
npm invocation via `npm --prefix <dir>` is an established pattern. The
`check` roll-up (mise.toml:451) and the bare `default` task are where
`docs:check`/`docs:build` join; `node_modules/` is globally gitignored but a
new site's `dist/` needs its own ignore entry.

CI is a single workflow whose push trigger only fires on `main`, so a
deploy job gates with `if: github.event_name == 'push'` (the release-job
pattern). **No Pages plumbing exists** (no `pages` permission, no
deploy-pages action, only the `release` environment). Two guardrails
constrain the new job: `tests/unit/tasks/test_workflows.py` (topology
invariants — mostly name-agnostic and compatible with a `github-pages`
environment/concurrency group) and actionlint, whose task **hardcodes
`main.yml` only** (tasks/lint/workflows.py:7), so the deploy job must live
in `main.yml` or the lint task must be extended.

## Detailed Findings

### The docs/ tree: adaptation inventory

**Frontmatter and titles** — none of the 17 files has YAML frontmatter; the
H1 on line 1 is the title. Two pages share the H1 "Development Loop"
(`docs/development-loop.md:1` concept page vs
`docs/skills/development-loop.md:1` reference page) — no slug collision
(different directories), but identical `title:` values would be ambiguous in
sidebar and search; existing footers disambiguate as "Development Loop
(skills)".

**Prev/next footers** — every page ends with:

```
---

[← Prev](prev.md) · [Docs home](../README.md#documentation) · [Next →](next.md)
```

All 17 locations enumerated (e.g. `docs/philosophy.md:24-26` chain start,
`docs/skills/design-convergence.md:145-147` chain end). The footer chain
defines the sidebar order: philosophy → workflow → development-loop →
visualiser → internals → configuration → migrations →
releases-and-compatibility → skills/README → skills/development-loop →
investigation → work-items → issue-trackers → adrs → vcs-and-pr →
review-system → design-convergence.

**Links escaping docs/** (must be reworked):
- All 17 footer links to `../README.md#documentation` /
  `../../README.md#documentation` — removed with the footers; the "Docs
  home" concept needs a new target (site index page).
- `docs/releases-and-compatibility.md:3` — body link to `../README.md`.
- `docs/skills/work-items.md:14` →
  `../../skills/config/configure/SKILL.md#work` and
  `docs/skills/design-convergence.md:79` →
  `../../skills/design/inventory-design/PROTOCOL.md` — links to repo source
  files that won't exist on the site; likely become GitHub URLs.

**Internal cross-links** — relative `.md` links with heading anchors
throughout (e.g. `docs/workflow.md:75` →
`skills/development-loop.md#research-codebase`; the entire
`docs/skills/README.md` table, lines 13–98). Every one breaks under
extensionless slugs unless rewritten or handled by a rehype plugin. Anchors
into `### <img …> /skill-name` headings are fragile — GitHub's and Astro's
sluggers may generate different IDs for headings containing raw HTML.

**Inline HTML** — heavy `<img>` (iconify, remote URLs) before skill
headings and in every `docs/skills/README.md` table row; `<br>` in that
table's description cells; a manual anchor `<a id="jira-integration"></a>`
at `docs/skills/issue-trackers.md:16`. All fine in Starlight `.md` pages
(real CommonMark). One mermaid fenced block at `docs/workflow.md:14-50`
needs a mermaid plugin or pre-rendering.

**skills/README.md** — the skills-section index ("All Skills"); Starlight
convention maps it to `skills/index.md`, changing every link that targets
`skills/README.md` (`docs/workflow.md:12,46,52,101`, root `README.md:67`,
etc.).

**Root README** — the Documentation section (`README.md:50-82`) links every
docs page plus `README.md:44` → `docs/releases-and-compatibility.md`. These
should point at the published site URL once it exists (AC3 requires the URL
documented in the README).

**Assets** — none local; nothing to relocate.

### Build system: where docs tasks slot in

- **Wrapper pattern**: leaf mise tasks are
  `run = "invoke <ns>.<task>"` with optional `depends` (e.g.
  `build:frontend` at `mise.toml:98-101` depends on `deps:install:node`).
  Component checks are pure roll-ups, e.g. `frontend:check` at
  `mise.toml:431-433`.
- **Roll-ups to join**: `check` depends list at `mise.toml:451-453`
  (append `docs:check`); bare `default` at `mise.toml:455-457` (append
  `docs:build`); `fix` at `mise.toml:447-449` only if docs gain
  format/lint fixers.
- **npm pattern**: `npm --prefix {DIR} ci` / `run build` from invoke
  (`tasks/deps.py:106-109`, `tasks/build.py:213-216`); paths in
  `tasks/shared/paths.py`. A docs site with its own `package.json` needs a
  parallel `deps:install:*` task (mise `deps:install:node` at
  `mise.toml:61-63`).
- **Registering `tasks/docs.py`**: one import + one
  `ns.add_collection(Collection.from_module(...))` in
  `tasks/__init__.py` (pattern at lines 3–19, 38, 55–63). Automatically in
  pyrefly-strict scope (`pyproject.toml:148-151`) and ruff ALL
  (`pyproject.toml:62,77-104`); S603/S607 already permit npm-by-name;
  `replace-imports-with-any` (`pyproject.toml:154-165`) already covers
  `invoke.*`.
- **Node pin**: `node = "22.22.2"` at `mise.toml:9`; lockfiles committed,
  consumed via `npm ci`.
- **Gitignore**: global `node_modules/` (line 30) already covers a new
  site; its build output (e.g. `docs-site/dist/`) needs a new entry —
  `/dist/` (line 22) is root-anchored only.
- **Tests to touch**: `tests/unit/tasks/test_mise.py:18` pins
  `_CHECK_GATES = ["cli:check", "deny:check", "pup:check"]` — adding
  `docs:check` to `check.depends` breaks nothing, but add it to the gate
  list if it should be guarded. `test_python_coverage.py` covers a new
  `tasks/docs.py` automatically.
- **Precedent**: the visualiser frontend is the only toolchain-integrated
  npm project today; other `package.json`s (examples, playwright scripts)
  are unwired.

### CI: the Pages deploy job

- **Topology** (`main.yml`): triggers are `push` to `main` only +
  `pull_request` (`main.yml:3-12`); 11 check/test jobs → `prerelease`
  (needs all, `main.yml:343-354`) → `approve-release`
  (environment `release`) → `release`. Standard job shape: pinned-SHA
  checkout → `jdx/mise-action` (`install: true, cache: true`) →
  `mise run <task>` (e.g. `main.yml:104-119`).
- **Push-to-main gating**: release jobs use
  `if: github.event_name == 'push'` (`main.yml:355,438,465`) — sufficient
  because push only fires on `main`. A docs deploy job follows the same
  pattern; a docs *build/check* job can run on PRs like the other checks.
- **No existing Pages plumbing**: permissions blocks exist only on
  `prerelease`/`release` (`id-token: write` for attestation only,
  `main.yml:371-374,480-483`); no `configure-pages`/`upload-pages-artifact`/
  `deploy-pages`, and only the `release` environment. The docs deploy needs
  `pages: write` + `id-token: write` and (conventionally) an
  `environment: github-pages`, plus the one-time repo Pages-enablement
  setting.
- **Workflow-topology test** (`tests/unit/tasks/test_workflows.py`):
  invariants are mostly name-agnostic and compatible with a docs job —
  a `github-pages`-gated job is fine so long as it never carries the
  `accelerator-release` concurrency group (lines 51-59; exactly 2 jobs may
  hold that lock, lines 84-90). Constraints to respect: don't name steps
  `Sign*`/`Prepare*` around the release secret (lines 115-144); no job
  outside the release lane may `needs: check-architecture` (lines 313-341).
- **actionlint**: `tasks/lint/workflows.py:7` hardcodes `main.yml` as the
  only linted file (actionlint 1.7.12 pinned at `mise.toml:18`) — put the
  deploy job in `main.yml`, or extend `_WORKFLOW` if a separate workflow
  file is chosen.
- **Caching**: mise-action caches tools (incl. node); there is no npm
  cache anywhere — deps reinstall per run via `deps:install:*` task
  dependencies. A docs job would mirror `check-visualiser-frontend`
  (`main.yml:322-338`, default cache prefix).
- **Version coherence** (`tasks/build.py:184-210`) never reads any
  `package.json` — a docs-site package version is outside enforcement.

## Code References

- `docs/philosophy.md:24-26` — footer pattern (chain start, no prev)
- `docs/skills/README.md:13-98` — skills index table: `<img>`/`<br>` HTML + `family.md#anchor` links
- `docs/skills/issue-trackers.md:16` — manual `<a id>` anchor
- `docs/workflow.md:14-50` — mermaid block needing a rendering strategy
- `docs/skills/work-items.md:14`, `docs/skills/design-convergence.md:79` — links to repo source files outside docs/
- `mise.toml:98-101` — build:frontend wrapper pattern
- `mise.toml:431-433` — frontend:check pure roll-up (model for docs:check)
- `mise.toml:451-453` — `check` roll-up depends (append docs:check)
- `mise.toml:455-457` — bare default task depends
- `tasks/deps.py:106-109`, `tasks/build.py:213-216` — `npm --prefix` invocation pattern
- `tasks/__init__.py:3-19,38,55-63` — module registration pattern
- `pyproject.toml:148-165` — pyrefly scope + replace-imports-with-any
- `tests/unit/tasks/test_mise.py:18,30-35` — check-gate assertions
- `.github/workflows/main.yml:3-12` — triggers (push=main only)
- `.github/workflows/main.yml:104-119` — standard job shape
- `.github/workflows/main.yml:355,438,465` — push gating pattern
- `tests/unit/tasks/test_workflows.py:51-90,115-144,313-341` — invariants a docs job must respect
- `tasks/lint/workflows.py:7,20-29` — actionlint hardcoded to main.yml
- `.gitignore:22,26-30` — dist/node_modules ignore patterns

## Architecture Insights

- The repo's uniform "mise wrapper → invoke task" pattern means the docs
  toolchain integrates with ~4 touch points: `tasks/docs.py`,
  `tasks/__init__.py`, a mise task block, and roll-up depends — everything
  else (strict typing, ruff, coverage probes) applies automatically.
- The workflow-topology test suite is the real CI gatekeeper: its
  invariants were designed around the release lane and happen to be
  name-agnostic, so a `github-pages` environment passes — but any deviation
  (step naming, needs edges, concurrency groups) fails tests, not just
  review.
- The strict Starlight build with `starlight-links-validator` becomes the
  repo's **first** enforced markdown link check; the `.md`-extension link
  style used throughout docs/ was written for GitHub rendering and is the
  single largest content-migration cost.
- Scaffold location (e.g. `docs-site/` with a content loader pointed at
  `docs/`, vs relocating pages under `src/content/docs/`) remains the key
  open planning decision (ADR-0056 explicitly defers it); the loader
  approach keeps GitHub-rendered docs working, relocation is
  Starlight-conventional.

## Historical Context

- `meta/decisions/ADR-0056-astro-starlight-for-documentation-site.md` —
  generator selection (proposed; AC1 requires acceptance)
- `meta/work/0145-documentation-improvements.md` — parent epic
- `meta/plans/2026-06-29-0175-slim-readme-split-docs-tree.md` and
  `meta/research/codebase/2026-06-29-0175-slim-readme-split-docs-tree.md` —
  how the docs/ tree was created
- `meta/plans/2026-06-29-0176-skill-reference-index-and-subsections.md` and
  `meta/research/codebase/2026-06-29-0176-workflows-rename-and-skill-catalogue.md`
  — the docs/skills/ reference layer (0176, on this branch)
- `meta/notes/2026-06-22-ideas-backlog.md` — origin of the README split idea
- `meta/reviews/prs/11-review-1.md`, `meta/prs/12-description.md` — reviews
  of the 0175/0176 PRs

## Related Research

- `meta/research/codebase/2026-06-29-0175-slim-readme-split-docs-tree.md`
- `meta/research/codebase/2026-06-29-0176-workflows-rename-and-skill-catalogue.md`

## Open Questions

- **Scaffold location**: `docs-site/` with an Astro content loader pointed
  at `docs/` (keeps GitHub rendering of raw Markdown) vs relocating pages
  under the site's `src/content/docs/` — deferred by ADR-0056 to planning.
- **Mermaid**: which rendering approach for `docs/workflow.md:14-50`
  (e.g. rehype-mermaid at build time vs client-side) — no plugin selected.
- **`.md` link strategy**: rewrite all links extensionless vs adopt a
  rehype relative-markdown-link plugin; also whether anchors into
  HTML-containing headings survive Astro's slugger (needs a build-time
  check via the links validator).
- **Docs site as check gate**: should `docs:check` join
  `test_mise.py:_CHECK_GATES` (guarded) or just the `check` roll-up?
- **Source-file links**: exact treatment of the two links to
  `skills/**/SKILL.md`/`PROTOCOL.md` (GitHub URLs vs drop).
- **Title disambiguation**: final `title:`/sidebar labels for the two
  "Development Loop" pages.
