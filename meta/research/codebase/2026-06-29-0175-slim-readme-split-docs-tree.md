---
type: codebase-research
id: "2026-06-29-0175-slim-readme-split-docs-tree"
title: "Research: Slim the README and split content into a docs/ tree (0175)"
date: "2026-06-29T12:52:42+00:00"
author: "Phil Helm"
producer: research-codebase
status: complete
work_item_id: "0175"
parent: "work-item:0175"
topic: "Slim the README and split content into a docs/ tree"
tags: [research, codebase, readme, docs, documentation, information-architecture]
revision: "a78e30e55a553b280a4c72de4de223144dbe41e0"
repository: "accelerator"
last_updated: "2026-06-29T12:52:42+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

# Research: Slim the README and split content into a docs/ tree (0175)

**Date**: 2026-06-29T12:52:42+00:00 (UTC)
**Author**: Phil Helm
**Git Commit**: a78e30e55a553b280a4c72de4de223144dbe41e0
**Branch**: barcelona
**Repository**: accelerator

## Research Question

Support work item **0175 — "Slim the README and split content into a docs/
tree"**. Verify the source→destination mapping table (the AC5 baseline) against
the actual `README.md`, confirm the current state of `README.md`,
`CONTRIBUTING.md` and whether `docs/` exists, find homes for the two unassigned
sections (Migrations; Prerelease + Compatibility), and surface conventions and
gotchas from the prior restructure plan and decision that the new multi-page
architecture must honour.

## Summary

- **The mapping table's line ranges in 0175 are systematically off by one.** The
  work item ends each section at the line *before the next heading's blank gap*;
  the correct rule (section end = line immediately before the next heading)
  pushes nearly every end-line up by one. A corrected table is below — use it as
  the AC5 verification baseline, not the one in the work item.
- **`docs/` does not exist today** and nothing in the README links to one. The
  whole `docs/` tree is forward-looking work owned by 0175/0176/0177.
- **`CONTRIBUTING.md` has no "Development" / local-setup section.** The
  local-checkout snippet lives only in the README (`### Development`, 865–872).
  Folding it into `CONTRIBUTING.md` means *creating* a new H2 section, most
  naturally inserted before the existing `## Linting, formatting, and
  type-checking` (line 3).
