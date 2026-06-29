---
type: plan
id: "2026-06-29-0175-slim-readme-split-docs-tree"
title: "Slim the README and split content into a docs/ tree (0175)"
date: "2026-06-29T13:04:39+00:00"
author: "Phil Helm"
tags: [readme, docs, documentation, information-architecture]
revision: "a78e30e55a553b280a4c72de4de223144dbe41e0"
repository: "accelerator"
last_updated: "2026-06-29T13:04:39+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
parent: "work-item:0175"
relates_to: ["codebase-research:2026-06-29-0175-slim-readme-split-docs-tree"]
---

# Slim the README and split content into a docs/ tree (0175)

## Overview

Reduce the root `README.md` (886 lines) to its essentials — §1 *What it is* and
§2 *Install + run your first skill* — and relocate every other section into a
new `docs/` tree. Per the decisions taken during planning, this plan goes
slightly beyond 0175's narrow scope: it **relocates *all* remaining sections
verbatim now** — including the seven per-skill-family sections nominally owned by
sibling 0176 — so the README is fully slim at the end of this work. 0176 then
*refines* the `docs/skills/` stub pages rather than creating them. Two sections
with no home in the agreed IA each get a new page: Migrations →
`docs/migrations.md`, and Prerelease + Compatibility → `docs/installation.md`.

This plan supersedes the single-file `meta/plans/2026-03-15-readme-restructure.md`
structure while honouring its content conventions (logo `<picture>` block,
philosophy-first ordering, install commands, MIT link).

## Current State Analysis

`README.md` is **886 lines**, no H1 — it opens with a `<picture>` logo block +
tagline (1–18), first heading `## Getting Started` at line 19. `docs/` does not
exist. `CONTRIBUTING.md` (45 lines) has no Development/setup section. The
authoritative section map (corrected line ranges — section end = line before the
next heading) is in the linked research doc. Key structural traps already
identified:

- **"The Development Loop" is a single H2 (64–106).** The narrative (64–87) goes
  to `docs/development-loop.md`; the skill listings (88–105) go to
  `docs/skills/planning.md`. This is a **mid-section split**, not a clean move.
- **Installation (829–882) splits four ways:** intro + stable-install (829–840)
  → README §2; `### Development` (865–872) → `CONTRIBUTING.md`;
  `### Prerelease Versions` (841–864) and `### Claude Code compatibility`
  (873–882) → `docs/installation.md`.
- **Inline anchor `<a id="jira-integration"></a>` at line 381** and the
  tagline's `[Jump to installation](#installation)` link are anchor references
  that may break on relocation.
- **Relative asset paths** (`assets/accelerator_logo_*.png`, any screenshots)
  resolve from repo root today; once content lives under `docs/` they need a
  `../` (one level) or `../../` (for `docs/skills/`) prefix.

### Key Discoveries

- The prior plan's success criteria are `grep` counts against `README.md` (e.g.
  `grep -c '/accelerator:' README.md ≥ 9`). If any of those live in CI / a
  task / a script, slimming the README will break them — **Phase 0 must find
  them**. Its asserted counts (9 skills, 7 lenses, 7 agents) are stale.
- 0176 (`meta/work/0176-...`) defines the exact `docs/skills/` page names and H1
  titles; this plan adopts them verbatim so 0176 inherits the right files.

## Desired End State

- `README.md`: §1 (logo/tagline/pitch/optional screenshot), §2 (install + run
  first skill), a "Documentation" links block into every `docs/` page, and the
  License footer. Target **under ~150 lines**.
- `docs/` narrative pages: `how-it-works.md`, `development-loop.md`,
  `configuration.md`, `visualiser.md`, `internals.md`, plus the two
  decision-added pages `migrations.md` and `installation.md`. No `docs/` index.
- `docs/skills/` reference stubs (verbatim relocation; 0176 refines):
  `work-items.md`, `issue-trackers.md`, `review-system.md`,
  `design-convergence.md`, `adrs.md`, `vcs-and-pr.md`, `planning.md`.
- `CONTRIBUTING.md`: a new local-development H2 holding the local-checkout
  snippet, linked from the README.
- No content lost: every source section present in exactly one destination.

### Master source → destination mapping (AC5 baseline)

