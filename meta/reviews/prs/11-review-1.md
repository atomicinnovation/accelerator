---
type: pr-review
id: "11-review-1"
title: "Slim the README and split content into a docs/ tree (0175)"
date: "2026-06-29T13:44:41+00:00"
author: "Phil Helm"
producer: review-pr
status: complete
target: "pr:11"
reviewer: "Phil Helm <phil@go-atomic.io>"
verdict: "COMMENT"
lenses: [documentation, standards, test-coverage, architecture, usability]
review_number: 1
pr_number: 11
tags: []
last_updated: "2026-06-29T13:44:41+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

## Code Review: #11 - Slim the README and split content into a docs/ tree (0175)

**Verdict:** COMMENT

This is a clean, well-executed documentation-only relocation: content moves
verbatim, the README index covers all 14 new pages, link text matches each
destination H1 (AC6), relative links are correctly re-depthed, the
`#jira-integration` anchor is preserved, and heading hierarchy and kebab-case
filenames are consistent throughout. The one substantive issue is an
**incomplete cross-reference repair** — two `CHANGELOG.md` links still point at
README anchors this PR deleted (`#linear`, `#visualiser`), so they now
dead-end. That is below the configured `critical` threshold for
REQUEST_CHANGES, but it is a real regression of the same class the PR fixed for
`#jira-integration`, and worth fixing before merge. The remaining findings are
minor IA/navigation polish.

### Cross-Cutting Themes

- **Incomplete cross-reference repair** (flagged by: Documentation) — the PR
  repointed `CHANGELOG.md`'s `#jira-integration` link but left two sibling
  CHANGELOG links pointing at now-deleted README anchors (`#linear`,
  `#visualiser`). Verified directly: the slim README has no such anchors;
  the targets now live at `docs/skills/issue-trackers.md#linear` and
  `docs/visualiser.md`. Contradicts the AC6 "cross-references repaired" claim.
- **`meta/` directory concept split / index mis-attribution** (flagged by:
  Documentation, Architecture) — the README index attributes "the `meta/`
  directory" to *How It Works*, but the `meta/` deep-dive lives in
  *Internals* (which the index also describes as "the `meta/` directory
  deep-dive"). One concept, two index entries, pointing readers at the page
  that doesn't own it.
- **Navigation dead-ends in relocated docs** (flagged by: Usability,
  Architecture) — most new docs pages have no back-link to the README index or
  lateral nav, and a couple lose standalone context after the split
  (`planning.md`'s "this loop" has no antecedent; `development-loop.md` no
  longer points at its companion skills).

### Tradeoff Analysis

- **Per-page navigation vs single-source index**: Usability wants a back-link /
  nav footer on every docs page to fix dead-ends; Architecture notes the
  README-as-sole-index keeps one source of truth for the page list. A small
  consistent "← Documentation index" footer satisfies both without duplicating
  the index. Recommendation: add the footer, keep the README as the canonical
  list.

### Strengths

- ✅ Every page in the README Documentation index resolves to a real file;
  all 7 concept pages and 7 skills pages are covered with no orphans.
- ✅ Index link text matches each destination's H1 exactly (AC6 holds for the
  README itself).
- ✅ Relative links correctly re-depthed for the new location
  (`../../skills/...` from `docs/skills/`, `../README.md` from `docs/`).
- ✅ The released `#jira-integration` anchor was preserved via an explicit
  `<a id>` and the CHANGELOG link repointed to it.
- ✅ Heading hierarchy is clean (single H1, no skipped levels) and filenames
  are uniformly kebab-case, matching repo conventions.
- ✅ The two pre-existing README-coupled tests were repointed correctly — every
  repointed assertion was verified to hold against its new target file.
- ✅ The 0176 seam is well-designed: `docs/skills/` stubs use 0176's exact
  filenames and H1 titles, so the sibling work refines in place with no churn.

### General Findings

- 🟡 **Documentation**: Two `CHANGELOG.md` cross-references still point at
  deleted README anchors — `CHANGELOG.md:132` (`README.md#linear`) and
  `CHANGELOG.md:266` (`README.md#visualiser`). Repoint to
  `docs/skills/issue-trackers.md#linear` and `docs/visualiser.md`. (Outside the
  diff hunks, so reported here rather than inline.) Re-run
  `grep -rn "README.md#"` to confirm none remain.
