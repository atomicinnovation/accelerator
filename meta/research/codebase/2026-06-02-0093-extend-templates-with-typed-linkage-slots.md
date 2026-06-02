---
type: codebase-research
id: "2026-06-02-0093-extend-templates-with-typed-linkage-slots"
title: "Research: Extend Templates With Typed-Linkage Slots (0093)"
date: "2026-06-02T11:23:51+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0093"
topic: "Extend artifact templates with empty optional slots for the typed-linkage keys defined in ADR-0034"
tags: [research, codebase, templates, frontmatter, schema, linkage, adr-0034]
revision: 1486bcbe3adbdab4cf852d4716f339b47c1b08e3
repository: build-system
last_updated: "2026-06-02T16:07:06+00:00"
last_updated_by: Toby Clemson
last_updated_note: "Rebased on top of typed-linkage clustering rollout (commits xzkvrrlw..tntvnkpq); added Follow-up Research section covering the new typed_ref.rs / cluster_key.rs / widened indexer consumers and recording the Pass 3 APPROVE verdict on the 0093 work-item review."
schema_version: 1
---

# Research: Extend Templates With Typed-Linkage Slots (0093)

**Date**: 2026-06-02T11:23:51+00:00
**Author**: Toby Clemson
**Git Commit**: df26dd2310c762c45daa2cf0f77491a522bb37d2
**Branch**: HEAD (jj workspace `build-system`)
**Repository**: build-system

## Research Question

What is the current state of the codebase that work item [0093 — Extend
Templates With Typed-Linkage Slots](../../work/0093-extend-templates-with-typed-linkage-slots.md)
will modify, and what implementation-relevant constraints fall out of that
state? Specifically:

1. What linkage-related keys do the twelve in-scope templates carry today,
   and what are their value shapes and inline-comment styles?
2. What is the current shape of `scripts/templates-schema.tsv` and how does
   `scripts/test-template-frontmatter.sh` consume it?
3. Across the fifteen affected `SKILL.md` files, what is the current
   Populate-frontmatter snippet shape — is it actually uniform, as 0093
   assumes, or has it drifted?
4. What does ADR-0034 require, and how do related work items (0065, 0066,
   0070) constrain the implementation?
5. What CI/test infrastructure must continue to pass?
6. What downstream code reads typed-linkage frontmatter today?

## Summary

The work-item's scope assumptions are mostly correct, with three
implementation-relevant qualifications:

- **Two template filenames differ from 0093's text.** `templates/issue-research.md`
  does not exist — the kind:rca template is `templates/rca.md`.
  `templates/plan-validation.md` does not exist — the plan-validation template
  is `templates/validation.md`. The TSV rows already use the actual filenames,
  so the implementation should follow the filesystem, not the work item's
  text.

- **The "canonical Populate-frontmatter snippet" 0093 references is in the
  0065 plan, not in 0065's work item.** It lives at
  `meta/plans/2026-05-30-0065-update-artifact-templates-to-unified-schema.md:198-236`.
  More importantly: **the current state across the fifteen SKILL.md files is
  not actually uniformly canonical** — there are two internal canons (writer
  vs reviewer) plus four real drift sites that 0093 will collide with:
  `refine-work-item` (no Populate-frontmatter heading; uses `—` not `←`;
  writes `parent` as a bare-id scalar `"0001"` not the typed-linkage
  `"work-item:0001"` form), `create-plan` (writes `work_item_id` as a
  bare-id scalar instead of a `target:`-shaped typed ref), `analyse-design-gaps`
  (writes `current_inventory`/`target_inventory` as filesystem paths not
  typed refs), and the four reviewer skills (no literal `Populate
  frontmatter` heading at all). 0093 cannot just "extend the existing
  canonical snippet" — it must first decide whether to normalise these
  divergences in this story or scope them out.

- **The current test does not assert typed-linkage slots at all.** Today's
  `scripts/test-template-frontmatter.sh` (227 lines) treats `target`,
  `supersedes`, etc. as either per-type extras (presence-only) or simply
  ignores them. The TSV has six columns; 0093's seventh column
  `typed_linkage_keys` is genuinely new infrastructure, requiring a new
  assertion block, an in-script cardinality map keyed by linkage-key name,
  and a closed-set walk to detect spurious slots.

The infrastructure for adding the new assertions is solid: `extract_frontmatter`
and `assert_in_block` helpers are reusable; the script uses POSIX ERE
consistently (no PCRE features available); CI is a single `mise run test`
entry point invoked from `.github/workflows/main.yml`; and only the
visualiser Rust server actually programmatically reads typed-linkage keys
(via `skills/visualisation/visualise/server/src/frontmatter.rs`,
`indexer.rs`, `related.rs`), so adding empty slots to templates carries no
runtime risk for that consumer.

## Detailed Findings

### Current state of the twelve templates

All twelve templates exist under `templates/`. Filename clarifications:

| 0093 text                  | Actual file                  |
|----------------------------|------------------------------|
| `templates/issue-research.md` | `templates/rca.md` (declares `type: issue-research`) |
| `templates/plan-validation.md` | `templates/validation.md` (declares `type: plan-validation`) |
| (other ten)                | match 0093 exactly           |

**Linkage-key coverage matrix today** — only five keys appear anywhere in
frontmatter; six of the eleven keys 0093 expects are entirely absent:

| Key               | Templates carrying it today                                        |
|-------------------|--------------------------------------------------------------------|
| `parent`          | `work-item.md` (`""`), `plan.md` (`""`)                            |
| `supersedes`      | `adr.md` (`[]`)                                                    |
| `superseded_by`   | _none_                                                             |
| `blocks`          | _none_                                                             |
| `blocked_by`      | _none_                                                             |
| `target`          | `validation.md`, `plan-review.md`, `work-item-review.md`, `pr-review.md` (all `""`) |
| `derived_from`    | _none_                                                             |
| `relates_to`      | _none_                                                             |
| `source`          | `design-inventory.md` (`"{source-id}"` — bare placeholder, not typed-linkage shape) |
| `current_inventory` | `design-gap.md` (path-string placeholder, not typed-linkage shape) |
| `target_inventory` | `design-gap.md` (same)                                            |

