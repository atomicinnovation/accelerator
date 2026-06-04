---
type: plan
id: "2026-06-02-0093-extend-templates-with-typed-linkage-slots"
title: "Extend Templates With Typed-Linkage Slots Implementation Plan"
date: "2026-06-02T18:00:37+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "0093"
parent: "work-item:0093"
reviewer: "review-plan (3 passes, APPROVE)"
tags: [templates, frontmatter, schema, linkage, adr-0034, emission]
revision: "2121d6cd6f4472d3c48e156116c469694876f9c5"
repository: build-system
last_updated: "2026-06-04T15:50:30+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Extend Templates With Typed-Linkage Slots Implementation Plan

## Overview

Two coupled changes:

1. **Typed-linkage slots.** Project the ADR-0034 typed-linkage vocabulary
   onto the twelve artifact templates as documented optional slots, and
   extend the template- and skill-shape contract tests to enforce the
   slot/comment/closed-set rules.
2. **Omit-when-empty emission.** Adopt a corpus-wide rule that producers
   emit an *optional non-base* frontmatter key only when it resolves to a
   non-empty value, and omit it entirely otherwise. This covers the new
   linkage keys **and** the existing optional fields (`work_item_id`,
   `external_id`, `decision_makers`, and the lifecycle markers `reviewer`,
   `pr_url`, `merge_commit`). The convention is recorded as **ADR-0040**.

Templates keep every documented slot present-but-empty as the authoring
surface; generated artifacts carry only the keys that have values. Phase 0
records the ADR; the remaining six phases implement templates, tests, and
the producer-skill sweep, tests-first per phase.

## Current State Analysis

The codebase research at
`meta/research/codebase/2026-06-02-0093-extend-templates-with-typed-linkage-slots.md`
is authoritative; this plan does not re-derive its findings. The
implementation-relevant state is:

- Five of the eleven required linkage keys appear in templates today
  (`parent`, `supersedes`, `target`, plus the two design-gap inventory
  carve-outs); six are entirely absent (`superseded_by`, `blocks`,
  `blocked_by`, `derived_from`, `relates_to`, `source` in its
  typed-linkage sense).
- Three inline-comment styles drift across templates — the canonical
  list form on `adr.md:10` matches the normative grammar; the single-ref
  form on `work-item.md:11` uses `key:` / `empty` wording instead of
  `ref:` / `""`; the reviewer/validation comments carry lifecycle
  annotations (`per ADR-0034`, `(filled by review-plan)`).
- The corpus already emits optional keys inconsistently: the 0093 work
  item carries `parent: "0057"` (valued) and `external_id: ""` (empty);
  the root epic 0057 carries `parent: ""` (empty). Both empty and absent
  optional keys are read as "no value" by every consumer (see below), so
  the corpus is *already* tolerant of omit-when-empty — this plan makes
  it the deliberate, uniform convention.
- ADR-0033 mandates the **base fields** (`type`, `id`, `title`, `date`,
  `author`, `producer`, `status`, `tags`, `last_updated`,
  `last_updated_by`, `schema_version`) are present on every artifact;
  `tags` is explicitly "possibly empty". Neither ADR-0033 nor ADR-0034
  mandates present-but-empty emission for *optional non-base* fields — so
  omitting those when empty contradicts neither accepted ADR.
- `scripts/templates-schema.tsv` has six columns; rows for the four
  review/validation templates carry `target` in the `extras` column.
- `scripts/test-template-frontmatter.sh` performs presence-only extras
  assertions; no value-shape, comment-grammar, or closed-set checks.
- `scripts/test-skill-frontmatter-population.sh` already walks
  `scripts/skills-schema.tsv` per producer skill, asserting each field
  appears in a fenced block or imperative section. The "fill" / "omit"
  guidance assertion is new.
- Producer SKILL.md files split into four shape groups (Research §
  "SKILL.md Populate-frontmatter snippet current state"):
  Group A writer canonical (8), Group B reviewer (4), Group C design
  (2), Group D refine-work-item (1).
- The visualiser server's typed-linkage parser
  (`skills/visualisation/visualise/server/src/typed_ref.rs:26` +
  `cluster_key.rs:138-152`) tolerates both typed-linkage form
  (`"work-item:0001"`) and bare-id form (`"0001"`) for work-item
  references, and treats empty `""`/`[]` and absent keys identically as
  absent edges.
- The full test suite is invoked via `mise run test`; the template
  and skill schema tests run as `mise run test:unit:templates`.

## Desired End State

After all phases land:

- All twelve in-scope templates carry exactly the typed-linkage slots
  enumerated in work item §2 — present-but-empty, with the normative
  comment grammar (see *Normative comment grammar* below). The
  design-gap inventory carve-out (`current_inventory`,
  `target_inventory`) is preserved verbatim.
- **ADR-0040** records the omit-when-empty emission convention
  (supplements ADR-0033) and is `accepted`.
- Generated artifacts use **omit-when-empty** emission: producers write
  an optional non-base key only when it has a non-empty value and omit
  it otherwise. Base fields (incl. `tags`) and always-valued per-type
  extras remain present. No generated artifact carries empty `""` / `[]`
  optional placeholders — the empty slots live only in the templates, as
  the documented authoring surface (mirroring how the template's
  `status: ""  # …` becomes a bare `status: ready` in artifacts).
- `scripts/templates-schema.tsv` has a seventh column
  `typed_linkage_keys`; `scripts/test-template-frontmatter.sh` asserts
  per template: (a) every expected linkage slot is present,
  (b) value shape matches cardinality, (c) inline comment matches
  the cardinality-specific regex (including the inverse-key trailing
  sentence for `superseded_by` / `blocked_by`), (d) the template
  carries no linkage-vocabulary key absent from the row.
- `scripts/skills-schema.tsv` has a new `omit_when_empty` column;
  `scripts/test-skill-frontmatter-population.sh` asserts each named
  field is present in the Populate-frontmatter section of the named
  skill AND that a one-line guidance note containing "fill" or
  "omit" accompanies it.
- All fifteen producer SKILL.md files name every applicable
  omit-when-empty field (linkage keys + foreign refs + lifecycle
  markers) in their Populate-frontmatter step, with the guidance note.
  The four reviewer skills gain a literal `Populate frontmatter`
  heading. The design skills (`inventory-design`, `analyse-design-gaps`)
  coexist with the design-gap inventory carve-out unchanged.