- **The README is 886 lines** (matches the work item's "~886"). There is **no
  H1** — it opens with a `<picture>` logo block + tagline (1–18); the first
  heading is `## Getting Started` at line 19.
- **Two structural traps in the work item's table** that will block "no content
  lost" (AC5) if taken literally:
  1. "The Development Loop" is a **single H2 (64–106)**. The
     narrative/skill-listings split (64–87 vs 88–105) is *content-only, not
     heading-delimited* — yet 0175 sends the narrative to
     `docs/development-loop.md` and 0176 sends the listings to
     `docs/skills/planning.md`. This is a **mid-section split**, not a clean
     section move.
  2. The row "Installation → Prerelease + Compatibility (~841–881)" conflates
     **two non-adjacent H3s** and spans the intervening `### Development` H3
     (865–872) that is going to `CONTRIBUTING.md`. They must be relocated as two
     separate blocks: `### Prerelease Versions` (841–864) and `### Claude Code
     compatibility` (873–882).
- **The Installation H2 (829–882) is being split three ways**, which the work
  item's table under-specifies: its intro + stable-install content (829–840) is
  the basis for README §2 (stays), `### Development` → `CONTRIBUTING.md`, and
  `### Prerelease Versions` + `### Claude Code compatibility` are the
  still-unassigned blocks.
- **Prior art conflicts structurally but not in spirit.** The 2026-03-15 plan
  and work item 0019 produced today's single-file README. Their *ordering
  decisions* (philosophy → loop → meta → skills; install below the fold) and
  *content conventions* (logo `<picture>` block, install commands, MIT link)
  should be honoured on the new README landing page; their *single-file
  mechanics* (grep-against-`README.md` success criteria, the "above the fold"
  framing) are invalidated by a multi-page split and must be re-derived.

## Detailed Findings

### 1. Corrected source→destination mapping (AC5 baseline)

`README.md` is **886 lines**, no H1. The table below corrects every line range
and assigns each section a destination per 0175 (and 0176 for the
out-of-scope-here rows). Section end = the line immediately before the next
heading of equal/higher level.

| README section | Heading text | Corrected lines | 0175 destination |
|---|---|---|---|
| (front matter) | logo `<picture>` + tagline (no heading) | 1–18 | **stays README** (above fold) |
| H2 | Getting Started | 19–40 | **stays README §2** |
| H2 | Philosophy | 41–63 | `docs/how-it-works.md` |
| H2 | The Development Loop | 64–106 | **split** — narrative 64–87 → `docs/development-loop.md`; skill listings 88–105/106 → `docs/skills/planning.md` (**0176**) |
| H2 | The `meta/` Directory | 107–141 | `docs/internals.md` |
| H2 | Migrations | 142–171 | **UNASSIGNED** (see §4) |
| H2 | VCS Detection | 172–193 | `docs/how-it-works.md` |
| H2 | Configuration | 194–321 | `docs/configuration.md` |
| H2 | Work Item Management | 322–365 | `docs/skills/` (**0176**) |
| H2 | Remote Work Item Management (Jira & Linear) | 366–509 | `docs/skills/` (**0176**) |
| H2 | Architecture Decision Records | 510–535 | `docs/skills/` (**0176**) |
| H2 | VCS and PR Workflow Skills | 536–547 | `docs/skills/` (**0176**) |
| H2 | Visualiser | 548–632 | `docs/visualiser.md` |
| H2 | Review System | 633–672 | `docs/skills/` (**0176**) |
| H2 | Design Convergence | 673–801 | `docs/skills/` (**0176**) |
| H2 | Agents | 802–828 | `docs/internals.md` |
| H2 | Installation | 829–882 | **split three ways** (see below) |
| └ H3 | Prerelease Versions | 841–864 | **UNASSIGNED** (see §4) |
| └ H3 | Development | 865–872 | `CONTRIBUTING.md` |
| └ H3 | Claude Code compatibility | 873–882 | **UNASSIGNED** (see §4) |
| H2 | License | 883–886 | **stays README** (footer) |

Per-row verdict vs the work item's claimed ranges: Getting Started (claimed
19–39 → **40**), Philosophy (41–62 → **63**), meta/ (107–140 → **141**),
Migrations (142–170 → **171**), VCS Detection (172–192 → **193**), Configuration
(194–320 → **321**), Work Item Management (322–364 → **365**), Remote WIM
(366–508 → **509**), ADRs (510–534 → **535**), VCS/PR Skills (536–546 →
**547**), Visualiser (548–631 → **632**), Review System (633–671 → **672**),
Design Convergence (673–800 → **801**), Agents (802–827 → **828**), Development
(865–871 → **872**). **License (883–886) is the only accurate row.**

The Installation **H2 intro lines 829–840** (the marketplace-add + stable
`/plugin install accelerator@atomic-innovation` instructions) are not a row in
the work item's table but are the substance of what 0175 §2 keeps in the README
("marketplace add, stable install"). Track them explicitly.

### 2. H3 subsections omitted from the work item's table

The work item maps at H2 granularity, but four H2 sections carry substantial H3
content that a mover must carry across intact (relevant to both 0175 and 0176):

- **Configuration (194–321)** → `docs/configuration.md`: Config Files (200–209),
  File Format (210–244), Template Management (245–274), Managing Configuration
  (275–279), How It Works (280–287), Custom Review Lenses (288–294), Per-Skill
  Customisation (295–321).
- **Remote Work Item Management (366–509)** → 0176: Jira H3 (383–461, with H4s
  for Configuration/Skills/ADF-Markdown/state-cache) and Linear H3 (462–509,
  with H4s). Note an inline HTML anchor `<a id="jira-integration"></a>` at line
  381 — any cross-links to `#jira-integration` will break on relocation.
- **Visualiser (548–632)** → `docs/visualiser.md`: Launching (567–577),
  Lifecycle (578–588), First-run binary download (589–603), Customisation
  (604–622), Provenance verification (623–632).
- **Design Convergence (673–801)** → 0176: Requirements (714–724), Runtime
  browser dependency (725–737), Cache & cleanup (738–751), Troubleshooting
  (752–760), Authenticated browser crawls (761–785), Security considerations
  (786–801).

### 3. `CONTRIBUTING.md` and the `docs/` directory

`CONTRIBUTING.md` is **45 lines**, single-topic (CI checks):
- `# Contributing` (1) → `## Linting, formatting, and type-checking` (3) →
  `### Fixing one component` (25).