For 0093's slot-emission shape rule, the existing inline-comment styles fall
into three patterns — the implementation must converge them to the
normative grammar in Requirements §1:

1. **Single-ref, canonical form** (`templates/work-item.md:11`):
   ```
   parent: ""                                   # typed-linkage key: "work-item:NNNN" or empty
   ```
   The wording `typed-linkage key:` matches 0093's normative grammar
   `# typed-linkage ref: "<source-type>:NNNN" or ""` only in spirit — the
   words `key` vs `ref` differ, and `empty` vs `""` differs. The migration
   must rewrite this comment.

2. **List-cardinality form** (`templates/adr.md:10`):
   ```
   supersedes: []                               # typed-linkage list: ["adr:ADR-NNNN", ...] or []
   ```
   This already matches 0093's normative list-form grammar exactly. Worth
   preserving as the reference shape.

3. **Variant with lifecycle annotation** (`templates/plan-review.md:9`):
   ```
   target: ""                                   # typed-linkage key per ADR-0034: "plan:<plan-id>" (filled by review-plan)
   ```
   These reviewer-template comments carry extra `per ADR-0034` and
   `(filled by ...)` fragments. 0093's Requirements §1 grammar permits
   the source-type token in the comment but doesn't include the lifecycle
   annotation — the test regex must be careful here. Either the existing
   reviewer comments will need rewriting, or the regex must allow optional
   trailing prose.

Two further design-gap-specific facts:

- `templates/design-inventory.md:9` carries `source: "{source-id}"` with
  **no inline comment** and the value is a free-form template placeholder,
  not a typed-linkage `"kind:id"` reference. 0093 §2 expects design-inventory
  to carry `parent: ""` and `relates_to: []` but does not address the
  existing `source` field — it is **not** the typed-linkage `source` key
  (the design-inventory's `source` is a foreign reference per ADR-0033's
  identity vocabulary, not a typed-linkage edge).
- `templates/design-gap.md:9-10` carries `current_inventory` and
  `target_inventory` as path-string placeholders without inline comments.
  Per ADR-0034 §"Design-gap inventory keys" these stay verbatim — 0093
  AC #4 explicitly requires this carve-out be preserved.

**Code references:**
- `templates/work-item.md:1-17`
- `templates/plan.md:1-18`
- `templates/adr.md:1-15`
- `templates/codebase-research.md:1-17`
- `templates/rca.md:1-17` (filename ≠ work-item text)
- `templates/pr-description.md:1-19`
- `templates/design-inventory.md:1-21`
- `templates/design-gap.md:1-15`
- `templates/validation.md:1-15` (filename ≠ work-item text)
- `templates/plan-review.md:1-19`
- `templates/work-item-review.md:1-20`
- `templates/pr-review.md:1-19`

### `scripts/templates-schema.tsv` current shape

- Separator: literal tab. Six columns: `template`, `type`, `code_state_anchored`,
  `extras`, `status_vocab`, `forbidden_own_id_key`.
- 13 data rows + 1 header. The test's structural self-check is `awk -F'\t' 'NF != 6'`
  (lines 33-40 of the test) — it will need to change to `NF != 7`.
- Header (`scripts/templates-schema.tsv:1`):
  ```
  template	type	code_state_anchored	extras	status_vocab	forbidden_own_id_key
  ```
- **Critical observation for 0093's Requirements §4 ("seventh column ...
  the seventh-column form is mandatory; the extras-list alternative is
  rejected"):** today's `extras` column already carries `target` as an
  extra on the four review/validation rows. To keep AC #2(d) ("no template
  carries a linkage key not listed in its TSV row") sound and to avoid
  double-counting, the implementation must **move `target` out of `extras`
  into `typed_linkage_keys`** on:
  - `validation.md` (extras row L4: `result target` → `result`)
  - `plan-review.md` (extras row L11: `... target` → drop `target`)
  - `work-item-review.md` (extras row L12: `... target work_item_id` → drop `target`)
  - `pr-review.md` (extras row L6 [for adr] not affected; row L13:
    `... target pr_number` → drop `target`)

  Similarly `supersedes` is **not currently** in `extras` for `adr.md` —
  the test has not been asserting its presence at all. Moving it (or
  adding it) to the new column closes that gap.

  `current_inventory` and `target_inventory` stay in `extras` for
  `design-gap.md` per ADR-0034's carve-out — they are not in the
  typed-linkage vocabulary.

### `scripts/test-template-frontmatter.sh` current shape

- 192 lines. POSIX ERE throughout via `grep -E` / `grep -qE`. No PCRE
  features available (no `\d`, `\s`, `\w`, no lookaround, no non-greedy).
  Use `[[:space:]]`, `[[:digit:]]` etc., as the existing code does.
- The loop at lines 82-152 runs eight ordered assertion phases per
  template: base fields, type-literal, schema_version, id-quoting,
  forbidden-own-id, provenance, extras (presence-only), status-vocab.
- The natural insertion point for new typed-linkage assertions is
  **between line 135 (after the extras presence loop) and line 137
  (before the status-vocab block)**, preserving an order of:
  base → identity → forbidden-id → provenance → extras → typed-linkage → status.
- Reusable helpers in this script: `extract_frontmatter` (lines 42-53),
  `assert_in_block` (55-65), `assert_not_in_block` (67-77). Reusable in
  the sibling `scripts/test-helpers.sh`: `PASS`/`FAIL`/`SKIP` counters
  (lines 16-18), `assert_matches_regex` (118-129), `assert_not_matches_regex`
  (131-142), `test_summary` (347-357).