- All four bare-id producers move to the typed-linkage form uniformly
  (per the research's recommendation to treat them as one change):
  `create-work-item` and `extract-work-items` emit `parent:
  "work-item:NNNN"` (introduced via their Phase 3 guidance bullets);
  `refine-work-item` emits `parent: "work-item:NNNN"` (Phase 6); and
  `create-plan` emits `work_item_id: "work-item:NNNN"` rather than the
  bare `"NNNN"` form, omitting it when there is no linked work item
  (Phase 6). `refine-work-item` is also promoted from
  `NON_EMITTER_TEMPLATE_CONSUMERS` to `IN_SCOPE_PRODUCERS` in the
  skill-test allowlist and gains a proper Populate-frontmatter snippet.
  All four changes are runtime-compatible (`cluster_key.rs:138-152`
  tolerates both shapes). The id-placeholder convention is pinned:
  ADRs use the literal `ADR-NNNN` form (`adr:ADR-NNNN`); every other
  source-type uses `NNNN` (`work-item:NNNN`, `plan:NNNN`, etc.).
- `mise run test:unit:templates` passes; `mise run test` passes;
  CI green on `.github/workflows/main.yml`.

### Emission classification (the omit-when-empty boundary)

The boundary the convention (and ADR-0040) draws:

**Always emitted (present on every artifact):**

- **Base fields (ADR-0033 mandate):** `type`, `id`, `title`, `date`,
  `author`, `producer`, `status`, `tags`, `last_updated`,
  `last_updated_by`, `schema_version`. `tags: []` stays when empty.
- **Provenance (code-state-anchored types):** `revision`, `repository`.
  (design-inventory already omits these when the source is not a code
  repo — a pre-existing, separate carve-out, left as-is.)
- **Always-valued per-type extras:** `kind`, `priority` (work-item);
  `topic` (research); `result`, `verdict`, `lenses`, `review_number`,
  `review_pass` (reviews/validation); `pr_number` (pr-description,
  pr-review); `source`, `source_kind`, `source_location`, `crawler`,
  `sequence`, `screenshots_incomplete` (design-inventory);
  `current_inventory`, `target_inventory` (design-gap); the transitional
  `work_item_id` alias on work-item-review (mirrors `target`, always
  valued).

**Emitted only when non-empty (omit otherwise):**

- **Typed-linkage keys (ADR-0034):** `parent`, `blocks`, `blocked_by`,
  `derived_from`, `relates_to`, `source`, `supersedes`, `superseded_by`,
  `target`.
- **Foreign references:** `work_item_id`, `external_id`.
- **Optional extras / lifecycle markers:** `decision_makers` (adr),
  `reviewer` (plan), `pr_url` and `merge_commit` (pr-description).

### Normative comment grammar (extracted block — Pass 3 polish item a)

The template-shape test enforces these two regex shapes exactly. The
inverse-key guidance sentence lives on its own dedicated comment line
(not appended to the grammar comment), so the single shared list regex
below — anchored at `[]` end-of-line — matches every list slot
including `blocked_by`; a separate literal-substring post-check then
confirms the guidance line is present for `blocked_by`. The
`<source-type>` token is constrained to the curated set listed below.

**Single-ref slots** (`parent`, `target`, `source`):

```
<key>: ""                                   # typed-linkage ref: "<source-type>:NNNN" or ""
```

(`superseded_by` is intentionally absent from the active grammar: no
template carries it — ADRs are immutable, so only the canonical
`supersedes` is exposed and consumers derive the inverse per ADR-0034.
It is retained in the closed-set vocabulary below purely as a guard, so
the test rejects any template that adds it.)

POSIX ERE:

```
^(parent|target|source):[[:space:]]+""[[:space:]]+#[[:space:]]+typed-linkage[[:space:]]+ref:[[:space:]]+"(SOURCE_TYPE):[A-Za-z0-9-]+"[[:space:]]+or[[:space:]]+""$
```

**List-cardinality slots** (`supersedes`, `blocks`, `blocked_by`,
`derived_from`, `relates_to`):

```
<key>: []                                   # typed-linkage list: ["<source-type>:NNNN", ...] or []
```

POSIX ERE:

```
^(supersedes|blocks|blocked_by|derived_from|relates_to):[[:space:]]+\[\][[:space:]]+#[[:space:]]+typed-linkage[[:space:]]+list:[[:space:]]+\["(SOURCE_TYPE):[A-Za-z0-9-]+",[[:space:]]+\.\.\.\][[:space:]]+or[[:space:]]+\[\]$
```

**Inverse-key guidance line** (additional check for `blocked_by` only).
The sentence sits on its own full-line comment immediately below the
`blocked_by` slot — never appended to the grammar comment, so the list
regex's `\[\]$` anchor is never broken. After the main list regex
matches, the test runs `grep -qF` for this exact line against the whole
frontmatter block (not against the `blocked_by` line):

```
# inverse of blocks — producers SHOULD prefer writing blocks: on the canonical side
```

**`SOURCE_TYPE` curated set** (from the in-script associative array,
matched verbatim in the regex):

```
work-item | plan | adr | pr | codebase-research | issue-research |
pr-description | design-inventory | design-gap | plan-validation |
plan-review | work-item-review | pr-review
```

### Per-template typed-linkage slot set (closed)

Authoritative copy of work item §2, restated here so the plan is
self-contained:

| Template (file)              | Type                | Required typed-linkage slots                                                           |
|------------------------------|---------------------|----------------------------------------------------------------------------------------|
| `work-item.md`               | work-item           | `parent`, `blocks`, `blocked_by`, `derived_from`, `relates_to`, `source`               |
| `plan.md`                    | plan                | `parent`, `blocks`, `blocked_by`, `derived_from`, `relates_to`                         |
| `adr.md`                     | adr                 | `parent`, `supersedes`, `relates_to`                                                   |
| `codebase-research.md`       | codebase-research   | `parent`, `relates_to`                                                                 |
| `rca.md`                     | issue-research      | `parent`, `relates_to`                                                                 |
| `pr-description.md`          | pr-description      | `parent`, `relates_to`                                                                 |
| `design-inventory.md`        | design-inventory    | `parent`, `relates_to`                                                                 |
| `design-gap.md`              | design-gap          | `parent`, `relates_to` (plus existing `current_inventory`/`target_inventory` carve-out) |
| `validation.md`              | plan-validation     | `parent`, `target`, `relates_to`                                                       |
| `plan-review.md`             | plan-review         | `parent`, `target`, `relates_to`                                                       |
| `work-item-review.md`        | work-item-review    | `parent`, `target`, `relates_to`                                                       |
| `pr-review.md`               | pr-review           | `parent`, `target`, `relates_to`                                                       |

### Key Discoveries

- **Both empty and absent optional keys read as "no value"**.
  `typed_ref.rs:62-64` returns `None` on empty; `cluster_key.rs:138-152`
  tolerates bare-id and typed form; `read_ref_keys` treats absent keys
  as absent edges. So omit-when-empty changes no consumer behaviour.
- **`adr:` and `pr:` arms of `TypedRef` are silently ignored today**
  (`indexer.rs:885` `_ => None`, `frontmatter.rs:351`-onwards
  `WorkItem`-only). Emitting `parent: "adr:ADR-NNNN"` on an ADR is
  currently a no-op for the visualiser — consistent with "absent edge"
  semantics.
- **`carries_target_frontmatter()` covers every review/validation
  type** (`docs.rs:88-96`, regression test at `indexer.rs:2606-2633`).
  Adding `target: ""` to all four reviewer templates is already
  expected by the indexer.
- **`scripts/test-skill-frontmatter-population.sh:31-52` carries an
  allowlist split** (`IN_SCOPE_PRODUCERS` vs
  `NON_EMITTER_TEMPLATE_CONSUMERS`). `refine-work-item` is currently
  in the non-emitter list; it moves to `IN_SCOPE_PRODUCERS` in Phase 6.
- **The Schema Reference cross-check at lines 154-189 of
  `test-template-frontmatter.sh`** parses tables from
  `meta/work/0065-...md` and `meta/work/0066-...md`. Neither table
  carries a `typed_linkage_keys` column today; no change to the
  cross-check is needed.

## What We're NOT Doing

- **Carrying empty optional placeholders into generated artifacts**.
  Generated artifacts omit any optional non-base key with no value;
  only the templates carry the empty `""` / `[]` placeholders. See
  *Emission model* under Implementation Approach.
- **Omitting base fields**. ADR-0033 mandates the base set (incl.
  `tags`) is present on every artifact; `tags: []` stays. Base and
  always-valued per-type fields are out of scope for omit-when-empty.
- **Corpus migration on existing artifacts**. Existing files under
  `meta/` keep their current shape. 0070 owns inferring links and
  writing them into the new template slots. (0070's writes will follow
  the new emit-when-valued convention.)
- **No rename of `design-inventory.md`'s existing `source:` field**
  (Open Question #5 in research). It stays in the `extras` column as
  a foreign-source identifier per ADR-0033; it does not collide with
  the typed-linkage `source` key because work item §2 does not
  require `source:` on `design-inventory.md`. It is always valued, so
  omit-when-empty never applies to it. The closed-set check is
  name-based, so it must explicitly **exempt vocabulary names that the
  row declares as extras** (Phase 1 §2d) — otherwise it would
  misclassify this `source:` and FAIL on a template that is out of
  scope. The exemption is part of Phase 1.
- **No changes to `design-gap.md`'s `current_inventory` /
  `target_inventory` keys**. They remain in `extras` per ADR-0034
  §"Design-gap inventory keys"; AC #4 requires this carve-out be
  preserved verbatim. They are always valued.
- **No extension of the linkage-key vocabulary beyond ADR-0034**.
  This plan implements the vocabulary as currently written.
- **No discovery-pattern additions to
  `test-skill-frontmatter-population.sh`'s Phase 11 patterns**
  (lines 158-191). The existing patterns are sufficient.
- **No changes to the visualiser server**. The Rust code already
  treats empty and absent keys identically and tolerates both ref
  shapes; no audit required beyond the read-only checks in the research.
- **No artifact-level enforcement of omit-when-empty (accepted
  tradeoff)**. This plan enforces the convention only at the *guidance*
  level — the skill-test checks that each producer SKILL.md names the
  field with a fill/omit note. No test inspects a *generated artifact*
  to confirm an empty optional key was actually omitted, and because
  consumers read empty and absent identically, a producer that emits
  `external_id: ""` would violate the convention with no test signal.
  This is a deliberate scope boundary (the alternative — running each
  producer and asserting its output shape — is heavier than this story
  warrants); it is recorded here so **0070 does not assume the
  convention is structurally enforced** and so a future fixture-based
  producer-output test can be scoped as follow-up if drift appears.

## Implementation Approach

Seven phases. **Phase 0** records the emission-convention ADR (the
authoritative input). The remaining six implement it: Phases 1–2 own the
template surface and its test; Phase 3 lays the skill-test
infrastructure; Phases 4–6 sweep the producer skills. Phase 6's
normalisation work depends on Phase 4 having established the writer-canon
snippet shape, so it follows that sweep deliberately.

Within each implementing phase, the test extension is written first and
watched fail before the content moves to satisfy it (TDD discipline; not
separate merges). Templates land before SKILL.md updates so a partial
sweep can be reverted without touching templates (Pass 3 polish item d).

The plan does not introduce a feature flag. The new slots are additive
and optional, and empty/absent values are both "no value" to every
consumer — so nothing existing can break.

### Emission model: omit-when-empty (records ADR-0040; supersedes work item §1/§3 for artifacts)

Per a planning decision (2026-06-03), **generated artifacts** use
omit-when-empty emission for every *optional non-base* frontmatter key
(the boundary is the *Emission classification* table above). A producer
writes the key only when it resolves to a non-empty value and **omits it
entirely** otherwise. Templates still carry every documented slot
present-but-empty with its comment (the authoring surface) — producers
resolve that scaffold the same way they already resolve
`status: ""  # draft | ready | …` into a bare `status: ready`: comments
are dropped and empty optional keys are simply not written.

This is recorded as **ADR-0040** (Phase 0) because it is a corpus-wide
emission rule affecting every producer, and neither ADR-0033 nor ADR-0034
states it today. It contradicts neither: ADR-0033 mandates *base*-field
presence only; ADR-0034 governs linkage *value shape*, not slot presence.

It supersedes two pieces of the work item's recorded text (updated in the
work item alongside this plan):

- Requirements §1 / AC #1 mandate "present-but-empty" slots. That still
  holds for **templates**; for **generated artifacts** optional keys are
  omitted when empty.
- Requirements §3 states "Empty slots are the default" and AC #3
  requires the per-skill guidance note to contain "fill" or
  "leave empty". The default is now **omission**, and the guidance note
  contains "fill" / "omit"; the Phase 3 skill-test keyword set matches.
  (The single authoritative keyword pair is **"fill" / "omit"**; any
  remaining "leave empty" phrasing in the work item is obsolete.)
- Requirements §1 mandates the inverse-key guidance as a *trailing
  sentence appended to the slot comment* and lists `superseded_by` among
  the inverse slots. This plan instead places the guidance on its **own
  standalone comment line** below the slot (so the gated list regex's
  `\[\]$` anchor is never broken — see *Normative comment grammar*), and
  carries the inverse guidance for `blocked_by` **only**: no template
  carries `superseded_by` (ADRs are immutable; only `supersedes` is
  exposed), so it lives solely in the closed-set guard, never as a slot.

`work_item_id` and `external_id` are foreign references (ADR-0033), not
typed-linkage keys, but they *are* optional non-base fields and so are
in scope for omit-when-empty under ADR-0040.

---

## Phase 0: Author the emission-convention ADR (ADR-0040)

### Overview

Record the omit-when-empty emission convention as ADR-0040 before the
implementing phases depend on it. Authored via `/accelerator:create-adr`
and accepted via `/accelerator:review-adr`. This phase is a decision
record, not a code change — it gates the rest of the plan.

### Changes Required

#### 1. New ADR file

**File**:
`meta/decisions/ADR-0040-omit-when-empty-frontmatter-emission-supplement-to-adr-0033.md`
(next sequential id per ADR-0029 — verify `0040` is still free at
authoring time; ids are allocated sequentially so a concurrent ADR could
claim it). The `-supplement-to-adr-0033` suffix follows the established
supplement-ADR filename convention set by ADR-0035 and ADR-0037, since
this ADR supplements ADR-0033.

**Content** — follow the house ADR section structure (`templates/adr.md`
and sibling supplements ADR-0033/0035/0037): **Context**, **Decision
Drivers**, **Considered Options**, **Decision**, **Consequences** (split
into Positive / Negative / Neutral), **References**. The scope table
lives inside Decision; the supplement relationship lives in Context and
References. Authored as `status: proposed` via `/accelerator:create-adr`,
moved to `accepted` via `/accelerator:review-adr`.

Match three further house-style elements the sibling supplements carry,
which `/accelerator:create-adr` normally emits but which must be present:

- **Dual title** in the H1, naming the supplemented ADR:
  `# ADR-0040: Omit-When-Empty Frontmatter Emission — supplement to ADR-0033`
  (cf. ADR-0035/0037 headings).
- **In-body `**Date** / **Status** / **Author**` block** beneath the
  title — house convention (cf. the ADR-0030 template; present on
  ADR-0035/0037) so the status/date/author are visible in rendered
  markdown, which hides the YAML frontmatter. (Verify ADR-0030's exact
  wording before citing it as a hard mandate; the block itself is
  demonstrably carried by both sibling supplements.)
- **Recursive-supplement clause** (in Decision or Consequences) stating
  that once accepted this supplement is itself immutable, and a future
  extension of the omit-when-empty boundary (e.g. a new optional-field
  class) must be recorded in a further supplementary ADR — mirroring
  ADR-0037 §"Recursive supplement clause".

- **Context**: ADR-0033 mandates base-field presence (`tags` possibly
  empty) but is silent on optional non-base fields; template convention
  had them present-but-empty; this produces verbose, noisy frontmatter on
  real artifacts. ADR-0034 adds the linkage vocabulary. Consumers treat
  empty and absent identically. This ADR supplements ADR-0033.
- **Decision Drivers**: real artifacts are cluttered with empty optional
  placeholders; consumers already read empty and absent identically;
  the corpus is already inconsistent (some files carry empty optionals,
  some omit them) so a uniform rule is needed.
- **Considered Options**: (a) keep present-but-empty everywhere (status
  quo — verbose, and contradicted by existing omissions); (b)
  omit-when-empty for optional non-base keys (chosen); (c) omit *all*
  non-mandatory keys including always-valued extras (rejected — loses
  useful always-present signal and conflicts with ADR-0033's `tags`
  rule).
- **Decision**: producers emit an optional non-base frontmatter key only
  when it resolves to a non-empty value; otherwise the key is omitted.
  Base fields (incl. `tags`) and always-valued per-type extras are always
  present. Templates retain documented empty slots as the authoring
  surface. Include the **scope table** here — the *Emission
  classification* (always-emitted vs omit-when-empty) from this plan,
  verbatim. State that the typed-linkage form (`"work-item:NNNN"`) is the
  canonical shape for new writes and the bare-id form is a tolerated
  legacy shape.
- **Consequences**:
  - *Positive*: cleaner, less noisy artifacts; one uniform rule across
    producers; matches the corpus's existing tolerance.
  - *Negative / reader-facing rule*: an absent optional key MUST be read
    as "no value", never as an error or missing data — there is no
    in-artifact signal distinguishing a deliberate omission from an
    oversight, so consumers and readers rely on this rule. State it
    explicitly so downstream readers and 0070 can depend on it.
  - *Neutral*: template comments document the omit rule; producer
    SKILL.md guidance says fill/omit; no corpus migration (0070 owns it).
- **References / Relationship**: supplements ADR-0033 (base-field
  presence); ADR-0034 governs linkage value-shape, not slot presence;
  complements 0070 (its inferred-link writes follow this rule).

### Success Criteria

#### Automated Verification

- [x] ADR file exists: `test -f meta/decisions/ADR-0040-omit-when-empty-frontmatter-emission-supplement-to-adr-0033.md`
- [x] `mise run test:unit:templates` exits 0 (confirms this phase introduces no template/skill-schema regression; note this test validates the *template* files, not the authored ADR's frontmatter)

#### Manual Verification

- [x] ADR authored directly with full specified content and set `status: accepted` (content fully specified by this plan; not routed through interactive create-adr/review-adr)
- [x] ADR carries the full house section set (Context, Decision Drivers, Considered Options, Decision, Consequences split Positive/Negative/Neutral, References)
- [x] The scope table in the ADR matches the *Emission classification* in this plan exactly
- [x] The reader-facing rule ("an absent optional key means no value, never an error") appears in the Consequences section

---

## Phase 1: Extend template test infrastructure and update templates

### Overview

Extend `scripts/templates-schema.tsv` with the seventh column, extend
`scripts/test-template-frontmatter.sh` with the four new assertions
(slot presence, value shape, comment grammar, closed-set), and update
the twelve in-scope templates so the test passes. Tests-first within
the phase: the test extension is written and watched fail before the
templates are updated.

### Changes Required

#### 1. `scripts/templates-schema.tsv` — add seventh column

**File**: `scripts/templates-schema.tsv`

**Changes**: Add `typed_linkage_keys` as the seventh tab-separated
column. Populate per work item §2. Move `target` out of `extras` on
the four review/validation rows; `supersedes` is added to the new
column on `adr.md`. `current_inventory` / `target_inventory` stay
in `extras` on `design-gap.md`. `source` stays in `extras` on
`design-inventory.md` (foreign-source field, not typed-linkage).

```tsv
template	type	code_state_anchored	extras	status_vocab	forbidden_own_id_key	typed_linkage_keys
work-item.md	work-item	no	kind priority external_id	draft | ready | in-progress | review | done | blocked | abandoned	work_item_id	parent blocks blocked_by derived_from relates_to source
plan.md	plan	yes	reviewer	draft | ready | in-progress | done	-	parent blocks blocked_by derived_from relates_to
validation.md	plan-validation	no	result	complete	-	parent target relates_to
pr-description.md	pr-description	yes	pr_url pr_number merge_commit	complete	pr_title	parent relates_to
adr.md	adr	no	decision_makers	proposed | accepted | superseded | deprecated	adr_id	parent supersedes relates_to
codebase-research.md	codebase-research	yes	topic	complete	-	parent relates_to
rca.md	issue-research	yes	topic	complete	-	parent relates_to
design-inventory.md	design-inventory	yes	source source_kind source_location crawler sequence screenshots_incomplete	draft	-	parent relates_to
design-gap.md	design-gap	no	current_inventory target_inventory	draft	-	parent relates_to
plan-review.md	plan-review	no	reviewer verdict lenses review_number review_pass	complete	-	parent target relates_to
work-item-review.md	work-item-review	no	reviewer verdict lenses review_number review_pass work_item_id	complete	-	parent target relates_to
pr-review.md	pr-review	no	reviewer verdict lenses review_number pr_number	complete	pr_title review_pass	parent target relates_to
```

#### 2. `scripts/test-template-frontmatter.sh` — extend with linkage assertions

**File**: `scripts/test-template-frontmatter.sh`

**Changes**:

a. Update the field-count self-check from `NF != 6` to `NF != 7`
   (line 33). Update the surrounding error message and PASS line.

b. Extend the `while read` loop signature to bind the seventh column:

```bash
while IFS=$'\t' read -r template_file expected_type anchored extras status_vocab forbidden_own_id_key typed_linkage_keys; do
```

c. Add the cardinality lookup and supporting constants at script top
   (after `BASE_FIELDS`). The cardinality lookup is a `case`-based
   **function**, not a `declare -A` associative array: associative
   arrays require bash 4.0+, but the script's shebang is
   `#!/usr/bin/env bash` and macOS ships bash 3.2 by default, so a
   `declare -A` would abort the whole script (`declare: -A: invalid
   option`) under `set -euo pipefail` for any contributor running the
   tests locally — a regression CI (ubuntu, bash 5) would never catch.
   The `case` form carries identical logic and runs on 3.2:

```bash
# Cardinality lookup by linkage-key name. case-based (not `declare -A`)
# so the script keeps running on bash 3.2, the macOS default.
# Echoes `single`, `list`, or empty (unknown key).
linkage_cardinality() {
  case "$1" in
    parent|superseded_by|target|source)               echo single ;;
    supersedes|blocks|blocked_by|derived_from|relates_to) echo list ;;
    *)                                                echo "" ;;
  esac
}

# Curated source-type set used inside the comment regex. Kept as a
# pipe-joined string so it can be interpolated into ERE patterns.
SOURCE_TYPE_RE='work-item|plan|adr|pr|codebase-research|issue-research|pr-description|design-inventory|design-gap|plan-validation|plan-review|work-item-review|pr-review'

# The blocked_by inverse-key guidance line. It lives on its own
# full-line comment beneath the slot, so it never breaks the list
# regex's `\[\]$` end-anchor; the post-check greps for it across the
# whole block.
INVERSE_GUIDANCE_LINE='# inverse of blocks — producers SHOULD prefer writing blocks: on the canonical side'

# Union of all linkage-vocabulary key names (used by the closed-set
# check). Keep aligned with linkage_cardinality(). superseded_by is
# listed as a guard even though no template carries it, so the
# closed-set check rejects any template that adds it.
LINKAGE_VOCABULARY=(parent superseded_by target source supersedes blocks blocked_by derived_from relates_to)
```

   Also export `LC_ALL=C` at the top of the script (alongside the
   existing scripts' conventions) so the `[A-Za-z0-9-]` ranges and any
   `tolower`/`sort` behave deterministically regardless of the host
   locale — consistent with the project's existing `LANG=C` discipline.

d. Define the two assertions as **pure functions** that take a block +
   row metadata and **return** a status (0 = accept, non-zero = reject),
   touching no global counters. The live per-template loop wraps them
   for PASS/FAIL reporting, and the §2e self-test calls the SAME
   functions against fixtures — so the tested logic is byte-for-byte the
   live logic. This is the single authoritative form of these checks
   (there is no separate counter-mutating version):

```bash
# rc 0 = slot shape+comment valid (and, for blocked_by, the standalone
# inverse-guidance line is present); 1 = rejected; 2 = unknown key.
# The inverse-guidance check lives HERE (not in the live loop) so the
# §2e "missing inverse line" fixture exercises the same code path.
check_linkage_slot() {
  local block="$1" key="$2" regex
  case "$(linkage_cardinality "$key")" in
    single) regex="^${key}:[[:space:]]+\"\"[[:space:]]+#[[:space:]]+typed-linkage[[:space:]]+ref:[[:space:]]+\"(${SOURCE_TYPE_RE}):[A-Za-z0-9-]+\"[[:space:]]+or[[:space:]]+\"\"$" ;;
    list)   regex="^${key}:[[:space:]]+\\[\\][[:space:]]+#[[:space:]]+typed-linkage[[:space:]]+list:[[:space:]]+\\[\"(${SOURCE_TYPE_RE}):[A-Za-z0-9-]+\",[[:space:]]+\\.\\.\\.\\][[:space:]]+or[[:space:]]+\\[\\]$" ;;
    *)      return 2 ;;
  esac
  grep -qE "$regex" <<< "$block" || return 1
  if [ "$key" = blocked_by ]; then
    grep -qF -- "$INVERSE_GUIDANCE_LINE" <<< "$block" || return 1
  fi
  return 0
}

# rc 0 = no spurious linkage key; 1 = a vocabulary key is present in the
# block but absent from $keys and not exempt via $extras. The extras
# exemption: design-inventory carries a foreign-source `source:` (an
# extra, not a typed-linkage edge); without it the name-based walk would
# misclassify it and FAIL a template the plan leaves untouched.
# Assumption: a vocabulary name declared as an extra on a row is never
# ALSO a genuine typed-linkage slot on that same row (true here —
# design-inventory's `source:` is always a foreign-source id). If that
# changes, swap this for a value-shape check so a real typed edge cannot
# hide behind the extra.
check_closed_set() {
  local block="$1" extras="$2" keys="$3" vkey
  for vkey in "${LINKAGE_VOCABULARY[@]}"; do
    grep -qE "^${vkey}:[[:space:]]" <<< "$block" || continue
    case " $extras " in *" $vkey "*) continue ;; esac   # declared extra
    case " $keys "   in *" $vkey "*) continue ;; esac   # declared slot
    return 1                                            # spurious
  done
  return 0
}
```

   The live loop (inserted between the extras loop and the status-vocab
   block) wraps these — it is the ONLY place that mutates PASS/FAIL:

```bash
  for lkey in $typed_linkage_keys; do
    check_linkage_slot "$block" "$lkey"; rc=$?
    case "$rc" in
      0) echo "  PASS: $template_file: linkage slot '$lkey' shape+comment"; PASS=$((PASS + 1)) ;;
      2) echo "  FAIL: $template_file — unknown linkage key '$lkey'; add it to linkage_cardinality() (and LINKAGE_VOCABULARY) or correct the row"; FAIL=$((FAIL + 1)) ;;
      *) echo "  FAIL: $template_file: linkage slot '$lkey' bad shape/comment (or missing inverse-guidance line)"; FAIL=$((FAIL + 1)) ;;
    esac
  done

  if check_closed_set "$block" "$extras" "$typed_linkage_keys"; then
    echo "  PASS: $template_file: closed-set (no spurious linkage keys)"; PASS=$((PASS + 1))
  else
    echo "  FAIL: $template_file: closed-set violated (a linkage key not in the TSV row)"; FAIL=$((FAIL + 1))
  fi
```

   Note the inverse-guidance check folds into `check_linkage_slot` (rc 1
   when `blocked_by`'s standalone line is absent), so it no longer emits
   its own PASS line — inverse-guidance coverage is carried by the two
   `blocked_by` slots passing their shape+comment check (part of the 36),
   and the §2e "missing inverse line" fixture proves the rejection path.
   The closed-set check emits exactly one PASS per template (12), and the
   shape+comment check one PASS per slot (36, derived from the TSV),
   matching the success criteria.

e. **Negative-fixture self-test (proves each new assertion can fail).**
   Because this plan's deliverable *is* assertions, a green run against
   only-valid templates does not prove the assertions work — a regex
   that matches nothing, a loop that iterates zero times, or a
   miswired closed-set check all produce zero FAIL lines, which is
   indistinguishable from success. To close this, add a self-test that
   feeds known-bad frontmatter blocks through the same assertion logic
   and asserts each is **rejected**.

   The §2d `check_linkage_slot` and `check_closed_set` are already pure
   (rc-returning, no counter mutation) for exactly this reason — the
   self-test calls the **same** functions the live loop wraps, so the
   logic exercised is byte-for-byte the logic that runs live, with no
   double-counting. (The `blocked_by` inverse-guidance check is folded
   into `check_linkage_slot`, so the "missing inverse line" fixture
   below exercises it through that one function.)

   The self-test MUST live **in the same script, guarded so it runs on
   every invocation** (not a sibling file — a sibling would require
   amending `tasks/test/unit.py`'s driver list, an easy step to forget
   that would leave the backstop un-run by CI). It runs the functions
   over inline heredoc fixtures, one per failure mode, recording a PASS
   when the bad input is correctly rejected:

   - a list slot with a single-ref value (`blocks: ""`) — wrong cardinality;
   - a slot with a malformed comment (`parent: ""  # see ADR-0034`);
   - a `blocked_by` slot missing the inverse-guidance line;
   - a template carrying a linkage key absent from its TSV row
     (spurious slot) — `check_closed_set` must reject;
   - a slot present in the TSV row but absent from the frontmatter —
     `check_linkage_slot` must reject (no matching line);
   - a comment whose `<source-type>` token is outside the curated set
     (`parent: ""  # typed-linkage ref: "ticket:NNNN" or ""`).

   This is the mutation-test backstop: if any new assertion is broken
   or inert, at least one self-test fixture stops being rejected and the
   self-test FAILs. **Gate on the self-test PASS count** (exactly six,
   one per fixture above) so a self-test that silently stops exercising
   a fixture turns the suite red rather than merely dropping a PASS line.

   Add one further **structural** self-check (separate from the six
   fixtures): for every `LINKAGE_VOCABULARY` entry assert
   `linkage_cardinality` returns non-empty, emitting one PASS per entry,
   and **gate on the count (exactly 9)** so the guard itself cannot go
   inert. This stops the two hand-maintained lists drifting (a key added
   to one but not the other turns the suite red immediately rather than
   silently weakening the cardinality lookup or closed-set guard).

#### 3. Update the twelve templates

**Files** (all under `templates/`):

For each template, add the required typed-linkage slots from the
table above, using exactly the normative comment grammar. Existing
linkage slots (`parent` on `work-item.md`/`plan.md`; `supersedes` on
`adr.md`; `target` on the four reviewer/validation templates) are
rewritten to match the grammar — `key:`→`ref:`, `empty`→`""`; the
lifecycle annotations (`per ADR-0034`, `(filled by review-plan)`) move
to body prose of the relevant SKILL.md in Phase 4.

Templates keep all existing optional non-base fields present-but-empty
(they are the authoring surface). Rewrite their inline comments to
document the omit-when-empty rule per ADR-0040 (the template-shape test
does not assert these comments, so this is consistency polish, not a
gated assertion). The comment strings below are **illustrative** — the
exact wording is the author's discretion — but the rewrite MUST be
applied to **every** listed optional field so the omit cue is uniform
across templates (Phase 1 manual verification should checklist each
site, since nothing gates it).

The omit cue is deliberately *not* appended to the typed-linkage slot
comments themselves: their grammar is gated by the test regex (which
ends at `or ""` / `or []`), so an inline `; omitted when empty` would
break the assertion. Instead, **every template carries a single
full-line header comment immediately above its linkage block** to give
the omit rule an in-block signal without touching the gated grammar —
otherwise the linkage slots' `or ""` reads as "an empty string is a
value to keep", the opposite of the rule, while the adjacent
non-linkage optionals say "omitted when empty":

```
# typed-linkage slots — omit-when-empty in artifacts (drop any left empty); see ADR-0040
```

This header line is a standalone comment (matches no key regex, so it
trips no assertion). Note this is a **deliberate convention extension**:
no existing template carries a full-line comment *inside* the `---`
frontmatter block today (current frontmatter comments are all inline /
trailing on a key line). A full-line `#` comment is valid YAML, but
because it is new to the corpus, Phase 1 must confirm the frontmatter
consumers tolerate it — specifically that `scripts/test-metadata-helpers.sh`
and the visualiser's YAML frontmatter parser ignore standalone `#`
lines (they should; this is a verification, not an expected change).
The per-field omit cues on the non-linkage optionals are:

- `external_id: ""  # cross-system pointer …; omitted when not linked`
- `work_item_id: ""  # foreign reference; omitted when no linked work item`
- `reviewer: ""  # omitted until reviewed`
- `pr_url: ""  # omitted until populated from \`gh pr view\``
- `merge_commit: ""  # omitted until merged`
- `decision_makers: []  # omitted when empty`

Example final shape for `templates/work-item.md`:

```yaml
---
type: work-item
id: ""
title: ""
date: ""
author: ""
producer: ""
status: ""                                   # draft | ready | in-progress | review | done | blocked | abandoned
kind: ""
priority: ""
# typed-linkage slots — omit-when-empty in artifacts (drop any left empty); see ADR-0040
parent: ""                                   # typed-linkage ref: "work-item:NNNN" or ""
blocks: []                                   # typed-linkage list: ["work-item:NNNN", ...] or []
blocked_by: []                               # typed-linkage list: ["work-item:NNNN", ...] or []
# inverse of blocks — producers SHOULD prefer writing blocks: on the canonical side
derived_from: []                             # typed-linkage list: ["plan:NNNN", ...] or []
relates_to: []                               # typed-linkage list: ["work-item:NNNN", ...] or []
source: ""                                   # typed-linkage ref: "issue-research:NNNN" or ""
external_id: ""                              # cross-system pointer (e.g. Jira/Linear key); omitted when not linked
tags: []
last_updated: ""
last_updated_by: ""
schema_version: 1
---
```

Concrete per-template insertion order, comment `<source-type>`
choices, and existing-slot rewrites:

- **`work-item.md`**: insert after existing `parent:`; rewrite
  existing `parent:` comment. `source:` carries `issue-research:NNNN`.
  Immediately below the `blocked_by: []` slot, add the standalone
  inverse-guidance comment line (`# inverse of blocks — producers
  SHOULD prefer writing blocks: on the canonical side`).
- **`plan.md`**: insert after existing `parent:`. `derived_from:`
  carries `codebase-research:NNNN`. Add the same standalone
  inverse-guidance comment line below `blocked_by: []`.
- **`adr.md`**: add `parent: ""` and `relates_to: []`; rewrite the
  list-form comment on `supersedes:` only if needed (already canonical;
  verify against the new regex).
- **`codebase-research.md`**, **`rca.md`**, **`pr-description.md`**:
  add `parent: ""` and `relates_to: []`. For PR-description, `parent:`
  uses `work-item:NNNN`.
- **`design-inventory.md`**: add `parent: ""` and `relates_to: []`.
  Keep existing `source:` field (foreign-source, not typed-linkage).
- **`design-gap.md`**: add `parent: ""` and `relates_to: []`. Keep
  `current_inventory` / `target_inventory` lines verbatim — they are
  not linkage slots; the closed-set check ignores them (they are not
  in `LINKAGE_VOCABULARY`).
- **`validation.md`**, **`plan-review.md`**, **`work-item-review.md`**,
  **`pr-review.md`**: add `parent: ""` and `relates_to: []`; rewrite
  the existing `target:` comment to the normative single-ref form.
  `target:` carries `plan:NNNN` on validation+plan-review,
  `work-item:NNNN` on work-item-review, `pr:NNNN` on pr-review.

### Success Criteria

#### Automated Verification

- [ ] Field-count self-check passes: `bash scripts/test-template-frontmatter.sh` reports `PASS: templates-schema.tsv field-count self-check`
- [ ] **Exact PASS count, not just absence of FAIL.** The run reports exactly **36** `PASS: ... linkage slot ... shape+comment` lines (the sum of the per-template slot counts: 6+5+3+2+2+2+2+2+3+3+3+3) — assert with `[ "$(… | grep -c 'PASS:.*linkage slot.*shape+comment')" -eq 36 ]` — AND zero `FAIL:` lines. Gating on the exact count catches a silently-inert assertion (zero PASS + zero FAIL would otherwise read as success).
- [ ] Closed-set check exercised: exactly **12** closed-set PASS lines (one per template) and no `FAIL: ... closed-set ...` lines; in particular `design-inventory.md` passes (its extra `source:` is exempted, not flagged). Assert with `[ "$(… | grep -c 'PASS:.*closed-set')" -eq 12 ]`.
- [ ] Inverse-guidance coverage: `work-item.md`'s and `plan.md`'s `blocked_by` slots pass their `check_linkage_slot` shape+comment check (the inverse-line presence is folded into that check — a missing standalone line makes the slot FAIL). These two are part of the 36 above; the §2e "missing inverse line" fixture proves the rejection path works.
- [ ] **Negative-fixture self-test passes**: exactly **6** self-test PASS lines (one per §2e fixture), each proving a specific assertion rejects bad input; the suite is red if any bad fixture stops being rejected
- [ ] **Vocabulary-drift guard passes**: exactly **9** PASS lines (one per `LINKAGE_VOCABULARY` entry, each returning a non-empty `linkage_cardinality`); assert with `-eq 9`
- [ ] **Counts are derived, not hard-coded.** Compute the expected slot total from the TSV at runtime — `expected=$(awk -F'\t' 'NR>1{n=split($7,a," ");t+=n}END{print t}' scripts/templates-schema.tsv)` — and assert the shape+comment PASS count equals `$expected`, so adding a template/slot does not require editing a magic number (36 is the value today).
- [ ] Runs on bash 3.2: `bash --version` 3.2 (or `/bin/bash` on stock macOS) executes the script without `declare: -A: invalid option`
- [ ] Whole test suite: `mise run test:unit:templates` exits 0
- [ ] CI green: `mise run test` exits 0

#### Manual Verification

- [ ] Visual diff of each of the twelve templates shows only the new linkage slots, the standalone block-header comment, and the rewritten comments — no incidental edits
- [ ] Standalone full-line `#` comment inside the frontmatter block is tolerated: `mise run test:unit:templates` stays green AND the visualiser frontmatter parser still resolves a template carrying it (the standalone comment is a new in-`---` convention — confirm `test-metadata-helpers.sh` and the YAML parser ignore it)
- [ ] `design-gap.md`'s `current_inventory` and `target_inventory` lines are byte-identical to before
- [ ] `design-inventory.md`'s existing `source:` line is byte-identical to before

---

## Phase 2: Extend skill test infrastructure for omit-when-empty guidance

### Overview

Extend `scripts/skills-schema.tsv` with an `omit_when_empty` column and
`scripts/test-skill-frontmatter-population.sh` with a new assertion
mode that requires each named field to (a) appear in a Populate-
frontmatter section AND (b) be accompanied by a one-line guidance
note containing "fill" or "omit". The column is added empty
on every row in this phase — no SKILL.md changes are made yet. The
test extension is exercised only against empty columns, so it passes
as a no-op until Phases 4–6 populate the rows.

This phase exists as a separate step so the infra/contract change
lands atomically and is reviewable on its own, separate from the
per-skill content sweeps.

### Changes Required

#### 1. `scripts/skills-schema.tsv` — add omit_when_empty column

**File**: `scripts/skills-schema.tsv`

**Changes**: Add `omit_when_empty` as a fourth tab-separated column.
All fourteen data rows get a placeholder fourth column in this phase;
Phases 4–6 populate them. Use a `-` sentinel (mirroring the existing
`forbidden_own_id_key` convention) rather than a bare empty field, so
the column survives editors/formatters that strip trailing tabs — a
bare trailing-tab empty field would otherwise drop `NF` to 3 and trip
the self-check. The omit-when-empty loop skips `-` (see §2d).

```tsv
skill_path	producer_name	fields_to_assert	omit_when_empty
skills/work/create-work-item/SKILL.md	create-work-item	producer schema_version last_updated last_updated_by	-
... (13 more rows, all with `-` in the fourth column)
```

#### 2. `scripts/test-skill-frontmatter-population.sh` — new assertion mode

**File**: `scripts/test-skill-frontmatter-population.sh`

**Changes**:

a. Update field-count self-check from `NF != 3` to `NF != 4`
   (line 57). Update PASS/FAIL messages accordingly. Also add
   `export LC_ALL=C` at the top of the script (parity with
   `test-template-frontmatter.sh`), so the new awk helper's `tolower()`
   and `[[:alpha:]]`/`(fill|omit)` matching behave deterministically
   regardless of the host locale.

b. Extend the `while read` loop signature:

```bash
while IFS=$'\t' read -r skill_path producer_name fields omit_when_empty; do
```

c. Add a new helper after `in_imperative_section`. Two corrections
   over a naive version: (1) the field-match and guidance keyword use
   POSIX classes (`[[:space:]]`, whole-word `(fill|omit)`) so they
   behave identically under BSD awk (macOS) and gawk (CI) — a bare
   `[ \t]` class and a bare `fill|omit` substring would diverge across
   awks and false-match inside words like `backfill`/`fulfil`; (2) the
   guidance keyword is bound to the **field's own bullet window** (from
   the bullet line naming the field until the next bullet), not to the
   section as a whole — otherwise one field's `fill`/`omit` note would
   satisfy the check for every field in the section, and AC #3's
   per-field guarantee would not actually be verified. Returns 0 only
   when the named field's bullet carries its own fill/omit guidance:

```bash
# Heading predicate shared with in_imperative_section. Defined ONCE as a
# shell variable and passed to both awk helpers via -v, so the two
# detectors locate the same sections and a future vocabulary change is a
# one-line edit. (in_imperative_section is migrated to consume this in
# step (f) below — both helpers read $POPULATE_HEADING_RE, no inline copy.)
POPULATE_HEADING_RE='persistence|metadata|frontmatter|populate|capture metadata|step [0-9]'

in_populate_section_with_guidance() {
  local file="$1" field="$2"
  awk -v field="$field" -v headingre="$POPULATE_HEADING_RE" '
    BEGIN { fieldpat = "(^|[[:space:]]|`|\\*)" field ":" }
    # Commit a satisfied attribution window (the field bullet carried its
    # own fill/omit guidance) and reset it. Called wherever a window
    # closes — next bullet, heading boundary, AND EOF — mirroring
    # in_imperative_section so a window closed by a heading is not lost.
    function flush() {
      if (in_section && tracking && saw) found = 1
      tracking = 0; saw = 0
    }
    /^#/ {
      flush()
      heading = tolower($0)
      # Broad predicate (shared with in_imperative_section) — it LOCATES
      # the section for per-field guidance attribution; it does NOT
      # enforce the literal "Populate frontmatter" heading. AC #3's
      # literal-heading requirement for the four reviewer skills is
      # enforced separately (see Phase 4 §2a).
      in_section = (heading ~ headingre)
      next
    }
    in_section {
      if ($0 ~ /^[[:space:]]*[-*]/) flush()   # a new bullet closes the prior window
      # Arm only ONCE per window (!tracking guard): a continuation line
      # that re-mentions the field key must not reset saw and drop an
      # already-satisfied window.
      if (!tracking && $0 ~ fieldpat) { tracking = 1; saw = 0 }
      if (tracking && $0 ~ /(^|[^[:alpha:]])(fill|omit)([^[:alpha:]]|$)/) saw = 1
    }
    END { flush(); exit (found ? 0 : 1) }
  ' "$file"
}
```

   The `flush()` call in the `/^#/` branch is load-bearing: without it, a
   field whose fill/omit bullet is the **last bullet before the next
   heading** would have its satisfied window silently discarded by the
   heading reset, producing a spurious FAIL. `in_imperative_section`
   already flushes on heading transitions; this helper now matches that
   behaviour and shares its heading predicate via `$POPULATE_HEADING_RE`
   (the two helpers differ only in their per-line predicate — guidance
   bullet-window vs imperative-verb — not in their section skeleton).

