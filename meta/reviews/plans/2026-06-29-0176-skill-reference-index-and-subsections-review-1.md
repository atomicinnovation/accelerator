---
type: plan-review
id: "2026-06-29-0176-skill-reference-index-and-subsections-review-1"
title: "Plan Review: Skill Reference Index & Per-Skill Subsections"
date: "2026-06-29T20:29:39+00:00"
author: Phil Helm
producer: review-plan
status: complete
parent: "plan:2026-06-29-0176-skill-reference-index-and-subsections"
target: "plan:2026-06-29-0176-skill-reference-index-and-subsections"
reviewer: Phil Helm
verdict: REVISE
lenses: [documentation, test-coverage, correctness, standards, usability, architecture]
review_number: 1
review_pass: 1
tags: [docs, skills, documentation]
last_updated: "2026-06-29T20:29:39+00:00"
last_updated_by: Phil Helm
schema_version: 1
---

## Plan Review: Skill Reference Index & Per-Skill Subsections

**Verdict:** REVISE

This is a well-scoped, convention-aware documentation plan: it treats each
skill's `SKILL.md` frontmatter as canonical, closes three genuine
documentation gaps, sequences three independently-mergeable CI-green
increments in the correct dependency order (anchors before the index that
links them), and adds a TDD drift-guard for the one machine-testable
invariant. Its central factual claims are verifiably correct — the
user-invokable set is exactly **46** skills, the nine-family mapping sums to 46
and covers every invokable skill exactly once with no internal leakage, and
the prefix-collision boundary regex is sound. The reason for REVISE is not a
weak plan but a cluster of convergent gaps around **what the drift test
actually guards**: it verifies index *membership* but not anchor/link
resolution, description accuracy, or the 46-count, leaving most acceptance
criteria resting on manual click-through with no regression net — and the new
test's own enumeration/parsing correctness is under-specified against three
verified hazards (vendored `node_modules`, a frontmatter-less fixture, and
body-level `name:` lines in `configure/SKILL.md`).

### Cross-Cutting Themes

- **The drift guard checks membership, not link/anchor integrity**
  (flagged by: correctness, test-coverage, architecture, usability,
  documentation) — Five lenses independently converged here. The test asserts
  each `accelerator:<name>` token appears in `index.md` and no internal one
  does, but never verifies that a deep-link target (`work-items.md#create-work-item`)
  resolves to a real `### <name>` heading, that Phase 2 actually created the
  anchor, that family grouping is correct, or that the totals sum to 46. The
  most likely real defect — a Phase 2 heading renamed/omitted so an index
  link 404s on GitHub — passes the test green. This is the dominant theme and
  the cheapest to close: the test already enumerates `SKILL.md` and reads docs
  files, so a `### <name>`-exists assertion for family-page skills plus a
  `count == 46` self-test add the missing half of the contract with no new
  tooling.

- **The new test's enumeration & frontmatter parsing are under-specified**
  (flagged by: correctness [critical], standards, test-coverage) — Verified
  hazards: `find skills -name SKILL.md` with no exclusions returns 82 files
  (12 in `node_modules`, 1 frontmatter-less test-fixture); `configure/SKILL.md`
  carries three body-level `name:` lines (`compliance`, `work-item-style`) that
  a non-frontmatter-bounded `grep '^name:'` would treat as phantom invokable
  skills. The plan's pseudocode *does* name the `node_modules`/`test-fixtures`
  exclusions (step 1) — but it does **not** flag the body-`name:` hazard, and
  the cited model script (`test-skill-frontmatter-conformance.sh`) uses
  hardcoded fixtures and **does not dynamically enumerate at all**, so it
  offers no working glob to copy. Left unaddressed this either wedges the test
  permanently red or silently corrupts the invokable set the guard depends on.

- **Skill name + description now lives in 2–3 hand-authored copies with no
  guard on the text** (flagged by: documentation, usability) — After this plan
  each description exists in frontmatter (canonical), the family-page
  subsection, and the index bullet, yet the test only checks the invocation
  *token*, not that the prose matches. The likeliest long-term drift —
  reworded descriptions/argument-hints going stale in the index while the test
  stays green — is unguarded, and the plan never states which copy wins on
  disagreement.

- **Concept-page links are second-class and unevenly backed** (flagged by:
  usability, architecture, correctness, documentation) — 7 of 46 skills link
  to a whole page (`../development-loop.md`, `../internals.md`, …) with no
  per-skill anchor; three share one landing page, and `init` currently has
  only a one-line mention in `internals.md`. The index intro promises "each
  links to its reference section," which over-promises for these entries.