- 🔵 **Test Coverage**: `scripts/test-format.sh`'s hyphenation hygiene sweep
  scans `README.md`/`CHANGELOG.md` but not `docs/`, so the bulk of relocated
  prose is no longer covered by that lint. Add `docs/` to its search roots
  (follow-up; not in this diff).
- 🔵 **Documentation**: `agents/browser-analyser.md` and
  `agents/browser-locator.md` preload-failure guards tell users the baseline is
  "recorded in the plugin README"; it moved to `docs/installation.md`. Soft
  prose reference (not a link); optional fix.
- 🔵 **Architecture**: `docs/installation.md` defers the *stable* install back
  up to the README (`../README.md`), so the page titled "Installation" isn't
  self-contained for the most common path. Acceptable tradeoff to avoid
  duplication; revisit if install friction is reported.
- 🔵 **Architecture**: No `docs/` index page — reachability of every docs page
  depends on manual README upkeep, with no link-coverage check to catch a
  future orphan. Fine at this size; consider an enforced invariant when 0177
  (docs site) lands.
- 🔵 **Test Coverage**: The doc-content-coupling tests (grep a string in a named
  file) are inherently brittle to future doc moves. Consider asserting the
  string exists anywhere under `docs/` (`grep -rq`) so the contract is "this is
  documented" rather than "documented in this exact file". (Follow-up.)
