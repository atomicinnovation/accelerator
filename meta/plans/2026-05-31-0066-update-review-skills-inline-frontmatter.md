---
date: "2026-05-31T22:54:08+00:00"
type: plan
skill: create-plan
work_item_id: "0066"
status: accepted
---

# Move Review/Validation Skills' Frontmatter into Templates Implementation Plan

## Overview

Move the YAML frontmatter that the four review/validation skills (`review-plan`,
`review-work-item`, `review-pr`, `validate-plan`) currently bake inline in
their SKILL.md prose into template files under `templates/`, conforming to
the unified base schema (ADR-0033) and typed-linkage vocabulary (ADR-0034),
and rewire each skill to read its frontmatter from the template via the
canonical loader (`config-read-template.sh`). Three of the four artifact
types have no template file today — `plan-review`, `work-item-review`,
`pr-review` — and are created here; `plan-validation` reuses
`templates/validation.md` (whose unified frontmatter block was added by
0065). After this story lands, all ten in-source artifact-producer skills
read frontmatter from `templates/`, so a future schema change touches only
template files.

The work is decomposed into one foundation phase (test-scaffolding
extension that lands RED), four independent per-skill phases that move
the four skills from RED to GREEN, a discovery-pass phase that records
the post-rewire producer set, and a closing visualiser consumer-update
phase that teaches the Rust indexer/frontmatter readers to accept the
typed-linkage `target:` form (`"plan:<id>"`, `"work-item:<id>"`) and
to extract work-item ids from `target:` values. Every phase follows
test-driven development: tests are extended first and start RED;
implementation moves them to GREEN.

## Current State Analysis

All four SKILL.md files instruct the model to emit a YAML frontmatter
block inline today:

- `skills/planning/review-plan/SKILL.md:412-446` — `plan-review` inline block
- `skills/work/review-work-item/SKILL.md:346-381` — `work-item-review` inline block (also emits both `target` (path) and `work_item_id` — duplicates the same edge)
- `skills/github/review-pr/SKILL.md:448-496` — `pr-review` inline block (uses synthetic `target: "PR #{n}"`; emits legacy `pr_title:` and no `review_pass`)
- `skills/planning/validate-plan/SKILL.md:133-144` — `plan-validation` inline block (already loads `templates/validation.md` at line 116 for the report body, but re-specifies frontmatter inline)

The post-0065 ground state is:

- `templates/validation.md` already carries the unified frontmatter block
  with `result: ""` and `target: ""` slots awaiting population by
  `validate-plan` (`templates/validation.md:1-15`).
- `scripts/templates-schema.tsv` and `scripts/skills-schema.tsv` exist and
  are TSV-driven sources of truth for the two test drivers.
- The four 0066 skill paths are pre-listed in `OWNED_BY_0066` in
  `scripts/test-skill-frontmatter-population.sh:44-49`; they currently sit
  in the allowlist so the Phase-11 discovery pass does not fail.
- The Phase-11 discovery patterns at `test-skill-frontmatter-population.sh:160-195`
  already include `verdict`, `review_pass`, `target`, `result`, `pr_number`,
  `producer`, `schema_version` — anticipating 0066.
- `meta/work/0065-...md` carries the Schema Reference table that
  `test-template-frontmatter.sh:147-169` cross-checks against the TSV;
  that table currently has 9 rows and must remain in sync with the TSV.

The corpus migration (0070) handles existing `meta/` files; this story
explicitly does not touch `meta/` review/validation artifacts.

## Desired End State

After this plan is complete:

- Three new template files exist under `templates/` —
  `plan-review.md`, `work-item-review.md`, `pr-review.md` — each emitting
  the unified base fields, the per-artifact extras pinned by ADR-0033, and
  a `target` typed-linkage key per ADR-0034.
- The four review/validation skills read their frontmatter via
  `!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh <name>`` rather
  than emitting it inline; each SKILL.md contains no inline YAML
  frontmatter block listing two or more base or extra field names as
  `key: value` pairs outside fenced template-example blocks.
- The Phase-11 discovery assertion in
  `test-skill-frontmatter-population.sh` shows the four 0066 paths in
  `IN_SCOPE_PRODUCERS` (not `OWNED_BY_0066`).
- `scripts/templates-schema.tsv` lists 12 rows total (the 9 from 0065 plus
  the 3 new review templates); `scripts/skills-schema.tsv` lists 14 rows
  total (the 10 from 0065 plus the 4 0066 skills).
- The 0066 work item carries its own `## Schema Reference` table for the 3
  new templates, and `test-template-frontmatter.sh`'s cross-check reads
  the union of both tables.
- A reproducible discovery-pass record is appended to the 0066 work item;
  re-running the recorded greps produces the same producer split.
- `mise run test:unit:templates` passes.

Verification: `mise run test:unit:templates` passes; the four skill rows
and three template rows added in Phase 1 are GREEN; the recorded discovery
greps produce the recorded output verbatim.

### Key Discoveries:

- Template-shape, skill-prose, and helper-output test scaffolding already
  exists from 0065 (`scripts/test-template-frontmatter.sh`,
  `test-skill-frontmatter-population.sh`, `test-metadata-helpers.sh`) and
  is TSV-driven — 0066 extends rows rather than building new harnesses.
- The canonical template-inclusion pattern (`!`config-read-template.sh
  <name>`` near the top of the skill) and the canonical persistence-step
  prose snippet (with `Substitute` verb + per-field bullets) were
  established by 0065 (see `meta/plans/2026-05-30-0065-...md` §"Canonical
  persistence-step prose snippet"). 0066 reuses both verbatim.
- `templates/validation.md:1-15` is the closest analogue to the three
  review templates: same `target: ""` typed-linkage slot, same
  `status: complete`-only vocabulary, same not-code-state-anchored
  property. The review templates differ from it only by adding more
  extras (`reviewer`, `verdict`, `lenses`, `review_number`,
  `review_pass`) and by the `target` payload shape.
- Review/validation artifacts are not code-state-anchored: they describe
  a target artifact, not the codebase at a point in time. They omit
  `revision:` and `repository:`.
- `config-read-template.sh` already returns the raw template content
  (wrapped in markdown fences when not already fenced) — sufficient for
  0066 with no helper extension required (`scripts/config-read-template.sh:36-50`).
- The Phase-11 discovery patterns (`test-skill-frontmatter-population.sh:160-195`)
  already include every literal 0066 will retire from inline prose
  (`verdict`, `review_pass`, `target`, `result`, `pr_number`); after
  rewiring, those literals appear only inside fenced template-example
  blocks, which the discovery pattern still matches — every hit must
  resolve to a SKILL.md in `IN_SCOPE_PRODUCERS` post-rewire.
- The verdict-enum inconsistency (`review-plan` and `review-work-item`
  use `APPROVE | REVISE | COMMENT`; `review-pr` uses
  `APPROVE | REQUEST_CHANGES | COMMENT`) is preserved verbatim — verdict-
  enum alignment is explicitly excluded by parent epic 0057.

## What We're NOT Doing

- Touching existing `meta/` review/validation artifacts (the corpus
  migration 0070 owns that). Two narrow carve-outs are accepted within
  this story (both additive, neither a frontmatter rewrite):
  - **Phase 1 appends a `## Schema Reference` section to
    `meta/work/0066-update-review-skills-inline-frontmatter.md`** so the
    test-template cross-check has a per-story authority for the 3 new
    templates (mirroring the 0065 precedent).
  - **Phase 6 appends a `## Discovery Pass Record` section to the same
    work item** recording the post-rewire producer split.
  These carve-outs do not extend to any other `meta/` file.
- Aligning verdict enums across the three review types — explicitly out of
  scope per the parent epic 0057.
- Adding optional typed-linkage slots (`blocks`, `blocked_by`,
  `derived_from`, `relates_to`) to the three new review templates —
  sibling story 0093 owns that and should land after 0066.
- Introducing a re-review lifecycle (in-place artifact update flow) in
  `review-pr/SKILL.md` — `review-pr` keeps its current
  no-`review_pass`, no-in-place-update behaviour. The template omits
  `review_pass` accordingly (see Design Decisions below).
- Extending `config-read-template.sh` — the helper already returns raw
  template content, which is sufficient for 0066. The work item's
  helper-extension carve-out is closed by this plan as "not needed".
- Adding new test scaffolding — 0065 already built
  `test-template-frontmatter.sh`, `test-skill-frontmatter-population.sh`,
  `test-metadata-helpers.sh`, and the `test:unit:templates` mise task.
  0066 extends rows, not infrastructure.
- Updating prose in the four skills beyond what is required to retire the
  inline frontmatter block and add the canonical persistence-step snippet
  — re-review-flow prose, lens-selection prose, JSON-extraction prose,
  etc. remain untouched.

## Implementation Approach

Test-driven, mirroring 0065 phase-by-phase:

1. **Phase 1** extends the TSV-driven test scaffolding to surface the
   three new templates and four rewired skills as RED rows. After Phase 1:
   `bash scripts/test-template-frontmatter.sh` fails for 3 missing
   templates; `bash scripts/test-skill-frontmatter-population.sh` fails
   for 4 not-yet-rewired skills.
2. **Phases 2-5** are independent per-skill phases. Each creates the
   template (or reuses validation.md), retires the inline block from the
   SKILL.md, and adds the canonical persistence-step snippet. After each
   phase, the corresponding template-shape row and skill-prose row turn
   GREEN.
3. **Phase 6** re-runs the discovery assertion, appends the recorded
   producer split to the 0066 work item, and applies a small AC edit to
   the work item to reflect the `review_pass`-on-pr-review decision (see
   Design Decisions).
4. **Phase 7** updates the visualiser server (Rust) to consume the new
   typed-linkage `target:` shape on `plan-review` and `work-item-review`
   artifacts, and to extract work-item ids from `target:` values so the
   transitional `work_item_id:` alias can be retired in a follow-up.

Phases 2-5 are file-independent (no two phases edit the same file) but
not runtime-independent. Ordering constraints:

- **Phase 1 must land first** — it provides the failing tests every later
  phase satisfies and updates the cross-check to read both work-item
  Schema Reference tables.
- **Phases 2, 3, 4, 5 may be parallelised** by different engineers once
  Phase 1 has landed; they touch disjoint files (one new template + one
  SKILL.md per phase, plus the `evals/` files in Phase 3).
- **Phase 6** must land after Phases 2-5 — it depends on Phases 2-5
  having retired the inline frontmatter from every SKILL.md.
- **Phase 7's consumer-update is back-compatible** (the refactored
  `target_path_from_entry` accepts both the existing path-form and the
  new typed-linkage `doc-type:id` form), so Phase 7 may land *before*
  Phases 2-3 without breaking the existing path-form consumers — and
  doing so avoids any divergence window where plan-reviews emit typed-
  linkage `target:` but the visualiser only resolves path-form. The
  team is encouraged to land Phase 7 first; if not, **all seven phases
  must ship in the same release** to avoid recreating the silent-orphan
  window flagged in review-1. Either way, Phase 7 is independent of
  Phases 4-6 and may be parallelised with them.

### Design Decisions

Five design decisions made during planning (recorded here so the plan is
self-contained):