- **Plan-relevant regex pattern.** The existing idiom
  `([[:space:]]+#.*)?$` (e.g. lines 105, 108, 111) accepts an *optional*
  inline comment. For 0093 the comment becomes mandatory and structured —
  e.g. for single-ref:
  ```
  ^(parent|target|source|superseded_by):[[:space:]]+""[[:space:]]+#[[:space:]]+typed-linkage[[:space:]]+ref:[[:space:]]+\"[a-z-]+:[A-Za-z0-9-]+\"[[:space:]]+or[[:space:]]+\"\"$
  ```
  with an additional clause for the `superseded_by`/`blocked_by`
  trailing-sentence requirement. A small bash-associative-array
  `cardinality` map keyed by linkage-key name will determine which regex
  to apply.
- **Closed-set check (AC #2(d)).** Implementable as: extract every key
  name from the frontmatter (regex `^([a-z_][a-z0-9_]*):`), set-subtract
  the union of `BASE_FIELDS + PROVENANCE_FIELDS + extras + typed_linkage_keys`,
  intersect what remains with the linkage-key vocabulary set, and fail if
  the intersection is non-empty. The script already has the building blocks.

**Code references:**
- `scripts/templates-schema.tsv:1-13`
- `scripts/test-template-frontmatter.sh:33-40` — structural self-check
- `scripts/test-template-frontmatter.sh:42-77` — helpers
- `scripts/test-template-frontmatter.sh:82-152` — main loop with eight assertion phases
- `scripts/test-template-frontmatter.sh:132-135` — extras loop (insertion point above)
- `scripts/test-template-frontmatter.sh:137-150` — status-vocab (insertion point below)
- `scripts/test-helpers.sh:118-142` — generic regex matchers

### SKILL.md Populate-frontmatter snippet current state

All fifteen SKILL.md files exist and have some form of frontmatter-population
step. They fall into **four shape groups**, not one canonical form. This is
the most important constraint 0093 missed in its drafting.

**Group A — "writer canonical" (8 skills, internally consistent):**
`create-work-item`, `extract-work-items`, `create-plan`, `create-adr`,
`extract-adrs`, `research-codebase`, `research-issue`, `describe-pr`.
Heading style varies (bold list-item label, `### Step N:` H3, mixed
title), but the substitution-list shape with `←` arrows is uniform.
`research-codebase` and `research-issue` drop the nested
`1. Invoke / 2. Substitute / 3. Write` numbering; the other six keep it.

**Group B — "reviewer canonical" (4 skills, parallel internal shape):**
`validate-plan`, `review-plan`, `review-work-item`, `review-pr`. **None of
them carry a literal `Populate frontmatter` heading or
`**Populate frontmatter**:` bold lead-in** — the population step is folded
into prose-only sub-steps of the surrounding write-artifact step. All four
already emit `target:` in the canonical ADR-0034 `"doc-type:id"` form
with `per ADR-0034` cited inline. `review-work-item` additionally carries
a transitional `work_item_id:` alias (explicitly flagged as transitional
in its own prose).

**Group C — "design skills" (2 skills, partial drift):**
`inventory-design`, `analyse-design-gaps`. Both use a flat list with no
nested numbering. `analyse-design-gaps:165-168` emits `current_inventory`
and `target_inventory` as absolute filesystem paths — **NOT** typed-linkage
refs. ADR-0034 §"Design-gap inventory keys" sanctions this carve-out
(they are not typed-linkage keys), but 0093 §2 requires design-gap to
*also* carry `parent: ""` and `relates_to: []` in the new typed-linkage
slot set — those new slots must coexist with the existing inventory keys.

**Group D — "refine-work-item" (1 skill, maximum drift):**
`refine-work-item` has no `Populate frontmatter` heading at all; the
field-population logic is inlined into the **decompose** operation
(`skills/work/refine-work-item/SKILL.md:180-205`). Uses em-dash `—`
separators instead of arrow `←`. Most critically: **writes `parent:` as
a bare-id scalar (`"0001"` form), not as a typed-linkage `"work-item:0001"`
ref.** This is a head-on conflict with 0093's Requirements §1 single-ref
shape (`# typed-linkage ref: "<source-type>:NNNN" or ""`). The plan must
either:
- normalise refine-work-item to write `parent: "work-item:0001"` (which
  also requires updating `create-work-item` since it currently emits
  `parent: ""` with the same legacy expectation), OR
- explicitly carve out work-item `parent` as a special case where the
  scalar is the bare ID (consistent with current visualiser logic in
  `skills/visualisation/visualise/server/src/frontmatter.rs`), OR
- declare the canonicalisation out of scope and accept the drift.

The work item as written does not surface this question. **It is the
single highest-risk implementation decision** because it touches the
visualiser's reference-resolution logic (`indexer.rs` — `parent` is read
via the same code path as `target`, see `read_ref_keys`).

Similarly, `create-plan` (`skills/planning/create-plan/SKILL.md:243-244`)
writes `work_item_id:` as a bare-id scalar (`"PROJ-0001"`), not as a
typed `target:` ref or typed-linkage `parent:`. 0093 §2 requires
`plan.md` to carry `parent: ""` (already does) plus `blocks: []`,
`blocked_by: []`, `derived_from: []`, `relates_to: []` — none of which
touch `work_item_id`, so this drift is technically out of scope, but it
is the same legacy-scalar pattern as `refine-work-item`'s `parent` and
hints at a broader normalisation question.

**Code references:**
- `skills/work/create-work-item/SKILL.md:436-459` — Group A canon example
- `skills/decisions/create-adr/SKILL.md:148-173` — Group A canon, with `supersedes` already in typed form
- `skills/planning/review-plan/SKILL.md:420-449` — Group B canon example
- `skills/github/review-pr/SKILL.md:458-494` — Group B canon
- `skills/design/analyse-design-gaps/SKILL.md:145-171` — Group C, design-gap inventory paths
- `skills/work/refine-work-item/SKILL.md:180-205` — Group D drift (bare-id `parent`)
- `skills/planning/create-plan/SKILL.md:243-244` — bare-id `work_item_id` drift
- `meta/plans/2026-05-30-0065-update-artifact-templates-to-unified-schema.md:198-236` — the canonical snippet 0093 references

### ADR-0034 vocabulary (authoritative input)

- **Status: accepted.** (`meta/decisions/ADR-0034-typed-linkage-vocabulary.md:5`).
  No re-audit required.