- 🔵 **Usability**: The prerelease channel now requires a click to
  `docs/installation.md`, but the README signposts it well ("where the newest
  features land first"). No action — just keep that framing on the link.
- 🔵 **Usability**: When 0176 refines the `docs/skills/` stubs, lead each with a
  one-line orientation sentence and a back-link to the relevant concept page.
- 🔵 **Standards**: Relocated content carries American spellings ("artifact",
  "behavioral") into the new docs, but the wider repo already mixes both forms,
  so this is a pre-existing inconsistency, not a breach introduced here.
  Normalise repo-wide in a dedicated follow-up if desired.
- 🔵 **Standards**: A handful of new-doc lines exceed 80 columns, all
  unwrappable (tables, code fences, URLs, inline-code links) — matching the
  original README. Markdown is not line-width-linted in this repo; no action.

## Inline Comments

### `README.md:50` — `meta/` directory attributed to the wrong index entry
**Severity**: minor | **Confidence**: high | **Lens**: documentation, architecture

🔵 **Documentation / Architecture**

The index attributes "the `meta/` directory" to *How It Works*, but
`docs/how-it-works.md` contains only Philosophy and VCS Detection — the `meta/`
deep-dive (the directory table and rationale) lives in `docs/internals.md`,
which the next-but-one entry (lines 57-58) already describes as "the `meta/`
directory deep-dive". The single concept is split across two pages and the
index points readers at the page that does *not* own it.

**Impact**: A reader following "How It Works" for `meta/` coverage lands on a
page that lacks it, and the same topic appears in two index descriptions — the
navigation contract doesn't match where the content cohesively lives.

**Suggestion**: Drop "the `meta/` directory" from the How It Works description
(e.g. "— the phase model and VCS detection."), leaving the deep-dive attributed
solely to the Internals entry.

---

### `docs/configuration.md:100` — only cross-tier sibling link in the docs tree
**Severity**: minor | **Confidence**: high | **Lens**: architecture

🔵 **Architecture**

This is the one place a Concepts-tier page (`docs/configuration.md`) links
sideways into a Skills-tier page (`docs/skills/review-system.md`). The target
resolves correctly, but it couples the two documentation tiers that the
`docs/` vs `docs/skills/` boundary otherwise keeps separate.

**Impact**: When 0176 refines `review-system.md`, this cross-tier reference
becomes a dependency that must be tracked across the boundary the split was
meant to avoid.

**Suggestion**: Acceptable as-is (the lens catalogue genuinely lives in
review-system.md), but flag it as the one intentional cross-tier edge so 0176
doesn't break the `skills/review-system.md` target.

---

### `docs/skills/planning.md:12` — "this loop" has no antecedent on a standalone page
**Severity**: minor | **Confidence**: high | **Lens**: usability

🔵 **Usability**

Relocated from the README "Development Loop" section, this page refers to "this
loop" ("Three complementary skills support this loop"), but the loop now lives
on a separate page (`docs/development-loop.md`) that this page never links to. A
reader arriving directly (via search or the README "Planning" link) has no
antecedent for "this loop".

**Impact**: The page no longer stands alone; the cross-reference dangles, making
the relationship between the companion skills and the core loop unclear.

**Suggestion**: Open with a sentence linking the loop, e.g. "These companion
skills complement the core [development loop](../development-loop.md):", and
rephrase "this loop" to reference it explicitly.

---

### `docs/development-loop.md:23` — no pointer to the companion/review skills
**Severity**: minor | **Confidence**: medium | **Lens**: usability

🔵 **Usability**

The original README section presented the core loop *and* the companion/review
skills together; the split sends the companions to `docs/skills/planning.md` but
this page ends at the three core steps with no pointer onward. A reader landing
here has no path to research-issue / review-plan / stress-test-plan /
validate-plan.

**Impact**: Discoverability loss — the previously-adjacent companion skills are
now invisible from the page a reader most naturally starts on.

**Suggestion**: Add a closing line such as "See [Planning](skills/planning.md)
for the issue-investigation, note-capture, and plan-review companions."

---

### `docs/how-it-works.md:1` — no back-link / lateral nav across the new docs tree
**Severity**: minor | **Confidence**: medium | **Lens**: usability

🔵 **Usability**

Most relocated pages (`how-it-works.md`, `development-loop.md`, `internals.md`,
`migrations.md`, `visualiser.md`, and all of `docs/skills/`) have no back-link
to the README or lateral nav to siblings — only `installation.md` and
`configuration.md` link outward. Once a reader follows an index link they are in
a dead end with no in-page way back or sideways.

**Impact**: Navigation friction across the whole new tree; a reader who lands on
a deep page (especially via search) must use the browser back button and cannot
discover related pages.

**Suggestion**: Add a consistent footer link such as
"← [Documentation index](../README.md#documentation)" to every docs page (or a
short `docs/README.md` landing page) so each page has at least one route back
into the tree.

---

### `README.md:19` — intro paragraph duplicates the relocated Philosophy
**Severity**: minor | **Confidence**: medium | **Lens**: architecture

🔵 **Architecture**

This new intro paragraph restates the Philosophy now relocated to
`docs/how-it-works.md` — same phases, same filesystem-not-conversation framing,
same context-window rationale. The README and how-it-works.md now carry
near-duplicate statements of the core thesis.

**Impact**: The single-source-of-truth for the central concept is split; a
future refinement must be applied in two places and they can drift (mild tension
with the PR's no-duplication intent).

**Suggestion**: A defensible "index teaser vs full treatment" pattern — just
keep the README paragraph deliberately shorter/distinct from the how-it-works
opening so the two don't converge into verbatim duplicates over time.

---

### `scripts/test-design.sh:51` — loose substring weakens mutation resistance
**Severity**: suggestion | **Confidence**: medium | **Lens**: test-coverage

🔵 **Test Coverage**

The assertion searches `docs/configuration.md` for the substring
`design-inventory`, which also matches the unrelated `design-inventories` token.
It passes for the right reason today (the template key is genuinely present),
but the loose match means a future edit removing the template key while leaving
any `design-inventories` mention would not be caught. This looseness is
inherited from the original README assertion, not introduced here.

**Impact**: Low — a latent mutation-resistance weakness, not a present defect.

**Suggestion**: Optionally tighten to the backtick-delimited key
(`` `design-inventory` ``) so the test fails specifically when the template-key
entry is removed. Not blocking for a docs-relocation PR.

---

## Per-Lens Results

### Documentation

**Summary**: A clean, well-executed documentation-only split — content
relocated verbatim, README index covers every new page, link text matches each
destination H1 (AC6), relative links correctly re-depthed, `#jira-integration`
anchor preserved. The one substantive gap: the CHANGELOG cross-reference
repointing was incomplete — two README anchor links (`#linear`, `#visualiser`)
were left untouched and now resolve to nothing.

**Strengths**: Index covers all 7 concept + 7 skills pages with no orphans;
link text == destination H1; relative links re-depthed correctly;
`#jira-integration` anchor carried over and CHANGELOG repointed; sub-headings
promoted consistently; local-checkout snippet cleanly relocated to CONTRIBUTING.

**Comments**: README.md:50 (meta/ index mis-attribution — suggestion/medium).

**General Findings**: 🟡 major — `CHANGELOG.md:132` (`README.md#linear`) and
`CHANGELOG.md:266` (`README.md#visualiser`) point at deleted anchors; repoint to
`docs/skills/issue-trackers.md#linear` and `docs/visualiser.md`. 🔵 minor —
browser agent preload guards reference a baseline "recorded in the plugin
README" that moved to `docs/installation.md`.

### Standards

**Summary**: Strong standards hygiene — kebab-case filenames, single H1 with no
skipped levels, aligned-pipe tables, hand-wrapped ~80-col prose preserved.
Spelling mixes British/American forms, but the repo is already inconsistent on
those words, so it is not a new breach. No blocking violations.

**Strengths**: Consistent kebab-case naming; correct heading hierarchy in every
file; table formatting preserved; CHANGELOG `#jira-integration` anchor repoint
sound; docs placement follows the documented IA decisions.

**Comments**: None.

**General Findings**: 🔵 suggestion — mixed British/American spelling carried in
verbatim (pre-existing repo inconsistency; normalise in a follow-up). 🔵
suggestion — over-80 lines are all unwrappable (tables/code/URLs); markdown is
not line-width-linted; no action.

### Test Coverage

**Summary**: The four repointed content-coupling assertions in test-config.sh
and test-design.sh were each verified to still hold against their new target
files, and no other shell/hook test was left asserting relocated README
content. The repointing is correct and preserves intent.

**Strengths**: `work.integration` confirmed in docs/skills/work-items.md;
`design-inventories/`/`design-gaps/` in docs/internals.md;
`design-inventory`/`design-gap` in docs/configuration.md; README grep fully
removed (no straggler); descriptions updated so failure messages name the right
file; no orphaned doc-coupling test elsewhere in scripts/ or hooks/.

**Comments**: scripts/test-design.sh:51 (loose substring — suggestion/medium).

**General Findings**: 🔵 minor — test-format.sh hyphenation sweep scans
README/CHANGELOG but not docs/, so relocated prose lost that lint coverage. 🔵
suggestion — doc-coupling tests are brittle to future doc moves; consider
`grep -rq` across docs/.

### Architecture

**Summary**: The README→docs split is well-structured: a slim README acts as the
index, links flow downward into docs/, and the docs/ (concepts) vs docs/skills/
(reference) boundary is principled. Concerns are minor: the `meta/` concept is
split across two pages with the index attributing it to the wrong one, one
cross-tier sibling link, and the README intro duplicates the relocated
Philosophy.

**Strengths**: Sound link direction (README is a pure index, no circular page
deps); principled concept-vs-reference boundary; well-designed 0176 seam (exact
filenames/H1s, no rename churn); anchor preservation keeps the external deep-link
contract stable; each page has a single clear responsibility.

**Comments**: README.md:50 (meta/ index attribution — minor/high);
docs/configuration.md:100 (cross-tier sibling link — minor/high); README.md:19
(intro duplicates Philosophy — minor/medium).

**General Findings**: 🔵 suggestion — installation.md defers the stable install
up to the README, so it isn't self-contained. 🔵 suggestion — no docs/ index;
reachability depends on manual README upkeep with no link-coverage check.

### Usability

**Summary**: The slimmed README is a strong onboarding artefact — pitch +
install + quickstart give fast time-to-first-success, and the grouped Concepts/
Skills index with per-link descriptions lets readers self-route. Main DX gaps:
navigation friction once on a docs page (no back-link/lateral nav on most
pages), and a couple of relocated pages that no longer stand alone.

**Strengths**: README answers what/how-to-start/where-next in 84 lines; index is
grouped and scannable with descriptions; link text == destination H1 (least
surprise); installation.md and configuration.md show the intended cross-link
pattern.

**Comments**: docs/skills/planning.md:12 ("this loop" dangling — minor/high);
docs/development-loop.md:23 (no companion pointer — minor/medium);
docs/how-it-works.md:1 (no back-link/nav across the tree — minor/medium).

**General Findings**: 🔵 suggestion — prerelease channel behind a click, but
well-signposted; no action. 🔵 suggestion — give each docs/skills page a one-line
orientation header when 0176 refines them.

---
*Review generated by /accelerator:review-pr*