- **No** Development / Local development / Setup / Getting started section
  exists. The local-checkout instructions are *not* here today.

The local-checkout snippet to fold in lives at `README.md:865–872`:
```
### Development

To load from a local checkout:

```bash
claude --plugin-dir /path/to/accelerator
```
```
Recommended home: a **new H2 in `CONTRIBUTING.md` inserted before line 3** (so
setup precedes the pre-push check workflow), then link to it from README §2.

**`docs/` does not exist at the repo root.** `Glob docs/**/*` returns nothing
relevant; the only `docs` matches are config-migration test fixtures
(`skills/config/migrate/scripts/test-fixtures/**/docs/...`) and the visualiser's
Rust API namespace (`server/src/api/docs.rs`) — neither is repo documentation.
`Grep "docs/"` in the README returns no matches: nothing links to a `docs/`
tree yet.

### 4. Homes for the unassigned sections (Open Question in 0175)

Three blocks have no destination in the agreed IA; AC5 cannot pass until each
lands in exactly one place:

- **Migrations (142–171)** — documents the `/accelerator:migrate` skill: safety
  guards (refuses on dirty tree, per-migration preview, state in
  `.accelerator/state/migrations-applied`, `ACCELERATOR_MIGRATE_FORCE=1`),
  skip/unskip mechanics, and the `SessionStart` reminder hook. Candidate homes:
  `docs/configuration.md` (operational/maintenance affinity) or
  `docs/internals.md`. This is a *user-facing maintenance* feature, which leans
  toward `docs/configuration.md`.
- **Prerelease Versions (841–864)** — the `X.Y.Z-pre.N` channel, prerelease
  marketplace file, install/uninstall steps. Candidate homes: a new
  `docs/installation.md`, or appended to `CONTRIBUTING.md` alongside the
  local-checkout content (both are install-channel concerns).
- **Claude Code compatibility (873–882)** — minimum supported Claude Code
  **v2.1.144** and the subagent skill-preload dependency. Candidate homes: stays
  in README §2 (it is short, install-adjacent, and matters to first-time users)
  or `docs/installation.md`.

A decision is needed here before implementation; this is the principal blocker
in the work item's Open Questions.

### 5. Conventions to honour (from prior plan + work item 0019)

The 2026-03-15 plan embeds the literal README content and is the source of
truth for these load-bearing details:

- **Logo block**: a `<picture>` element switching on `prefers-color-scheme`
  (`assets/accelerator_logo_dark_bg.png` / `..._light_bg.png`),
  `<img alt="Accelerator" ... width="342px">`. Preserve the dark/light switch
  and 342px width on the new README.
- **Ordering decisions** (still valid, structure-agnostic): philosophy → the
  development loop → `meta/` → skills; **installation below the fold**; `meta/`
  explained before any skill references it.
- **Install commands**: `/plugin marketplace add atomicinnovation/accelerator`
  then `/plugin install accelerator@atomic-innovation`; local dev
  `claude --plugin-dir /path/to/accelerator`.
- **Reference conventions**: skills cited with full invocation incl. example
  args (`/accelerator:research-codebase "..."`, `/accelerator:implement-plan
  @meta/plans/plan.md`); the `@path` arg convention; the dev-loop ASCII diagram
  must render in GitHub markdown.