- **Nine canonical keys** plus design-gap's carve-out (lines 46-56):
  - **Single ref:** `parent`, `superseded_by`, `target`, `source`
  - **List:** `supersedes`, `blocks`, `blocked_by`, `derived_from`, `relates_to`
  - **Carve-out (not typed-linkage):** `current_inventory`, `target_inventory`
- **Bidirectional sufficiency rule** (lines 58-65): "writing either side
  is sufficient to express the relationship — consumers MUST be able to
  derive the inverse direction by traversing the corpus". This justifies
  0093's decision that ADRs only carry `supersedes` (no `superseded_by`)
  while mutable artifacts carry both canonical and inverse keys.
- **Type-pair table** (lines 89-104, 15 rows) — describes sanctioned edges.
  0093's §2 per-template slot set is consistent with this table; spot-checked
  every row.
- **Design-gap inventory carve-out** (lines 106-108) confirms
  `current_inventory`/`target_inventory` are not folded into the generic
  vocabulary; 0093 AC #4 must preserve them verbatim.

### Related work items

- **0065 (Update Artifact Templates to Unified Schema): done.** Established
  the unified base + the four linkage keys 0093 inherits. The "canonical
  Populate-frontmatter snippet" 0093 references is in 0065's plan
  (`meta/plans/2026-05-30-0065-update-artifact-templates-to-unified-schema.md:198-236`),
  not the work item — quoted in full above. 0065's Requirements bullet 4
  (line 35) explicitly defers the broader typed-linkage vocabulary to a
  follow-up.
- **0066 (Review-skills inline frontmatter → templates): done.** Created
  `templates/plan-review.md`, `templates/work-item-review.md`,
  `templates/pr-review.md`, each carrying `target:` in typed-linkage form.
  No other linkage keys added — exactly the gap 0093 fills.
- **0070 (Corpus migration): draft.** Body-section parser infers links
  from prose `## References` / `## Related Documents` blocks and writes
  them to existing artifacts' frontmatter. **0093 blocks 0070** because
  0070 wants stable template slots to write into; 0070 does not block
  0093. 0070 does NOT mention 0093 as input — that linkage is only
  recorded on the 0093 side.

### CI / test infrastructure that must continue to pass

- **Single CI entry point:** `.github/workflows/main.yml:31` → `mise run test`.
  No Husky, no pre-commit, no `Makefile`, no root `package.json`.
- **Three template/skill schema tests** run as part of
  `mise run test:unit:templates` via `tasks/test/unit.py:35`:
  - `scripts/test-template-frontmatter.sh` — extended by 0093
  - `scripts/test-skill-frontmatter-population.sh` — driven by `scripts/skills-schema.tsv`,
    asserts each SKILL.md instructs population of the unified base fields.
    **0093 does NOT extend this script's TSV** — the SKILL.md changes are
    asserted by grep-based checks in 0093 AC #3, not by this test. Worth
    confirming during planning whether the grep check should be folded into
    this test (DRY) or stay separate (the latter is what 0093 writes).
  - `scripts/test-metadata-helpers.sh` — orthogonal; unaffected.
- **Integration-test discovery** (`tasks/test/integration.py`) auto-globs
  `**/test-*.sh` per skill tree — no test file rename or new top-level
  test will be picked up unless wired through `tasks/test/unit.py`.

### Downstream consumers of typed-linkage frontmatter

Only the **visualiser Rust server** programmatically reads typed-linkage
keys:

- `skills/visualisation/visualise/server/src/frontmatter.rs` — YAML
  parser; reads `target`, `parent`, `related` (and the legacy
  `work_item_id` foreign-ref) via a generic `read_ref_keys` path that
  strips the typed-linkage prefix (`plan:`, `work-item:`, `pr:`) on
  resolution.
- `skills/visualisation/visualise/server/src/indexer.rs` — maintains
  `reviews_by_target` index; resolves `target: "plan:<id>"` against
  `plans_by_id`.
- `skills/visualisation/visualise/server/src/related.rs` — consumes the
  indexer to compute declared inbound links.

The frontend (`skills/visualisation/visualise/frontend/`) does not parse
linkage keys directly; it consumes already-resolved references from the
server's API. **Implication for 0093:** adding empty `[]`/`""` slots to
templates carries no runtime risk for any consumer — empty slots are
absent edges per ADR-0034 §"Slot emission shape" and the visualiser's
parser treats empty values as absent (verified via the existing
`parent: ""` slot on `work-item.md` and `plan.md`).

The implementation question that **does** carry risk: if the plan
normalises refine-work-item's bare-id `parent` writes to typed-linkage
form, the visualiser's `read_ref_keys` path must be audited to confirm
it strips the new `work-item:` prefix correctly on existing-artifact
reads. This is the load-bearing reason to keep that normalisation out
of 0093's scope unless explicitly added to Requirements.

## Code References

