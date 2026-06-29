---
type: plan
id: "2026-06-29-0176-skill-reference-index-and-subsections"
title: "Skill Reference Index & Per-Skill Subsections Implementation Plan"
date: "2026-06-29T20:21:32+00:00"
author: Phil Helm
producer: create-plan
status: draft
work_item_id: "0176"
parent: "work-item:0176"
derived_from: ["codebase-research:2026-06-29-0176-workflows-rename-and-skill-catalogue"]
tags: [docs, skills, documentation]
revision: "7c12519d1173e25c10c85e22dd1f60b4916d5434"
repository: "barcelona"
last_updated: "2026-06-29T20:21:32+00:00"
last_updated_by: Phil Helm
schema_version: 1
---

# Skill Reference Index & Per-Skill Subsections Implementation Plan

## Overview

Complete the per-skill-family reference layer (work item 0176) with three
additions on top of the already-relocated `docs/skills/` family pages:

1. Close documentation gaps — three user-invokable skills are currently
   documented nowhere.
2. Give **every** user-invokable skill a templated per-skill subsection on its
   home page, following one consistent pattern (*what it does / how to use it /
   advice & guidelines*), so every skill has a stable, deep-linkable anchor.
   This converts the family pages' tables/prose into **H3 subsections** and
   augments the five concept pages (which keep their narrative) with a
   subsection for each skill they home. Sections are optional: simple skills get
   a short two-line entry (no advice block), but the shape is uniform.
3. Add a master **`docs/skills/index.md`** listing every user-invokable skill
   grouped by family, guarded by a new automated drift test (written first,
   TDD-style), and linked from the README.

The "skill family" framing is retained (no rename to "workflows"). No standalone
per-skill page is created — every skill is a subsection on an existing page.

## Current State Analysis

- `docs/skills/` already holds seven family pages from the 0175 split. Most use
  a compact `Skill | Usage | Description` table (`work-items.md`,
  `vcs-and-pr.md`, `issue-trackers.md`); `planning.md` uses prose bullets;
  `adrs.md` / `design-convergence.md` use tables + diagrams; `review-system.md`
  lists review *lenses*, not skills.
- There is **no** master index of skills and **no** per-skill "advice &
  guidelines" anywhere — that is the new editorial layer.
- The complete user-invokable set is **46 skills** (those not carrying
  `user-invocable: false`). The 23 internal skills — 18 review lenses, 3
  review output-formats, `browser-executor`, `paths` — must be excluded.
- Three user-invokable skills are documented **nowhere** today:
  `conduct-spike`, `refine-work-item`, `stress-test-work-item` (confirmed by a
  repo-wide grep of `docs/` + `README.md`).
- Several skills live in **concept pages**, not family pages: `research-codebase`
  / `create-plan` / `implement-plan` → `docs/development-loop.md`; `visualise` →
  `docs/visualiser.md`; `migrate` → `docs/migrations.md`; `configure` / `init` →
  `docs/configuration.md` / `docs/internals.md`.

### Key Discoveries

- **Automated docs coverage is grep-only.** No markdown linter, link-checker, or
  formatter exists. CI's only docs coupling is string-presence assertions in
  `scripts/test-config.sh` (requires `work.integration` in
  `docs/skills/work-items.md`), `scripts/test-design.sh` (requires
  `design-inventories/`/`design-gaps/` in `docs/internals.md` and
  `design-inventory`/`design-gap` in `docs/configuration.md`), and a
  hyphenation sweep in `scripts/test-format.sh` over `README.md`/`CHANGELOG.md`
  (not `docs/`). All are auto-discovered and run by `mise run
  test:integration:config`.
- **No skill→docs cross-check exists.** `scripts/test-skill-frontmatter-conformance.sh`
  enumerates `SKILL.md` files and validates their frontmatter, but never checks
  them against any docs listing — so nothing guards the new index from drift.
  This is the TDD opening.