### Tradeoff Analysis

- **Deep-linkability vs scannability (issue-trackers.md)**: Converting the two
  compact Jira/Linear CRUD tables into 16 H3 subsections gives every verb a
  stable anchor but loses the side-by-side parity view that makes Jira↔Linear
  symmetry legible at a glance. Recommendation: keep a compact summary table at
  the top of each tracker section (also preserving grep tokens) *and* add H3
  subsections below — get both affordances rather than forcing one format.

- **Hand-authored index + drift test vs full generation**: The plan's choice is
  proportionate to the repo's grep-only docs tooling and explicitly defers
  generation to 0177 — a sound call. The cost is the description-drift and
  link-integrity gaps above; the mitigation is to widen the drift test, not to
  pull generation forward.

### Findings

#### Critical

- 🔴 **Correctness**: Skill enumeration must exclude `node_modules`/`test-fixtures` and parse `name`/`user-invocable` strictly within frontmatter
  **Location**: Phase 3, Section 1 — `scripts/test-skills-index.sh` (enumeration + frontmatter parsing)
  A no-exclusion `find` returns 82 SKILL.md (12 vendored, 1 frontmatter-less fixture), and `configure/SKILL.md` has three body-level `name:` lines. A faithful but careless port classifies these as invokable, demanding non-existent `accelerator:<name>` tokens (test wedged red) or corrupting the invokable set. *Verification note: the plan's pseudocode already names the `node_modules`/`test-fixtures` exclusions, so this is partly pre-empted — but the body-`name:` hazard is unflagged and the cited model script provides no enumeration pattern to copy.*

#### Major

- 🟡 **Test Coverage / Architecture / Usability / Correctness**: Drift guard verifies membership only — anchor resolution, link targets, and the 46-count are untested
  **Location**: Phase 3 Testing Strategy; Section 1 contract
  A renamed/omitted Phase 2 heading produces a dead in-page anchor that the green test accepts; "every index link resolves" has no regression net. Extend the guard to assert each family-page skill has a matching `### <name>` heading on its target page and that the derived invokable count equals 46 — both mechanical, bash-3.2-safe, no new tooling.

- 🟡 **Documentation / Usability**: Descriptions and argument-hints are copied by hand into 2–3 places with no drift guard on the text
  **Location**: Phase 2 subsections; Phase 3 Section 2 index bullets
  The largest body of copied text (46 index descriptions + ~40 subsection descriptions/usage strings) is guarded only by manual diffing, despite frontmatter being named canonical. Either keep index bullets descriptionless (link text + family suffices to navigate), or assert the frontmatter `argument-hint`/`description` appears in the corresponding entry — at minimum state which copy is authoritative.

- 🟡 **Documentation / Usability / Correctness**: Concept-page links resolve to page-top with uneven coverage; index intro over-promises
  **Location**: Phase 3 Section 2 — Development Loop / Config & Maintenance rows
  7 skills link to a whole page, not an anchor; `init` has only a one-line mention in `internals.md`. Soften the intro wording for these entries, pin `init`'s single home (configuration.md vs internals.md), and optionally add stable `<a id>` anchors to the concept pages.

- 🟡 **Architecture**: The "exactly one documented home per skill" invariant is asserted but unenforced
  **Location**: Desired End State; mapping table
  Nothing prevents a skill gaining a second home; the token-presence check passes regardless. Either downgrade to a documented editorial convention, or assert each skill's invocation token appears in exactly one `docs/` home page (index references are separate from home declarations).

#### Minor

- 🔵 **Test Coverage / Standards / Correctness**: Frontmatter extraction must be bounded to the first `---`…`---` block (not a repo-wide `grep '^name:'`), and a liveness/empty-name guard should fail loudly on malformed/absent frontmatter.
  **Location**: Phase 3 Section 1, step 2