- `templates/work-item.md:11` — current `parent: ""` slot with normative-ish comment
- `templates/adr.md:10` — canonical list-form comment (`# typed-linkage list: [...] or []`)
- `templates/plan-review.md:9`, `work-item-review.md:9`, `pr-review.md:9`, `validation.md:10` — current `target: ""` with lifecycle-annotated comments
- `templates/design-gap.md:9-10` — `current_inventory` / `target_inventory` carve-out, no comments
- `scripts/templates-schema.tsv:1-13` — six-column TSV; rows L4, L11-L13 carry `target` in `extras` today (must move to new column)
- `scripts/test-template-frontmatter.sh:33-40` — `NF != 6` self-check (becomes `NF != 7`)
- `scripts/test-template-frontmatter.sh:42-77` — reusable helpers
- `scripts/test-template-frontmatter.sh:82-152` — main per-template assertion loop; insert new block between lines 135 and 137
- `scripts/test-template-frontmatter.sh:132-135` — current presence-only extras loop
- `scripts/test-helpers.sh:118-142` — `assert_matches_regex` / `assert_not_matches_regex`
- `meta/plans/2026-05-30-0065-update-artifact-templates-to-unified-schema.md:198-236` — canonical Populate-frontmatter snippet (referenced by 0093)
- `skills/work/refine-work-item/SKILL.md:180-205` — Group D drift; writes `parent` as bare ID
- `skills/planning/create-plan/SKILL.md:225-250` — Group A canon, `work_item_id` as bare ID drift
- `skills/decisions/create-adr/SKILL.md:148-173` — Group A canon with typed `supersedes` already in place
- `skills/planning/review-plan/SKILL.md:420-449` — Group B canon; no `Populate frontmatter` heading
- `skills/design/analyse-design-gaps/SKILL.md:145-171` — Group C: emits `current_inventory`/`target_inventory` as paths
- `skills/visualisation/visualise/server/src/frontmatter.rs` — `read_ref_keys` strips typed-linkage prefixes
- `skills/visualisation/visualise/server/src/indexer.rs` — uses `target:` for `reviews_by_target` index
- `.github/workflows/main.yml:31` — CI entry point (`mise run test`)
- `mise.toml` + `tasks/test/unit.py:35` — `test:unit:templates` task runs the three schema test scripts

## Architecture Insights

- **Test infrastructure is the right shape for 0093's needs but encodes
  presence-only assertions on extras.** Moving from "presence only" to
  "shape + comment grammar + closed-set" is the genuine architectural step
  this story takes. The seventh TSV column is the right way to express
  per-template linkage-vocabulary closed sets without overloading the
  extras column (which carries non-linkage extras like `reviewer`, `verdict`,
  `pr_url`, etc.).

- **POSIX ERE constrains comment-regex design.** No PCRE features means the
  inverse-key trailing-sentence requirement ("Producers SHOULD prefer the
  canonical side ...") must be expressed as a literal substring check after
  the regex match — not as a named-capture group. The most idiomatic approach
  is a two-stage check: (a) match the main `# typed-linkage ref: ...` regex;
  (b) for `superseded_by`/`blocked_by`, additionally `grep -qF` the trailing
  sentence on the same line.

- **The "canonical Populate-frontmatter snippet" abstraction is aspirational,
  not enforced.** Today the snippet exists as documentation in a plan file;
  the SKILL.md instances drift in numbering, heading style, separator
  character, and even structure (refine-work-item has no dedicated step).
  0093's claim that "per-skill divergence from that shape is not permitted"
  is enforced by reviewer judgement, not by a test. If the plan wants to
  enforce uniformity programmatically, an extension to
  `scripts/test-skill-frontmatter-population.sh` (asserting each SKILL.md's
  Populate-frontmatter section names the slot **and** includes "fill" or
  "leave empty") is the natural place — and AC #3 is essentially a manual
  version of that.

- **Two artifact types have linkage-shaped keys that are NOT in the
  typed-linkage vocabulary** and must not be touched: `design-inventory.md`'s
  `source` (a foreign-source identifier per ADR-0033, not a typed-linkage
  edge), and `design-gap.md`'s `current_inventory` / `target_inventory`
  (ADR-0034 §"Design-gap inventory keys" carve-out). The closed-set check
  in AC #2(d) must therefore allow these names to exist in extras without
  being mis-classified as linkage keys — straightforward as long as the
  closed-set check is against `typed_linkage_keys` only and not against
  the linkage vocabulary as a global concept.

- **ADR immutability shapes the asymmetry.** ADRs carry only `supersedes`
  (no `superseded_by`) because an accepted ADR cannot be edited to add
  inverse-edge information. Mutable artifacts (work-item, plan) carry
  both canonical and inverse keys. This asymmetry is encoded in 0093 §2
  and must be preserved — the closed-set check on `adr.md` must reject
  `superseded_by` if a producer were to add it.

## Historical Context

- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` — sister
  ADR that defines the unified-base schema and explicitly defers
  relationship-named keys to ADR-0034 (line 35 of work item 0065
  paraphrases this rule).
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md` — authoritative
  vocabulary; status accepted. Type-pair table at lines 89-104.
- `meta/decisions/ADR-0038-interactive-validation-parameters-for-unified-schema-linkage-migration.md` —
  Interactive validation parameters for 0070's migration; orthogonal to
  template extension but relevant for the downstream context.
- `meta/plans/2026-05-30-0065-update-artifact-templates-to-unified-schema.md` —
  Implementation plan for 0065; contains the canonical Populate-frontmatter
  snippet at lines 198-236.
- `meta/research/codebase/2026-05-30-0065-update-artifact-templates-to-unified-schema.md` —
  Research feeding 0065; useful prior art for how to plan a sweep across
  many templates and SKILL.md files in one story.
- `meta/research/codebase/2026-05-31-0066-update-review-skills-inline-frontmatter.md` —
  Research feeding 0066; established the reviewer-template shape that 0093
  now extends.
- `meta/reviews/work/0093-extend-templates-with-typed-linkage-slots-review-1.md` —
  A review of the work item itself. Plan authors should read this before
  starting the plan to understand any standing concerns.

## Related Research

- `meta/research/codebase/2026-05-30-0065-update-artifact-templates-to-unified-schema.md`
  — Predecessor research that established the unified-base sweep pattern.
- `meta/research/codebase/2026-05-31-0066-update-review-skills-inline-frontmatter.md`
  — Predecessor research for the review-template work this story extends.

## Open Questions

1. **How to handle `refine-work-item`'s bare-id `parent` writes.** Today
   `templates/work-item.md:11` declares `parent: ""` with comment
   `# typed-linkage key: "work-item:NNNN" or empty`. The template comment
   already promises typed-linkage form. But `refine-work-item` writes a
   bare ID (`"0001"`) — a direct contradiction with the template's own
   comment. Is this a producer bug to fix in 0093, or a known
   visualiser-compatibility constraint to honour? The visualiser's
   `read_ref_keys` (`skills/visualisation/visualise/server/src/frontmatter.rs`)
   strips typed-linkage prefixes — strongly suggesting the typed form is
   correct and `refine-work-item` is drifted. **Recommendation: surface
   this as an explicit plan decision before starting implementation.**

2. **Whether AC #3's grep-based check should be folded into
   `scripts/test-skill-frontmatter-population.sh`.** The work item
   describes AC #3 as "a grep-based check ... passes for every (slot,
   SKILL.md) pair" but does not say where the check lives. Folding it
   into the existing skill-frontmatter-population test (which already
   walks `scripts/skills-schema.tsv`) is the DRY choice; leaving it as
   a separate ad-hoc grep is what 0093's text suggests. **Recommendation:
   plan should propose folding it in, with a fallback to standalone if
   that turns out to require a new TSV column.**