- **New shell tests are auto-discovered.** `tasks/test/helpers.py:17-44`
  globs executable `scripts/**/test-*.sh`; a new `scripts/test-skills-index.sh`
  is picked up by `test:integration:config` with no wiring.
- **Bash 3.2 floor** applies to the new test (no associative arrays, no
  `${var,,}`); it is an entrypoint script, so it must be `chmod +x` and pass
  `shellcheck` + `shfmt` + the bashisms linter + the exec-bit invariant.
- **GitHub heading anchors**: an `### create-work-item` heading yields the
  anchor `#create-work-item`, so headings must be the bare skill name (the
  `/accelerator:` invocation goes in the body, not the heading).
- Cross-link convention in this repo: link text = destination page H1 title;
  `docs/configuration.md:100` already links `[Review System]` (satisfies 0176's
  custom-lens AC).

### The authoritative skill → family → documentation-home mapping

The index groups the 46 skills by **nine navigational families** (the dir
families don't map 1:1 to doc pages). Each row's link target is shown.

| Family (index group) | Skills | Link target |
|---|---|---|
| Development Loop | research-codebase, create-plan, implement-plan | `../development-loop.md#<name>` |
| Planning | review-plan, stress-test-plan, validate-plan, research-issue, **conduct-spike** (new), create-note | `planning.md#<name>` |
| Work Items | create-work-item, extract-work-items, list-work-items, update-work-item, **refine-work-item** (new), review-work-item, **stress-test-work-item** (new), sync-work-items | `work-items.md#<name>` |
| Issue Trackers | init-jira, search-jira-issues, show-jira-issue, create-jira-issue, update-jira-issue, comment-jira-issue, transition-jira-issue, attach-jira-issue, init-linear, search-linear-issues, show-linear-issue, create-linear-issue, update-linear-issue, comment-linear-issue, transition-linear-issue, attach-linear-issue | `issue-trackers.md#<name>` |
| ADRs | create-adr, review-adr, extract-adrs | `adrs.md#<name>` |
| VCS & PR | commit, describe-pr, review-pr, respond-to-pr | `vcs-and-pr.md#<name>` |
| Design Convergence | inventory-design, analyse-design-gaps | `design-convergence.md#<name>` |
| Config & Maintenance | configure, init, migrate | `../configuration.md#configure`, `../configuration.md#init`, `../migrations.md#migrate` |
| Visualiser | visualise | `../visualiser.md#visualise` |

Every link is now a per-skill anchor: `init` is homed on `configuration.md`
(not `internals.md`), and the four concept pages homing skills
(`development-loop.md`, `configuration.md`, `migrations.md`, `visualiser.md`)
each gain `### <name>` subsections alongside their existing narrative.
`internals.md` no longer homes a skill.

Total: 3 + 6 + 8 + 16 + 3 + 4 + 2 + 3 + 1 = **46**. `review-pr` / `review-plan` /
`review-work-item` are listed once each under their own family; `review-system.md`
remains the mechanism page and is cross-referenced from those rows, not a skill
home.

## Desired End State

- Every user-invokable skill has exactly one documented home and a templated
  `### <name>` subsection there with a stable anchor — across the six family
  pages and the four concept pages that home skills. The three
  currently-undocumented skills are documented; `review-system.md` retains its
  lens tables.
- All 46 subsections follow one consistent pattern (what it does / how to use it
  / advice & guidelines), with the advice block omitted for simple skills.
- `docs/skills/index.md` ("All Skills") lists all 46 user-invokable skills
  grouped by the nine families, each **deep-linking to its subsection anchor**,
  and is linked from the README **Skills** section.
- `scripts/test-skills-index.sh` passes: it derives the user-invokable set from
  `SKILL.md` frontmatter and asserts the index references every one (and no
  internal skill), that the set numbers exactly 46, that every deep link
  resolves to a real `### <name>` heading on its target page, and that each
  skill's "what it does" / index gloss matches the first sentence of its
  `SKILL.md` `description` — failing if a skill is added/removed/renamed or a
  description drifts without the docs being updated.
- `mise run test:integration:config` and `mise run check` are green.

## What We're NOT Doing

- No standalone per-skill pages — every skill is an `### <name>` subsection on
  an existing page.
- No rename of "skill family" to "workflows".
- No wholesale rewrite of the concept pages (`development-loop.md`,
  `configuration.md`, `migrations.md`, `visualiser.md`, `internals.md`): they
  **keep their existing narrative**, but each gains a per-skill `### <name>`
  subsection (same template) for the skill(s) it homes, so all 46 skills are
  uniformly deep-linkable. `init` is pinned to a single home (`configuration.md`)
  rather than being split across `configuration.md` and `internals.md`.
- No change to `review-system.md`'s lens tables.
- No general markdown linter / formatter, and no *external* (http) link
  checker. The drift test does verify in-repo `#<name>` anchor resolution for
  all 46 skills, but full markdown link-checking stays out of scope.
- No frontmatter-driven *generation* of the index/subsections — hand-authored,
  guarded by the drift test (which asserts descriptions match `SKILL.md`).
  (Full generation is deferred to 0177's open question.)
- No padding: skills with nothing non-obvious to say get a short two-line
  subsection and no "Advice & guidelines" block.

## Implementation Approach

Three sequenced phases, each an independently mergeable, CI-green increment.
Phase 3 writes the drift test red-first, then authors the index to green. The
test now guards more than membership — the 46-count, anchor resolution, and
description-to-`SKILL.md` match are all machine-checked; only prose *quality*
(legibility, advice accuracy) and concept-page link *filenames* remain manual.

---

## Phase 1: Close documentation gaps

### Overview

Document the three currently-undocumented user-invokable skills, in each page's
*current* format, so the docs are complete and the page stays internally
consistent. (Phase 2 reformats these alongside everything else; adding them now
keeps this a small, standalone, de-risking merge.)

### Changes Required

#### 1. `docs/skills/planning.md` — add `conduct-spike`

**Changes**: Add a bullet for `conduct-spike` to the companion-skills list,
matching the existing prose-bullet style.

```markdown
- `/accelerator:conduct-spike @meta/work/0042-spike-x.md` — Interactively run a
  time-boxed spike against a spike work item (or brief), recording the outcome
  back on the work item.
```

#### 2. `docs/skills/work-items.md` — add `refine-work-item`, `stress-test-work-item`

**Changes**: Add two rows to the existing skills table.

```markdown
| **refine-work-item**      | `/accelerator:refine-work-item [work-item-ref]`      | Decompose, enrich, size, or link a drafted work item before planning |
| **stress-test-work-item** | `/accelerator:stress-test-work-item [work-item-ref]` | Adversarially grill scope, assumptions, and acceptance criteria before planning |
```

### Success Criteria

#### Automated Verification

- [x] Docs grep contracts still pass: `mise run test:integration:config`
- [x] (Fast inner loop) `bash scripts/test-config.sh` exits 0
- [x] `grep -q "accelerator:conduct-spike" docs/skills/planning.md`
- [x] `grep -q "accelerator:refine-work-item" docs/skills/work-items.md && grep -q "accelerator:stress-test-work-item" docs/skills/work-items.md`

#### Manual Verification

- [x] The three new entries read consistently with their page's existing style.
- [x] Descriptions are faithful to each skill's `SKILL.md` frontmatter.

---

## Phase 2: Give every skill a templated subsection on its home page

### Overview

Establish the per-skill anchors the index will link to, across **all home
pages**: rewrite the six family pages that carry tables/prose into
H3-per-skill subsections, and augment the four concept pages with a templated
subsection for each skill they home (keeping their existing narrative). One
phase; each page is an independently mergeable commit within it.
`review-system.md` is untouched.

Pages in scope:
- **Family pages (table/prose → H3 subsections):** `work-items.md`,
  `vcs-and-pr.md`, `issue-trackers.md`, `adrs.md`, `design-convergence.md`,
  `planning.md`.
- **Concept pages (narrative retained, H3 subsections added):**
  `development-loop.md` (research-codebase, create-plan, implement-plan),
  `configuration.md` (configure, **init** — newly homed here),
  `migrations.md` (migrate), `visualiser.md` (visualise).

### Per-skill subsection template

The same template applies to every skill on every page — uniform shape, optional
blocks:

```markdown
### <skill-name>

**What it does** — <the first sentence of the skill's SKILL.md `description`,
reproduced verbatim; optionally followed by a clause of page-specific context>.

**How to use it** — `/accelerator:<skill-name> <argument-hint>`

**Advice & guidelines** — <OPTIONAL; only where non-obvious: gotchas, pairings,
when to prefer this over a sibling, confirmation/safety notes. Omit this line
entirely for simple skills.>
```

**Why first-sentence-verbatim**: every `SKILL.md` `description` follows the
pattern "`<what-it-does>.` Use when `<triggers>`", so the first sentence is the
canonical "what it does". Reproducing it verbatim is what lets Phase 3's test
assert it (and the index gloss) against frontmatter and catch drift.

### Changes Required

#### 1. Family pages: intro prose + H2 grouping retained, table → H3 subsections

**Files**: the six family pages above.
**Changes**:
- Keep each page's existing intro paragraph and H2 structure (e.g.
  `issue-trackers.md` keeps its `## Jira` / `## Linear` split and the
  Configuration / ADF / state-cache subsections).
- Convert each table row (or prose bullet) into an `### <skill-name>`
  subsection using the template.
- Heading is the bare skill name (clean anchor); the `/accelerator:` invocation
  lives in **How to use it**.
- Author **Advice & guidelines** only where there is something non-obvious
  (e.g. `sync-work-items` → run `--preview` first; `create-work-item` → pair
  with `refine-work-item`; write skills → payload-preview/confirm; read skills →
  auto-trigger on natural language). Simple skills (e.g. `create-note`,
  `list-work-items`, `commit`) get no advice block.
- For `issue-trackers.md`'s 16 near-identical CRUD skills, **keep a compact
  summary table** at the top of each `## Jira` / `## Linear` section (preserving
  the side-by-side parity view) and add the H3 subsections below it for the
  deep-link anchors — both affordances, not one.

#### 2. Concept pages: narrative retained, per-skill subsections added

**Files**: `development-loop.md`, `configuration.md`, `migrations.md`,
`visualiser.md`.
**Changes**:
- Leave each page's existing narrative/diagrams in place. **Add** an
  `### <skill-name>` subsection (same template) for each skill the page homes,
  so all 46 skills are uniformly deep-linkable.
- `init` is newly homed on `configuration.md` and needs a **real subsection**
  (it is currently only a one-line mention in `internals.md`): give it a
  faithful "What it does" plus a short "How to use it". Remove `init` as a home
  from `internals.md` (a cross-reference link to `configuration.md#init` is
  fine; do not leave a second documented home).

#### 3. Preserve grep-tested tokens

**Critical**: keep the literal strings the shell tests assert on:
- `work.integration` must remain in `docs/skills/work-items.md`
  (`scripts/test-config.sh`).
- `configuration.md` IS now edited (configure + init subsections), so its design
  tokens `design-inventory` / `design-gap` must be preserved
  (`scripts/test-design.sh`); `internals.md`'s `design-inventories/` /
  `design-gaps/` tokens likewise if the `init` mention is removed from it.
- Do not remove `inventory-design` / `analyse-design-gaps` invocations from
  `design-convergence.md`.

### Success Criteria

#### Automated Verification

- [x] Docs grep contracts still pass: `mise run test:integration:config`
- [x] `grep -q "work\.integration" docs/skills/work-items.md`
- [x] Design tokens preserved on the now-edited `configuration.md`:
      `bash scripts/test-design.sh` exits 0
- [x] No accidental `work item`-spaced identifiers introduced (README sweep
      unaffected, but keep prose clean): spot-check passes
      `mise run test:integration:config`

#### Manual Verification

- [x] Every skill on every home page now has an `### <name>` subsection with a
      working GitHub anchor (`#name`).
- [x] Per-page subsection counts match the mapping (no skill dropped):
      `work-items.md` = 8, `issue-trackers.md` = 16, `vcs-and-pr.md` = 4,
      `adrs.md` = 3, `design-convergence.md` = 2, `planning.md` = 6
      (research-issue, create-note, conduct-spike, review-plan, stress-test-plan,
      validate-plan); concept pages: `development-loop.md` = 3,
      `configuration.md` = 2 (configure, init), `migrations.md` = 1,
      `visualiser.md` = 1. Total = 46.
- [x] Each subsection's "What it does" reproduces the first sentence of the
      skill's `SKILL.md` `description` verbatim; usage matches `argument-hint`.
- [x] `init` has a real subsection on `configuration.md` and is no longer a
      second home on `internals.md`.
- [x] Each review skill subsection (`review-pr`, `review-plan`,
      `review-work-item`) links to `review-system.md` (load-bearing
      cross-reference).
- [x] "Advice & guidelines" appears only where it adds genuine value; no filler.
- [x] `review-system.md` is unchanged; its lens tables remain.
- [x] Pages render correctly on GitHub (tables gone, subsections legible).

---

## Phase 3: Master index + drift-guard test (TDD) + README link

### Overview

Write the drift-guard test first (red), then author `docs/skills/index.md` to
make it green, then link the index from the README. Ships last, so every
anchor it links to already exists.

### Changes Required

#### 1. `scripts/test-skills-index.sh` (write first — red)

**File**: `scripts/test-skills-index.sh` (new entrypoint; `chmod +x`, commit)
**Changes**: Source `scripts/test-helpers.sh` and use its `PASS`/`FAIL`
counters and `test_summary` exit gate, matching every other `scripts/test-*.sh`
suite. For frontmatter parsing reuse the **bounded** parser from
`scripts/validate-corpus-frontmatter.sh` — `extract_frontmatter` (awk between
the first two `---` fences), then `parse_fm` + `bk_value` (bash-3.2 parallel
arrays). **Do not** model enumeration on
`scripts/test-skill-frontmatter-conformance.sh`: it is fixture-based and does
not enumerate `SKILL.md` on disk, so it offers no glob to copy, and its
`extract_literal` reads inline doc-bullets, not YAML — using it would misparse.

```sh
# Pseudocode (bash 3.2 — no associative arrays)
# 1. Enumerate SKILL.md under skills/, PRUNING */node_modules/* and
#    */test-fixtures/* (a bare `find skills -name SKILL.md` returns 82 files;
#    12 are vendored node_modules, 1 is a frontmatter-less migrate fixture).
#      find skills -name SKILL.md \
#        -not -path '*/node_modules/*' -not -path '*/test-fixtures/*'
# 2. For each file: extract the frontmatter block (extract_frontmatter), then
#    read `name`, `user-invocable`, and `description` via parse_fm/bk_value.
#    Reading ONLY the fenced block is load-bearing: configure/SKILL.md has
#    body-level `name: compliance` / `name: work-item-style` lines (config-key
#    examples) that a repo-wide `grep '^name:'` would wrongly treat as skills.
#    NOTE: `description` is a multi-line YAML plain scalar (continuation lines
#    indented 2 spaces), so FOLD continuation lines into one string before use
#    (parse_fm captures only the first line on its own).
#      - empty `name` after parsing  -> FAIL loudly (malformed/absent
#        frontmatter; defence-in-depth if a file slips the path filter)
#      - user-invocable: false       -> internal set
#      - otherwise                   -> invokable set
#    Derive first_sentence(name) = folded description truncated at the first
#    ". " (period-space). Every description follows "<what>. Use when <triggers>"
#    so this is well-defined for all 46.
# 3. Liveness gate: assert the invokable set has exactly 46 members, so an
#    enumeration/exclusion regression fails loudly rather than silently.
# 4. For each invokable name: assert docs/skills/index.md contains the
#    invocation token, matched with a trailing non-identifier boundary so
#    prefixes don't collide (e.g. `init` must not be satisfied by `init-jira`):
#      grep -Eq "accelerator:${name}([^A-Za-z0-9-]|\$)" docs/skills/index.md
# 5. For each internal name: assert that same token is ABSENT (internal skills
#    are never written as /accelerator:<name>, so this is collision-safe even
#    for common lens words like "scope" or "clarity").
# 6. Anchor-resolution check (ALL 46, no exemption — every skill now has a
#    home-page subsection): parse each invokable name's `<page>.md#<name>` deep
#    link from index.md and assert the target page (family OR concept page)
#    contains a matching `### <name>` heading.
# 7. Description-match check (Theme 3-B drift guard): for each invokable name,
#    assert first_sentence(name) appears verbatim (whitespace-normalised, e.g.
#    grep -F) BOTH in its `### <name>` home-page subsection AND in its index
#    bullet. Catches a SKILL.md description being reworded without the docs
#    following.
# 8. Negative self-test (wiring proof): build a temp index missing one known
#    invokable name AND containing one known internal name; assert the checker
#    reports FAIL — proving the assertions are live, not vacuous. Mirrors the
#    model script's per-axis mutation self-test.
# 9. Use test_summary to print the PASS/FAIL tally and exit non-zero on any
#    failure.
```

The contract the index + pages must satisfy: **(a) each user-invokable skill is
referenced in `index.md` via its `/accelerator:<name>` invocation; (b) no
internal skill is; (c) the invokable set numbers exactly 46; (d) every deep
link (all 46) resolves to a real `### <name>` heading on its target page; and
(e) each skill's index gloss and home-page "what it does" reproduce the first
sentence of its `SKILL.md` `description` verbatim.**

#### 2. `docs/skills/index.md` (author — green)

**File**: `docs/skills/index.md` (new)
**Changes**: H1 "All Skills"; short intro; one `##` section per navigational
family (the nine from the mapping table), each a bullet list of
`[/accelerator:<name>](<target>) — <first sentence of SKILL.md description>`.
**Every** skill deep-links to its subsection anchor `<page>.md#<name>` (family
AND concept pages alike — concept-page subsections are added in Phase 2). The
one-line gloss must be the verbatim first sentence of the skill's `description`
so Phase 3's test can assert it. Add a note under VCS & PR / Planning / Work
Items that `review-pr`, `review-plan`, and `review-work-item` use the
[Review System](review-system.md).

```markdown
# All Skills

Every user-invokable Accelerator skill, grouped by family. Each links to its
reference subsection.

## Development Loop

- [`/accelerator:research-codebase`](../development-loop.md#research-codebase) —
  <first sentence of its SKILL.md description, verbatim>.
- [`/accelerator:create-plan`](../development-loop.md#create-plan) — …
- [`/accelerator:implement-plan`](../development-loop.md#implement-plan) — …

## Work Items

- [`/accelerator:create-work-item`](work-items.md#create-work-item) — …
- … (all 8)

## Config & Maintenance

- [`/accelerator:configure`](../configuration.md#configure) — …
- [`/accelerator:init`](../configuration.md#init) — …
- [`/accelerator:migrate`](../migrations.md#migrate) — …
```

#### 3. README — link the index

**File**: `README.md`
**Changes**: Add the index as the first entry under the **Skills** heading
(link text = page H1 "All Skills"), so it leads the family-page links.

```markdown
**Skills**

- [All Skills](docs/skills/index.md) — the full index of every skill, grouped by
  family.
- [Planning](docs/skills/planning.md) — research, issue investigation, and plan
  review companions.
  …
```

### Success Criteria

#### Automated Verification

- [x] New test fails before the index exists, passes after:
      `bash scripts/test-skills-index.sh`
- [x] Full suite green: `mise run test:integration:config`
- [x] New script passes lint + exec-bit invariant: `mise run scripts:check`
      (shellcheck, shfmt, bashisms 3.2 floor, exec-bits)
- [x] Index references all 46 invokable skills and no internal skill (asserted
      by the test above)
- [x] Liveness gate holds: the test's derived invokable set is exactly 46
      (fails loudly on an enumeration/exclusion regression)
- [x] Every deep link resolves (all 46): the test confirms a `### <name>`
      heading exists on each target page (family AND concept pages)
- [x] Description-match holds: the test confirms each skill's index gloss and
      home-page "what it does" reproduce the first sentence of its `SKILL.md`
      `description` verbatim
- [x] The test's negative self-test passes (a deliberately broken temp index is
      reported FAIL), proving the assertions are not vacuous