d. Add the omit-when-empty loop after the existing `for field in $fields`
   loop (around line 148):

```bash
  for fld in $omit_when_empty; do
    [ "$fld" = "-" ] && continue   # `-` sentinel = no omit-when-empty fields
    if in_populate_section_with_guidance "$stripped" "$fld"; then
      echo "  PASS: $skill_path: instructs population of omit-when-empty field '$fld' with fill/omit guidance"
      PASS=$((PASS + 1))
    else
      echo "  FAIL: $skill_path: omit-when-empty field '$fld' missing or lacks fill/omit guidance in Populate frontmatter section"
      FAIL=$((FAIL + 1))
    fi
  done
```

e. **Liveness self-test for `in_populate_section_with_guidance`
   (proves the helper can FAIL before any row exercises it).** Because
   every `omit_when_empty` column is `-` in this phase, the loop in (d)
   iterates zero times — so without a self-test the new helper ships
   across Phases 2–3 never having been observed to reject bad input.
   Add a small self-test **in the same script, guarded so it runs on
   every invocation** (not a sibling file — that would require amending
   `tasks/test/unit.py`'s driver list, which is easy to forget and would
   leave the backstop un-run by CI). It runs the helper against inline
   fixtures and asserts the expected verdict:

   - a section that names a field **with** a fill/omit bullet → expect 0 (PASS);
   - the same field named **without** any fill/omit note → expect 1 (rejected);
   - a field whose name appears but whose fill/omit note is on a
     *different* field's bullet → expect 1 (proves per-field binding);
   - a `fill`/`omit` substring buried in a word (`backfill`) only →
     expect 1 (proves whole-word matching);
   - a field with proper guidance under a **bold lead-in**
     (`**Populate frontmatter**:`, no `#`) → expect 1 (proves the `^#`
     heading requirement is enforced — the regression Phase 4 §2a
     guards against).

   The self-test makes the Phase 2 change verifiable on its own rather
   than relying on Phase 4+ to first exercise the path. **Gate on the
   self-test PASS count** (exactly five, one per fixture).