- **License**: `MIT — see [LICENSE](LICENSE).`
- **Locator-vs-analyser agent split** rationale ("find, no Read" vs "understand,
  with Read", to keep agent context bounded) — preserve when relocating the
  Agents section to `docs/internals.md`.

## Architecture Insights

- **The work item's own AC5 baseline is the wrong artefact to verify against** —
  its line ranges are off by one and two rows are structurally malformed
  (mid-section narrative/listings split; non-adjacent Prerelease+Compatibility
  spanning Development). Implementation should adopt the corrected table in §1.
- **"No content lost" is harder than a section-move** because of the
  Development-Loop mid-section split and the three-way Installation split. AC5
  verification needs to check *content presence*, not section-heading presence.
- **The "fold" metaphor changes meaning in a multi-page tree.** Prior art's
  "install below the fold" was a single-scroll-page decision; on a landing
  README the equivalent is "philosophy/loop links appear before the install
  block", and sub-pages have no fold. Re-state the intent rather than copying
  the rule.
- **0176 and 0175 share a hard dependency on heading granularity.** Because four
  H2s carry rich H3/H4 trees (§2) and one H2 must be split mid-body, the two
  work items should agree on exact byte/line boundaries before either moves
  content, or they will double-claim or drop the Development-Loop listings.

## Code References

- `README.md:1-18` — logo `<picture>` block + tagline (above fold; stays)
- `README.md:64-106` — "The Development Loop" single H2; mid-section split point
  at ~line 88 (narrative vs skill listings)
- `README.md:142-171` — Migrations (unassigned)
- `README.md:381` — inline `<a id="jira-integration"></a>` anchor (link-break risk)
- `README.md:829-882` — Installation H2 (three-way split: intro→README §2,
  Development 865–872→CONTRIBUTING, Prerelease 841–864 + Compatibility
  873–882→unassigned)
- `README.md:883-886` — License (stays as footer; only accurate row in 0175's table)
- `CONTRIBUTING.md:1-3` — insertion point for new local-development H2
- `meta/work/0175-slim-readme-and-split-into-docs-tree.md` — the work item
- `meta/plans/2026-03-15-readme-restructure.md` — prior single-file plan (content
  source of truth; embeds literal README content lines 116–282)
- `meta/work/0019-readme-structure.md` — ADR-form record of the philosophy-first
  ordering decision

## Historical Context

- `meta/plans/2026-03-15-readme-restructure.md` — the implemented single-file
  README rewrite (philosophy-first, install-below-fold). Its automated success
  criteria are `grep` counts against `README.md` (e.g. `grep -c '/accelerator:'
  README.md ≥ 9`) — these **will break** under a docs/ split and must be
  rewritten to scan the docs tree. Counts it asserts (9 skills, 7 lenses, 7
  agents) are **stale** — re-verify before reuse. Its "no tutorial/getting-
  started guide" scope exclusion may be in tension with a multi-page tree.
- `meta/work/0019-readme-structure.md` — ADR-form decision capturing the
  ordering rationale (`external_id: PP-41`, `source: plan:2026-03-15-readme-
  restructure`). Treat its ordering/precedence decisions as constraints the new
  landing page must still satisfy; it never contemplated a multi-page split.
- `meta/decisions/ADR-0019-ephemeral-file-separation-via-paths-tmp.md` — **NOT**
  about the README (it is about `paths.tmp`). The "decision 0019" referenced in
  the work item's References is actually the work item `0019-readme-structure.md`.
- Related README/CHANGELOG work (different scope, useful precedent for README
  editing): `meta/work/0123-changelog-readme-1-23-0-update.md`,
  `meta/research/codebase/2026-06-23-0123-changelog-readme-1.23.0-update.md`,
  `meta/plans/2026-06-23-0123-changelog-readme-1.23.0-update.md`,
  `meta/research/codebase/2026-06-17-readme-changelog-1.22.0-refresh.md`.

## Related Research

- `meta/work/0145-documentation-improvements.md` — parent epic.
- `meta/work/0176-per-skill-family-reference-docs.md` — sibling; owns the
  `docs/skills/` reference pages and the Development-Loop skill listings
  (88–105 → `docs/skills/planning.md`). Hard ordering dependency: 0176 needs the
  `docs/` tree from 0175 to exist.
- `meta/work/0177-documentation-site-for-docs-tree.md` — sibling; documentation
  site over the `docs/` tree (no docs-site tooling exists today). Depends on 0175.

## Open Questions

1. **Where do the three unassigned blocks land?** Migrations (142–171),
   Prerelease Versions (841–864), Claude Code compatibility (873–882). Candidate
   homes in §4; a decision is the principal blocker for AC5. (Author's call —
   see 0175 Open Questions.)
2. **How is the Development-Loop H2 split executed** so the narrative (→0175
   `docs/development-loop.md`) and the skill listings (→0176
   `docs/skills/planning.md`) don't drop or duplicate content at the ~line-88
   boundary? Needs coordination with 0176.
3. **Should a new `docs/installation.md` be created** (the work item lists it as
   a candidate but the agreed IA has no installation page), or is the README §2
   the home for stable-install + compatibility, with prerelease folded into
   `CONTRIBUTING.md`?
4. **How is AC5 ("no content lost") verified mechanically** now that the prior
   plan's `grep`-against-`README.md` checks no longer apply across a multi-file
   tree?