3. **Whether `templates/validation.md`'s existing `target:` comment
   (`# typed-linkage key: "plan:..." (filled by validate-plan)`) and the
   reviewer-template variants (`# typed-linkage key per ADR-0034: "plan:<plan-id>" (filled by review-plan)`)
   must be rewritten to match 0093's normative grammar exactly
   (`# typed-linkage ref: "<source-type>:NNNN" or ""`).** They differ in
   wording (`key` vs `ref`), in punctuation, and carry lifecycle
   annotations not present in the normative form. **Recommendation: yes,
   rewrite — uniformity is the point of the test. The lifecycle
   annotation can move to the body prose of each SKILL.md.**

4. **Whether the four reviewer skills (which currently lack a literal
   `**Populate frontmatter**:` heading) must grow one as part of 0093.**
   AC #3 requires "the Populate-frontmatter step of every SKILL.md"
   names every new slot — but for reviewers there is no such step today,
   only an embedded sub-step. **Recommendation: plan should require
   adding the heading and lift the reviewer skills into Group B canon
   with an explicit heading, since the test that AC #3 will eventually
   become can only find sections by heading.**

5. **Whether `templates/design-inventory.md`'s `source` field (today a
   foreign-source identifier per ADR-0033) needs renaming or annotating
   to avoid name-collision with the typed-linkage `source` key.** ADR-0034
   defines `source` as "External or non-meta origin for extracted
   artifacts". `design-inventory`'s `source` already carries that
   semantic (it identifies the external design source) — but its
   value-shape is a free-form id, not a typed-linkage `"kind:id"` ref.
   **Recommendation: keep `design-inventory`'s `source` as a per-type
   extra (it stays in the extras column), and add `source: ""` as
   typed-linkage **only if** the work-item template carries it. Per
   0093 §2, work-item carries `source: ""` but design-inventory's slot
   set does not include `source` — so no collision in scope. Worth a
   confirming line in the plan.**

## Follow-up Research 2026-06-02T16:07:06+00:00

### Context for the follow-up

The original research recorded `revision: df26dd2310...`. The working
tree has since been rebased on top of twelve commits that landed the
"lifecycle clustering composite typed-linkage key" feature plus a
refinement of the 0093 work item and the review that approved it. The
prior research is re-validated against the new tree below.

Commits between the prior revision and `1486bcbe3adbdab4cf852d4716f339b47c1b08e3`
(in chronological order):

```
rxvmxmzqwzzr  Bump version to 1.21.0 [skip ci]
mpuxvmoqqoml  Update work item statuses.
xryrvqnzqpzo  Fix flaky tests
plmnlxootsuo  Converge lifecycle index and cluster detail pages with prototype
yxmqukmrpuun  Research and plan lifecycle clustering improvements.
vpsoqxvytpvu  Tighten lifecycle clustering slug derivation
xzkvrrlwovpz  Add central typed-linkage reference parser           ← new module
xlmtoyzxxppu  Widen target resolution to every target-carrying doc type
zuzxnuqunluo  Cluster lifecycle entries by composite typed-linkage key   ← new module
rzullvpvyvlq  Surface clusterKey on the wire and bridge slug-only siblings
oupvvrxzwtow  Restore green test suite after typed-linkage clustering rollout
tntvnkpqyzxs  Refine template typed linkage story, and research and plan implementation.
```

### What is unchanged (prior findings still authoritative)

Re-checked via `jj diff -r df26dd2310...@ <path>`:

- **All twelve in-scope templates under `templates/`** — no changes. The
  linkage-key coverage matrix, inline-comment patterns, and filename
  clarifications (`rca.md`, `validation.md`) in the original Detailed
  Findings remain accurate.
- **`scripts/templates-schema.tsv`** — no changes. Still six columns,
  thirteen data rows, with `target` in `extras` on the four
  review/validation rows.
- **`scripts/test-template-frontmatter.sh`** — no changes. Insertion
  point between lines 135 and 137 still valid; POSIX-ERE constraints
  unchanged.
- **All fifteen SKILL.md files** under `skills/work/`, `skills/planning/`,
  `skills/decisions/`, `skills/research/`, `skills/github/`,
  `skills/design/` — no changes. The four-group drift assessment (Group
  A writer canonical / Group B reviewer / Group C design / Group D
  refine-work-item) is unmodified.
- **ADR-0034 and the predecessor work items (0065, 0066, 0070)** — no
  changes.

### What is new — the visualiser server's typed-linkage consumers

The prior research described the visualiser as the sole programmatic
consumer of typed-linkage frontmatter, reading via a `read_ref_keys`
path that stripped prefixes inline. That description is now outdated.
The rebase introduces a dedicated module hierarchy.

#### `skills/visualisation/visualise/server/src/typed_ref.rs` (NEW, 147 lines)

The "central typed-linkage reference parser" — a syntactic module that
all three downstream consumers now delegate to. Entry point
(`typed_ref.rs:26`):

```rust
pub fn parse_typed_ref(raw: &str) -> Option<TypedRef>
```

with output type (`typed_ref.rs:12-19`):

```rust
pub enum TypedRef {
    WorkItem(String),
    Plan(String),
    Adr(String),
    Pr(String),
    Path(PathBuf),
}
```

Behaviour relevant to 0093:

- Recognises exactly four typed prefixes: `work-item:`, `plan:`, `adr:`,
  `pr:`. Lowercase, hyphenated, exact-match — `WorkItem:` or `workitem:`
  return `None` (unless path-shaped).