f. **Migrate `in_imperative_section` to the shared predicate.** The
   existing `in_imperative_section` helper hardcodes the heading regex
   inline (currently line 105). Change it to take `-v headingre="$POPULATE_HEADING_RE"`
   and use `heading ~ headingre`, so the heading vocabulary lives in
   exactly one place and the two detectors cannot drift. This is a
   small, same-file edit and is required (not optional) — without it the
   `POPULATE_HEADING_RE` "single source of truth" is only half-true.

### Success Criteria

#### Automated Verification

- [ ] Field-count self-check passes: `bash scripts/test-skill-frontmatter-population.sh` reports `PASS: skills-schema.tsv field-count self-check`
- [ ] No-op when all `omit_when_empty` columns empty: every PASS line from before the change still appears; no new FAIL lines introduced
- [ ] **Helper liveness self-test passes** (§2e): exactly **5** self-test PASS lines — the without-guidance, cross-field-guidance, buried-substring, and bold-lead-in fixtures are all rejected (helper returns 1); the with-guidance fixture passes (returns 0). Self-test runs in-script under `mise run test:unit:templates` (not a hand-run sibling)
- [ ] `export LC_ALL=C` is set at the top of `test-skill-frontmatter-population.sh` (parity with the template script), pinning `tolower()`/`[[:alpha:]]` behaviour regardless of host locale
- [ ] Whole test suite: `mise run test:unit:templates` exits 0