#### Manual Verification

- [x] Index groups match the nine families; counts total 46.
- [x] README **Skills** section leads with the All Skills index and renders.
- [x] `review-system.md` cross-reference notes are present and correct.

---

## Testing Strategy

### Automated (TDD where applicable)

- `scripts/test-skills-index.sh` (written red-first in Phase 3) guards four
  machine-checkable invariants: index membership (all 46, no internal), the
  exact-46 count, deep-link anchor resolution for all 46, and
  description-to-`SKILL.md` match (first sentence verbatim). The index is then
  authored to green.
- All phases must keep `mise run test:integration:config` green (preserved grep
  contracts).
- The new shell script must pass `mise run check` (it is linted/typed shell).
- What remains manual-only: prose *quality* (does the "advice & guidelines"
  read well, is the page legible) and concept-page link-target *filenames* —
  the test checks anchors exist, not that prose is good.

### Manual

1. On GitHub, open each rewritten family page and each augmented concept page;
   confirm subsections read cleanly and the issue-tracker parity tables remain.
2. From the README **Skills** list, confirm the All Skills link works.
3. Skim the "advice & guidelines" blocks: present only where they add value,
   and accurate against each skill's actual behaviour.
4. Diff the rewritten pages against the originals to confirm no skill was
   dropped and no narrative content was lost from the concept pages.

## Migration Notes

None — documentation-only. No schema, data, or runtime changes. The new test is
additive and auto-discovered, so no `mise.toml`/`tasks/` wiring is required for
it to run. One optional consistency edit: `tasks/test/integration.py` holds an
at-least floor `_EXPECTED_CONFIG_SUITES = 19` over discovered `scripts/test-*.sh`
suites (currently 20 executable suites; the new test makes 21). Bumping the
floor to 21 keeps the exec-bit regression net tight — without it, the new
suite silently losing its exec bit would still satisfy the stale floor.

## References

- Work item: `meta/work/0176-per-skill-family-reference-docs.md`
- Parent epic: `meta/work/0145-documentation-improvements.md`
- Research: `meta/research/codebase/2026-06-29-0176-workflows-rename-and-skill-catalogue.md`
- Sibling (done): `meta/work/0175-slim-readme-and-split-into-docs-tree.md`
- Test harness: `tasks/test/helpers.py:17-44`, `tasks/test/integration.py:61-80`
- Model for the new test: `scripts/test-skill-frontmatter-conformance.sh`
- Grep contracts to preserve: `scripts/test-config.sh` (`work.integration`),
  `scripts/test-design.sh` (design tokens)