1. **`review-pr` omits `review_pass`** (deviation from work item AC #5).
   The `review-pr` skill has no `review_pass` field today and no in-place
   re-review update flow — re-running it writes a fresh
   `-review-{N+1}.md` rather than mutating the prior file. Introducing a
   `review_pass` lifecycle is out of scope for this story. The
   `pr-review.md` template therefore omits `review_pass`; Phase 6 amends
   work item AC #5 to read "`review_pass` is emitted by `plan-review` and
   `work-item-review` only".

2. **`work-item-review` keeps `work_item_id` as a transitional alias
   alongside `target`**. The current skill emits both `target` (path) and
   `work_item_id` (4-digit identifier). The unified emission adopts
   `target: "work-item:<id>"` per ADR-0034, but retains `work_item_id:`
   as a transitional alias because the visualiser server's `read_ref_keys`
   (`skills/visualisation/visualise/server/src/frontmatter.rs:330`) reads
   `work_item_id:` as the primary scalar key for work-item
   cross-reference aggregation across all artifact types — dropping it
   would silently break the work-item detail page's "Referenced By"
   section for any work-item-review emitted post-0066. The transitional
   alias mirrors the existing `work-item:` legacy fallback at
   `frontmatter.rs:334-341` and will be removed by the visualiser
   consumer-update tracked in Phase 7. Per ADR-0033 §Identity-value
   shape contract, relationship-named keys (`target`) are excluded from
   the `<type>_id` rule, so both keys coexist legally.

3. **`pr-review.target` uses `"pr:<pr-number>"`**, pinned to the regex
   `^"pr:[0-9]+"$`. ADR-0034's published vocabulary does not include a
   `pr:` doc-type discriminator today (the listed prefixes cover only
   meta-artifact types). Using `pr:` keeps the `target` shape uniform
   across all four review/validation types — important for the future
   visualiser-graph epic. The plan queues a **new supplementary ADR**
   under 0057's open items to extend ADR-0034's vocabulary with `pr` (and
   any other external-entity prefixes the corpus needs); the
   supplementary ADR sits alongside ADR-0034 rather than amending it.
   This story does not block on that follow-up.

4. **`config-read-template.sh` is not extended**. The helper already
   returns raw template content wrapped in markdown fences
   (`scripts/config-read-template.sh:36-50`) — exactly what 0066 needs.
   The work item's helper-extension carve-out is closed here as "not
   needed"; should a real need surface during implementation, file a
   follow-up rather than expanding this story's scope.

5. **Re-review mutation drops `date` from the mutation list; only
   `last_updated`/`last_updated_by` advance.** The existing re-review
   flows for `review-plan` and `review-work-item` bumped `date` on
   every re-review pass (3-field mutation: `verdict`, `review_pass`,
   `date`). The unified schema introduces `last_updated` and
   `last_updated_by` as the canonical mutation timestamp fields
   (ADR-0033). Carrying `date` in the mutation list as well would
   conflate creation and last-mutation timestamps. Per ADR-0033's
   convention (`date` = creation timestamp, `last_updated` =
   mutable), the four-field mutation set is `verdict`, `review_pass`,
   `last_updated`, `last_updated_by` — `date` retains the
   original-review timestamp across all re-review passes. This is a
   behavioural change relative to pre-0066 behaviour; Migration Notes
   record it explicitly.

### Token-marker and fence syntax conventions

The work item review-1 Pass 2 flagged "unsubstituted template tokens"
and "outside fenced template-example blocks" as undefined. Pinning these
conventions for the plan's verification steps:

- **Template tokens use single-brace mustaches** `{...}` (e.g.
  `{ISO timestamp}`, `{filename-stem}`, `{plan-id}`) — matching the
  existing token style in `templates/validation.md`, `templates/plan.md`,
  `templates/work-item.md` and every template 0065 touched. A regex
  detecting unsubstituted tokens is `\{[^{}\n]+\}` against the
  frontmatter block of a generated artifact.
- **"Outside fenced template-example blocks" means outside triple-
  backtick fenced code blocks** in the SKILL.md (` ```yaml `,
  ` ```markdown `, or bare ` ``` `). The Phase-1 SKILL-prose test's
  `in_fenced_block` helper already handles this exactly
  (`scripts/test-skill-frontmatter-population.sh:77-87`). The grep recipe
  for the manual verification step strips fenced blocks first.

### Canonical template-inclusion + persistence-step pattern (reused from 0065)

Every per-skill phase (2, 3, 4, 5) adds two things to its target SKILL.md:

1. **Template-inclusion block** near the top of the skill (after the
   `config-read-context.sh` / `config-read-skill-context.sh` /
   `config-read-agents.sh` calls, before Step 1). Use this exact prose:

   > ## {Artifact-Type} Template
   >
   > The template below defines the frontmatter and (where applicable)
   > body structure that every {artifact-type} must carry. Read it now
   > — use it to guide what information you gather in the review/
   > validation steps and what shape you persist in the final step.
   >
   > !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh {template-name}`

2. **Canonical persistence-step snippet** replacing the inline
   frontmatter block. Apply 0065's canonical snippet verbatim with the
   per-phase parameters (see each phase). Wrapped reminder of the shape:

   > ### Step N: Populate frontmatter
   >
   > Before writing the {artifact-type} file, capture metadata and
   > substitute the unified base fields and per-type extras into the
   > template's frontmatter block:
   >
   > 1. Invoke `${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh`
   >    to obtain `Current Date/Time (UTC):`.
   > 2. **Substitute** every field below with the indicated value:
   >    - `type:` ← `{type-literal}`
   >    - `id:` ← `{id-source}` (always quoted as a YAML string)
   >    - `title:` ← `{title-source}`
   >    - `date:` ← the `Current Date/Time (UTC):` value
   >    - `author:` ← the author value resolved per `create-work-item/SKILL.md:578-580`
   >    - `producer:` ← `{producer-name}` (this skill's name, literally)
   >    - `status:` ← `complete`
   >    - `last_updated:` ← the same `Current Date/Time (UTC):` value
   >    - `last_updated_by:` ← the same value resolved for `author`
   >    - `schema_version:` ← `1` (bare integer, not quoted)
   >    - `target:` ← `{target-shape}` (typed-linkage key per ADR-0034; always emitted as a single quoted YAML string in `"doc-type:id"` form)
   >    - `reviewer:` ← `{reviewer-source}` *(review templates only)*
   >    - `verdict:` ← `{verdict-source}` *(review templates only)*
   >    - `lenses:` ← `{lenses-source}` *(review templates only)*
   >    - `review_number:` ← `{review-number-source}` *(review templates only)*
   >    - `review_pass:` ← `{review-pass-source}` *(plan-review and work-item-review only — pr-review omits this field)*
   >    - `work_item_id:` ← `{work-item-id-source}` (4-digit identifier, same value as the `target` payload's id portion) *(work-item-review only — transitional alias per Design Decision #2)*
   >    - `pr_number:` ← `{pr-number-source}` (bare integer; foreign reference to the external PR per ADR-0033 §Identity-value shape contract) *(pr-review only)*
   >    - `result:` ← `{result-source}` *(plan-validation only)*
   > 3. Write the file with the substituted frontmatter block.

   Heading convention: `{Artifact-Type}` in the template-inclusion
   heading (`## {Artifact-Type} Template`) follows Title Case with
   preserved acronyms — i.e. `Plan Review`, `Work Item Review`,
   `PR Review` (not `Pr Review`), `Plan Validation`.

Per-skill phases below list each `{braces}` parameter explicitly. Do not
rewrite the snippet shape per phase — the SKILL-prose test relies on
the canonical verb (`Substitute`) and per-field bullet shape.

### Inline-comment policy for the three new templates

Follow the 0065 inline-comment convention verbatim
(`meta/plans/2026-05-30-0065-...md` §"Template inline-comment convention"):

- **Comment required** on:
  - `type:` (cite ADR-0033)
  - `status:` (enumerate the per-type vocabulary verbatim —
    `complete` for all four artifact types)
  - `schema_version:` (cite ADR-0033 §Schema versioning)
  - `target:` (cite ADR-0034; state typed-linkage `"doc-type:id"`
    form)
  - `work_item_id:` (transitional alias note + cite Design
    Decision #2) — work-item-review only
  - `pr_number:` (cite ADR-0033 §Identity-value shape contract;
    state "bare integer; foreign reference to the external PR")
    — pr-review only
- **Comment recommended** on per-type extras whose shape is
  non-obvious:
  - `verdict:` (list the enum values verbatim — e.g.
    `APPROVE | REVISE | COMMENT` for plan-review/work-item-review;
    `APPROVE | REQUEST_CHANGES | COMMENT` for pr-review)
  - `lenses:` (describe the list shape — array of lens names)
  - `review_number:` (one-line purpose note — "which review of
    this target; 1-indexed")
  - `review_pass:` (one-line purpose note — "latest re-review
    pass within this review_number")
  - `result:` (list the enum values verbatim —
    `pass | partial | fail`) — plan-validation only
- **Comment omitted** on self-evident base fields (`id`, `title`,
  `date`, `author`, `producer`, `tags`, `last_updated`,
  `last_updated_by`, `reviewer`).

---

## Phase 1: Test Scaffolding Extension (RED Baseline)

### Overview

Extend the TSV-driven test scaffolding 0065 built so the three new
templates and four 0066 skills appear as failing rows, and update the
test-template cross-check to read both 0065's and 0066's Schema
Reference tables. After this phase: `bash scripts/test-template-frontmatter.sh`
fails on 3 rows; `bash scripts/test-skill-frontmatter-population.sh`
fails on 4 skills × 4 mandatory fields each (16 failures). This is the
RED baseline.

### Changes Required:

#### 1. Extend `templates-schema.tsv`

**File**: `scripts/templates-schema.tsv`
**Changes**: Append three rows for the new review templates. Per-row
fields: template • type • code_state_anchored • extras • status_vocab •
forbidden_own_id_key.

```
plan-review.md	plan-review	no	reviewer verdict lenses review_number review_pass target	complete	-
work-item-review.md	work-item-review	no	reviewer verdict lenses review_number review_pass target work_item_id	complete	-
pr-review.md	pr-review	no	reviewer verdict lenses review_number target pr_number	complete	pr_title review_pass
```

Notes:

- All three are not code-state-anchored (review artifacts describe a
  target, not the codebase at a point in time).
- All three status vocabularies are `complete` — review artifacts are
  written once per review pass with frontmatter reflecting the latest
  pass state.
- `pr-review` omits `review_pass` from its extras (per Design Decision
  #1) and adds `pr_number` (a foreign reference to the external PR).
- `work-item-review.md`'s extras include `work_item_id` (per Design
  Decision #2 — kept as a transitional alias alongside the typed
  `target`). The `forbidden_own_id_key` is therefore `-`; there is no
  legacy key to guard against on this template.
- `pr-review.md`'s `forbidden_own_id_key` is `pr_title review_pass`
  (space-separated multi-value — see Phase 1 §1.5 schema extension). The
  existing skill carries `pr_title:` inline (retired to base `title:`,
  closing the 0065 → 0066 rename carryover); `review_pass:` must remain
  absent per Design Decision #1.
- `plan-review.md`'s `forbidden_own_id_key` is `-` (no legacy key to
  guard against).

#### 1a. Extend `templates-schema.tsv` schema to support multi-value `forbidden_own_id_key`

**File**: `scripts/templates-schema.tsv` (header) and
`scripts/test-template-frontmatter.sh` (parser)
**Changes**: The existing `forbidden_own_id_key` column accepts a
single key or the `-` sentinel. `pr-review.md`'s row needs to forbid
both `pr_title:` and `review_pass:`, so the column semantics extend to
a space-separated list (preserving `-` as the sentinel for "no
forbidden key").

The TSV column itself is unchanged in shape (still one tab-separated
field). The change is in the test driver's parser: the
`assert_not_in_block` invocation (around `test-template-frontmatter.sh`
line where the forbidden-key check runs) splits the column on
whitespace and iterates, asserting each listed key is absent from the
template's frontmatter block. Single-value rows (e.g.
`work_item_id`) behave identically to the pre-change behaviour; the
`-` sentinel still short-circuits the check.

The 0065 cohort's rows that carry single-value forbidden keys
(`work-item.md` → `work_item_id`, `pr-description.md` → `pr_title`,
`adr.md` → `adr_id`) need no change — they remain space-tokenised
lists of length one.

#### 1b. Add `target` to `validation.md`'s TSV extras

**File**: `scripts/templates-schema.tsv`
**Changes**: Update the existing `validation.md` row's extras column
from `result` to `result target`. The template file already carries a
`target: ""` slot (added by 0065); this row update brings the TSV
contract into alignment with the file shape and ensures the
template-shape test fails if a future change removes the `target:`
slot from `templates/validation.md`. The row is otherwise unchanged.

#### 2. Extend `skills-schema.tsv`

**File**: `scripts/skills-schema.tsv`
**Changes**: Append four rows for the rewired skills. Per-row fields:
skill_path • producer_name • fields_to_assert.

```
skills/planning/review-plan/SKILL.md	review-plan	producer schema_version last_updated last_updated_by target reviewer verdict lenses review_number review_pass
skills/work/review-work-item/SKILL.md	review-work-item	producer schema_version last_updated last_updated_by target reviewer verdict lenses review_number review_pass work_item_id
skills/github/review-pr/SKILL.md	review-pr	producer schema_version last_updated last_updated_by target reviewer verdict lenses review_number pr_number
skills/planning/validate-plan/SKILL.md	validate-plan	producer schema_version last_updated last_updated_by target result
```

The `fields_to_assert` set extends 0065's non-code-state-anchored
producer base (`producer schema_version last_updated last_updated_by`)
with the per-type extras each skill is required to instruct
substitution of in its persistence step. This closes the gap flagged
by review-1 where dropping a `verdict:` or `target:` bullet from a
SKILL.md's persistence snippet would have silently passed the SKILL-
prose test. Review/validation artifacts do not carry `revision` or
`repository`, so those fields are not asserted.

Per-row rationale:
- `review-plan` / `review-work-item`: all five review extras
  (`target`, `reviewer`, `verdict`, `lenses`, `review_number`,
  `review_pass`) are asserted; `review-work-item` additionally
  asserts `work_item_id` (transitional alias per Design Decision #2).
- `review-pr`: asserts all review extras *except* `review_pass`
  (omitted per Design Decision #1), plus `pr_number`.
- `validate-plan`: asserts `target` and `result` only (no review
  extras — validation artifacts are not reviews).

The TSV's 3-column schema (skill_path • producer_name •
fields_to_assert) is unchanged; only the `fields_to_assert` values
grow per row.

#### 3. Allowlist stays unchanged in Phase 1; paths move per-skill

**File**: `scripts/test-skill-frontmatter-population.sh`
**Changes in Phase 1**: **None**. The four 0066 paths remain in
`OWNED_BY_0066` (lines 44-49) throughout Phase 1 and through any
Phases 2-5 that have not yet landed. Each per-skill phase (Phase 2, 3,
4, 5) moves *its own* path from `OWNED_BY_0066` to
`IN_SCOPE_PRODUCERS` as its first step. This keeps the discovery
assertion meaningful through the entire rewire window: a SKILL.md
in `IN_SCOPE_PRODUCERS` actually carries a `config-read-template.sh`
inclusion and the canonical persistence snippet; a SKILL.md still in
`OWNED_BY_0066` is allowlisted as a known pre-rewire emitter and
exempt from the per-field assertions.

The Phase-1 RED baseline is therefore:
- `IN_SCOPE_PRODUCERS` = 10 entries from 0065 (unchanged from current
  script state).
- `OWNED_BY_0066` = the same 4 entries it carries today (review-plan,
  review-work-item, review-pr, validate-plan) — also unchanged.

The four new TSV rows in `skills-schema.tsv` (Phase 1 §2) still fail
the per-field assertions because the four skill paths are not yet
in `IN_SCOPE_PRODUCERS`; the test driver's per-skill assertion loop
runs only over `IN_SCOPE_PRODUCERS`. The TSV rows turn GREEN
incrementally as each per-skill phase moves its path across.

After all four per-skill phases land:
```bash
IN_SCOPE_PRODUCERS=(
  skills/work/create-work-item/SKILL.md
  skills/work/extract-work-items/SKILL.md
  skills/planning/create-plan/SKILL.md
  skills/github/describe-pr/SKILL.md
  skills/decisions/create-adr/SKILL.md
  skills/decisions/extract-adrs/SKILL.md
  skills/research/research-codebase/SKILL.md
  skills/research/research-issue/SKILL.md
  skills/design/inventory-design/SKILL.md
  skills/design/analyse-design-gaps/SKILL.md
  skills/planning/review-plan/SKILL.md
  skills/work/review-work-item/SKILL.md
  skills/github/review-pr/SKILL.md
  skills/planning/validate-plan/SKILL.md
)
# OWNED_BY_0066 array is removed entirely once empty — see Phase 6.
```

`OWNED_BY_0066` is removed from the script entirely once empty (rather
than kept as an empty array) to avoid the `set -u` empty-array
expansion failure mode on older bash versions; the
`printf '%s\n' "${IN_SCOPE_PRODUCERS[@]}" "${NON_EMITTER_TEMPLATE_CONSUMERS[@]}"`
block at lines 179-185 is updated to drop the `OWNED_BY_0066`
reference in Phase 6's wrap-up step.

#### 4. Add 0066 work-item Schema Reference table

**File**: `meta/work/0066-update-review-skills-inline-frontmatter.md`
**Changes**: Append a `## Schema Reference` section immediately before
`## References` (mirroring 0065's structure). The table covers the three
new templates only — `validation.md` is already covered by 0065's table
and does not need duplicating.

```markdown
## Schema Reference

The three new template files created by this story emit the unified base
schema plus per-type extras per ADR-0033 and a `target` typed-linkage key
per ADR-0034. Authoritative source: ADR-0033 and ADR-0034. On any
discrepancy the ADRs win and this table should be re-synced.

| Template file | Artifact `type` | `schema_version` | Provenance bundle? | Per-type extras (beyond base) |
|---|---|---|---|---|
| `plan-review.md` | `plan-review` | 1 | no | `reviewer`, `verdict`, `lenses`, `review_number`, `review_pass`, `target` (= `"plan:<id>"`) |
| `work-item-review.md` | `work-item-review` | 1 | no | `reviewer`, `verdict`, `lenses`, `review_number`, `review_pass`, `target` (= `"work-item:<id>"`), `work_item_id` (transitional alias — see plan §Design Decisions #2; consumed by visualiser frontmatter.rs:330 until Phase 7) |
| `pr-review.md` | `pr-review` | 1 | no | `reviewer`, `verdict`, `lenses`, `review_number`, `target` (= `"pr:<pr-number>"`; the `pr` prefix is queued for inclusion in ADR-0034's vocabulary via supplementary ADR — see follow-up under `meta/work/0057-...md`), `pr_number` (bare integer; foreign reference to the external PR per ADR-0033 §Identity-value shape contract). `review_pass` is omitted — see plan §Design Decisions #1. |
```

#### 5. Update test-template cross-check to union both Schema Reference tables

**File**: `scripts/test-template-frontmatter.sh`
**Changes**: Replace the single `WORK_ITEM_MD` variable and the
`wi_templates` extraction at lines 19 and 149-153 with a union read
across the two work-item tables.

```bash
# Cross-check inputs (both 0065 and 0066 carry Schema Reference tables;
# the union must match the TSV exactly).
WORK_ITEM_MDS=(
  "meta/work/0065-update-artifact-templates-to-unified-schema.md"
  "meta/work/0066-update-review-skills-inline-frontmatter.md"
)
```

At the cross-check site, iterate the array and collect template filenames
from every existing `## Schema Reference` section:

```bash
# Count how many of the configured work-item files exist; if none, SKIP
# (preserves the pre-0066 SKIP semantic — authoritative source absent).
existing_count=0
for wi in "${WORK_ITEM_MDS[@]}"; do
  [ -f "$wi" ] && existing_count=$((existing_count + 1))
done
if [ "$existing_count" -eq 0 ]; then
  echo "SKIP: no work-item Schema Reference file present"
  SKIP=$((SKIP + 1))
else
  wi_templates=$(
    for wi in "${WORK_ITEM_MDS[@]}"; do
      if [ -f "$wi" ]; then
        awk '
          /^## Schema Reference/ { in_section=1; next }
          in_section && /^## / { in_section=0 }
          in_section && /^\| `[a-z0-9-]+\.md` \| / { print $0 }
        ' "$wi" | sed -E 's/^\| `([a-z0-9-]+\.md)` \|.*$/\1/'
      fi
    done | sort
  )
  # ... existing equality comparison against tsv_templates ...
fi
```

Notes on the pipeline:

- The outer existence count enforces the "SKIP only when both (all) files
  are absent" semantic the prose claims.
- The pipeline uses `| sort` (not `| sort -u`) so an accidental duplicate
  template row inside a single work-item Schema Reference table still
  produces a diff against `tsv_templates` (which has at most one row per
  template). Cross-file duplication (one template listed in both
  Schema Reference tables) is a separate authoring error and should
  also fail loudly — accept the noise rather than silently dedup.

#### 6. Self-verification — confirm RED baseline

Run the three test drivers and confirm the expected failure pattern:

```bash
bash scripts/test-template-frontmatter.sh
# Expected: PASS on 9 existing template rows; FAIL on 3 new rows
# (plan-review.md, work-item-review.md, pr-review.md — files do not
# exist yet, so the per-file existence assert fails first).
# Cross-check: PASS (TSV now has 12 rows, work-item union now has 12).

bash scripts/test-skill-frontmatter-population.sh
# Expected: PASS on 10 existing skill rows; the 4 new skill rows in
# skills-schema.tsv are not iterated by the per-field assertion loop
# (because the four skill paths remain in OWNED_BY_0066, not
# IN_SCOPE_PRODUCERS, throughout Phase 1). The four rows turn GREEN
# incrementally as each per-skill phase moves its path across (see
# Phase 2-5 §0).
# Discovery assertion: PASS (every Pass-A/Pass-B hit resolves to a
# SKILL.md in one of the three allowlists — the 4 skills remain in
# OWNED_BY_0066 throughout Phase 1).
```

### Success Criteria:

#### Automated Verification:

- [x] `bash scripts/test-template-frontmatter.sh; [ $? -ne 0 ]` — exits non-zero (RED baseline; 3 new template rows fail because the files do not exist yet).
- [x] `bash scripts/test-skill-frontmatter-population.sh; [ $? -ne 0 ]` — exits non-zero (RED baseline; 16 per-field assertions fail across the 4 not-yet-rewired skills).
- [x] `awk -F$'\t' 'NR>1 {print $1}' scripts/templates-schema.tsv | sort | uniq -d` — no duplicates (no template listed twice).
- [x] `awk -F$'\t' 'NF != 6 {exit 1}' scripts/templates-schema.tsv` — every row has 6 tab-separated fields (self-check inside the test driver also asserts this).
- [x] `awk -F$'\t' 'NF != 3 {exit 1}' scripts/skills-schema.tsv` — every row has 3 tab-separated fields.
- [x] `grep -n "^## Schema Reference$" meta/work/0066-update-review-skills-inline-frontmatter.md` returns exactly 1 line.
- [x] The test-template cross-check's union read produces exactly 12 unique template filenames matching the 12 TSV rows.
- [x] `bash scripts/test-format.sh` passes (no regressions).

#### Manual Verification:

- [x] Failure messages from the two test drivers point an implementer at the right file (e.g. "FAIL: plan-review.md — template file not found at templates/plan-review.md").
- [x] The work-item Schema Reference table in 0066 visually matches the TSV row data for the three new templates.

---

## Phase 2: `plan-review` Template & `review-plan` Rewire

### Overview

Create `templates/plan-review.md` and rewire `skills/planning/review-plan/SKILL.md`
to read its frontmatter from the new template. Independent of Phases 3,
4, 5.

### Changes Required:

#### 0. Move `review-plan` from `OWNED_BY_0066` to `IN_SCOPE_PRODUCERS`

**File**: `scripts/test-skill-frontmatter-population.sh`
**Changes**: Remove `skills/planning/review-plan/SKILL.md` from
`OWNED_BY_0066` and append to `IN_SCOPE_PRODUCERS`. This activates
the per-field assertion for the `review-plan` TSV row added in
Phase 1 §2 — the assertions now run against the SKILL.md changes
made in §1-§2 below.

#### 1. Template

**File**: `templates/plan-review.md` (new)
**Changes**: New file with the unified base frontmatter, per-type extras,
and an explanatory body skeleton mirroring the existing inline body
that `review-plan/SKILL.md:341-446` produces.

```yaml
---
type: plan-review                            # ADR-0033 artifact-type discriminator
id: "{filename-stem}"                        # e.g. "2026-05-31-some-plan-review-1"
title: "Plan Review: {Plan Name}"
date: "{ISO timestamp}"
author: "{author from VCS}"
producer: review-plan
status: complete                             # complete
target: ""                                   # typed-linkage key per ADR-0034: "plan:<plan-id>" (filled by review-plan)
reviewer: ""                                 # name/email of reviewer (filled by review-plan)
verdict: ""                                  # APPROVE | REVISE | COMMENT (filled by review-plan)
lenses: []                                   # list of lens names used in this review (filled by review-plan)
review_number: 0                             # which review of this plan (1-indexed; filled by review-plan)
review_pass: 1                               # latest re-review pass within this review_number (filled by review-plan)
tags: []
last_updated: "{ISO timestamp}"
last_updated_by: "{author from VCS}"
schema_version: 1
---
```

Body skeleton (below the frontmatter) reproduces the structure currently
emitted by `review-plan/SKILL.md` Step 4.7 — `## Plan Review: {name}`
heading, verdict line, cross-cutting themes, tradeoff analysis,
findings by severity, strengths, recommended changes, per-lens results.
The body is purely a structural example; the skill produces the actual
content per review.

#### 2. Skill rewire

**File**: `skills/planning/review-plan/SKILL.md`
**Changes**:

a. **Add template inclusion** between line 26 (`**Plan reviews directory**:`)
   and line 28 (the start of "You are tasked with..."). Insert:

   ```markdown
   ## Plan Review Template

   The template below defines the frontmatter and body structure that every
   plan review must carry. Read it now — use it to guide what information
   you record in Steps 3-4 and what shape you persist in Step 4.8.

   !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh plan-review`
   ```

b. **Replace the inline frontmatter block at lines 412-446** with the
   canonical persistence-step snippet. Parameters:

   - `{artifact-type}` = `plan review`
   - `{type-literal}` = `plan-review`
   - `{id-source}` = the review filename stem (without `.md`)
   - `{title-source}` = `Plan Review: {plan title}`
   - `{producer-name}` = `review-plan`
   - `{target-shape}` = `"plan:<plan-id>"` where `<plan-id>` is the plan
     filename stem (e.g. `"plan:2026-05-30-0065-update-artifact-templates-to-unified-schema"`)
   - `{reviewer-source}` = the reviewer value resolved per
     `create-work-item/SKILL.md:578-580` (config → VCS user → prompt)
   - `{verdict-source}` = the verdict from Step 4.5 (`APPROVE | REVISE | COMMENT`)
   - `{lenses-source}` = the list of lens names used (from Step 2)
   - `{review-number-source}` = `N` (next available review number from the glob in Steps 1.4 / 4.8)
   - `{review-pass-source}` = `1` (initial-write pass count; re-reviews bump per the existing Step 7 flow)

   Specifically, replace lines 412-446 (the existing "Write the review
   document with YAML frontmatter followed by..." prose plus the YAML
   block) with the canonical snippet shape introduced above (under
   "Canonical template-inclusion + persistence-step pattern"). Leave the
   "Include the per-lens results as a final section" prose at lines
   429-446 intact — the body-content prose is unchanged; only the
   frontmatter-block prose is replaced.

c. **Update the re-review flow at lines 526-562**: the in-place
   single-write update pattern stays; the in-memory mutation list
   changes from `verdict, review_pass, date` to
   `verdict, review_pass, last_updated, last_updated_by` —
   `date` is **removed from the mutation list** (per Design Decision
   #5 below; it now records the creation timestamp only, while
   `last_updated` advances on every re-review). Update the bullet at
   lines 530-532 to read:

   > 2. In memory, update exactly four frontmatter fields — `verdict`,
   >    `review_pass`, `last_updated`, and `last_updated_by` —
   >    preserving all other fields and body content verbatim. The
   >    `date` field retains the original-review timestamp; only
   >    `last_updated` advances on re-review.

   (`last_updated_by` may match `reviewer` if the re-reviewer is the
   same person, but is computed independently to handle the
   cross-reviewer case.)

   **Pre-0066-artifact handling**: when the re-reviewed artifact lacks
   `last_updated:` and/or `last_updated_by:` (it was written pre-0066),
   the in-memory mutation **inserts** those fields rather than treating
   their absence as malformed-frontmatter. Only an unparseable YAML
   block or missing `---` delimiters triggers the fresh-`-review-{N+1}.md`
   fallback. See also §Migration Notes for the corpus-transition
   rationale.

#### 3. Re-run tests

```bash
bash scripts/test-template-frontmatter.sh
# Expected: PASS row for plan-review.md (all base fields present,
# type/schema_version/id quoted, all 6 extras present, status complete).
# Other 11 rows: unchanged (9 PASS from 0065, 2 still FAIL — work-item-review.md, pr-review.md).

bash scripts/test-skill-frontmatter-population.sh
# Expected: PASS rows for review-plan (4 fields, all in the new persistence
# section). Other 3 0066 rows still FAIL (12 individual FAILs).
```

### Success Criteria:

#### Automated Verification:

- [x] `bash scripts/test-template-frontmatter.sh` PASS row for `plan-review.md`.
- [x] `bash scripts/test-skill-frontmatter-population.sh` PASS row for `review-plan` (all 4 mandatory fields).
- [x] `grep -c "^type:\|^schema_version:\|^verdict:" skills/planning/review-plan/SKILL.md` — count matches the number of inline YAML key occurrences inside fenced template-example blocks only (manually verify zero matches outside fenced blocks).
- [x] `rg -nE "^type:[[:space:]]" skills/planning/review-plan/SKILL.md` — only matches inside ` ``` `-fenced blocks.
- [x] `rg -n "config-read-template\.sh plan-review" skills/planning/review-plan/SKILL.md` returns at least one match.
- [x] `bash scripts/test-format.sh` passes.

#### Manual Verification:

- [ ] Running `review-plan` against a real plan produces a `plan-review` artifact with `id:` quoted, `schema_version: 1`, non-empty `producer`/`last_updated`/`last_updated_by`/`target`/`reviewer`/`verdict`/`lenses`/`review_number`/`review_pass`, and no `\{[^{}\n]+\}` mustache token survivors in the frontmatter block.
- [ ] Re-running the review (Step 7) updates the artifact in place with the four-field frontmatter mutation (`verdict`/`review_pass`/`last_updated`/`last_updated_by`; `date` retains the original-review timestamp per Design Decision #5); the body is appended-only.
- [ ] The `target:` value parses as `"plan:<plan-id>"` where `<plan-id>` matches the source plan's filename stem.

---

## Phase 3: `work-item-review` Template & `review-work-item` Rewire

### Overview

Create `templates/work-item-review.md` and rewire
`skills/work/review-work-item/SKILL.md`. Keep `work_item_id` as a
transitional alias alongside `target` per Design Decision #2 (revised).
Update eval fixtures (`evals/evals.json`, `evals/benchmark.json`)
to match the new emission. Independent of Phases 2, 4, 5.

### Changes Required:

#### 0. Move `review-work-item` from `OWNED_BY_0066` to `IN_SCOPE_PRODUCERS`

**File**: `scripts/test-skill-frontmatter-population.sh`
**Changes**: Remove `skills/work/review-work-item/SKILL.md` from
`OWNED_BY_0066` and append to `IN_SCOPE_PRODUCERS`. Activates the
per-field assertion for this skill against the SKILL.md changes made
in §1-§3 below.

#### 1. Template

**File**: `templates/work-item-review.md` (new)
**Changes**: New file with the unified base frontmatter, the same six
per-type extras as `plan-review.md`, and the same `status: complete`
vocabulary. The only differences from `plan-review.md` are the `type:`
value (`work-item-review`), the `producer:` value, the `target:` value
shape (`"work-item:<id>"`), and the title format.

```yaml
---
type: work-item-review                       # ADR-0033 artifact-type discriminator
id: "{filename-stem}"                        # e.g. "0042-improve-search-review-1"
title: "Work Item Review: {Work Item Title}"
date: "{ISO timestamp}"
author: "{author from VCS}"
producer: review-work-item
status: complete                             # complete
target: ""                                   # typed-linkage key per ADR-0034: "work-item:<id>" (filled by review-work-item)
work_item_id: ""                             # transitional alias — same 4-digit id as in target; consumed by visualiser frontmatter.rs:330 until Phase 7 teaches it to extract from target. Removed by the visualiser consumer-update.
reviewer: ""                                 # name/email of reviewer (filled by review-work-item)
verdict: ""                                  # APPROVE | REVISE | COMMENT (filled by review-work-item)
lenses: []                                   # list of work-item lens names used (filled by review-work-item)
review_number: 0                             # which review of this work item (1-indexed; filled by review-work-item)
review_pass: 1                               # latest re-review pass within this review_number (filled by review-work-item)
tags: []
last_updated: "{ISO timestamp}"
last_updated_by: "{author from VCS}"
schema_version: 1
---
```

**Transitional `work_item_id:` alias** (Design Decision #2 — see plan
§Design Decisions for rationale). Both `target` and `work_item_id`
carry the same edge; the duplication is bounded by Phase 7's visualiser
update, after which `work_item_id:` may be retired in a follow-up
release (mirroring the `work-item:` → `work_item_id:` migration path
established by 0070).

Body skeleton reproduces the structure currently emitted by
`review-work-item/SKILL.md:275-323` — `## Work Item Review: {title}`
heading, verdict, cross-cutting themes, findings by severity, strengths,
recommended changes, per-lens results.

#### 2. Skill rewire

**File**: `skills/work/review-work-item/SKILL.md`
**Changes**:

a. **Add template inclusion** between line 27 (`**Work item reviews directory**:`)
   and line 29 (start of "You are tasked with..."). Insert the canonical
   template-inclusion block with `{artifact-type}` = `work item review`
   and `{template-name}` = `work-item-review`.

b. **Replace the inline frontmatter block at lines 346-381** with the
   canonical persistence-step snippet. Parameters:

   - `{type-literal}` = `work-item-review`
   - `{id-source}` = the review filename stem
   - `{title-source}` = `Work Item Review: {work item title}`
   - `{producer-name}` = `review-work-item`
   - `{target-shape}` = `"work-item:<4-digit-id>"` (e.g. `"work-item:0042"`),
     resolved from the work item filename's 4-digit prefix via
     `${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-read-field.sh id {path}`
     (per ADR-0033's identity-value contract, `id` is the canonical own-identity
     key post-0064). Argument order is **field-name first, file-path
     second** per the script's documented usage; this fixes the
     pre-existing reversed-arg invocation at the current SKILL.md
     line 343 (`{path} number`).
   - `{reviewer-source}`, `{verdict-source}`, `{lenses-source}`,
     `{review-number-source}`, `{review-pass-source}` follow the same
     pattern as Phase 2

c. **Keep `work_item_id`-emission prose; reframe as transitional alias**
   (per Design Decision #2). The skill currently emits `work_item_id:`
   as the primary id field at lines 383-385. Reframe that paragraph as:

   > The `target:` field stores the work item's stable 4-digit identifier
   > as a typed-linkage key (e.g. `"work-item:0042"`) per ADR-0034,
   > providing resilience against work item renames. A `work_item_id:`
   > field is also emitted as a transitional alias carrying the same
   > 4-digit identifier (unquoted-or-quoted scalar; the visualiser's
   > `read_ref_keys` consumes it as the primary work-item
   > cross-reference key today). Both fields encode the same edge; the
   > duplication is bounded by Phase 7's visualiser update.

   The canonical persistence-step snippet must therefore include a
   `work_item_id:` ← `{work-item-id-source}` bullet for this phase
   only (same 4-digit value as the `target` payload).

d. **Update the re-review flow at lines 426-490**: same four-field
   in-memory mutation as Phase 2 (`verdict`, `review_pass`,
   `last_updated`, `last_updated_by` — `date` is preserved as
   creation timestamp per Design Decision #5). Preserve the
   malformed-frontmatter fallback at lines 462-464 (write a fresh
   `-review-{N+1}.md`) — the fallback's logic is unchanged; only its
   in-memory mutation field list changes.

   **Pre-0066-artifact handling**: same insert-if-missing rule as
   Phase 2 §2.c — absence of `last_updated:`/`last_updated_by:` on a
   pre-0066 review artifact triggers field insertion alongside the
   verdict/review_pass mutation, NOT the malformed-frontmatter
   fallback. See §Migration Notes for the corpus-transition rationale.

#### 3. Update eval fixtures

**File**: `skills/work/review-work-item/evals/evals.json` (lines 30-40)
**Changes**: Update the expected-frontmatter assertions to reflect the
new emission. Specifically:

- Replace `"skill": "review-work-item"` with `"producer": "review-work-item"`
  in JSON keys.
- **Also rewrite assertion-text strings** (any `expected_output` or
  `text` field whose value contains the literal phrase
  `skill: review-work-item` — e.g. line 30/34 currently reads
  `"frontmatter (type: work-item-review, skill: review-work-item)"`).
  Rewrite to `"frontmatter (type: work-item-review, producer: review-work-item)"`.
- Update `work_item_id` assertions: the new emission carries
  `target: "work-item:<id>"` AND `work_item_id: "<id>"` (transitional
  alias per Design Decision #2). Assert both fields.
- Add assertions for `id` (quoted), `schema_version: 1`,
  `producer: review-work-item`, `last_updated`, `last_updated_by`.
- **Add a negative assertion**: the frontmatter does NOT contain a
  literal `skill:` key — i.e. the legacy `skill: review-work-item`
  field is fully retired in favour of `producer:`. This guards
  against partial-rewire regressions where both `skill:` and
  `producer:` get emitted.

The verdict-enum assertion (`APPROVE | REVISE | COMMENT`) stays
unchanged — verdict alignment is out of scope.

**File**: `skills/work/review-work-item/evals/benchmark.json` (lines 89-100)
**Changes**: Same edits as above (including the assertion-text rewrite
at line 89 and the negative `skill:`-absence assertion).

#### 4. Re-run tests

```bash
bash scripts/test-template-frontmatter.sh
# Expected: PASS rows for plan-review.md AND work-item-review.md.
# pr-review.md still FAIL (file does not exist yet).

bash scripts/test-skill-frontmatter-population.sh
# Expected: PASS rows for review-plan AND review-work-item.
# 2 0066 rows still FAIL (8 individual FAILs).
```

### Success Criteria:

#### Automated Verification:

- [x] `bash scripts/test-template-frontmatter.sh` PASS row for `work-item-review.md`.
- [x] `bash scripts/test-skill-frontmatter-population.sh` PASS row for `review-work-item`.
- [x] `rg -n "^work_item_id:" templates/work-item-review.md` returns at least one match (the transitional alias slot per Design Decision #2; the no-match assertion from review-1 is inverted here).
- [x] `rg -nE "^[[:space:]]*work_item_id:" skills/work/review-work-item/SKILL.md` returns at least one match outside fenced blocks (the new persistence-step bullet) AND any matches inside ` ``` `-fenced blocks are template-example occurrences only. *(Implementation note: persistence bullet uses backtick-quoted form `` `work_item_id:` `` per the canonical snippet; the test driver's `in_imperative_section` matches the backtick prefix and reports PASS.)*
- [x] `rg -n "config-read-template\.sh work-item-review" skills/work/review-work-item/SKILL.md` returns at least one match.
- [x] `rg -n "^[[:space:]]+\"producer\":[[:space:]]+\"review-work-item\"" skills/work/review-work-item/evals/evals.json skills/work/review-work-item/evals/benchmark.json` returns at least one match per file. *(The eval assertion-text strings reference `producer: review-work-item`; the JSON key `skill_name: "review-work-item"` is preserved as the file's metadata identifier per the canonical eval shape.)*
- [x] `bash scripts/test-format.sh` passes.

#### Manual Verification:

- [ ] Running `review-work-item` against a real work item produces an artifact with `target: "work-item:<id>"` (quoted, typed-linkage form), `work_item_id: "<id>"` (transitional alias carrying the same 4-digit identifier per Design Decision #2), `id:` quoted, `schema_version: 1`, all base + extra fields populated.
- [ ] Running the work-item-review eval suite (mise task or direct invocation) passes with the updated assertions.

---

## Phase 4: `pr-review` Template & `review-pr` Rewire

### Overview

Create `templates/pr-review.md` and rewire `skills/github/review-pr/SKILL.md`.
Drop `pr_title` (use base `title:`). Use `target: "pr:<pr-number>"` per
Design Decision #3. Omit `review_pass` per Design Decision #1. Independent
of Phases 2, 3, 5.

### Changes Required:

#### 0. Move `review-pr` from `OWNED_BY_0066` to `IN_SCOPE_PRODUCERS`

**File**: `scripts/test-skill-frontmatter-population.sh`
**Changes**: Remove `skills/github/review-pr/SKILL.md` from
`OWNED_BY_0066` and append to `IN_SCOPE_PRODUCERS`. Activates the
per-field assertion for this skill against the SKILL.md changes made
in §1-§2 below.

#### 1. Template

**File**: `templates/pr-review.md` (new)
**Changes**: New file with the unified base frontmatter and five
per-type extras (`reviewer`, `verdict`, `lenses`, `review_number`,
`target`, `pr_number`). **No `review_pass`** (Design Decision #1).
**No `pr_title`** — the base `title:` carries the PR title.

```yaml
---
type: pr-review                              # ADR-0033 artifact-type discriminator
id: "{filename-stem}"                        # e.g. "123-review-1"
title: "{PR Title}"
date: "{ISO timestamp}"
author: "{author from VCS}"
producer: review-pr
status: complete                             # complete
target: ""                                   # typed-linkage key per ADR-0034: "pr:<pr-number>" (filled by review-pr); see plan §Design Decisions for the pr: prefix follow-up
reviewer: ""                                 # name/email of reviewer (filled by review-pr)
verdict: ""                                  # APPROVE | REQUEST_CHANGES | COMMENT (filled by review-pr; enum matches GitHub Reviews API event values)
lenses: []                                   # list of lens names used in this review (filled by review-pr)
review_number: 0                             # which review of this PR (1-indexed; filled by review-pr)
pr_number: {number}                          # bare integer; foreign reference to the external PR per ADR-0033 §Identity-value shape contract (filled by review-pr)
tags: []
last_updated: "{ISO timestamp}"
last_updated_by: "{author from VCS}"
schema_version: 1
---
```

**Explicitly absent**: `pr_title:` (renamed to base `title:` per
ADR-0033 + 0065's `pr-description.md` precedent) and `review_pass:`
(per Design Decision #1).

The `target:` value shape is pinned to the regex `^"pr:[0-9]+"$` — the
template-shape test does not currently assert regex shapes on extras,
but the manual verification step in this phase checks it explicitly.
The Phase-6 discovery-pass record names the regex.

#### 2. Skill rewire

**File**: `skills/github/review-pr/SKILL.md`
**Changes**:

a. **Add template inclusion** between line 36 (end of the second
   IMPORTANT note about sub-agent placeholder resolution) and line 38
   (start of "You are tasked with..."). This placement keeps the two
   IMPORTANT notes adjacent to each other and the template-inclusion
   block sits after all upper-block setup, matching the placement
   pattern used by review-plan, review-work-item, and validate-plan.
   Insert the canonical template-inclusion block with
   `{artifact-type}` = `PR review` and `{template-name}` = `pr-review`.

b. **Replace the inline frontmatter block at lines 451-462** (inside
   Step 4.10) with the canonical persistence-step snippet. Parameters:

   - `{type-literal}` = `pr-review`
   - `{id-source}` = `{number}-review-{N}` where `{number}` is the PR
     number and `{N}` is the next review number
   - `{title-source}` = the PR title from `gh pr view --json title`
   - `{producer-name}` = `review-pr`
   - `{target-shape}` = `"pr:<pr-number>"` (e.g. `"pr:123"`); the snippet
     prose pins the regex `^"pr:[0-9]+"$`
   - `{reviewer-source}` = reviewer resolved per `create-work-item/SKILL.md:578-580`
   - `{verdict-source}` = the verdict from Step 4.6 (`APPROVE | REQUEST_CHANGES | COMMENT`)
   - `{lenses-source}` = the list of lens names used
   - `{review-number-source}` = `N` (next available review number from the glob in Step 4.10)
   - **No `{review-pass-source}` parameter** — the canonical snippet's
     `review_pass:` bullet is omitted for this phase only (Design
     Decision #1).
   - Add a `pr_number:` bullet to the snippet for this phase only:
     - `pr_number:` ← the PR number from `gh pr view --json number`

c. **Drop `pr_title`-emission prose**. The skill currently emits
   `pr_title: "{title}"` as a separate field on top of `target:`.
   Replace with a one-line note in the persistence step:

   > The PR title is recorded in the base `title:` field; no separate
   > `pr_title:` field is emitted (the unified schema uses the base
   > `title:` for all artifact titles per ADR-0033).

d. **No re-review flow change** — `review-pr` does not currently have an
   in-place re-review update flow, and this story does not introduce
   one. Re-running the skill produces a fresh `-review-{N+1}.md`.

#### 3. Re-run tests

```bash
bash scripts/test-template-frontmatter.sh
# Expected: PASS rows for plan-review.md, work-item-review.md, AND pr-review.md.
# All 12 template-shape rows GREEN.

bash scripts/test-skill-frontmatter-population.sh
# Expected: PASS rows for review-plan, review-work-item, AND review-pr.
# 1 0066 row still FAIL (validate-plan; 4 individual FAILs).
```

### Success Criteria:

#### Automated Verification:

- [x] `bash scripts/test-template-frontmatter.sh` PASS row for `pr-review.md`. All 12 template rows GREEN.
- [x] `bash scripts/test-skill-frontmatter-population.sh` PASS row for `review-pr`.
- [x] `rg -n "^pr_title:" templates/pr-review.md` returns no matches.
- [x] `rg -n "^review_pass:" templates/pr-review.md` returns no matches.
- [x] `rg -n "config-read-template\.sh pr-review" skills/github/review-pr/SKILL.md` returns at least one match.
- [x] `rg -nE "^[[:space:]]*pr_title:" skills/github/review-pr/SKILL.md` returns matches ONLY inside ` ``` `-fenced blocks (or zero matches outside fences).
- [x] `bash scripts/test-format.sh` passes.

#### Manual Verification:

- [ ] Running `review-pr` against a real PR produces an artifact whose `target:` value matches `^"pr:[0-9]+"$` exactly (e.g. `"pr:123"`), with `id:` quoted, `schema_version: 1`, `title:` carrying the PR title (no `pr_title:`), and no `review_pass:` field.
- [ ] The artifact carries `pr_number:` as a bare integer (e.g. `pr_number: 123`) — the foreign-reference shape per ADR-0033 since `pr` is a numeric external entity.
- [ ] No `\{[^{}\n]+\}` mustache token survivors in the generated frontmatter.

---

## Phase 5: `validate-plan` Rewire (Template Already Exists)

### Overview

Rewire `skills/planning/validate-plan/SKILL.md` to read frontmatter from
`templates/validation.md` (whose frontmatter block was added by 0065)
rather than re-specifying a narrower block inline. The template file
itself is unchanged — only the skill prose changes. Independent of
Phases 2, 3, 4.

### Changes Required:

#### 0. Move `validate-plan` from `OWNED_BY_0066` to `IN_SCOPE_PRODUCERS`

**File**: `scripts/test-skill-frontmatter-population.sh`
**Changes**: Remove `skills/planning/validate-plan/SKILL.md` from
`OWNED_BY_0066` and append to `IN_SCOPE_PRODUCERS`. Do **not** remove
the (now-empty) `OWNED_BY_0066` array declaration here — Phases 2-5
may land in any order under the parallelisation guarantee, so this
cleanup is deferred to Phase 6 (which is guaranteed by ordering
constraints to land after all per-skill phases). See Phase 6 §0a.

#### 1. Skill rewire

**File**: `skills/planning/validate-plan/SKILL.md`
**Changes**:

a. **Move and rename the template inclusion**. The skill currently calls
   `config-read-template.sh validation` at line 116 (inside Step 3
   "Generate Validation Report"), framed as a body-only template. Move
   the inclusion to the top of the skill (between line 24 (`**Validations
   directory**:`) and line 26 (start of "You are tasked with...")) and
   reframe as the full-artifact template:

   ```markdown
   ## Plan Validation Template

   The template below defines the frontmatter and body structure that
   every plan validation report must carry. Read it now — use it to guide
   what information you record in the validation process and what shape
   you persist in Step 4.

   !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh validation`
   ```

   Then **delete the inclusion call at line 116** — the template is read
   once at the top of the skill, not re-read mid-process.

b. **Replace the inline frontmatter block at lines 134-141** (inside
   Step 4 "Persist the Validation Report") with the canonical
   persistence-step snippet. Parameters:

   - `{type-literal}` = `plan-validation`
   - `{id-source}` = the validation filename stem (e.g.
     `2026-05-30-0065-update-artifact-templates-to-unified-schema-validation`)
   - `{title-source}` = `Validation Report: {plan title}`
   - `{producer-name}` = `validate-plan`
   - `{target-shape}` = `"plan:<plan-id>"` (e.g.
     `"plan:2026-05-30-0065-update-artifact-templates-to-unified-schema"`),
     matching the same shape `review-plan` uses (the typed-linkage key
     points at the plan being validated)
   - **No review-extras parameters** (`{reviewer-source}`,
     `{verdict-source}`, `{lenses-source}`, `{review-number-source}`,
     `{review-pass-source}`) — `plan-validation` does not carry any of
     the review extras. The canonical snippet's review-extras bullets are
     omitted for this phase.
   - Add a `result:` bullet to the snippet for this phase only:
     - `result:` ← `pass | partial | fail` per the report's
       Implementation Status section (see lines 146-150 for the
       derivation rule)

c. **Drop legacy `skill:`-emission prose.** The current skill emits
   `skill: validate-plan` as a header field at lines 133-141; the
   unified schema replaces this with `producer: validate-plan`. The
   canonical persistence-step snippet substitutes `producer:` per the
   §"Canonical template-inclusion + persistence-step pattern". No
   in-source consumer reads `skill:` (confirmed by grep); the field
   is fully retired post-0066. Mirror the Phase 4 §2.c callout
   pattern.

d. **Preserve the side-effect at lines 152-154**: if `result: pass`,
   update the validated plan's frontmatter `status` field to `complete`.
   This is orthogonal to the frontmatter-shape change and stays
   unchanged.

e. **Renumber Step 3** if needed (the body of Step 3 currently includes
   the template-inclusion line at 116). After the move, Step 3 is just
   the body-content prose; the template inclusion belongs to the
   skill-level setup.

#### 2. Re-run tests

```bash
bash scripts/test-template-frontmatter.sh
# Expected: all 12 rows PASS (validation.md was already PASS from 0065).

bash scripts/test-skill-frontmatter-population.sh
# Expected: PASS rows for all 4 0066 skills.
# All 14 skill rows GREEN. Discovery assertion PASS.
```

### Success Criteria:

#### Automated Verification:

- [x] `bash scripts/test-skill-frontmatter-population.sh` PASS row for `validate-plan`. All 14 skill rows GREEN. Discovery assertion PASS.
- [x] `bash scripts/test-template-frontmatter.sh` — all 12 template rows PASS, cross-check PASS.
- [x] `rg -n "config-read-template\.sh validation" skills/planning/validate-plan/SKILL.md` returns exactly one match (the new top-level inclusion; the old line-116 occurrence is removed).
- [x] `rg -nE "^[[:space:]]*type:[[:space:]]+plan-validation" skills/planning/validate-plan/SKILL.md` returns matches ONLY inside ` ``` `-fenced blocks (or zero matches outside fences).
- [x] `rg -nE "^[[:space:]]*skill:[[:space:]]+validate-plan" skills/planning/validate-plan/SKILL.md` returns zero matches (the legacy `skill:` key is fully retired in favour of `producer:`).
- [x] `mise run test:unit:templates` passes (all three sub-drivers GREEN).
- [x] `bash scripts/test-format.sh` passes.

#### Manual Verification:

- [ ] Running `validate-plan` against a real plan produces a `plan-validation` artifact with `target: "plan:<plan-id>"`, `result:` populated (`pass`/`partial`/`fail`), `id:` quoted, `schema_version: 1`, all base fields populated.
- [ ] If the result is `pass`, the validated plan's `status:` is correctly updated to `complete` (existing side-effect preserved).
- [ ] No `\{[^{}\n]+\}` mustache token survivors in the generated frontmatter.

---

## Phase 6: Discovery Pass & Work-Item Updates

### Overview

Final compliance step required by work item AC #9 (reproducible
discovery pass). Re-run the existing test-driver discovery assertion,
record the post-rewire producer split in the 0066 work item, and apply a
small AC edit to the work item to reflect Design Decision #1
(`review_pass` omitted from `pr-review`). After this phase: 0066's
acceptance criteria are wholly satisfied; future maintenance changes to
the schema touch only template files.

### Changes Required:

#### 0a. Remove empty `OWNED_BY_0066` array declaration

**File**: `scripts/test-skill-frontmatter-population.sh`
**Changes**: At this point, Phases 2-5 have all landed in some order
and each has moved its own path out of `OWNED_BY_0066`. The array is
empty. Remove the array declaration entirely AND the
`"${OWNED_BY_0066[@]}"` expansion at the `printf` block (lines
179-185) so the script runs cleanly under `set -u` on older bash
versions (bash 3.2 default on macOS). After this step the
discovery-pass loop references only `IN_SCOPE_PRODUCERS` and
`NON_EMITTER_TEMPLATE_CONSUMERS`.

#### 1. Re-run discovery assertion

The discovery-pass logic is already implemented in
`scripts/test-skill-frontmatter-population.sh:160-195` (Phase 11 of
0065). 0066 does not add a new test driver — it just runs the existing
one and records the post-rewire output:

```bash
bash scripts/test-skill-frontmatter-population.sh
# Expected: every Pass-A / Pass-B hit resolves to a SKILL.md in
# IN_SCOPE_PRODUCERS or NON_EMITTER_TEMPLATE_CONSUMERS. OWNED_BY_0066 is
# now empty. "PASS: every discovered SKILL.md is allowlisted" appears.
```

The same patterns surface `verdict:`, `review_pass:`, `target:`,
`result:`, `pr_number:` inside fenced template-example blocks inside
the four rewired SKILL.md files — those hits are expected and the four
files are now in `IN_SCOPE_PRODUCERS`, so the allowlist comm-comparison
still produces an empty `unexpected` set.

#### 2. Append Discovery Pass Record to 0066 work item

**File**: `meta/work/0066-update-review-skills-inline-frontmatter.md`
**Changes**: Append a `## Discovery Pass Record` section immediately
before `## References`, mirroring 0065's precedent. Capture the verbatim
greps and the resulting producer split.

```markdown
## Discovery Pass Record

Commands executed (run from the workspace root, after Phases 1-5 have landed):

```
# Pass A — template-using and unified-schema-emitting producers
rg -n "config-read-template\.sh|^[[:space:]]*producer:|^[[:space:]]*schema_version:" skills --glob '**/SKILL.md'

# Pass B — legacy inline-frontmatter emitters (now empty for 0066 scope)
rg -n "verdict:|review_pass:|review_target:|^[[:space:]]*target:|^[[:space:]]*result:|pr_number:" skills --glob '**/SKILL.md'
```

Pass A surfaces every skill that reads a template via the canonical
loader or directly emits a unified base field. Pass B surfaces every
SKILL.md that mentions a review/validation extra literal — post-0066,
those literals appear only inside fenced template-example blocks, but
the discovery patterns still match them; every Pass-B hit must resolve
to a SKILL.md in `IN_SCOPE_PRODUCERS`.

Producer split (post-0066):

- **Unified template-based emitters (10 from 0065 + 4 from 0066 = 14
  total)**: create-work-item, extract-work-items, create-plan,
  describe-pr, create-adr, extract-adrs, research-codebase,
  research-issue, inventory-design, analyse-design-gaps, **review-plan,
  review-work-item, review-pr, validate-plan**.
- **Inline-only emitters owned by 0066**: NONE (formerly:
  review-plan, review-work-item, review-pr, validate-plan).
- **Non-emitter template consumers**: refine-work-item, update-work-item,
  list-work-items.

Other inline producers found: NONE. The Phase-6 grep recipe and the
existing test driver's discovery assertion together form the
reproducible verification of work item AC #9.

For the `pr-review.target` regex shape pinned by Design Decision #3:

```
rg -n "^target:[[:space:]]+\"pr:[0-9]+\"" templates/pr-review.md
# Expected: zero matches (the template carries `target: ""` empty slot).

# Manual: against an artifact produced by `review-pr`, the same regex
# should match exactly once in the frontmatter block.
```
```

#### 3. Amend work item AC #5

**File**: `meta/work/0066-update-review-skills-inline-frontmatter.md`
**Changes**: Edit AC #5 (currently asserts all three review types carry
`review_pass`) to reflect Design Decision #1. Rewrite from:

> - [ ] Each review template emits the extras per ADR-0033 on all three review types (`plan-review`, `work-item-review`, `pr-review`): `reviewer`, `verdict`, `lenses`, `review_number`, `review_pass`. ...

to:

> - [ ] Each review template emits the per-ADR-0033 review extras: `reviewer`, `verdict`, `lenses`, `review_number` on all three review types (`plan-review`, `work-item-review`, `pr-review`); `review_pass` on `plan-review` and `work-item-review` only (`pr-review` omits `review_pass` — see plan §Design Decisions #1 — until a future story introduces a re-review lifecycle for the skill). Each review/validation template additionally emits `target` (per ADR-0034's typed-linkage vocabulary) as a single quoted YAML string in `"doc-type:id"` form — `"plan:<id>"` for `plan-review` and `plan-validation`, `"work-item:<id>"` for `work-item-review`, `"pr:<pr-number>"` for `pr-review` (regex: `^"pr:[0-9]+"$`).

Also amend AC #9's discovery-pass language to name the exact grep
recipe captured in §Discovery Pass Record above (rather than the
implementer-chosen recipe the current AC text implies).

#### 4. Queue follow-up under 0057

**File**: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
**Changes**: This is the parent epic and is the canonical home for
follow-up items. Append a one-line follow-up under the epic's existing
follow-up list (or under Technical Notes if no follow-up list exists):

> - **Supplementary ADR to extend ADR-0034's vocabulary with external-entity
>   prefixes** (e.g. `pr`, possibly others). Surfaced by 0066, which
>   emits `target: "pr:<pr-number>"` on `pr-review` artifacts; ADR-0034's
>   currently-published vocabulary covers only meta-artifact types. The
>   ADR should sit alongside ADR-0034 rather than amend it (the existing
>   meta-artifact vocabulary is unchanged).
> - **Retire the transitional `work_item_id:` alias on `work-item-review`
>   artifacts.** Surfaced by 0066 Design Decision #2; the alias is
>   kept alongside `target: "work-item:<id>"` because the visualiser's
>   `read_ref_keys` (frontmatter.rs:330) reads `work_item_id:` as the
>   primary scalar. Once Phase 7 ships and the corpus migration 0070
>   has run across userspace repos, drop `work_item_id:` from
>   `templates/work-item-review.md` and the work-item-review skill
>   prose. Mirrors the `work-item:` → `work_item_id:` retirement
>   pattern 0070 closes for older work-item artifacts.

This is the only `meta/` edit outside the 0066 work item that this
phase makes. It is one additive bullet in the parent epic.

### Success Criteria:

#### Automated Verification:

- [ ] `bash scripts/test-skill-frontmatter-population.sh` passes including the discovery assertion ("PASS: every discovered SKILL.md is allowlisted").
- [ ] `mise run test:unit:templates` passes (all three sub-drivers GREEN).
- [ ] `grep -n "^## Discovery Pass Record$" meta/work/0066-update-review-skills-inline-frontmatter.md` returns exactly 1 line.
- [ ] The Pass-A and Pass-B greps recorded in the work item, run fresh from the workspace root, produce a producer set that matches the recorded split exactly (every hit resolves to a SKILL.md named in `IN_SCOPE_PRODUCERS` or `NON_EMITTER_TEMPLATE_CONSUMERS`).
- [ ] `grep -n "review_pass" meta/work/0066-update-review-skills-inline-frontmatter.md` — AC #5 now distinguishes the three review types per Design Decision #1.
- [ ] `grep -n "Supplementary ADR" meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md` returns at least 1 match (the queued follow-up).
- [ ] `bash scripts/test-format.sh` passes.

#### Manual Verification:

- [ ] A different engineer can re-run the recorded discovery greps and produce the recorded producer split verbatim.
- [ ] The 0066 work item Schema Reference table, AC #5, AC #9, and Discovery Pass Record are mutually consistent.
- [ ] The follow-up under 0057 is discoverable when someone later asks "why does `pr-review` use `pr:` if ADR-0034 doesn't list it?"

---

## Phase 7: Visualiser Consumer Update

### Overview

Update the visualiser server (Rust) to consume the new typed-linkage
`target:` shape so the plan-review→plan resolution and work-item-review
aggregation continue to work after Phases 2 and 3 land. Two functions
need to accept both the existing path-form and the new typed-linkage
`doc-type:id` form per ADR-0034 §"Forms":

1. `target_path_from_entry` (`skills/visualisation/visualise/server/src/indexer.rs:744-750`)
   — extend to recognise `target: "plan:<plan-id>"` and resolve to a
   path via the existing entries index (the same `id` field that the
   indexer already keys plans on).
2. `read_ref_keys` (`skills/visualisation/visualise/server/src/frontmatter.rs:305-356`)
   — extend to also extract a work-item id from `target:` values
   matching `^"?work-item:[0-9]{4}"?$` when no `work_item_id:` scalar
   is present.

After Phase 7 lands, the transitional `work_item_id:` alias on
`work-item-review` artifacts becomes redundant for the visualiser. It
is *not* removed by this story — removal is a follow-up release
(mirroring the `work-item:` → `work_item_id:` migration that 0070
closes).

This phase depends on Phases 2 and 3 (the emission shapes it consumes)
but is otherwise independent of Phases 4-6.

### Sequencing note (release boundary)

**Phase 7's consumer-update is back-compatible**: the refactored
`target_path_from_entry` accepts BOTH the existing path-form (e.g.
`target: "meta/plans/2026-...md"`) and the new typed-linkage form
(`target: "plan:<id>"`). Therefore Phase 7 may safely land *before*
Phases 2 and 3 — there is no risk of breaking the existing path-form
consumers. To avoid recreating the silent-orphan window flagged in
review-1, **either** sequence Phase 7 first in the actual landing order
**or** require all seven phases to ship in a single release. The plan's
narrative ordering keeps Phase 7 last for readability, but the
implementation team is encouraged to land Phase 7 early so subsequent
emission changes (Phases 2 and 3) are immediately resolved by the
visualiser.

### Changes Required:

#### 1. Add `plans_by_id` secondary index (indexer.rs)

**File**: `skills/visualisation/visualise/server/src/indexer.rs`
**Changes**: Add a new secondary index `plans_by_id: Arc<RwLock<HashMap<String, PathBuf>>>`
mirroring the existing `work_item_by_id` pattern (`indexer.rs:206`,
`248`, `268`, `317`, `345`, `350`, `386`, `414`, `451`, `591`,
`639`, `804`, `858`). The index maps a plan's `id:` field (typically
the filename stem, e.g. `"2026-05-30-0065-update-artifact-templates-to-unified-schema"`)
to the plan file's canonicalised absolute path.

Discrete sub-changes (all parallel to the existing `work_item_by_id`
plumbing):

1a. **Field on `Indexer`**: add `plans_by_id: Arc<RwLock<HashMap<String, PathBuf>>>`
    immediately after the `work_item_by_id` field declaration at
    `indexer.rs:206`.
1b. **Constructor (`build`)**: at `indexer.rs:248`, initialise the
    new field with `Arc::new(RwLock::new(HashMap::new()))`.
1c. **Rescan local accumulator**: at `indexer.rs:268`, declare a
    local `let mut plans_by_id: HashMap<String, PathBuf> = HashMap::new();`.
1d. **Rescan loop population**: insert a branch parallel to lines
    313-318 that calls a new `plan_id_from_entry(&entry)` helper and
    inserts into `plans_by_id` when present.
1e. **Rescan lock acquisition**: extend the lock-acquisition block
    at lines 343-352 to also acquire `plans_by_id.write().await` and
    assign the local accumulator. Lock-ordering invariant remains
    `entries → adr → work_item → plans → reviews_by_target →
    work_item_refs_by_target` — the new lock slots between
    `work_item` and `reviews_by_target`.
1f. **Helper `plan_id_from_entry`**: add adjacent to
    `work_item_id_from_entry` at `indexer.rs:1093`. Returns
    `Some(entry.frontmatter.get("id")?.as_str()?.to_string())`
    when `entry.r#type == DocTypeKey::Plans`, else `None`.
1g. **Helpers `update_plans_by_id` and `remove_from_plans_by_id`**:
    add parallel to `update_work_item_by_id` / `remove_from_work_item_by_id`
    at `indexer.rs:804-820` and `858-865` respectively. Signatures
    and bodies mirror the work-item helpers verbatim, substituting
    `plan_id_from_entry` for `work_item_id_from_entry`.
1h. **`refresh_one` integration**: at the three call sites that
    invoke the work-item helpers (`indexer.rs:386`, `414`, `451`),
    add a parallel call to the plans helpers immediately after. The
    lock-ordering invariant (caller holds `entries.write()`; helpers
    acquire their own per-index write lock inside) extends
    unchanged.
1i. **Public accessor**: optionally add `pub async fn plans_by_id(&self, id: &str) -> Option<IndexEntry>`
    parallel to `work_item_by_id` at `indexer.rs:590-593`. Not
    strictly required by Phase 7, but parallel to the existing API
    shape; downstream UI consumers (visualiser-graph epic) may use
    it. May be deferred if there is no immediate consumer.

#### 2. Refactor `target_path_from_entry` to use `plans_by_id`

**File**: `skills/visualisation/visualise/server/src/indexer.rs`
**Changes**: Refactor `target_path_from_entry` (currently lines
744-750) to accept the additional `plans_by_id` snapshot and resolve
typed-linkage `target:` values via the new index. Updated signature:

```rust
fn target_path_from_entry(
    entry: &IndexEntry,
    plans_by_id: &HashMap<String, PathBuf>,
    project_root: &Path,
) -> Option<PathBuf> {
    if entry.r#type != DocTypeKey::PlanReviews {
        return None;
    }
    let raw = entry.frontmatter.get("target")?.as_str()?;
    // ADR-0034 §"Forms": `target:` may be path-form or `doc-type:id` form.
    if let Some(id) = raw.strip_prefix("plan:") {
        return plans_by_id.get(id).cloned();
    }
    normalize_target_key(raw, project_root)
}
```

All five existing call sites must be updated to pass a `plans_by_id`
snapshot:

| Call site | Lock context | Snapshot source |
|---|---|---|
| Pass B (new — see "Rescan-loop ordering caveat" below; replaces the line-319 call site, which is deleted as part of the two-pass split) | local fully-populated `plans_by_id` HashMap from Pass A | pass `&plans_by_id` by reference |
| `declared_outbound` (line 644) | already holds `entries.read()` AND `work_item_by_id.read()` | acquire `self.plans_by_id.read().await` AFTER both (lock-ordering: entries → work_item → plans) and pass `&*guard` |
| `update_reviews_by_target` for prev (line 836) | caller (`refresh_one`) holds `entries.write()` | accept new `plans_by_id` parameter; caller acquires `plans_by_id.read().await` after entries.write() and passes through |
| `update_reviews_by_target` for next (line 837) | same as above | same |
| `remove_from_reviews_by_target` (line 872) | caller (`refresh_one`) holds `entries.write()` | accept new `plans_by_id` parameter; caller acquires read snapshot after entries.write() |

All read-side call sites must acquire `plans_by_id.read()` BEFORE
any `reviews_by_target` lock (the new invariant slot
`entries → adr → work_item → plans → reviews_by_target →
work_item_refs_by_target`). Update the existing invariant doc-comment
at `indexer.rs:341-342` to reflect the new ordering.

**Rescan-loop ordering caveat**: in the build loop at line 319, the
local `plans_by_id` HashMap is built incrementally — when processing
entry K, only entries 0..K-1 are in `plans_by_id`. A plan-review
processed before its referenced plan would fail to resolve. To make
the resolution order-independent, split the rescan loop into two
passes:

- **Pass A** (lines 305-337 today): parse and fold all entries into
  `entries`, `adr_by_id`, `work_item_by_id`, **`plans_by_id`**, and
  `work_item_refs_by_target`. Defer building `reviews_by_target`.
- **Pass B** (new, after Pass A completes): iterate the populated
  `entries` map, call `target_path_from_entry(&entry, &plans_by_id, &self.project_root)`
  for every plan-review entry, and populate `reviews_by_target` with
  the resolved keys.

This is a deliberate behavioural change — `reviews_by_target` now
sees a complete `plans_by_id` regardless of `read_results` iteration
order — and is the correctness fix that supports the new id-based
resolution.

#### 3. Extend `read_ref_keys` (frontmatter.rs)

**File**: `skills/visualisation/visualise/server/src/frontmatter.rs`
**Changes**: At lines 327-346, after the existing
`work_item_id:` / `work-item:` / `ticket:` fallback chain, add a
final fallback that extracts a work-item id from a typed-linkage
`target:` scalar:

```rust
} else if let Some(v) = m.get("target") {
    // Final fallback per ADR-0034 §"Forms": typed-linkage `doc-type:id`
    // form. Currently only `work-item:` is extracted here; other
    // prefixes are not aggregated into work-item refs.
    if let Some(s) = extract_scalar(v) {
        if let Some(id) = s.strip_prefix("work-item:") {
            if !id.is_empty() {
                refs.push(id.to_string());
            }
        }
    }
}
```

The transitional `work_item_id:` alias on `work-item-review`
artifacts continues to be preferred when present; the `target:`
extraction is reached only when the alias is absent (post-cleanup
state).

#### 4. Add Rust unit tests

**File**: `skills/visualisation/visualise/server/src/indexer.rs` (test module)
**Changes**: Add unit tests covering:
- **`target: "plan:<existing-id>"` resolves via `plans_by_id`** to
  the matching plan's path; returns `None` when no plan with that
  id is indexed.
- **`target: "meta/plans/...md"` (existing path-form)** still
  resolves correctly via `normalize_target_key`.
- **Build-loop ordering independence**: a plan-review whose source
  plan appears *after* it in the file-driver enumeration still
  resolves correctly post-rescan (validates the two-pass build
  approach from §2).
- **Malformed prefixes** (`target: "plan:"` empty id,
  `target: "plan:/../../etc/passwd"`, `target: "plan:no-such-id"`)
  return `None` without path-traversal escape.
- **`plans_by_id` lifecycle**: `refresh_one` correctly inserts on
  add, updates on id-change, and removes on plan deletion
  (parallel to existing `work_item_by_id_lifecycle` tests at
  `indexer.rs:~1820-1865`).

**File**: `skills/visualisation/visualise/server/src/frontmatter.rs` (test module)
**Changes**: Add unit tests covering:
- `target: "work-item:0042"` with no `work_item_id:` returns
  `vec!["0042"]` (the post-cleanup steady state).
- `target: "work-item:0042"` with `work_item_id: "0007"` returns
  `vec!["0007"]` (the alias still wins — precedence rule).
- **Both fields, same value**: `target: "work-item:0042"` with
  `work_item_id: "0042"` returns `vec!["0042"]` (single entry — no
  double-counting through the alias-then-target fallback chain).
- **Non-work-item prefix exactness**: `target: "plan:2026-..."`
  with no other keys returns an empty vec (no spurious
  extraction).
- `target: "pr:123"` returns an empty vec (prefix-match is exact;
  pr-prefix does not contribute to work-item refs).

#### 5. Re-run cargo tests

```bash
cd skills/visualisation/visualise/server && cargo test --lib
# Expected: all existing tests still pass; new tests added in §4 pass.
```

### Success Criteria:

#### Automated Verification:

- [ ] `cargo test --lib` passes inside `skills/visualisation/visualise/server/`.
- [ ] `rg -n "plans_by_id" skills/visualisation/visualise/server/src/indexer.rs` returns hits for the new field declaration (line ~206), constructor init, rescan accumulator, lock acquisition, refresh_one helpers (update_plans_by_id / remove_from_plans_by_id), and `plan_id_from_entry` helper.
- [ ] `rg -n "target_path_from_entry\(" skills/visualisation/visualise/server/src/indexer.rs` shows the refactored 3-arg signature (`entry`, `plans_by_id`, `project_root`) at every call site.
- [ ] New unit tests asserting `plan:<id>` resolution via `plans_by_id` exist and pass; build-loop ordering independence test passes.
- [ ] New unit tests asserting `work-item:` prefix extraction in `read_ref_keys` exist and pass (including the alias-precedence and same-value-no-double-counting cases).
- [ ] `bash scripts/test-format.sh` passes (no regressions).

#### Manual Verification:

- [ ] Run the visualiser server against a workspace with at least one Phase-2 plan-review (`target: "plan:<id>"`); confirm the plan detail page's "Reviews" section resolves and displays the review correctly.
- [ ] Run the visualiser server against a workspace with at least one Phase-3 work-item-review (`target: "work-item:<id>"`); confirm the work-item detail page's "Referenced By" section lists the review.
- [ ] Repeat both checks with the transitional `work_item_id:` alias removed from a sample work-item-review (manually) to confirm the `target:`-only extraction path works.

---

## Testing Strategy

### Unit Tests:

- **Template-shape contract** (`scripts/test-template-frontmatter.sh`,
  existing): three new rows added in Phase 1 fail until Phases 2-4
  create the corresponding template files; one existing row
  (`validation.md`) was already GREEN from 0065.
- **SKILL-prose population** (`scripts/test-skill-frontmatter-population.sh`,
  existing): four new rows added in Phase 1 fail until Phases 2-5
  rewire the corresponding skills.
- **Discovery assertion** (existing phase-11 logic in the same test
  driver): asserts every SKILL.md surfaced by Pass-A or Pass-B is
  allowlisted. After Phase 1's allowlist move, this stays GREEN
  throughout the per-skill phases. Phase 6 confirms the final state.
- **No new test scripts are created.** 0066 reuses 0065's TSV-driven
  scaffolding verbatim.

### Integration Tests:

- **End-to-end production of one artifact per template**: for each
  rewired skill in Phases 2-5, run the skill once against a
  representative input and parse the resulting file's frontmatter.
  Confirm:
  - `producer`, `schema_version`, `last_updated`, `last_updated_by`,
    `target`, `reviewer` (review templates only), `verdict` (review only),
    `lenses` (review only), `review_number` (review only), `review_pass`
    (plan-review + work-item-review only), `result` (plan-validation
    only) are populated with non-empty, non-tokenised values;
  - `schema_version == 1` (bare integer);
  - the two timestamps parse as ISO UTC;
  - `target` matches the regex pinned per skill (`^"plan:.+"$`,
    `^"work-item:[0-9]{4}"$`, `^"pr:[0-9]+"$`). Note: the work-item
    regex is `{4}` not `{4,}` — it asserts exactly 4 digits per the
    work-item id canonical form pinned by ADR-0033 §Identity-value
    shape contract.

These integration checks are predominantly manual because the producer
skills are LLM-driven (this matches 0065's accepted limitation —
the SKILL-prose test enforces the substitution-instruction contract;
the end-to-end populated-values check remains manual).

### Manual Testing Steps:

1. Run `review-plan` against a real plan (e.g. one of the existing
   plans in `meta/plans/`). Confirm the resulting `-review-N.md` file
   has `target: "plan:<plan-id>"`, `verdict` populated, all base
   fields populated, no `\{...\}` token survivors.
2. Run `review-work-item` against a real work item. Confirm
   `target: "work-item:0042"` (or similar, matching the work item's
   `id`), no `work_item_id:` field present, all base fields populated.
3. Run `review-pr` against a real PR (any small PR for which review
   posting can be skipped — use option 4 "Discuss findings" at the end
   to avoid actually posting). Confirm `target: "pr:<n>"` matches
   `^"pr:[0-9]+"$`, `title` carries the PR title (no `pr_title:`), no
   `review_pass:` field present, `pr_number` is a bare integer.
4. Run `validate-plan` against a real plan. Confirm
   `target: "plan:<plan-id>"`, `result` populated, all base fields
   populated. If `result: pass`, confirm the plan's `status:` was
   updated to `complete` (existing side-effect).
5. Re-run a previously-reviewed plan or work item (Step 7 of the
   respective skill). Confirm the in-place update preserves all
   non-mutated fields verbatim, the four-field mutation
   (`verdict`/`review_pass`/`last_updated`/`last_updated_by`) applies,
   `date` retains the original-review timestamp per Design Decision #5,
   and the body's prior content is preserved.
6. Re-run the discovery greps from §"Discovery Pass Record" in a
   fresh shell. Confirm the recorded producer split matches.

## Performance Considerations

None of substance. Templates and SKILL.md files are small; the test
scripts run in <1s each (same scale as 0065). The
`config-read-template.sh` call adds a single bash invocation per skill
load — already in the per-skill cost envelope established by 0065.

## Migration Notes

- **Existing `meta/` review/validation artifacts are NOT touched by this
  story.** The corpus migration (0070) handles them. After 0066,
  newly-produced review/validation artifacts diverge from the existing
  corpus in shape — that is expected and is what 0070 will reconcile.
- **0066 closes the 0065 → 0066 "born unified" gap window for
  plan-validation.** Between 0065 landing and 0066 landing,
  `validate-plan/SKILL.md` continues to emit its legacy inline
  frontmatter (`skill:`, narrow shape) — the template's new frontmatter
  block is dead code. Phase 5 of this plan reaches into that gap and
  closes it; order 0066 close behind 0065 (already the dependency
  pattern).
- **0066 closes the `pr_title:` cross-artifact-rename carryover from
  0065.** 0065 renamed `pr_title:` → `title:` on `pr-description.md`
  but left `review-pr/SKILL.md:458`'s inline `pr_title:` in place.
  Phase 4 retires that legacy field from `pr-review` artifacts.
  Post-0066, no in-source SKILL.md carries `pr_title:` outside fenced
  template-example blocks.
- **0066 does NOT introduce a re-review lifecycle in `review-pr`.**
  Re-running `review-pr` continues to produce a fresh
  `-review-{N+1}.md` rather than mutating the prior file (Design
  Decision #1). If a future story introduces in-place re-review for
  `review-pr`, that story owns adding `review_pass:` to
  `templates/pr-review.md` and the canonical persistence-step snippet
  here.
- **0093 (sibling story) should land after 0066.** It adds optional
  typed-linkage slots (`blocks`, `blocked_by`, `derived_from`,
  `relates_to`) to every template, including the three new review
  templates this story creates. 0066 deliberately does NOT add those
  slots — that is 0093's territory.
- **Re-review mutation behaviour change for `review-plan` and
  `review-work-item`.** Pre-0066 re-reviews bumped `date` on every
  pass. Per Design Decision #5, `date` is now removed from the
  mutation list — it retains the original-review timestamp, while
  `last_updated`/`last_updated_by` advance. Re-running a re-review on
  a *pre-0066 review artifact* (one that lacks `last_updated`/
  `last_updated_by` fields) requires the in-memory mutation to
  **insert** the missing fields rather than treat their absence as
  malformed-frontmatter. Pin this in the per-skill prose: only an
  unparseable YAML block or missing `---` delimiters triggers the
  fresh-`-review-{N+1}.md` fallback; an otherwise-valid frontmatter
  block missing the two new fields has them inserted alongside the
  other mutation-list fields.
- **`validate-plan`'s `skill:`→`producer:` divergence.** Pre-0066
  validation reports carry `skill: validate-plan`; post-0066 reports
  carry `producer: validate-plan`. `validate-plan` has no in-place
  re-review flow, so re-running it on a previously-validated plan
  writes a fresh report with the new shape — the two reports coexist
  with diverging shape until 0070 reconciles. No in-source consumer
  reads the legacy `skill:` field (confirmed by grep), so the
  divergence is observational only.
- **Consumer-side breakage surface:** the four rewired SKILL.md files
  consume their own emitted frontmatter on re-review (Step 7
  self-loop). The visualiser server (Rust) is also an in-source
  consumer:
  - `indexer.rs:744-750` (`target_path_from_entry`) reads `target:` on
    `PlanReviews` entries and expects a project-root-relative path
    string (e.g. `meta/plans/2026-...md`). Phase 2 emits
    `target: "plan:<plan-id>"` instead, which would not normalise to a
    valid path — Phase 7 teaches `target_path_from_entry` to accept
    both forms.
  - `frontmatter.rs:305-356` (`read_ref_keys`) reads `work_item_id:`
    (not `target:`) as the primary scalar key for work-item
    cross-reference aggregation across all artifact types. Phase 3
    keeps `work_item_id:` as a transitional alias on
    `work-item-review` (Design Decision #2 revised) so the aggregation
    continues to resolve; Phase 7 teaches `read_ref_keys` to also
    extract work-item ids from `target: "work-item:<id>"` values,
    after which the transitional alias can be retired (follow-up
    release).
  - No other in-source skill or script parses these four artifact
    types' frontmatter today (confirmed by the Phase 6 discovery
    pass).

## References

- Original work item: `meta/work/0066-update-review-skills-inline-frontmatter.md`
- Related research: `meta/research/codebase/2026-05-31-0066-update-review-skills-inline-frontmatter.md`
- Authoritative schema ADR: `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`
- Authoritative linkage ADR: `meta/decisions/ADR-0034-typed-linkage-vocabulary.md`
- Parent epic: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Blocked-by predecessors (all done): 0060 (ADR-0033), 0061 (ADR-0034), 0065 (validation.md frontmatter + test scaffolding handoff)
- Blocks: 0070 (corpus migration)
- Related: 0064 (canonicalised `work_item_id` foreign-reference shape this story drops in favour of typed `target`), 0093 (typed-linkage slot extension to every template, including the 3 new review templates)
- Predecessor plan (canonical structure mirrored here): `meta/plans/2026-05-30-0065-update-artifact-templates-to-unified-schema.md`
- Predecessor review (process patterns adopted): `meta/reviews/work/0066-update-review-skills-inline-frontmatter-review-1.md`
- Template-loader (no extension needed): `scripts/config-read-template.sh:36-50`
- Closest-analogue existing template: `templates/validation.md:1-15`
- Canonical inclusion + persistence pattern (mirror these): `skills/work/create-work-item/SKILL.md:25-32` and `:436-459`
- Test driver — template shape: `scripts/test-template-frontmatter.sh`
- Test driver — SKILL prose + discovery: `scripts/test-skill-frontmatter-population.sh`
- Mise task wiring: `mise.toml` (`test:unit:templates`)
- Schema TSVs: `scripts/templates-schema.tsv`, `scripts/skills-schema.tsv`
- Eval fixtures to update (Phase 3): `skills/work/review-work-item/evals/evals.json:30-40`, `skills/work/review-work-item/evals/benchmark.json:89-100`