| README section | Lines | Destination | Page H1 |
|---|---|---|---|
| logo + tagline | 1–18 | **README §1** | — |
| Getting Started | 19–40 | **README §2** (merged with install intro) | — |
| Philosophy | 41–63 | `docs/how-it-works.md` | How It Works |
| Development Loop — narrative | 64–87 | `docs/development-loop.md` | The Development Loop |
| Development Loop — skill listings | 88–105 | `docs/skills/planning.md` | Planning |
| The `meta/` Directory | 107–141 | `docs/internals.md` | Internals |
| Migrations | 142–171 | `docs/migrations.md` | Migrations |
| VCS Detection | 172–193 | `docs/how-it-works.md` | How It Works |
| Configuration | 194–321 | `docs/configuration.md` | Configuration |
| Work Item Management | 322–365 | `docs/skills/work-items.md` | Work Items |
| Remote Work Item Management | 366–509 | `docs/skills/issue-trackers.md` | Issue Trackers (Jira & Linear) |
| Architecture Decision Records | 510–535 | `docs/skills/adrs.md` | Architecture Decision Records (ADRs) |
| VCS and PR Workflow Skills | 536–547 | `docs/skills/vcs-and-pr.md` | VCS & PR Workflow |
| Visualiser | 548–632 | `docs/visualiser.md` | Visualiser |
| Review System | 633–672 | `docs/skills/review-system.md` | Review System |
| Design Convergence | 673–801 | `docs/skills/design-convergence.md` | Design Convergence |
| Agents | 802–828 | `docs/internals.md` | Internals |
| Installation — intro + stable install | 829–840 | **README §2** | — |
| Installation → Prerelease Versions | 841–864 | `docs/installation.md` | Installation |
| Installation → Development | 865–872 | `CONTRIBUTING.md` | (existing) |
| Installation → Claude Code compatibility | 873–882 | `docs/installation.md` | Installation |
| License | 883–886 | **README** (footer) | — |

## What We're NOT Doing

- Not standing up a documentation site (0177).
- Not re-authoring or restructuring the `docs/skills/` content beyond a verbatim
  move + H1 + link fixes — 0176 owns the reference-doc rewrite.
- Not changing any skill, agent, hook, or behaviour; this is documentation only.
- Not adding badges/CI decorations (prior-plan exclusion still holds).
- Not creating a `docs/` index or a `docs/contributing.md`.

## Implementation Approach

Content-only restructure. Move text verbatim into new files, then fix the seams
(H1 titles, relative asset paths, internal anchors, cross-links), then slim the
README last so the originals remain available as the source of truth until every
destination exists.

---

## Phase 0: Pre-flight — find load-bearing README couplings

### Changes

- Grep the repo (this workspace only; **never** the parent checkout) for
  anything that depends on README structure or section anchors:
  - `grep -rn "README" tasks/ scripts/ hooks/ .github/ mise.toml` — any check
    that counts skills/lenses/agents in the README, or asserts README content.
  - `grep -rn "README.md#\|](#" README.md` and repo-wide for `#jira-integration`,
    `#installation`, and other intra-doc anchors that slimming would orphan.
  - Check `.claude-plugin/plugin.json` and any marketplace metadata for README
    references.
- Locate the exact mid-section boundary in the Development Loop (the last
  narrative line before the skill listings begin, around line 88) and the exact
  Installation intro boundary (end of stable-install content before
  `### Prerelease Versions` at 841).
- Inventory every relative asset/image path in the sections being moved.

### Success Criteria

**Automated:**
- [x] A written list of all README-coupled checks/links exists (command output
      captured in the PR or a scratch note).

**Manual:**
- [x] Confirmed whether any CI/task/script greps the README (and, if so, a
      decision recorded on whether to update or remove it in Phase 6).
      Found: `test-config.sh` (work.integration), `test-design.sh`
      (design-inventories/gaps + template keys) — both repoint to docs tree in
      Phase 6; `test-format.sh` includes README in a path-list sweep (not
      content-coupled). External anchor referrer: `CHANGELOG.md#jira-integration`.
- [x] Development-Loop split line and Installation intro boundary pinned to exact
      line numbers. Dev-loop seam at 87/88; Installation intro = 829–840.

---

## Phase 1: Create the `docs/` narrative pages

### Changes

Create `docs/` and these pages, moving the mapped content verbatim, adding the
H1 from the mapping table, and fixing relative paths (asset refs gain a `../`
prefix; refs to `meta/` become `../meta/`):