#### Manual Verification

- [ ] Diff of the TSV shows only the new fourth column on every row (use a `-` sentinel rather than a bare trailing tab if the repo's tooling strips trailing whitespace, mirroring the `forbidden_own_id_key` convention)
- [ ] Diff of the script shows the new helper, the new loop, the loop-signature change, and the liveness self-test — no incidental edits

---

## Phase 3: Writer-canon SKILL.md sweep (eight Group A skills)

### Overview

Update the eight writer-canon SKILL.md files
(`create-work-item`, `extract-work-items`, `create-plan`, `create-adr`,
`extract-adrs`, `research-codebase`, `research-issue`, `describe-pr`)
to name their omit-when-empty fields — linkage keys **plus** the
applicable foreign-ref / optional-extra / lifecycle fields — in their
canonical Populate-frontmatter snippet, each with a one-line "fill" or
"omit" guidance bullet. Populate the corresponding `omit_when_empty`
columns in `scripts/skills-schema.tsv`. The shape rule from the 0065
plan's canonical persistence-step snippet
(`meta/plans/2026-05-30-0065-update-artifact-templates-to-unified-schema.md:198-236`)
is preserved verbatim — only the substitution-list grows.

Value-shape normalisation of `create-plan`'s bare-id `work_item_id:` is
deferred to Phase 6 (this phase only adds its fill/omit guidance bullet).
Note that the `parent:` bullets added here for `create-work-item` and
`extract-work-items` specify the typed `"work-item:NNNN"` form, which is
itself a bare-id→typed-form value-shape change for those producers (they
emit bare ids today). This is deliberate and is part of the single,
uniform normalisation tracked in Phase 6's narrative — it is called out
here so the change is not silent.

### Changes Required

#### 1. `scripts/skills-schema.tsv` — populate writer rows

**File**: `scripts/skills-schema.tsv`

**Changes**: Populate the fourth column for the eight writer-canon
rows. Each row's `omit_when_empty` set = the template's linkage keys
(work item §2) **plus** that artifact's optional foreign-ref /
extra / lifecycle fields per the *Emission classification*:

```tsv
skills/work/create-work-item/SKILL.md	create-work-item	producer schema_version last_updated last_updated_by	parent blocks blocked_by derived_from relates_to source external_id
skills/work/extract-work-items/SKILL.md	extract-work-items	producer schema_version last_updated last_updated_by	parent blocks blocked_by derived_from relates_to source external_id
skills/planning/create-plan/SKILL.md	create-plan	producer schema_version last_updated last_updated_by revision repository	parent blocks blocked_by derived_from relates_to work_item_id reviewer
skills/github/describe-pr/SKILL.md	describe-pr	producer schema_version last_updated last_updated_by revision repository	parent relates_to work_item_id pr_url merge_commit
skills/decisions/create-adr/SKILL.md	create-adr	producer schema_version last_updated last_updated_by	parent supersedes relates_to decision_makers
skills/decisions/extract-adrs/SKILL.md	extract-adrs	producer schema_version last_updated last_updated_by	parent supersedes relates_to decision_makers
skills/research/research-codebase/SKILL.md	research-codebase	producer schema_version last_updated last_updated_by revision repository	parent relates_to work_item_id
skills/research/research-issue/SKILL.md	research-issue	producer schema_version last_updated last_updated_by revision repository	parent relates_to work_item_id
```

#### 2. Update the eight writer SKILL.md files

**Files**:

- `skills/work/create-work-item/SKILL.md` (extend snippet at lines 436-459)
- `skills/work/extract-work-items/SKILL.md`
- `skills/planning/create-plan/SKILL.md` (extend snippet at lines 225-250)
- `skills/decisions/create-adr/SKILL.md` (extend snippet at lines 148-173)
- `skills/decisions/extract-adrs/SKILL.md`
- `skills/research/research-codebase/SKILL.md`
- `skills/research/research-issue/SKILL.md`
- `skills/github/describe-pr/SKILL.md`

**Changes**: In each Populate-frontmatter section, lead the
omit-when-empty group with one **omit-by-default** sentence so the
common case (a fresh draft with no explicit cross-edges — nearly all
these keys absent) is stated once rather than inferred from a stack of
per-bullet negatives, then add one bullet per field following the
canonical shape. The lead sentence also reconciles the two authoring
surfaces — the template shows the slot as `""`/`[]`, but the producer
**omits the key entirely** in the written artifact rather than copying
the empty placeholder:

```markdown
Optional linkage/foreign-ref keys are omit-by-default: the template
shows each as `""`/`[]`, but write a key into the artifact **only**
when it has a value, and omit it entirely otherwise (do not carry the
empty placeholder through). By default a new draft names none of them.
```

Then the per-field bullets. Each bullet keeps a short `Fill when …;
otherwise omit` cue even with the lead sentence present — this is
deliberate, not redundant: the Phase 2 skill-test asserts a whole-word
`fill`/`omit` keyword **within each field's own bullet window**, so
every bullet MUST retain at least one of those keywords or the contract
test FAILs. The lead sentence states the default; the per-bullet cue is
what the test gates on. (Do not strip the cue from a bullet.)

```markdown
- `parent:` ← the parent work item's ID as a typed-linkage ref
  (`"work-item:NNNN"`). Fill when the user names a parent at draft
  time; otherwise omit the key entirely.
- `blocks:` ← list of typed-linkage refs to work items this work item
  blocks (`["work-item:NNNN", ...]`). Fill when blocking edges are
  explicit at draft time; otherwise omit the key.
- `blocked_by:` ← list of typed-linkage refs to work items that block
  this one. Prefer writing the canonical `blocks:` on the other side;
  emit `blocked_by:` only when the canonical side cannot be written,
  and omit it otherwise.
- `derived_from:` ← list of typed-linkage refs to artifacts this work
  item is derived from (`["plan:NNNN", ...]`). Fill when derivation is
  explicit; otherwise omit the key.
- `relates_to:` ← list of typed-linkage refs to related artifacts.
  Fill when relationships are explicit; otherwise omit the key.
- `source:` ← typed-linkage ref to the originating source artifact
  (`"issue-research:NNNN"` or similar). Fill when the source is
  explicit; otherwise omit the key.
- `external_id:` ← cross-system pointer (e.g. a Jira/Linear key). Fill
  when the work item is linked to an external tracker; otherwise omit
  the key.
```

The exact `<source-type>` token per slot must match what the template's
inline comment uses (and what the test's curated set permits). Per-skill
variants of the omit-when-empty bullet set:

- `create-work-item` / `extract-work-items`: the seven bullets above
  (linkage + `external_id`).
- `create-plan`: `parent`, `blocks`, `blocked_by`, `derived_from`
  (uses `codebase-research:NNNN`), `relates_to`, plus `work_item_id`
  (fill in `"work-item:NNNN"` form when linked, else omit — value-shape
  normalised in Phase 6) and `reviewer` (omit until reviewed). No
  `source` slot.
- `create-adr` / `extract-adrs`: `parent`, `supersedes`
  (`adr:ADR-NNNN`), `relates_to`, plus `decision_makers` (omit when
  empty).
- `research-codebase` / `research-issue`: `parent`, `relates_to`, plus
  `work_item_id` (omit when no linked work item).
- `describe-pr`: `parent` (`work-item:NNNN`), `relates_to`, plus
  `work_item_id`, `pr_url`, `merge_commit` (each omit until populated).

Lifecycle annotations that previously lived in reviewer/validation
template comments do not apply to writer skills; they are handled in
Phase 4.

### Success Criteria

#### Automated Verification

- [ ] `bash scripts/test-skill-frontmatter-population.sh` reports PASS for every omit-when-empty field on each of the eight writer skills
- [ ] No FAIL lines reference any of the eight writer SKILL.md files
- [ ] `mise run test:unit:templates` exits 0

#### Manual Verification

- [ ] Each of the eight SKILL.md files retains the canonical
  Populate-frontmatter snippet shape (substitution list with `←`
  arrows, nested `1. Invoke / 2. Substitute / 3. Write` numbering
  preserved where present)
- [ ] The "fill"/"omit" guidance is one short line per bullet,
  not multi-paragraph prose

---

## Phase 4: Reviewer-canon heading lift (four Group B skills)

### Overview

Lift the four reviewer skills (`validate-plan`, `review-plan`,
`review-work-item`, `review-pr`) into Group B canon by giving each a
literal `Populate frontmatter` heading. Add the new linkage slots
(`parent`, `relates_to`) plus the existing `target:` slot to each
Populate-frontmatter section with fill/omit guidance, and populate the
corresponding `omit_when_empty` columns in `scripts/skills-schema.tsv`.
Migrate the lifecycle annotations that previously lived in the template
comments (`(filled by review-plan)`, etc.) into the body prose of each
reviewer SKILL.md.

`reviewer`, `verdict`, `lenses`, `review_number`, `review_pass` on
reviews are always filled by the producer, so they are *not*
omit-when-empty and stay in `fields_to_assert` only.

### Changes Required

#### 1. `scripts/skills-schema.tsv` — populate reviewer rows

**File**: `scripts/skills-schema.tsv`

```tsv
skills/planning/review-plan/SKILL.md	review-plan	producer schema_version last_updated last_updated_by target reviewer verdict lenses review_number review_pass	parent target relates_to
skills/work/review-work-item/SKILL.md	review-work-item	producer schema_version last_updated last_updated_by target reviewer verdict lenses review_number review_pass work_item_id	parent target relates_to
skills/github/review-pr/SKILL.md	review-pr	producer schema_version last_updated last_updated_by target reviewer verdict lenses review_number pr_number	parent target relates_to
skills/planning/validate-plan/SKILL.md	validate-plan	producer schema_version last_updated last_updated_by target result	parent target relates_to
```

#### 2. Update the four reviewer SKILL.md files

**Files**:

- `skills/planning/validate-plan/SKILL.md`
- `skills/planning/review-plan/SKILL.md` (current population step at lines 420-449)
- `skills/work/review-work-item/SKILL.md`
- `skills/github/review-pr/SKILL.md` (current population step at lines 458-494)

**Changes**:

a. Add the heading `### Populate frontmatter` (a real `#`-prefixed
   heading) at the start of each skill's existing prose-folded
   population step — for **all four** reviewer skills, one form, no
   alternative. Two distinct reasons, kept separate to avoid the
   conflation the pass-1 wording introduced:

   - *Detection*: the `in_populate_section_with_guidance` awk detector
     keys section boundaries on `^#` heading lines, so a bold lead-in
     (`**Populate frontmatter**:`) would never register as a section
     and the guidance assertion could never run. A `#`-prefixed heading
     is therefore mandatory for the guidance check to attribute fields
     at all. (The detector's predicate is broad — `step [0-9]|populate|
     …` — so it merely *locates* a section; it does not by itself
     require the literal phrase.)
   - *AC #3 literal heading*: because the broad predicate would also
     accept the reviewers' pre-existing `### Step N:` headings, the
     literal-`Populate frontmatter`-heading requirement must be enforced
     by its **own assertion in the contract test**, not left to the awk
     predicate or a one-off success-criterion grep. Add to
     `test-skill-frontmatter-population.sh` a check that fires inside the
     existing per-row `while IFS=$'\t' read` loop, keyed on the row being
     a **reviewer producer** — discriminated from the TSV, not a
     hardcoded path list (the natural discriminant is the row declaring
     `target` in its `fields_to_assert`, which exactly the four reviewer
     rows do; `producer_name ∈ {validate-plan, review-plan,
     review-work-item, review-pr}` is the equivalent set). For each such
     row, assert the file contains a heading matching
     `^#+[[:space:]]+Populate frontmatter[[:space:]]*$`, emitting one
     PASS per reviewer. **Gate on the count (exactly 4)** so the
     assertion cannot go inert. The Phase 4 verification grep below
     mirrors this.

   Heading-*style* note: reviewers use a bare `### Populate frontmatter`
   peer heading (their population step is being lifted out of a
   prose-folded sub-step into its own section), which is a deliberate
   departure from the Group A `### Step N: …` numbered style. This is
   intentional — do not read the shared awk predicate as implying the
   two heading *styles* are the same; it only means both *match the
   detector*.

b. Under the new heading, list the three linkage slots
   (`parent`, `target`, `relates_to`) plus any existing field
   bullets that were already present, each with one-line guidance:

```markdown
### Populate frontmatter

- `parent:` ← typed-linkage ref to the parent artifact (`"plan:NNNN"`
  for review-plan, `"work-item:NNNN"` for review-work-item, etc.). Fill
  when the user names a parent; otherwise omit the key.
- `target:` ← typed-linkage ref to the artifact under review
  (`"plan:<plan-id>"` for review-plan; filled automatically from the
  skill argument). Always fill — every review has a target.
- `relates_to:` ← list of typed-linkage refs to related artifacts.
  Fill when prior reviews or related artifacts are explicit; otherwise
  omit the key.
```

c. Body prose absorbs the lifecycle annotations that previously
   lived in template comments. Example for `review-plan`:

```markdown
The `target:` field is filled automatically from the `$ARGUMENTS`
plan reference — this is what makes the review traceable back to the
plan it covers. Per ADR-0034, the typed-linkage form is `"plan:<plan-id>"`.
```

The `<source-type>` token per reviewer:

- `validate-plan`: `target:` carries `plan:NNNN`
- `review-plan`: `target:` carries `plan:NNNN`
- `review-work-item`: `target:` carries `work-item:NNNN`
- `review-pr`: `target:` carries `pr:NNNN`

### Success Criteria

#### Automated Verification

- [ ] `bash scripts/test-skill-frontmatter-population.sh` reports PASS for `parent`, `target`, `relates_to` on each of the four reviewer skills
- [ ] The contract test reports exactly **4** literal-heading PASS lines — one per reviewer producer row (the new `^#+ Populate frontmatter$` assertion, keyed on rows declaring `target`) — assert with `-eq 4`. AC #3 is gated by the test, not just by the grep below
- [ ] No FAIL lines reference any of the four reviewer SKILL.md files
- [ ] `grep -l '^### Populate frontmatter' skills/planning/validate-plan/SKILL.md skills/planning/review-plan/SKILL.md skills/work/review-work-item/SKILL.md skills/github/review-pr/SKILL.md` lists all four

#### Manual Verification

- [ ] Each reviewer SKILL.md reads cleanly with the new heading — the population step is no longer folded into prose
- [ ] The lifecycle-annotation prose (previously in template comments) appears in each reviewer SKILL.md and reads naturally — verify **per skill** (validate-plan, review-plan, review-work-item, review-pr) that the specific annotation its template comment carried now lives in that skill's body, so none is dropped in the template→SKILL.md move

---

## Phase 5: Design carve-out SKILL.md sweep (two Group C skills)

### Overview

Update `inventory-design` and `analyse-design-gaps` Populate-frontmatter
sections to name the new `parent` and `relates_to` slots with
fill/omit guidance. The design-gap-specific
`current_inventory` / `target_inventory` lines in
`skills/design/analyse-design-gaps/SKILL.md:165-168` are preserved
unchanged — they are always-valued, not omit-when-empty, and the linkage
test ignores them. Populate the corresponding `omit_when_empty` columns
in `scripts/skills-schema.tsv`.

### Changes Required

#### 1. `scripts/skills-schema.tsv` — populate design rows

**File**: `scripts/skills-schema.tsv`

```tsv
skills/design/inventory-design/SKILL.md	inventory-design	producer schema_version last_updated last_updated_by revision repository	parent relates_to
skills/design/analyse-design-gaps/SKILL.md	analyse-design-gaps	producer schema_version last_updated last_updated_by	parent relates_to
```

#### 2. Update the two design SKILL.md files

**Files**:

- `skills/design/inventory-design/SKILL.md`
- `skills/design/analyse-design-gaps/SKILL.md` (current population step at lines 145-171)

**Changes**: In each Populate-frontmatter section, add the two
linkage-slot bullets following the same canonical shape used in
Phase 3 (omit-when-empty — fill when explicit, otherwise omit the
key). For `analyse-design-gaps`, the existing
`current_inventory` / `target_inventory` bullets (which emit
filesystem paths per the ADR-0033 / ADR-0034 design-gap carve-out)
are left exactly as they are; the new linkage-slot bullets appear
alongside them without disturbing the inventory-key bullets.

`<source-type>` token: `parent:` carries `work-item:NNNN` on both
design skills (designs derive from work-items per ADR-0034's
type-pair table).

### Success Criteria

#### Automated Verification

- [ ] `bash scripts/test-skill-frontmatter-population.sh` reports PASS for `parent` and `relates_to` on `inventory-design` and `analyse-design-gaps`
- [ ] The `current_inventory` and `target_inventory` bullets in `analyse-design-gaps/SKILL.md` are byte-identical to before (verify via `jj diff`)

#### Manual Verification

- [ ] The new linkage-slot bullets sit naturally alongside the existing inventory-key bullets without confusing the reader about which set is which

---

## Phase 6: refine-work-item lift and bare-id producer normalisation

### Overview

Promote `refine-work-item` from the `NON_EMITTER_TEMPLATE_CONSUMERS`
allowlist to `IN_SCOPE_PRODUCERS`, add a proper Populate-frontmatter
snippet for the decompose path that emits new child work items, and
normalise the bare-id `parent` write to typed-linkage form
(`parent: "work-item:NNNN"`). Apply the same bare-id→typed-linkage
normalisation to `create-plan`'s `work_item_id:` emit (and make it
omit-when-empty per ADR-0040). Both changes are runtime-compatible per
the visualiser parser audit (`cluster_key.rs:138-152` accepts both
shapes and treats empty/absent identically).

### Changes Required

#### 1. `scripts/test-skill-frontmatter-population.sh` — move refine-work-item

**File**: `scripts/test-skill-frontmatter-population.sh`

**Changes**:

a. Move the `skills/work/refine-work-item/SKILL.md` line from
   `NON_EMITTER_TEMPLATE_CONSUMERS` (line 49) to `IN_SCOPE_PRODUCERS`
   (insert near line 33 in alphabetical order within `skills/work/`).

#### 2. `scripts/skills-schema.tsv` — add refine-work-item row

**File**: `scripts/skills-schema.tsv`

**Changes**: Add a fifteenth data row for refine-work-item:

```tsv
skills/work/refine-work-item/SKILL.md	refine-work-item	producer schema_version last_updated last_updated_by	parent blocks blocked_by derived_from relates_to source external_id
```

(The `fields_to_assert` column matches what create-work-item asserts;
the `omit_when_empty` set matches create-work-item, since both write
child work-items from the work-item template.)

#### 3. `skills/work/refine-work-item/SKILL.md` — add Populate-frontmatter snippet

**File**: `skills/work/refine-work-item/SKILL.md`
(current decompose op at lines 180-205)

**Changes**:

a. Within the decompose op's child-work-item creation steps, add an
   explicit `#`-prefixed `Populate frontmatter` heading (matching the
   op's subsection depth, e.g. `#### Populate frontmatter`) that
   follows the canonical Group A snippet shape (substitution list with
   `←` arrows, not the current em-dash form). A `#`-prefixed heading
   (not a bold lead-in) is required so the `in_populate_section_with_guidance`
   awk detector registers the section. Convert the **entire**
   decompose substitution list — the pre-existing base-field bullets
   as well as the new linkage bullets — to the `←` form, so the op
   does not end up with two substitution styles (em-dash base fields,
   arrow linkage) in one place.

b. Change the `parent:` emit from bare-id (`"0001"`) to typed-linkage
   form (`"work-item:0001"`). The bullet wording becomes:

```markdown
- `parent:` ← the parent work item's ID as a typed-linkage ref
  (`"work-item:NNNN"`, where NNNN is the parent's `id` field). Always
  fill — every decomposed child has a parent.
```

c. Add the remaining omit-when-empty bullets (`blocks`, `blocked_by`,
   `derived_from`, `relates_to`, `source`, `external_id`) matching
   Phase 3's create-work-item shape, since decompose creates fresh
   children that rarely have explicit cross-edges at decompose time
   (so those keys are usually omitted).

#### 4. `skills/planning/create-plan/SKILL.md` — normalise work_item_id emit

**File**: `skills/planning/create-plan/SKILL.md`
(current `work_item_id:` emit at lines 243-244)

**Changes**: Change `work_item_id:` from the bare `"NNNN"` form to the
`"work-item:NNNN"` form, and make it omit-when-empty per ADR-0040. The
Phase 3 fill/omit guidance bullet for `work_item_id` is updated to the
typed form here; the rest of the snippet is untouched.

Wording becomes:

```markdown
- `work_item_id:` ← the linked work item as a typed-linkage ref
  (`"work-item:NNNN"`). Fill in this form when invoked with a work item
  argument; otherwise omit the key entirely.
```

#### 5. Visualiser-parser sanity check

**Files** (read-only audit; no edits):

- `skills/visualisation/visualise/server/src/typed_ref.rs`
- `skills/visualisation/visualise/server/src/cluster_key.rs:138-152`
- `skills/visualisation/visualise/server/src/frontmatter.rs:305-368`

**Changes**: None. The audit confirms `cluster_key.rs:138-152`
accepts both shapes; `frontmatter.rs`'s `read_ref_keys` strips
the `work-item:` prefix and treats absent keys as absent edges;
existing artifacts with bare-id or omitted `parent:` / `work_item_id:`
continue to resolve.

Add — unconditionally — a both-shapes **equivalence** regression test
in `skills/visualisation/visualise/server/src/cluster_key.rs`'s test
module: `parent_typed_form_resolves_same_as_bare_id`, asserting that for
the same id N, `resolve_cluster_key` on `parent: "work-item:N"` returns
the same canonical id as `parent: "N"`. The existing tests cover the
typed form and the bare form *separately* (and lines 500-513 cover
empty-typed-parent), but none asserts the two are *equivalent* — which
is the exact property Phase 6's producer change relies on. Adding the
equivalence assertion outright (rather than "verify or add") guarantees
a future divergence is caught.

### Success Criteria

#### Automated Verification

- [ ] `bash scripts/test-skill-frontmatter-population.sh` reports PASS for every omit-when-empty field on `refine-work-item`
- [ ] The discovery-pass assertion (lines 158-191) still passes — refine-work-item appears in IN_SCOPE_PRODUCERS, not in NON_EMITTER_TEMPLATE_CONSUMERS
- [ ] `mise run test:unit:templates` exits 0
- [ ] Visualiser tests pass: `mise run test:unit:visualiser` exits 0
- [ ] The equivalence regression test `parent_typed_form_resolves_same_as_bare_id` exists and passes (asserts `resolve_cluster_key` returns the same id for `"work-item:N"` and `"N"`)
- [ ] Whole test suite: `mise run test` exits 0
- [ ] `grep -nE '^[[:space:]]*-?[[:space:]]*\`parent:\`[[:space:]]+←[[:space:]]+.*work-item:' skills/work/refine-work-item/SKILL.md` returns a match (typed-linkage form confirmed)
- [ ] `grep -nE '^[[:space:]]*-?[[:space:]]*\`work_item_id:\`[[:space:]]+←[[:space:]]+.*work-item:' skills/planning/create-plan/SKILL.md` returns a match

#### Manual Verification

- [ ] Refine-work-item's decompose op reads naturally with the new
  Populate-frontmatter section — the em-dash → arrow switch and the
  bare-id → typed-form switch are coherent in context
- [ ] Visualiser smoke test: start the visualiser server, open a
  decomposed work item (parent + child), confirm the parent-child
  relationship still renders in the cluster view

---

## Testing Strategy

### Unit Tests

- **`scripts/test-template-frontmatter.sh`** (Phase 1): one
  shape+comment PASS per slot — **36** today, but the success criterion
  derives the expected total from the TSV's `typed_linkage_keys` column
  at runtime rather than hard-coding it, so adding a template/slot does
  not require editing a magic number. The closed-set check contributes
  exactly **12** (one per template). The `blocked_by` inverse-guidance
  line is verified inside `check_linkage_slot` (folded into the two
  `blocked_by` slot checks, not a separate PASS). The negative-fixture
  self-test (§2e) adds exactly **6** PASS lines (one per failure-mode
  fixture) and the vocabulary-drift structural guard adds exactly **9**
  (one per `LINKAGE_VOCABULARY` entry). The success criteria gate on
  these **exact counts**, not merely the absence of FAIL lines, so an
  inert assertion (zero PASS + zero FAIL) is caught.
- **`scripts/test-skill-frontmatter-population.sh`** (Phase 2 onward):
  each populated row contributes one PASS per omit-when-empty field.
  After Phase 6, fifteen rows × variable field counts. Gate the
  omit-when-empty PASS total on a count **derived from the TSV** —
  `expected=$(awk -F'\t' 'NR>1{n=split($4,a," ");for(i in a)if(a[i]!="-")t++}END{print t}' scripts/skills-schema.tsv)` —
  so a silently-dropped `omit_when_empty` entry (which simply would not
  be iterated, producing no FAIL) is caught by the count mismatch,
  mirroring the template side. Plus the 4 literal-heading PASS lines
  (reviewer rows) and the 5 liveness self-test PASS lines, each
  separately count-gated.

### Integration Tests

- **`mise run test:unit:templates`** runs all three schema test scripts
  (`test-template-frontmatter.sh`, `test-skill-frontmatter-population.sh`,
  `test-metadata-helpers.sh`). The third is unaffected.
- **`mise run test`** runs the full suite; the visualiser tests run
  to confirm Phase 6's runtime-compatibility assumption.

### Manual Testing Steps

1. After Phase 0, confirm ADR-0040 is accepted and its scope table
   matches this plan's *Emission classification*.
2. After each implementing phase, run `mise run test:unit:templates`
   and confirm no FAIL lines for the phase's scope.
3. After Phase 6, start the visualiser locally
   (`mise run visualise:start` or equivalent) and inspect a
   work-item cluster page that includes a parent + child relationship
   created via refine-work-item — confirm both render and the parent
   edge is correctly drawn.
4. After Phase 6, create a new child work item via `refine-work-item`
   decompose and a new plan via `create-plan` with no linked work item;
   inspect the resulting frontmatter — confirm `parent:` is written in
   `"work-item:NNNN"` form and that empty optional keys (e.g.
   `work_item_id`, `external_id`) are omitted, not present-but-empty.

## Performance Considerations

The template-shape test currently runs in well under a second; adding
linkage assertions (~50 additional `grep -E` invocations across twelve
templates) keeps it under a second. No performance work needed.

The visualiser parser already does typed-ref parsing on every
frontmatter read; omitting empty keys removes parse work rather than
adding it (absent keys short-circuit before `parse_typed_ref`).

## Migration Notes

- **Omit-when-empty for generated artifacts (ADR-0040)**. Producers
  write an optional non-base key only when it has a value and omit it
  otherwise; the empty `""` / `[]` placeholders live only in templates.
  Base fields (incl. `tags`) stay present. This matches the corpus's
  existing tolerance and keeps real artifacts uncluttered.
- **No corpus migration**. Existing artifacts under `meta/` retain
  their current shape. 0070 owns the corpus migration; its inferred-link
  writes follow the new convention.
- **Runtime compatibility**. The visualiser server treats empty and
  absent keys identically and tolerates both ref shapes
  (`typed_ref.rs:62-64`, `cluster_key.rs:138-152`). No coordinated
  rollout needed.
- **Phase ordering**. Phase 0 (ADR) gates the rest. Each implementing
  phase's tests are written before the content moves to satisfy them;
  phases land in sequence; CI is green at the end of each phase. Because
  the producer sweeps (Phases 3–5) change emitted-frontmatter semantics
  that the visualiser consumes, run the full `mise run test` (not just
  `test:unit:templates`) at every phase boundary, not only Phases 1 and
  6 — the suite is cheap and this narrows the blast radius of any
  regression to the phase that introduced it.
- **TDD red state**. To make the "watch it fail first" discipline
  auditable (tests and content land in one merge), land each phase's
  assertion in a commit *preceding* its content commit, or paste the
  failing-run output (FAIL lines + non-zero exit) into the phase's
  verification notes — so the observed-red state is evidenced, not just
  asserted.

## References

- Work item: `meta/work/0093-extend-templates-with-typed-linkage-slots.md`
- Research: `meta/research/codebase/2026-06-02-0093-extend-templates-with-typed-linkage-slots.md`
- Authoritative ADRs: `meta/decisions/ADR-0034-typed-linkage-vocabulary.md` (linkage vocabulary), `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` (base schema / field presence)
- New ADR (Phase 0): `meta/decisions/ADR-0040-omit-when-empty-frontmatter-emission-supplement-to-adr-0033.md`
- Predecessor work: `meta/work/0065-update-artifact-templates-to-unified-schema.md`, `meta/work/0066-update-review-skills-inline-frontmatter.md`
- Downstream consumer (no edits): `skills/visualisation/visualise/server/src/typed_ref.rs`, `cluster_key.rs:138-152`, `frontmatter.rs:305-368`, `indexer.rs:869-887`
- Canonical Populate-frontmatter snippet (referenced verbatim): `meta/plans/2026-05-30-0065-update-artifact-templates-to-unified-schema.md:198-236`
- Work-item review (Pass 3 APPROVE, polish items): `meta/reviews/work/0093-extend-templates-with-typed-linkage-slots-review-1.md:273-289`
- Test infra: `scripts/templates-schema.tsv`, `scripts/skills-schema.tsv`, `scripts/test-template-frontmatter.sh`, `scripts/test-skill-frontmatter-population.sh`, `scripts/test-helpers.sh`
- CI entry: `.github/workflows/main.yml:31`, `mise run test`