- 🔵 **Test Coverage**: No in-suite negative self-test / wiring proof, unlike the model script (which mutates fixtures to prove it isn't green-only); the guard could rot into a vacuous pass.
  **Location**: Phase 3 Section 1
- 🔵 **Test Coverage**: Phases 1–2 carry no durable regression net before Phase 3 lands — the grep checklist items are one-shot, not suite assertions. Fold the three new-skill checks into the Phase 3 guard.
  **Location**: Phase 1 & 2 Success Criteria
- 🔵 **Standards**: New test should source `scripts/test-helpers.sh` and use its `PASS`/`FAIL`/`test_summary` convention rather than a bespoke printer.
  **Location**: Phase 3 Section 1
- 🔵 **Documentation**: `work-items.md` is currently 6 rows and must become 8 subsections; add explicit per-page expected-skill counts (work-items=8, issue-trackers=16, vcs-and-pr=4, adrs=3, design-convergence=2, planning=6) to the Phase 2 checklist so "no skill dropped" is verifiable.
  **Location**: Current State / Phase 1–2
- 🔵 **Documentation**: Enumerate `planning.md`'s six target subsections explicitly (research-issue, create-note, conduct-spike, review-plan, stress-test-plan, validate-plan) and add an anchor-existence manual check.
  **Location**: Phase 1 / Phase 2
- 🔵 **Documentation**: The "Advice & guidelines" block is the only net-new editorial content with no source of truth and no test; add a manual check that each claim matches the skill's behaviour, and prefer linking to existing prose over restating safety notes.
  **Location**: Phase 2
- 🔵 **Documentation**: README/index link text for the Development Loop group should reconcile with the destination H1 "The Development Loop" per the repo's link-text convention.
  **Location**: Phase 3 Section 3
- 🔵 **Usability**: The three-place update for adding a skill (frontmatter → family subsection → index) is undocumented; add an "Adding a skill" checklist to CONTRIBUTING.md and have the test's failure output name all three edits.
  **Location**: contributor workflow
- 🔵 **Usability**: H3-per-skill scales poorly for the 16 near-identical issue-tracker verbs (see Tradeoff Analysis).
  **Location**: Phase 2 — issue-trackers.md
- 🔵 **Architecture**: The nine navigational families diverge from the ~12 directory families with no machine-checkable binding; a new skill has no derivable navigational home. Acknowledge this is a manual editorial axis.
  **Location**: mapping table; Phase 3 Section 2
- 🔵 **Architecture**: Config & Maintenance fans out to three concept pages with `init` documented across two; pin its single home.
  **Location**: Phase 3 Section 2
- 🔵 **Architecture**: Treat the three review-system.md cross-reference notes as load-bearing — add a per-page Phase 2 success criterion that each review skill subsection links to review-system.md.
  **Location**: Phase 2 — review-system.md handling

#### Suggestions

- 🔵 **Standards**: Optionally bump `_EXPECTED_CONFIG_SUITES` from 19 to 20 in `tasks/test/integration.py` when the new suite lands, so a future dropped exec bit can't silently shrink the net.
  **Location**: Phase 3 Section 1
- 🔵 **Usability**: The index's nine families vs the README/family-pages' seven present two taxonomies; align them or add a one-line note explaining why some families live on concept pages.
  **Location**: mapping table

### Strengths

- ✅ Treats each skill's `SKILL.md` frontmatter as the canonical source and repeatedly mandates faithfulness to it.
- ✅ The "46 user-invokable skills" count and the nine-family mapping are **verified correct**: 69 SKILL.md − 23 internal (18 lenses + 3 output-formats + browser-executor + paths) = 46, the family counts sum to 46, and every invokable skill is mapped exactly once with no internal leakage.
- ✅ The prefix-collision boundary regex (`accelerator:<name>([^A-Za-z0-9-]|$)`) is sound — `-` is excluded from the trailing class so `init` is not satisfied by `init-jira`; all names use only `[a-z-]`.
- ✅ Red-first TDD sequencing is genuine: the test is written before the index and the criteria assert it fails before / passes after.
- ✅ Phase ordering respects the dependency direction (anchors before the index) and each phase is an independently-mergeable, CI-green increment.
- ✅ Closes three verified documentation gaps (conduct-spike, refine-work-item, stress-test-work-item).
- ✅ Preserves the load-bearing grep contracts (`work.integration`, design tokens) and calls them out explicitly.
- ✅ The new test is additive and auto-discovered by `test:integration:config` with no `mise.toml`/`tasks/` wiring — matches the established convention.
- ✅ Pragmatic progressive disclosure: thin skills get no padded "Advice & guidelines" block.

### Recommended Changes

1. **Widen the drift test to guard link/anchor integrity and the count**
   (addresses: "Drift guard verifies membership only", architecture anchor
   gap, correctness index-only finding). For each family-page-targeted skill,
   assert a matching `### <name>` heading exists on the linked page; assert the
   derived invokable count equals 46. Both reuse the enumeration/file-reads the
   test already does.

2. **Specify the test's enumeration and frontmatter parsing precisely**
   (addresses: the Critical finding + frontmatter-scoping minor). State that
   enumeration prunes `*/node_modules/*` and `*/test-fixtures/*`, that
   `name`/`user-invocable` are read only from the leading `---`…`---` block
   (so `configure`'s body `name:` lines are ignored), and add an empty-name
   liveness assertion. Note the model script is fixture-based and provides no
   glob to copy.

3. **Decide the authoritative copy for descriptions and state it** (addresses:
   "Descriptions copied by hand"). Prefer descriptionless index bullets, or add
   a frontmatter-match assertion; either way, document which copy wins.

4. **Clarify concept-page link expectations** (addresses: "Concept-page links
   second-class"). Soften the index intro for the 7 page-level links, pin
   `init`'s single home, and consider `<a id>` anchors on concept pages.

5. **Add per-page expected-skill counts and the planning.md/review-system
   anchor checks to the manual checklist** (addresses: work-items count,
   planning.md scope, review-system cross-ref). Makes "no skill dropped" and
   "anchors exist" concretely verifiable.

6. **Adopt the shared `test-helpers.sh` scaffolding and a negative self-test**
   (addresses: test-helpers convention + wiring-proof minors), mirroring the
   model script so the guard can't rot into a vacuous pass.

7. **Document the contributor "Adding a skill" workflow in CONTRIBUTING.md**
   (addresses: undocumented three-place update) and have the test failure
   output name all three required edits.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Documentation

**Summary**: A well-scoped, documentation-only plan that correctly identifies
the canonical source of truth (`SKILL.md` frontmatter), closes three genuine
gaps, and adds the right structural defence (a drift-guard test). Audience fit
and cross-link conventions are handled thoughtfully. Main risks: residual drift
the test does not cover (descriptions/argument-hints are still hand-copied), a
couple of link-target/anchor accuracy hazards, and internal inconsistencies
between the plan's count/mapping claims and the existing pages.

**Strengths**: canonical-frontmatter discipline; closes three verified gaps;
TDD drift-guard for the one testable invariant; preserves grep contracts and
names load-bearing strings; respects link-text and anchor conventions;
pragmatic brevity for thin skills.

**Findings**:
- 🟡 (high) *Phase 2 & 3 — subsections/index descriptions*: The drift test
  verifies token presence only, not that hand-authored descriptions /
  argument-hints / index summaries match each `SKILL.md`. The largest body of
  copied text is guarded only by manual diffing — the likeliest failure
  (silent description drift) passes green. Add a frontmatter-diff manual step
  for the index too, or assert the argument-hint appears in "How to use it".
- 🟡 (medium) *Phase 3 — concept-page link targets*: Concept-page skills link
  to the page, not an anchor; `development-loop.md` loop skills appear only in
  a narrative diagram and `init` has a single passing mention in
  `internals.md`. "Each links to its reference section" over-promises. Soften
  wording or add minimal anchored coverage (notably `init`).
- 🔵 (high) *Phase 3 — README link text*: Index group "Development Loop" links
  to a page whose H1 is "The Development Loop"; reconcile inline link text with
  the H1-title convention.
- 🔵 (high) *Current State / Phase 1 — work-items count*: Existing table has 6
  rows; Phase 1 adds 2 → 8, but the plan never states the baseline and the
  criteria only grep the two new tokens. Add per-page expected counts.
- 🔵 (medium) *Phase 1/2 — planning.md scope*: Enumerate planning.md's six
  target subsections explicitly and add an anchor-existence check.
- 🔵 (medium) *Phase 2 — Advice & guidelines*: The net-new editorial layer has
  no source of truth and no test; incorrect advice is worse than none. Add a
  manual accuracy check and prefer linking existing prose over restating it.

### Test Coverage

**Summary**: Correctly identifies index completeness as the sole
machine-testable artefact and sequences it red-first, modelling on an existing
conformance test. Coverage is proportional to a doc-only change. But the
guard's scope is narrower than implied: it verifies membership, not anchor
validity, link targets, the 46-count, or prose faithfulness — leaving most
acceptance criteria on manual verification with no regression net. The
enumeration logic also has unstated exclusion requirements that would silently
corrupt the invokable set.

**Strengths**: genuine red-first TDD loop; guard scoped to the one mechanically
verifiable invariant; reuses the established shell-test harness and
auto-discovery; pre-empts the prefix-collision hazard; bidirectional
(present-invokable AND absent-internal) assertion gives mutation resistance.

**Findings**:
- 🟡 (high) *Phase 3 §1 enumeration*: Must exclude `node_modules` and
  `test-fixtures` or the invokable set is corrupted; add a `count == 46`
  self-test so an exclusion regression fails loudly.
- 🟡 (high) *Testing Strategy*: Drift guard verifies membership only — anchors,
  link targets, and the 46-count are untested; the likely defect (renamed
  Phase 2 heading) passes green. Extend cheaply within the existing
  enumeration.
- 🔵 (medium) *Phase 3 §1*: Parse `user-invocable` from the YAML block, not the
  body; reuse the conformance test's bounded extraction and add a negative
  fixture.
- 🔵 (medium) *Phase 3 §1*: No negative self-test / wiring proof, unlike the
  model script — the guard could rot into a vacuous pass.
- 🔵 (low) *Phases 1 & 2*: No durable regression net before Phase 3; the greps
  are one-shot. Fold the three new-skill checks into the Phase 3 guard.

### Correctness

**Summary**: The plan's central factual claims hold up under verification: the
user-invokable set is exactly 46, the nine-family mapping sums to 46 and maps
every invokable skill exactly once (none missing, no internal leaked), and the
prefix-collision boundary regex correctly handles `init` vs `init-jira`. The
principal risk is the test's frontmatter extraction: the enumeration model does
not exclude `node_modules`/body `name:` lines, and several SKILL.md files will
be misclassified as invokable unless the parser is strictly frontmatter-bounded
and path-filtered.

**Strengths**: 46 count verified; mapping arithmetic and coverage verified
exact; boundary regex sound; internal-absent check collision-safe; phase
ordering logically correct for the anchor dependency.

**Findings**:
- 🔴 (high) *Phase 3 §1 enumeration + parsing*: `node_modules` (12 files), a
  frontmatter-less fixture, and `configure` body `name: compliance` /
  `name: work-item-style` lines will be misclassified as invokable unless
  enumeration is path-filtered and `name`/`user-invocable` are read only from
  the first `---`…`---` block. Otherwise the test never reaches green or its
  set diverges from reality.
- 🟡 (high) *Phase 3 §1 step 2 partition*: A SKILL.md with absent/malformed
  frontmatter falls through to "invokable" with an empty name; add a liveness
  guard that fails loudly (mirroring the model script's empty-literal guard).
- 🔵 (medium) *mapping / manual verification*: 7 concept-page links land at
  page top with no anchor; the test never verifies any target resolves. Note
  the test checks completeness, not link validity.
- 🔵 (low) *Phase 3 §1 step 3*: Presence in `index.md` counts a skill as
  documented even if its family-page subsection was never authored — a dangling
  deep-link the guard accepts.

### Standards

**Summary**: Unusually convention-aware for a docs change: correctly identifies
the bash 3.2 floor, exec-bit invariant, `scripts/test-*.sh` auto-discovery,
GitHub anchor derivation, and the grep-token contracts. Main risks: the new
test's enumeration must be frontmatter-scoped to avoid `configure` body
false-positives, and the index's `/accelerator:<name>` link text diverges from
the "link text = destination H1 title" rule for concept-page links.

**Strengths**: invokes bash 3.2 floor and names the exact gates (shellcheck,
shfmt, bashisms, exec-bit); identifies auto-discovery so no wiring needed;
models on the conformance test and its sourced helper; pre-empts the
prefix-collision hazard; honours grep-token preservation; correct anchor
derivation.

**Findings**:
- 🟡 (high) *Phase 3 §1 pseudocode step 2*: Enumeration must be
  frontmatter-scoped — `configure/SKILL.md` body has `name: compliance`
  (×2) and `name: work-item-style`, and browser-executor/paths bodies mention
  `user-invocable: false` in prose; a naive `grep '^name:'` invents phantom
  skills.
- 🔵 (medium) *Phase 3 §2 index*: `/accelerator:<name>` link text diverges from
  the repo's "link text = destination H1 title" convention for concept-page
  links (e.g. `[/accelerator:configure](../configuration.md)` → page H1
  "Configuration"). Note it as a deliberate catalogue convention or append
  destination context.
- 🔵 (medium) *Phase 3 §1*: Source `test-helpers.sh` and use its
  `PASS`/`FAIL`/`test_summary` gate rather than a bespoke printer.
- 🔵 (low) *Phase 1 §2*: New table rows exceed 80 cols but match the existing
  (over-80) table style and docs are unenforced — no change for tables; ensure
  new Phase 2 *prose* wraps at 80.
- 🔵 (suggestion, medium) *Phase 3 §1*: Optionally bump `_EXPECTED_CONFIG_SUITES`
  19 → 20 in `tasks/test/integration.py` so a dropped exec bit can't silently
  shrink the net.

### Usability

**Summary**: From a developer-experience standpoint the plan is well-structured:
a single discoverable "All Skills" index, deep-linkable anchors, and a
drift-guard against the most common rot. The principal risks are for the
*contributor* audience: the plan introduces a third hand-authored home for each
skill's name+description but guards only token presence, and never documents the
three-place update. Secondary reader friction comes from inconsistent link
destinations and from converting compact tables into long repetitive H3 runs.

**Strengths**: single discoverable landing index, linked first under README
Skills; clean stable anchors with explicit reasoning; well-judged progressive
disclosure; drift-guard closes the highest-value gap with a collision-aware
grep.

**Findings**:
- 🔴 (high) *Phase 3 §2 + Phase 2*: Skill name + description now lives in three
  hand-authored places; only the token is guarded, so the two prose copies
  drift on the first reword. Keep index bullets descriptionless or assert the
  description matches frontmatter; state which copy is authoritative.
- 🟡 (high) *Phase 3 §1/§2*: The index promises "each links to its reference
  section" but deep-link anchors are unguarded and silently rot. Assert the
  `### <name>` heading exists on the target page.
- 🟡 (medium) *mapping table*: Inconsistent link granularity — some entries land
  on a precise anchor, 7 on page-top, and three share one page. Add concept-page
  anchors or visually distinguish those entries.
- 🔵 (high) *contributor workflow*: The three-place update is undocumented;
  CONTRIBUTING.md has no mention of docs/skills. Add an "Adding a skill"
  checklist and have the test name all three edits.
- 🔵 (medium) *Phase 2 issue-trackers.md*: H3-per-skill scales poorly for the 16
  near-identical CRUD verbs; retain a compact parity table plus subsections.
- 🔵 (suggestion, medium) *mapping table*: Nine index families vs seven README
  family pages present two taxonomies; align them or add an explanatory note.

### Architecture

**Summary**: Structurally sound for a documentation change: three
independently-mergeable, CI-green increments with the correct dependency
ordering, and a hand-authored-plus-drift-test approach proportionate to the
repo's grep-only docs tooling (full generation deferred to 0177). Principal
weakness: the drift test guards only one half of the index's contract (set
membership), leaving link/anchor integrity — which the deep-linkable,
single-home design rests on — entirely to manual click-through. The nine-family
vs directory-family divergence and the Config & Maintenance three-page fan-out
are reasonable but hand-maintained with no machine binding.

**Strengths**: dependency-correct phase ordering; small-blast-radius increments;
explicitly-acknowledged generation tradeoff; canonical-frontmatter-derived skill
set; additive auto-discovered test.

**Findings**:
- 🟡 (high) *Phase 3 §1*: The guard validates membership but never that an index
  link resolves to a real anchor; the core deep-link invariant has no automated
  guard. Parse each `<page>.md#<anchor>` target and assert a matching `###`
  heading exists.
- 🟡 (medium) *Desired End State / mapping*: "Exactly one documented home per
  skill" is asserted but unenforced. Downgrade to a convention or assert each
  invocation token appears in exactly one home page.
- 🔵 (medium) *mapping / Phase 3 §2*: Nine navigational families diverge from
  ~12 directory families with no machine-checkable binding; a misplaced new
  skill passes the test. Acknowledge it's a manual editorial axis.
- 🔵 (medium) *Phase 3 §2 Config & Maintenance*: Fans `configure`/`init`/`migrate`
  to three concept pages with no anchors; `init` is documented across two pages.
  Pin its single home; optionally add `<a id>` anchors.
- 🔵 (low) *Current State / Phase 2 review-system.md*: The review skills' docs
  are split across two pages by design; treat the three cross-reference notes
  as load-bearing and assert each review subsection links to review-system.md.