- `docs/how-it-works.md` (H1 "How It Works") ← Philosophy (41–63) + VCS
  Detection (172–193). **Must reference the `meta/` directory** (AC).
- `docs/development-loop.md` (H1 "The Development Loop") ← Development Loop
  narrative (64–87). **Must reference the `meta/` directory** (AC).
- `docs/configuration.md` (H1 "Configuration") ← Configuration (194–321). Its
  custom-review-lenses subsection links to `docs/skills/review-system.md`
  (created in Phase 2).
- `docs/visualiser.md` (H1 "Visualiser") ← Visualiser (548–632).
- `docs/internals.md` (H1 "Internals") ← `meta/` Directory (107–141) + Agents
  (802–828).
- `docs/migrations.md` (H1 "Migrations") ← Migrations (142–171). *(Decision: own
  page.)*
- `docs/installation.md` (H1 "Installation") ← Prerelease Versions (841–864) +
  Claude Code compatibility (873–882). *(Decision: combined install page.)*

### Success Criteria

**Automated:**
- [x] All seven files exist: `for f in how-it-works development-loop
      configuration visualiser internals migrations installation; do test -f
      docs/$f.md; done`.
- [x] `grep -l "meta/" docs/how-it-works.md docs/development-loop.md` returns
      both files.

**Manual:**
- [x] Each page's relative asset/`meta/` links resolve from its `docs/` location.
      (Relocated narrative sections contain no asset paths or repo-relative
      markdown links — only inline-code path prose + external URLs — so no
      `../` rewrites were needed. `configuration.md` links to
      `skills/review-system.md` and `installation.md` to `../README.md`.)
- [x] No content from the mapped ranges was dropped or altered (verbatim move;
      H3→H2 promotion under each page H1 to keep heading hierarchy).

---

## Phase 2: Create the `docs/skills/` reference stubs (verbatim)

### Changes

Create `docs/skills/` and relocate the seven per-family sections verbatim, with
the H1 titles 0176 defines (so 0176 can refine in place). Fix asset paths with a
`../../` prefix:

- `docs/skills/work-items.md` (H1 "Work Items") ← 322–365
- `docs/skills/issue-trackers.md` (H1 "Issue Trackers (Jira & Linear)") ← 366–509
- `docs/skills/adrs.md` (H1 "Architecture Decision Records (ADRs)") ← 510–535
- `docs/skills/vcs-and-pr.md` (H1 "VCS & PR Workflow") ← 536–547
- `docs/skills/review-system.md` (H1 "Review System") ← 633–672
- `docs/skills/design-convergence.md` (H1 "Design Convergence") ← 673–801
- `docs/skills/planning.md` (H1 "Planning") ← Development Loop skill listings
  (88–105)

Handle the `#jira-integration` anchor: either preserve it in
`issue-trackers.md` or update referrers found in Phase 0.

### Success Criteria

**Automated:**
- [x] All seven files exist under `docs/skills/`.

**Manual:**
- [x] Every per-family skills table survives intact in exactly one page.
- [x] `docs/configuration.md`'s custom-lens link to
      `docs/skills/review-system.md` resolves (relative `skills/review-system.md`).
      `#jira-integration` anchor preserved in `issue-trackers.md`; `../../skills/`
      link fixes applied in `work-items.md` and `design-convergence.md`.

---

## Phase 3: Fold local-checkout into `CONTRIBUTING.md`

### Changes

- Add a new H2 (e.g. `## Local development`) to `CONTRIBUTING.md`, inserted
  before the existing `## Linting, formatting, and type-checking` (line 3),
  containing the local-checkout snippet from README 865–872
  (`claude --plugin-dir /path/to/accelerator`).

### Success Criteria

**Automated:**
- [x] `grep -q "plugin-dir" CONTRIBUTING.md`.

**Manual:**
- [x] Reads naturally ahead of the CI-checks section.

---

## Phase 4: Slim the README

### Changes

- Keep §1 (logo `<picture>` block + tagline + one-paragraph pitch; add the
  visualiser screenshot if one is sourced — otherwise note as a follow-up).
- Compose §2 *Install + run your first skill* from Getting Started (19–40) +
  Installation intro/stable-install (829–840): marketplace add, stable install,
  `/init`, and the research → plan → implement quickstart.