- **Empty `""`, whitespace-only, and empty typed suffix (e.g.
  `"work-item:"`) all return `None`** — the explicit "absent edge"
  contract that 0093's slot-emission shape relies on (verified by tests
  at `typed_ref.rs:113-126`).
- Path-shape escape hatch (`typed_ref.rs:62-64`): if the input contains
  `/` or ends `.md`, returns `TypedRef::Path` regardless of prefix.
  Important caveat for template comments — placeholders like
  `"work-item:NNNN.md"` would route to `Path` instead of `WorkItem`,
  so the comment grammar's `<source-type>:NNNN` example must not
  resemble a path.
- Never panics, never returns `Result` — purely `Option`.
- **`adr:` and `pr:` are parsed but no consumer reads those arms today**
  (silent-ignore by design). `cluster_key.rs:149` returns `None` for
  non-`WorkItem`/`Path`; `indexer.rs:885` has `_ => None`;
  `frontmatter.rs:351` only matches `WorkItem`. This is relevant
  context for AC #1 — emitting `parent: ""` with a `"adr:NNNN"` example
  in the comment is fine syntactically; emitting an actual `"adr:ADR-0034"`
  value will be silently ignored by every current consumer. That is
  acceptable per 0093's assumption that "Consumers ... read linkage keys
  from the unified base + linkage slots; keys absent from a template's
  frontmatter are treated as absent edges, not missing data" — but the
  plan should note it explicitly.

#### `skills/visualisation/visualise/server/src/cluster_key.rs` (NEW, 514 lines)

Composite cluster-key resolution for lifecycle entries. Entry point
(`cluster_key.rs:32-49`):

```rust
pub fn resolve_cluster_key<'a>(
    entry: &IndexEntry,
    entries_by_path: &HashMap<PathBuf, &'a IndexEntry>,
    work_item_by_id: &HashMap<String, PathBuf>,
    plans_by_id: &HashMap<String, PathBuf>,
    project_root: &Path,
    work_item_cfg: &WorkItemConfig,
) -> Option<String>
```

returning a canonical work-item ID (e.g. `"0040"`) by recursively
walking typed-linkage chains, with `MAX_DEPTH = 8` (line 30) and cycle
tolerance (test at lines 410-435).

Frontmatter keys consumed:

- `target:` on review/validation types (line 84) — short-circuits if
  `parse_typed_ref` returns `TypedRef::WorkItem`; recurses via the
  indexer otherwise.
- `parent:` on plans/research/pr-descriptions (line 120).
- `work_item_id:` as the legacy fallback when `parent:` is absent
  (line 125).

**Critical finding for Open Question #1 in the prior research.**
`cluster_key.rs:138-152`'s `id_from_value` accepts BOTH typed-form
(`"work-item:0001"`) AND bare-id (`"0001"`):

```rust
// Adapted from cluster_key.rs:138-152
fn id_from_value(raw: &str, cfg: &WorkItemConfig) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() { return None; }
    match parse_typed_ref(trimmed) {
        Some(TypedRef::WorkItem(id)) => canonicalise_one_id(&id, cfg),
        Some(TypedRef::Path(p))      => /* extract filename stem -> canonicalise */,
        Some(_)                      => None,  // adr:/plan:/pr: parents
        None                         => canonicalise_one_id(trimmed, cfg),
    }
}
```

The `None` arm is the explicit bare-id fallback: when `parse_typed_ref`
fails (e.g. on `"0001"` or `"PROJ-0042"`), the value is passed
straight to `indexer::canonicalise_one_id` which accepts `^\d+$` and
`^[A-Za-z][A-Za-z0-9]*-\d+$` (indexer.rs:1068-1104).

**Implication for Open Question #1 (now closed):** `refine-work-item`
writing `parent: "0001"` as a bare-id scalar is NOT a producer bug
that breaks the visualiser today — both shapes resolve identically.
The plan can therefore treat the bare-id-vs-typed-form question for
work-item `parent` as a normative choice (which 0093 makes via its
template comment grammar `# typed-linkage ref: "<source-type>:NNNN" or ""`),
not a runtime-compat hazard. The follow-up audit on
`refine-work-item`'s output shape can land either with 0093 (if the
plan chooses to normalise) or as a separate cleanup story (if scoped
out). Either choice is safe.

#### `skills/visualisation/visualise/server/src/frontmatter.rs` (now 785 lines)

`read_ref_keys` still exists (lines 305-368) and is still the
work-item-ref aggregator. The change: prefix-stripping is now delegated
to `typed_ref.rs` rather than done inline. The `target:` fallback
admits only `TypedRef::WorkItem` (verified by negative tests at
`frontmatter.rs:522-540`); `Plan`/`Adr`/`Pr`/`Path` are silently
dropped.

Keys still read by `read_ref_keys` (aggregation order):
`work_item_id:` → `work-item:` (transitional) → `ticket:` (legacy) →
`target:` → `parent:` → `related:`.

**Keys NOT read by any visualiser code anywhere:** `derived_from`,
`relates_to`, `blocks`, `blocked_by`, `superseded_by`, `supersedes`,
`source`. The grep is empty.

Bare-ID form is still accepted for `parent`, `related`,
`work_item_id`, `ticket` (tests at `frontmatter.rs:462-466`,
`480-484`, `557-565`). Reinforces the "no runtime risk" finding.

#### `skills/visualisation/visualise/server/src/indexer.rs` (3490 lines)

Two material changes:

1. **`target_path_from_entry` (lines 869-887)** delegates to
   `typed_ref.rs`. The function is now:
   ```rust
   // adapted
   pub(crate) fn target_path_from_entry(entry, plans_by_id, project_root) -> Option<PathBuf> {
       if !entry.r#type.carries_target_frontmatter() { return None; }
       let raw = entry.frontmatter.get("target")?.as_str()?;
       match parse_typed_ref(raw)? {
           TypedRef::Plan(id)  => plans_by_id.get(&id).cloned(),
           TypedRef::Path(p)   => normalize_target_key(p.to_str()?, project_root),
           _                   => None,
       }
   }
   ```
   `WorkItem`/`Adr`/`Pr` arms collapse to `None` here (the work-item
   short-circuit lives in `cluster_key.rs`).

