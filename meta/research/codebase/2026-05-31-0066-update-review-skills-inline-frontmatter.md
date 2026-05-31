---
date: "2026-05-31T22:49:35+01:00"
author: Toby Clemson
git_commit: 88b3cab89687dd2ca32bc4e38fb603198e46eb92
branch: HEAD
repository: accelerator
topic: "0066 — Move review/validation skills' frontmatter into templates on the unified schema"
tags: [research, codebase, frontmatter, templates, review-skills, schema, unified-schema, 0066]
status: complete
last_updated: "2026-05-31"
last_updated_by: Toby Clemson
---

# Research: 0066 — Move review/validation skills' frontmatter into templates on the unified schema

**Date**: 2026-05-31T22:49:35+01:00
**Author**: Toby Clemson
**Git Commit**: 88b3cab89687dd2ca32bc4e38fb603198e46eb92
**Branch**: HEAD
**Repository**: accelerator

## Research Question

What does the codebase currently look like for the four skills targeted by work item 0066 (`review-plan`, `review-work-item`, `review-pr`, `validate-plan`) and the surrounding template/helper/test infrastructure? Specifically: what shape do they emit today, what shape do they need to emit to comply with ADR-0033 and ADR-0034, what has 0065 already prepared for 0066, and what guardrails/test fixtures will the implementation need to extend?

## Summary

0066 is bounded and tractable. Concretely:

1. **Producer assumption confirmed.** The four named skills are the *only* in-source emitters of `plan-review`, `work-item-review`, `pr-review`, and `plan-validation` frontmatter. The only other hits (`evals/*.json` assertions, the visualiser-server test fixture, one `migrate` test-printf) are non-producers.
2. **0065 has already prepared the ground.** `templates/validation.md` now carries the unified frontmatter block (with `result: ""` and `target: ""` slots waiting for `validate-plan` to populate). `templates-schema.tsv`, `skills-schema.tsv`, `test-template-frontmatter.sh`, `test-skill-frontmatter-population.sh` already include accommodations for 0066 (the four SKILL.md paths are pre-listed in `OWNED_BY_0066`; discovery patterns already include `verdict`, `review_pass`, `target`, `result`).
3. **Three new templates required.** `plan-review.md`, `work-item-review.md`, `pr-review.md` do not exist yet; 0066 creates them following the conventions already established by `work-item.md`, `plan.md`, `pr-description.md`, and especially `validation.md` (which is the closest analogue — same `target` typed-linkage shape, same `status: complete`-only vocab).
4. **One existing template (validation.md) needs the skill it serves to be rewired.** `validate-plan/SKILL.md` already calls `config-read-template.sh validation` at line 116 but only consumes the report body; it still re-specifies a narrower frontmatter inline at lines 133-141. 0066 rewires it to use the template's frontmatter as source of truth.
5. **The verdict-enum inconsistency is real and explicitly out of scope.** `review-plan` and `review-work-item` use `APPROVE | REVISE | COMMENT`; `review-pr` uses `APPROVE | REQUEST_CHANGES | COMMENT` (driven by GitHub API event values); `validate-plan` uses `result: pass | partial | fail`. Per the 0057 epic, 0066 must not normalise these.
6. **One gap discovered: `review-pr` has no `review_pass` lifecycle.** Unlike `review-plan` and `review-work-item`, `review-pr/SKILL.md` does not emit `review_pass` and has no in-place re-review update flow. 0066 must decide whether to introduce one or omit `review_pass` from `pr-review` (in tension with the work item's AC #5 which requires `review_pass` on all three review types).
7. **One gap discovered: ADR-0033 `plan-validation` "baseline fields" is under-specified.** ADR-0033 §"Per-artifact-type extras" lists `result, baseline fields` for `plan-validation`, but "baseline fields" is not enumerated anywhere. 0065's plan resolved this informally as "no extra fields beyond `result`"; 0066 inherits that resolution.
8. **One gap discovered: ADR-0034 does not publish a `pr:` doc-type discriminator.** The work item knowingly emits `target: "pr:<pr-number>"` and proposes logging a follow-up under 0057. The 0066 review-1 Pass-2 flagged this as residual (lacks concrete regex/example).
9. **Cross-artifact carryover from 0065:** `review-pr/SKILL.md:458` still emits `pr_title:` inline. The unified-shape `pr-description.md` renamed `pr_title` → `title`; 0066's new `pr-review.md` template must follow suit, closing the legacy `pr_title:` divergence on review-pr artifacts.

The actual implementation is a near-mechanical mirror of 0065's Phase 3-10 pattern: extend two TSVs, create three templates, rewire four SKILL.md files using the canonical persistence-step snippet, run the existing test drivers.

## Detailed Findings

### A. Current state — the four skills emit frontmatter inline

All four SKILL.md files instruct the model to write a YAML frontmatter block directly. None of them call `config-read-template.sh` for the artifact's frontmatter (one — `validate-plan` — does call it for the report body but re-specifies frontmatter inline).

#### A.1 `skills/planning/review-plan/SKILL.md` (lines 412-446)

Inline frontmatter at lines 417-427:

```yaml
---
date: "{ISO timestamp}"
type: plan-review
skill: review-plan
target: "{plans directory}/{plan-stem}.md"
review_number: {N}
verdict: {APPROVE | REVISE | COMMENT}
lenses: [{list of lenses used}]
review_pass: 1
status: complete
---
```

- **Missing from unified base:** `id`, `title`, `author`, `producer`, `tags`, `last_updated`, `last_updated_by`, `schema_version`.
- **Uses legacy field name:** `skill:` (must become `producer:` per ADR-0033 §"ADR-0028 override").
- **`target` shape today:** project-root-relative *path* (`{plans directory}/{plan-stem}.md`) — must become typed-linkage `"plan:<id>"` per ADR-0034.
- **Verdict enum:** `APPROVE | REVISE | COMMENT`.
- **Filename pattern:** `{plan reviews directory}/{plan-stem}-review-{N}.md`. Lines 397-410 derive stem from plan basename and find next `N` by globbing.
- **Re-review flow (Step 7, lines 526-562):** reads existing artifact, in-memory mutates exactly three frontmatter fields (`verdict`, `review_pass`, `date`), appends a `## Re-Review (Pass {N})` section, writes whole content back. A re-review **bumps `review_pass`**; a brand-new review (different `review_number`) is a separate file.
- **Helper usage:** uses `config-read-context.sh`, `config-read-skill-context.sh`, `config-read-agents.sh`, `config-read-review.sh plan`, `config-read-path.sh plans`, `config-read-path.sh review_plans`, `config-read-agent-name.sh reviewer`, `config-read-skill-instructions.sh review-plan`. **No `config-read-template.sh`.**

#### A.2 `skills/work/review-work-item/SKILL.md` (lines 346-381)

Inline frontmatter at lines 351-362:

```yaml
---
date: "{ISO timestamp}"
type: work-item-review
skill: review-work-item
target: "{work_dir}/{work-item-stem}.md"
work_item_id: "{4-digit number, e.g. 0042}"
review_number: {N}
verdict: {APPROVE | REVISE | COMMENT}
lenses: [{list of lenses used}]
review_pass: 1
status: complete
---
```

- Same base-field gaps as A.1, plus `skill:` legacy.
- **Has both `target` (path) AND `work_item_id` (stable 4-digit identifier).** Lines 383-385 explain: "`work_item_id` provides resilience against work item renames; `target` remains as the path used at review time." Per ADR-0034, `target` should be the typed-linkage `"work-item:<id>"`; `work_item_id` is a foreign reference governed by ADR-0033 §"Identity-value shape contract". The two carry the same edge twice — the ADR-0033 contract excludes relationship-named keys (`target`) from the `<type>_id` rule, so 0066 should remove the duplication. **Recommendation:** keep `target: "work-item:<id>"` (typed linkage); drop `work_item_id` from the new template.
- **Re-review flow (Step 7, lines 426-490):** like A.1, with one twist at lines 462-464 — if existing frontmatter cannot be parsed, fall back to writing a new `-review-{N+1}.md`.
- **Eval coverage** at `evals/evals.json:30-40` and `evals/benchmark.json:89-100` asserts current literal values (`type: work-item-review`, `skill: review-work-item`, verdict ∈ {APPROVE, REVISE, COMMENT}). 0066 must update these assertions to match the new emission (e.g. `producer: review-work-item`, plus `schema_version`/`id` expectations).

#### A.3 `skills/github/review-pr/SKILL.md` (lines 448-496)

Inline frontmatter at lines 451-462:

```yaml
---
date: "{ISO timestamp}"
type: pr-review
skill: review-pr
target: "PR #{number}"
pr_number: {number}
pr_title: "{title}"
review_number: {N}
verdict: {APPROVE | REQUEST_CHANGES | COMMENT}
lenses: [{list of lenses used}]
status: complete
---
```

- Same base-field gaps; `skill:` legacy.
- **`target` is a synthetic label** (`"PR #{number}"`), not a path or typed-linkage. 0066 changes this to `"pr:<pr-number>"` per the work item — but see §D.2 below.
- **`pr_title:` is the cross-artifact-rename carryover.** 0065 renamed `pr_title` → `title` on `pr-description.md`. The new `pr-review.md` template must use `title:` not `pr_title:`. (Per 0065's review pass notes and the plan, this is explicitly 0066's job.)
- **Verdict enum: `APPROVE | REQUEST_CHANGES | COMMENT`** — the outlier driven by GitHub Reviews API event values (line 554). Out of scope to normalise (per 0057).
- **No `review_pass` field, no in-place re-review update flow.** This is the lifecycle gap noted in the Summary. The work item's AC #5 mandates all three review types carry `review_pass`. Options for 0066: (a) introduce a `review_pass` lifecycle in `review-pr` (more scope), (b) omit `review_pass` from the `pr-review` template and update AC #5 accordingly, or (c) emit `review_pass: 1` as a fixed initial value with no re-review semantics (compromise; documents intent without changing skill behaviour). **Recommendation:** option (c) — fixed `review_pass: 1` keeps the field present and template-symmetric with the other two review types, and any future re-review logic can populate it without further template change. The work item should be updated to make this explicit.

#### A.4 `skills/planning/validate-plan/SKILL.md` (lines 133-144)

Inline frontmatter at lines 134-141:

```yaml
---
date: "{ISO timestamp}"
type: plan-validation
skill: validate-plan
target: "{plans directory}/{plan-filename}.md"
result: {pass | partial | fail}
status: complete
---
```

- The simplest of the four. Same base-field gaps; `skill:` legacy; `target` is a path not typed-linkage.
- **No `review_number`, no `lenses`, no `verdict`, no `review_pass`.** Re-validating presumably overwrites; the skill is silent on the case where the file already exists.
- **Already uses `config-read-template.sh` for the report body** (line 116: `` !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh validation` ``), but the inline block at 134-141 is narrower than the template's now-populated frontmatter. 0066's job is to delete the inline block and rewire the skill's persistence step to substitute into the template's frontmatter via the canonical snippet, populating `result:` and `target:` at write time.
- **Side effect (lines 152-154):** if `result: pass`, set the validated plan's `status: complete`. Orthogonal to frontmatter shape; preserve.

### B. Target shape — ADR contracts

#### B.1 ADR-0033 base schema (every artifact, in order)

`type`, `id` (always quoted YAML string), `title`, `date` (quoted ISO UTC), `author` (human), `producer` (skill/agent — replaces `skill:`), `status`, `tags` (array, may be empty), `last_updated` (quoted ISO UTC), `last_updated_by`, `schema_version` (bare integer, currently `1`).

**Identity-value contract:** `id` and foreign `<snake_case_type>_id` references are always quoted YAML strings. Relationship-named keys (`parent`, `supersedes`, **`target`**, `derived_from`) are governed by ADR-0034, not by the `<type>_id` rule.

#### B.2 ADR-0033 per-type extras

```
plan-validation:                 result, baseline fields
plan-review / work-item-review / pr-review:
                                 reviewer, verdict, lenses, review_number, review_pass
                                 ("where applicable")
```

- The story's claim is correct for the review types but **misses "baseline fields"** for plan-validation. The ADR does not enumerate what "baseline fields" means. 0065's plan resolved this informally as "no extra fields beyond `result`" (validation reports describe the validated artifact, which already carries its own provenance; baseline fields are out-of-band). 0066 should keep `result` as the only extra on `plan-validation`, matching `templates/validation.md` as already prepared by 0065.

#### B.3 ADR-0034 typed-linkage `target` shape

- `target` is a **single quoted string** in `"doc-type:id"` form (or project-root-relative path for external entities). Examples in the ADR: `"plan:0042"`, `"adr:ADR-0033"`, `"work-item:0061"`.
- **Type-pair table includes** `plan-review --target--> plan`, `work-item-review --target--> work-item`, `plan-validation --target--> plan`.
- **`pr-review` row is missing.** Doc-type discriminator `pr` is not in the published vocabulary (ADR-0033 enumerates `pr-review`, `pr-description` but not `pr`). Per the ADR's "external entities use path form" guidance, the strictly-compliant emission for `pr-review` today is a path/URL string, not `"pr:<pr-number>"`. The work item knowingly diverges to keep the data model uniform and proposes a follow-up ADR-0034 amendment.

### C. Existing template + helper conventions (the shape 0066 must mirror)

#### C.1 `scripts/config-read-template.sh` contract

- Single positional argument = template key (basename without `.md`).
- Resolves via three-tier fallback in `config-common.sh` `config_resolve_template` (lines 189-229): tier 1 `templates.<key>` config override (if relative, resolved against project root); tier 2 user override `<paths.templates>/<key>.md`; tier 3 plugin default `<plugin_root>/templates/<key>.md`.
- **Wraps content in `` ```markdown ``...`` ``` `` fences** (lines 37-50) unless the file already starts with `` ``` ``. Rationale per header comment: makes the LLM treat the content as a template, not instructions.
- **No interpolation.** All tokens reach the caller verbatim; substitution is the caller's responsibility.
- Exit 1 if no argument; exit 1 if resolution fails (with "Available templates:" hint).

#### C.2 Canonical SKILL.md inclusion pattern

From `skills/work/create-work-item/SKILL.md:25-32`:

```markdown
## Work Item Template

The template below defines the sections and frontmatter fields that every
work item must contain. Read it now — use it to guide what information you gather
in Step 1 and what structure you produce in Steps 3–4.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh work-item`
```

The `!`...`` ` syntax is Claude Code's exec-substitution prefix — stdout is interpolated into the loaded prompt. The `allowed-tools` frontmatter (`Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)`) whitelists the invocation. The template appears **near the top** of the skill (before Step 0).

#### C.3 Canonical persistence-step pattern

From `skills/work/create-work-item/SKILL.md:436-459` (Step 5 "Populate frontmatter"):

```
5. **Populate frontmatter**: before writing the artifact file, capture
   metadata and substitute the unified base fields into the template's
   frontmatter block.

   1. Invoke `${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh`
      to obtain `Current Date/Time (UTC):`, `Current Revision:`, and
      `Repository Name:`.
   2. **Substitute** every field below with the indicated value:
      - `type:` ← `work-item`
      - `id:` ← ...
      - `title:` ← ...
      - `date:` ← the `Current Date/Time (UTC):` value
      - `author:` ← ...
      - `producer:` ← `create-work-item`
      - `status:` ← `draft`
      - `last_updated:` ← the same `Current Date/Time (UTC):` value
      - `last_updated_by:` ← the same value resolved for `author`
      - `schema_version:` ← `1` (bare integer, not quoted)
   3. Substitute ... throughout the draft body ...
   4. Write the file with the substituted frontmatter block.
```

This shape is what `test-skill-frontmatter-population.sh` recognises as "imperative section context". The verb `Substitute` and the per-field `` `field:` ←  `` notation are the testable surface.

#### C.4 `templates/validation.md` — current state (already updated by 0065)

Verbatim lines 1-15:

```yaml
---
type: plan-validation                        # artifact-type discriminator
id: "{filename-stem}"                        # e.g. "2026-05-18-0042-some-plan-validation"
title: "Validation Report: {Plan Name}"
date: "{ISO timestamp}"
author: "{author from VCS}"
producer: validate-plan
status: complete                             # complete
result: ""                                   # pass | partial | fail (filled by validate-plan)
target: ""                                   # typed-linkage key: "plan:..." (filled by validate-plan)
tags: []
last_updated: "{ISO timestamp}"
last_updated_by: "{author from VCS}"
schema_version: 1
---
```

This is the canonical shape the three new review templates should mirror. Notable elements:
- All string fields **double-quoted**; bare `producer`, `status`, `schema_version` integer.
- Token style is **single-brace mustache** `{filename-stem}`, `{ISO timestamp}`, `{author from VCS}`.
- `result: ""` and `target: ""` are **present-but-empty** — populated by the skill at write time. This is the pattern for "filled by skill" extras.
- `# status: complete` trailing comment is the verbatim status-vocab pin (asserted by `test-template-frontmatter.sh` line 133 via `grep -F`).
- Not code-state-anchored (no `revision`/`repository`). Review/validation reports describe a target artifact, not the codebase at a point in time.

#### C.5 Template + skill schema TSVs

`scripts/templates-schema.tsv` (6 tab-separated fields per row: file • type • code_state_anchored • extras • status_vocab • forbidden_own_id_key). Existing row for `validation.md`:

```
validation.md	plan-validation	no	result	complete	-
```

`scripts/skills-schema.tsv` (3 fields: skill_path • producer_name • fields_to_assert). 0066 will add four rows.

**Pre-baked accommodation:** `scripts/test-skill-frontmatter-population.sh:44-49` already includes:

```
OWNED_BY_0066=(
  "skills/planning/review-plan/SKILL.md"
  "skills/work/review-work-item/SKILL.md"
  "skills/github/review-pr/SKILL.md"
  "skills/planning/validate-plan/SKILL.md"
)
```

And the discovery patterns at lines 160-195 already include `^[[:space:]]*verdict:`, `^[[:space:]]*review_pass:`, `^[[:space:]]*review_target:`, `^[[:space:]]*target:`, `^[[:space:]]*result:`. When 0066 lands, the four paths move from `OWNED_BY_0066` to `IN_SCOPE_PRODUCERS`.

### D. Cross-story interfaces

#### D.1 0065 → 0066 handoff

0065's plan (`meta/plans/2026-05-30-0065-update-artifact-templates-to-unified-schema.md`) explicitly carves out 0066's scope:

- Creates the `validation.md` frontmatter block (above) with `result: ""` and `target: ""` slots.
- Establishes the canonical `${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh <name>` invocation pattern (Phases 4, 7).
- Establishes the canonical persistence-step prose snippet (Phase §"Canonical persistence-step prose snippet").
- Builds the TSV-driven test harness (`test-template-frontmatter.sh`, `test-skill-frontmatter-population.sh`, `test-metadata-helpers.sh`, mise task `test:unit:templates`).
- Pre-allowlists the four 0066 skill paths.
- Carries forward `pr_title` → `title` rename: 0065 did it on `pr-description.md`; 0066 must do it on the new `pr-review.md` template.

**Born-unified gap window.** 0065's plan notes: between 0065 and 0066, new plan-validation artifacts continue to ship the legacy `skill:`/`target:` path shape because `validate-plan/SKILL.md` still emits inline frontmatter. The template's new frontmatter block is dead code until 0066 lands. Implication: 0066 should ship close behind 0065 to minimise this window.

#### D.2 0066 → 0070 handoff

0070's "Blocked by" lists 0066. The corpus migration mechanically rewrites legacy on-disk reviews/validations into the unified shape that 0066's templates now emit. 0066 must hand 0070:
- Stable final template frontmatter for all four artifact types (so the migration's target shape is fixed).
- Pinned `target` value shapes per type: `"plan:<id>"` (plan-review, plan-validation), `"work-item:<id>"` (work-item-review), `"pr:<pr-number>"` (pr-review — caveat per §B.3).

#### D.3 0093 sibling (typed-linkage slots)

0093 will add empty optional typed-linkage slots (`blocks`, `blocked_by`, `derived_from`, `relates_to`, etc.) to every template, **including the three new review templates 0066 creates plus validation.md**. 0093's open question recommends it lands after 0066. 0066 should NOT pre-emptively add general typed-linkage slots beyond `target` — that is 0093's territory.

### E. Test guardrails (the bar 0066 must pass)

#### E.1 `scripts/test-template-frontmatter.sh`

Per row of `templates-schema.tsv`, asserts:
- File exists at `templates/<file>`, frontmatter non-empty.
- All 11 base fields present (`type`, `id`, `title`, `date`, `author`, `producer`, `status`, `tags`, `last_updated`, `last_updated_by`, `schema_version`).
- `type:` matches TSV column verbatim.
- `schema_version: 1` as bare integer (optional `#` comment).
- `id:` value is double-quoted (`^id:[[:space:]]+"[^"]*"`).
- Forbidden own-id legacy key absent (if TSV column is non-`-`).
- If `code_state_anchored=yes`: `revision:` and `repository:` present. Else they can be absent.
- `git_commit:` and `branch:` always absent.
- Each TSV-listed extra present.
- `status:` line contains TSV `status_vocab` verbatim (`grep -F`).
- Cross-check (lines 147-169): templates listed in 0065 work item's `## Schema Reference` table match TSV exactly. **0066 must update that table too** when adding new templates, or the cross-check fails.

#### E.2 `scripts/test-skill-frontmatter-population.sh`

Per row of `skills-schema.tsv`, asserts each `fields_to_assert` token appears in the SKILL.md either:
- inside a fenced YAML block as a key, OR
- inside a section whose heading matches `(?i)(persistence|metadata|frontmatter|populate|capture metadata|step \d)`, where the section contains both a verb match (`[Ss]ubstitute|[Pp]opulate|[Ss]et|[Ww]rite|[Ee]mit`) and a backticked field-name reference (verb and field need not be on the same line).

Plus a phase-11 discovery assertion: greps all `skills/**/SKILL.md` for nine discovery patterns (incl. `config-read-template\.sh`, `producer:`, `schema_version:`, `verdict:`, `review_pass:`, `target:`, `result:`, `pr_number:`); every match must appear in one of `IN_SCOPE_PRODUCERS`, `OWNED_BY_0066`, or `NON_EMITTER_TEMPLATE_CONSUMERS`. When 0066 lands, the four paths move from `OWNED_BY_0066` into `IN_SCOPE_PRODUCERS`.

### F. Residual issues from review-1 Pass 2 (must be addressed in the plan, not the work item)

From `meta/reviews/work/0066-update-review-skills-inline-frontmatter-review-1.md`:

1. **PR `target` shape lacks a concrete verifiable pattern.** Plan should pin a regex (e.g. `^"pr:[0-9]+"$`).
2. **Lifecycle wording inconsistency** — populated-values AC implies all extras are filled at generation time, but Technical Notes still hedges with "present-but-empty" language. Plan should pick one (probably "filled at generation"; `present-but-empty` only applies to fields the skill itself fills later, e.g. `target`/`result` in `validation.md` until `validate-plan` runs).
3. **Helper coupling not in Dependencies section** — but is in Requirements/AC/Technical Notes. Plan should record `config-read-template.sh` explicitly as a coupled artifact.
4. **ADRs not under "Blocked by"** — mirror 0065's call-out.
5. **Discovery-pass AC unspecified grep command** — plan should pin the exact `grep -E` recipe (or refer to `test-skill-frontmatter-population.sh`'s phase-11 grep).
6. **Token-marker syntax undefined.** Decide explicitly: single-brace mustaches `{...}` matching the existing templates. Plan should call this out as the canonical token form.
7. **Fence syntax for "outside fenced template-example blocks" undefined.** Specify: triple-backtick fenced blocks (` ```yaml ` or ` ```markdown `).
8. **AC bullets 5 + 6 split plan-validation coverage** — combine or cross-reference.
9. **Beneficiary not stated in Context** — minor.
10. **Helper-extension carve-out** is open-ended — plan should test whether any extension is in fact required (probably not; the helper already returns raw template content, which is all 0066 needs).
11. **"Or any additional producer found is folded into scope"** — the discovery pass run during research (this doc, §A) confirms only the four named skills emit those types, so this clause is dormant.

## Code References

### The four skills' current frontmatter emissions

- `workspaces/ticket-management/skills/planning/review-plan/SKILL.md:412-446` — `plan-review` inline block (Step 4.8)
- `workspaces/ticket-management/skills/planning/review-plan/SKILL.md:526-562` — re-review update flow
- `workspaces/ticket-management/skills/work/review-work-item/SKILL.md:346-381` — `work-item-review` inline block
- `workspaces/ticket-management/skills/work/review-work-item/SKILL.md:426-490` — re-review update flow (with malformed-frontmatter fallback at 462-464)
- `workspaces/ticket-management/skills/github/review-pr/SKILL.md:448-496` — `pr-review` inline block (note: no `review_pass`, uses `REQUEST_CHANGES`)
- `workspaces/ticket-management/skills/planning/validate-plan/SKILL.md:112-116` — existing `config-read-template.sh validation` call (used for report body only)
- `workspaces/ticket-management/skills/planning/validate-plan/SKILL.md:133-144` — inline frontmatter block (to be deleted)
- `workspaces/ticket-management/skills/planning/validate-plan/SKILL.md:152-154` — side-effect: set plan `status: complete` when `result: pass`

### Templates (existing post-0065)

- `workspaces/ticket-management/templates/validation.md:1-15` — unified frontmatter block with `result: ""` and `target: ""` slots (closest analogue for new review templates)
- `workspaces/ticket-management/templates/work-item.md:1-17` — base-field example with literal placeholder tokens
- `workspaces/ticket-management/templates/plan.md:1-18` — base-field + code-state-anchored example with mustache tokens
- `workspaces/ticket-management/templates/adr.md:1-15` — typed-linkage list example (`supersedes: []` with `["adr:ADR-NNNN", ...]` comment)
- `workspaces/ticket-management/templates/pr-description.md:1-19` — post-0065 example with `pr_title → title` rename

### Helper and tests

- `workspaces/ticket-management/scripts/config-read-template.sh:1-61` — helper contract
- `workspaces/ticket-management/scripts/config-common.sh:189-229` — `config_resolve_template` three-tier fallback
- `workspaces/ticket-management/scripts/templates-schema.tsv` — template schema TSV (extend with 3 new rows)
- `workspaces/ticket-management/scripts/skills-schema.tsv` — skill schema TSV (extend with 4 new rows)
- `workspaces/ticket-management/scripts/test-template-frontmatter.sh:77-143` — per-template assertion logic
- `workspaces/ticket-management/scripts/test-template-frontmatter.sh:147-169` — cross-check against 0065 work item's Schema Reference table
- `workspaces/ticket-management/scripts/test-skill-frontmatter-population.sh:32-54` — `IN_SCOPE_PRODUCERS`, `OWNED_BY_0066`, `NON_EMITTER_TEMPLATE_CONSUMERS` allowlists
- `workspaces/ticket-management/scripts/test-skill-frontmatter-population.sh:130-153` — per-skill assertion logic
- `workspaces/ticket-management/scripts/test-skill-frontmatter-population.sh:160-195` — phase-11 discovery-pass assertion

### Canonical inclusion + persistence pattern to mirror

- `workspaces/ticket-management/skills/work/create-work-item/SKILL.md:25-32` — `## Work Item Template` heading + framing prose + `!`config-read-template.sh work-item`` call
- `workspaces/ticket-management/skills/work/create-work-item/SKILL.md:436-459` — Step 5 "Populate frontmatter" with `Substitute` verb and per-field bullets
- `workspaces/ticket-management/scripts/artifact-derive-metadata.sh` — emits `Current Date/Time (UTC):`, `Current Revision:`, `Repository Name:` labels (date label is the only one the four 0066 skills need; reviews/validations are not code-state-anchored)

### Eval coverage that needs updating (`review-work-item` only)

- `workspaces/ticket-management/skills/work/review-work-item/evals/evals.json:30-40` — asserts `type: work-item-review`, `skill: review-work-item`, lenses field, verdict enum
- `workspaces/ticket-management/skills/work/review-work-item/evals/benchmark.json:89-100` — same assertions

### Non-producer hits (confirming the assumption)

- `workspaces/ticket-management/skills/visualisation/visualise/server/tests/fixtures/meta/validations/2026-01-01-first-plan-validation.md:3` — test fixture, not a producer
- `workspaces/ticket-management/skills/config/migrate/scripts/test-migrate.sh:52` — `printf` test scaffold, not a producer

## Architecture Insights

1. **The work item is a near-mechanical follow-on to 0065.** Every convention, helper, test harness, prose snippet, and TSV format already exists. The implementation pattern is "extend 0065's pattern to four more skills + three more templates" — no new architecture.
2. **`templates/validation.md` is the template-of-record for the canonical shape.** Same `target: ""` typed-linkage pattern, same `status: complete`-only vocab, same not-code-state-anchored property. The three new review templates differ from it only by having additional extras (`reviewer`, `verdict`, `lenses`, `review_number`, `review_pass`) and different `target` payload types.
3. **`target` typed-linkage vs `<type>_id` foreign reference is a known doubling risk.** `review-work-item` currently emits both `target` (path) and `work_item_id` (4-digit id). Per ADR-0033, relationship-named keys are excluded from the `<type>_id` rule — the unified emission should carry `target: "work-item:<id>"` only. This is a design call 0066 must make explicit.
4. **The `review-pr` skill is structurally weaker than the other three.** No `review_pass`, no in-place re-review update flow, different verdict enum (driven by GitHub API). 0066 must decide how much of this to normalise (recommended: keep verdict enum out of scope per epic; emit fixed `review_pass: 1` to keep field symmetry; do not add re-review flow as a separate concern).
5. **Three test drivers + two TSVs is the source-of-truth pattern.** Updating templates without updating the TSVs (or vice versa) fails tests. The cross-check against the 0065 work item's `## Schema Reference` table makes that document part of the test surface too — 0066 must update it.
6. **The `pr:` doc-type prefix is an open ADR question.** Strictly compliant emission for `pr-review.target` today is a project-root-relative path or URL string; the work item knowingly picks `"pr:<pr-number>"` for visualiser-graph uniformity. The plan should pin this with a regex (e.g. `^"pr:[0-9]+"$`) and queue a follow-up under 0057 to amend ADR-0034.

## Historical Context

- `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md` — parent epic; §79 lists per-type extras for review/validation; §94 names the four target skills; §149 lists template extraction as optional (0066 elevates to mandatory).
- `meta/work/0060-adr-unified-base-frontmatter-schema.md` — produced ADR-0033.
- `meta/work/0061-adr-typed-linkage-vocabulary.md` — produced ADR-0034.
- `meta/work/0064-canonicalise-work-item-id-and-author-fields.md` — canonicalised `work_item_id` and `author` field names; shipped as migration 0006.
- `meta/work/0065-update-artifact-templates-to-unified-schema.md` — direct predecessor; updates nine templates; adds frontmatter block to `templates/validation.md`.
- `meta/plans/2026-05-30-0065-update-artifact-templates-to-unified-schema.md` — 11-phase TDD plan for 0065; **the structural template 0066's plan should mirror**.
- `meta/research/codebase/2026-05-30-0065-update-artifact-templates-to-unified-schema.md` — 0065 research, captured the helper-pattern decisions 0066 inherits.
- `meta/reviews/work/0065-update-artifact-templates-to-unified-schema-review-1.md` — 0065 review, surfaces process patterns 0066's plan should adopt.
- `meta/reviews/work/0066-update-review-skills-inline-frontmatter-review-1.md` — review of 0066 itself; verdict APPROVE; lists 11 residual issues (§F above).
- `meta/work/0070-ship-meta-corpus-unified-schema-migration.md` — corpus migration; blocked by 0066.
- `meta/work/0093-extend-templates-with-typed-linkage-slots.md` — sibling story; adds optional typed-linkage slots to every template (incl. the three new review templates); should land after 0066.
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` — base-schema contract.
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md` — typed-linkage contract.

## Related Research

- `meta/research/codebase/2026-05-30-0065-update-artifact-templates-to-unified-schema.md` — sibling research for 0065.
- `meta/research/codebase/2026-05-21-0064-canonicalise-work-item-id-and-author-fields.md` — sibling research for 0064.

## Open Questions

1. **`review-pr.review_pass` lifecycle.** The work item's AC #5 requires `review_pass` on all three review types, but `review-pr/SKILL.md` has no `review_pass` field and no in-place re-review update flow today. Options: (a) introduce one, (b) drop `review_pass` from the `pr-review` template and amend AC #5, (c) emit fixed `review_pass: 1` without re-review semantics. **Recommendation: (c).**
2. **`work-item-review` `target` vs `work_item_id` duplication.** The current skill emits both. ADR-0033 excludes relationship-named keys from the `<type>_id` rule, so emitting `target: "work-item:<id>"` alone is the unified contract. **Recommendation: drop `work_item_id`, keep `target` only.**
3. **`pr-review.target` shape.** `"pr:<pr-number>"` is uniform across review types and supports the future visualiser-graph epic, but `pr` is not in ADR-0034's published vocabulary. Pin a regex (`^"pr:[0-9]+"$`) and queue a follow-up ADR-0034 amendment under 0057.
4. **`plan-validation` baseline fields.** ADR-0033 lists `result, baseline fields` for `plan-validation` extras, but "baseline fields" is not enumerated. 0065 resolved this informally as "no extra fields beyond `result`"; 0066 should adopt the same resolution and possibly flag a follow-up to clarify ADR-0033.
5. **Helper extension.** The story's helper-extension carve-out should be tested explicitly: in §C.1 the helper already returns raw template content — that is sufficient for 0066. Plan should declare no helper extension is needed (or scope one out if so).