- Add a **Documentation** section linking to every `docs/` page (narrative +
  `docs/skills/`), with **link text matching each page's H1** (AC6) and a link
  to `CONTRIBUTING.md` for local development.
- Keep the License footer (883–886).
- Remove every relocated section. Verify total under ~150 lines.

### Success Criteria

**Automated:**
- [x] `[ "$(grep -c '' README.md)" -lt 150 ]` (line count under 150 — now 84).
- [x] README links to all 14 destination pages: `grep -c "docs/" README.md`
      ≥ 14 (15: 7 narrative incl. the 2 new + 7 skills + inline Installation),
      plus the CONTRIBUTING link.

**Manual:**
- [x] Only §1, §2, Documentation links, and License remain.
- [x] Each docs link text equals the destination H1 from the mapping table.

---

## Phase 5: Fix cross-references and assets

### Changes

- Update every relative asset path moved into `docs/` (`../assets/...`) and
  `docs/skills/` (`../../assets/...`).
- Repair or redirect any intra-repo anchors orphaned by the move (from Phase 0
  inventory), including `#jira-integration` and `#installation`.
- Update any other repo file that linked into a relocated README section.

### Success Criteria

**Automated:**
- [x] A markdown link check over `README.md`, `CONTRIBUTING.md`, and `docs/**`
      reports no broken relative links / images (scripted Python check: 25
      relative links resolved; the sole "failure" is an illustrative format
      string in inline code at `CHANGELOG.md:487`, not a real link).

**Manual:**
- [x] Logo renders (dark + light) from the slimmed README and any page that
      embeds it. (README §1 keeps the `<picture>` block with root-relative
      `assets/` paths, unchanged; all four referenced assets exist. No relocated
      page embeds an asset, so no `../` rewrites were required. `#jira-integration`
      referrer in `CHANGELOG.md` repointed to `docs/skills/issue-trackers.md`.)

---

## Phase 6: Verification

### Changes

- Run the AC5 content-accounting check: for each row in the master mapping,
  confirm the source content appears in exactly one destination and nowhere
  else (a checklist walk, optionally scripted by diffing extracted ranges from
  `git show HEAD:README.md` against the new files).
- Update/remove any README-coupled CI check found in Phase 0.
- Run the repo's gate.

### Success Criteria

**Automated:**
- [ ] `mise run check` exits 0.
- [ ] Any retained README content-check passes (or has been updated to scan the
      docs tree).

**Manual:**
- [ ] Every mapping row verified: present in one destination, dropped/duplicated
      in none (AC5).
- [ ] Every `docs/` page reachable from the README with H1-matching link text
      (AC6).
- [ ] README renders correctly in GitHub markdown (logo, screenshot, links,
      dev-loop diagram if retained).

---

## Testing Strategy

This is a content move, not code — there is no unit-test surface. Verification is
(1) the AC5 content-accounting walk, (2) a relative-link/image checker across the
new tree, (3) `mise run check` for repo hygiene, and (4) a manual GitHub-render
check. The chief regression risk is a CI/task that greps the README; Phase 0
finds it and Phase 6 reconciles it.

## Risks & Mitigations

- **Mid-section Development-Loop split** drops or duplicates the narrative/listing
  seam → pin the exact boundary line in Phase 0; verify both halves in Phase 6.
- **Broken asset paths / anchors** after relocation → Phase 5 dedicated fix pass
  + Phase 6 link check.
- **README-coupled CI check** breaks the gate → Phase 0 discovery + Phase 6
  reconciliation.
- **Scope overlap with 0176** (this plan pre-creates `docs/skills/`) → stubs use
  0176's exact filenames/H1s so 0176 refines in place; note this in the 0176
  work item.
- **§1 visualiser screenshot** may not exist yet → source it or record a
  follow-up; do not block slimming on it.

## References

- Work item: `meta/work/0175-slim-readme-and-split-into-docs-tree.md`
- Research: `meta/research/codebase/2026-06-29-0175-slim-readme-split-docs-tree.md`
- Sibling: `meta/work/0176-per-skill-family-reference-docs.md` (page names + H1s)
- Parent epic: `meta/work/0145-documentation-improvements.md` (agreed IA)
- Prior plan (superseded): `meta/plans/2026-03-15-readme-restructure.md`
- Decision (ordering): `meta/work/0019-readme-structure.md`