2. **`carries_target_frontmatter()` widened** (`docs.rs:88-96`,
   regression test at `indexer.rs:2606-2633`) to return `true` for all
   four review/validation doc types: `PlanReviews | WorkItemReviews |
   PrReviews | Validations`. Previously only some. This is the
   "Widen target resolution to every target-carrying doc type" commit.
   It validates 0093 §2's slot table — every review type carrying
   `target: ""` is correctly recognised by the indexer.

Empty/absent tolerance is unchanged: `target:` absent → `get`
short-circuits at line 878; `target: ""` → `parse_typed_ref` returns
`None`; `target: null`/numeric/list → `.as_str()?` returns `None`.
All exercised by `target_path_from_entry_rejects_malformed_values`
(lines 2637-2680).

### Revisions to the original Open Questions

| # | Original status | Now |
|---|-----------------|-----|
| 1 | "How to handle refine-work-item's bare-id `parent` writes" — flagged as highest-risk decision | **Closed by codebase evidence.** `cluster_key.rs:138-152` explicitly tolerates both bare-id and typed-form. The plan can pick either normalisation stance; runtime compat is not a constraint. Whichever stance the plan picks should be applied uniformly to `create-work-item`, `refine-work-item`, AND `create-plan`'s `work_item_id:` write (which has the same bare-id pattern). |
| 2 | "Should AC #3's grep-based check fold into `test-skill-frontmatter-population.sh`?" | Unchanged — still a planning decision. |
| 3 | "Should existing `target:` comments be rewritten to the normative grammar?" | Unchanged — still a planning decision. The "rewrite for uniformity" recommendation stands; lifecycle annotations in the existing comments should migrate to body prose per 0093 Requirements §1. |
| 4 | "Should reviewer skills grow a literal `Populate frontmatter` heading?" | Unchanged — still a planning decision. AC #3's grep check needs a section to grep against. |
| 5 | "Does design-inventory's `source` collide with typed-linkage `source`?" | Unchanged — out of 0093 §2 scope. |

### 0093 work-item review reached Pass 3 APPROVE

`meta/reviews/work/0093-extend-templates-with-typed-linkage-slots-review-1.md`
now records three passes:

- **Pass 1 (REVISE)** — 11 findings (0 critical / 4 major / 7 minor).
  All four majors flagged the same root cause: three Open Questions
  leaking into Requirements and AC.
- **Pass 2 (COMMENT, 2026-06-02T11:07:20+00:00)** — Pass 1 majors all
  resolved by the "Decisions Made" section. Five new minor findings
  centred on design-inventory/design-gap producer-skill conditional.
- **Pass 3 (APPROVE, 2026-06-02T11:15:00+00:00)** — design-inventory /
  design-gap producer-skill paths confirmed against the filesystem and
  pinned in Requirements §3; six other SKILL.md paths corrected
  similarly. Work-item status transitioned `draft` → `ready`. Four
  polish findings remain at suggestion severity, deliberately deferred
  to planning.

The four deferred polish items are worth surfacing to the plan author:
(a) normative comment grammar should be extracted into a labelled
block/table; (b) "consumers derive the inverse" actor naming;
(c) regex tolerance for the `<source-type>` token (free word vs curated
set vs template-type-matched); (d) note in Implementation Approach that
templates land first and SKILL.md updates follow, so partial sweep
rollback doesn't require reverting templates.

### Status of the implementation plan

**No plan for 0093 exists yet.** The latest plan in `meta/plans/` is
`2026-06-01-lifecycle-clustering-composite-key.md` (102 KB) and the
preceding one is `2026-05-31-0040-pipeline-visualisation-overhaul.md`.
The top commit's description ("Refine template typed linkage story,
and research and plan implementation") sets the expectation that a
plan follows — but it has not yet been written. The next step on
the 0093 track is `/accelerator:create-plan`.

### New code references introduced by the rebase

- `skills/visualisation/visualise/server/src/typed_ref.rs:12-19` — `TypedRef` enum
- `skills/visualisation/visualise/server/src/typed_ref.rs:26` — `parse_typed_ref` entry point
- `skills/visualisation/visualise/server/src/typed_ref.rs:113-138` — exhaustive empty / malformed / path-route tests
- `skills/visualisation/visualise/server/src/cluster_key.rs:32-49` — `resolve_cluster_key` signature
- `skills/visualisation/visualise/server/src/cluster_key.rs:138-152` — `id_from_value` accepts both bare-id and typed-form (closes Open Question #1)
- `skills/visualisation/visualise/server/src/cluster_key.rs:410-435` — cycle-tolerance test
- `skills/visualisation/visualise/server/src/cluster_key.rs:500-513` — `malformed_empty_typed_parent_resolves_none`
- `skills/visualisation/visualise/server/src/frontmatter.rs:305-368` — `read_ref_keys` (now delegating to `typed_ref.rs`)
- `skills/visualisation/visualise/server/src/frontmatter.rs:346-357` — `target:` fallback admits only `TypedRef::WorkItem`
- `skills/visualisation/visualise/server/src/indexer.rs:869-887` — `target_path_from_entry` via `typed_ref`
- `skills/visualisation/visualise/server/src/indexer.rs:1068-1104` — `canonicalise_one_id` bare-id rules
- `skills/visualisation/visualise/server/src/indexer.rs:2606-2633` — `reviews_by_target_admits_every_target_carrying_doc_type` regression test
- `skills/visualisation/visualise/server/src/docs.rs:88-96` — `carries_target_frontmatter` widened to all review/validation types
- `skills/visualisation/visualise/server/src/clusters.rs:151` — call site for `resolve_cluster_key`
- `meta/reviews/work/0093-extend-templates-with-typed-linkage-slots-review-1.md:273-289` — Pass 3 APPROVE
