---
date: "2026-05-31T21:18:14Z"
type: work-item-review
producer: review-work-item
target: "work-item:0066"
work_item_id: "0066"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 2
status: complete
id: "0066-update-review-skills-inline-frontmatter-review-1"
title: "0066-update-review-skills-inline-frontmatter-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-31T21:18:14Z"
last_updated_by: Toby Clemson
---

## Work Item Review: Move Review/Validation Skills' Frontmatter into Templates on Unified Schema

**Verdict:** REVISE

Work item 0066 is well-structured and coherent: scope, the 0065/0066 boundary, dependencies, and the four-skills-plus-three-templates story are explicit and well-named throughout. The blocking concerns are concentrated in testability — three Acceptance Criteria use phrasing ("reads its frontmatter from the corresponding template", "where applicable", "does not duplicate the emitted-artifact frontmatter field list") that admits multiple readings, and there is no end-to-end criterion that a generated artifact actually carries populated, non-placeholder values for the new fields. Clarity flags the same ambiguities from a different angle, reinforcing that the criteria need to be pinned to observable checks before implementation.

### Cross-Cutting Themes

- **"Where applicable" qualifier for `review_number`/`review_pass`** (flagged by: clarity, testability) — both lenses flag that applicability per review type is undefined, making the criterion either unverifiable (clarity) or trivially satisfiable (testability).
- **`target` linkage key value shape** (flagged by: clarity, testability) — the parenthetical "the thing reviewed/validated" plus the bare "from the linkage vocabulary" reference leaves the concrete value shape unpinned for each of the four skills.
- **"Does not duplicate the emitted-artifact frontmatter field list"** (flagged by: clarity, testability) — both lenses note that this criterion combines a textual claim with a counterfactual one and lacks a precise pass/fail procedure.

### Findings

#### Major
- 🟡 **Testability**: 'Reads frontmatter from template' lacks an observable verification procedure
  **Location**: Acceptance Criteria (bullets 2 and 3)
  "Reading from a template" is an internal implementation detail; the criterion does not specify how to verify it. A skill that copies the template's contents into prose could be argued as satisfying the criterion under a generous reading.

- 🟡 **Testability**: 'Where applicable' for `review_number`/`review_pass` is undefined
  **Location**: Acceptance Criteria (bullet 5)
  The story does not name which of the three review types must carry these extras, so the per-template check is not binary.

- 🟡 **Testability**: No criterion verifies a generated artifact's frontmatter values are populated
  **Location**: Acceptance Criteria (overall)
  Requirements mention skills must populate `producer`, `schema_version`, `last_updated`, `last_updated_by`, `target`, `reviewer`, `verdict`, `lenses`, but no acceptance criterion asserts the populated-values outcome. A skill could leave placeholders unsubstituted and still satisfy every existing criterion.

#### Minor
- 🔵 **Clarity**: `target` linkage key's referent (what is being reviewed/validated) is named only parenthetically
  **Location**: Requirements / Acceptance Criteria
  Four different skills could populate `target` with four different value shapes (path vs id vs URL) and still believe they have satisfied the criterion.

- 🔵 **Clarity**: "where applicable" for `review_number` and `review_pass` leaves applicability undecided
  **Location**: Requirements / Acceptance Criteria
  The phrasing makes the criterion satisfiable by emitting the keys everywhere, nowhere, or any subset.

- 🔵 **Clarity**: "The four skills' inline frontmatter is the only producer of those four artifact-type bodies" has an ambiguous antecedent
  **Location**: Assumptions
  "Those four artifact-type bodies" could mean artifact `type:` values or template body files; the Context distinguishes these elsewhere.

- 🔵 **Clarity**: "does not duplicate the emitted-artifact frontmatter field list" lacks a precise pass/fail test
  **Location**: Acceptance Criteria
  Combines a textual claim and a counterfactual one; depends on what counts as "the field list".

- 🔵 **Completeness**: Open Questions section is empty
  **Location**: Open Questions
  The prior open question is recorded as resolved in Drafting Notes; either add an explicit "None — resolved" marker or remove the empty heading.

- 🔵 **Dependency**: Canonicalisation sibling 0064 not referenced even as related
  **Location**: Dependencies
  0064 canonicalised the `work_item_id` foreign-reference shape that 0066's templates emit; the lineage is not discoverable from this story.

- 🔵 **Testability**: "Does not duplicate" is a soft check without a defined procedure
  **Location**: Acceptance Criteria (bullet 4)
  "Duplication" is undefined — whether a passing reference to a field name in prose counts is unclear.

- 🔵 **Testability**: `target` value shape not pinned to a verifiable form
  **Location**: Acceptance Criteria (bullet 5)
  Without a pinned shape, a tester cannot check the templates against a single expected pattern.

- 🔵 **Testability**: Assumption that no other producers emit these frontmatter types is not made testable
  **Location**: Acceptance Criteria (overall)
  No criterion confirms the assumption was checked via a reproducible discovery pass.

#### Suggestions
- 🔵 **Clarity**: "The two stories therefore touch the same file from different angles" conflates two different files
  **Location**: Technical Notes
  `validate-plan`'s SKILL.md and `templates/validation.md` are two different files; "the same file" is literally untrue.

- 🔵 **Dependency**: Future visualiser-graph epic as a downstream consumer is not named
  **Location**: Dependencies
  The parent epic identifies it as the consumer of the typed-linkage frontmatter this story finalises for review artifacts.

- 🔵 **Dependency**: Implicit dependency on template-reading helper not named
  **Location**: Requirements
  The mechanism by which rewired skills read template frontmatter (e.g. `config-read-template.sh`) is unnamed; gaps in the helper's capabilities may surface mid-implementation.

- 🔵 **Scope**: Story bundles template creation with skill rewiring across four skills
  **Location**: Requirements
  Defensible cohesion, but worth confirming the team is comfortable delivering all four skill rewirings in one increment.

### Strengths
- ✅ Consistent naming of the four affected skills across every section — no risk of confusing scope.
- ✅ The 0065/0066 boundary is restated explicitly in both Context and Technical Notes, including the ordering rationale around `templates/validation.md`.
- ✅ Upstream blockers are named with their precise contributions (0060 base schema, 0061 linkage vocabulary, 0065 validation.md frontmatter handoff); downstream consumer 0070 is named under Blocks.
- ✅ Acceptance Criteria contains seven enumerable bullets that closely mirror Requirements, giving a clear definition of done at the structural level.
- ✅ Drafting Notes pre-empt likely reader questions (priority rationale, scope shift from inline rewrite to template extraction, verdict-enum exclusion) with explicit reasoning.
- ✅ Single unifying architectural purpose with a finite, enumerated set of in-scope artefacts; verdict-enum alignment is explicitly excluded to prevent scope creep.

### Recommended Changes

1. **Replace "where applicable" with explicit per-template applicability for `review_number` and `review_pass`** (addresses: clarity "'where applicable' for review_number and review_pass", testability "'Where applicable' for review_number/review_pass is undefined")
   In both Requirements and Acceptance Criteria, name which of `plan-review`, `work-item-review`, `pr-review` carry `review_number` and `review_pass`, citing the ADR-0033 row that fixes it.

2. **Pin the `target` value shape per skill** (addresses: clarity "`target` linkage key's referent", testability "`target` value shape not pinned to a verifiable form")
   Either inline the expected `target` encoding for each review type (e.g. `review-plan.target` = plan id; `review-pr.target` = PR identifier) or reference the exact section of 0061's linkage vocabulary that pins it.

3. **Replace "reads from the corresponding template" with an observable check** (addresses: testability "'Reads frontmatter from template' lacks an observable verification procedure")
   Rephrase the two affected Acceptance Criteria so verification is binary, e.g. "each SKILL.md invokes `config-read-template.sh` (or the canonical loader) for its template and contains no inline YAML frontmatter literal for the artifact's base or extra fields, verified by grep returning zero matches for `^type:`, `^schema_version:`, `^verdict:`, `^result:` in SKILL.md prose outside fenced example blocks."

4. **Add an end-to-end populated-values acceptance criterion** (addresses: testability "No criterion verifies a generated artifact's frontmatter values are populated")
   Mirror 0065's analogous criterion: "Generating one review/validation artifact via each of the four skills yields non-empty values with no unsubstituted template tokens for `producer`, `schema_version` (=1), `last_updated`, `last_updated_by`, `target`, `reviewer`, `verdict`, `lenses` (and `review_number`/`review_pass`/`result` where the per-type table dictates), with the two timestamps parsing as ISO-UTC."

5. **Tighten "does not duplicate the emitted-artifact frontmatter field list"** (addresses: clarity "lacks a precise pass/fail test", testability "'Does not duplicate' is a soft check")
   Rephrase to a checkable form, e.g. "SKILL.md contains no YAML frontmatter block listing more than one base or extra field name as `key: value` pairs outside fenced template-example blocks; narrative references to individual field names are permitted."

6. **Add a discovery-pass acceptance criterion** (addresses: testability "Assumption that no other producers emit these frontmatter types is not made testable")
   "A reproducible discovery pass (recorded grep command + matched files) confirms the four named skills are the only inline producers of `plan-review`, `work-item-review`, `pr-review`, and `plan-validation` frontmatter, or any additional producer found is included in scope."

7. **Disambiguate the "those four artifact-type bodies" phrasing in Assumptions** (addresses: clarity "ambiguous antecedent")
   Replace with the explicit list of artifact `type` values (`plan-review`, `work-item-review`, `pr-review`, `plan-validation`).

8. **Reword the "same file" sentence in Technical Notes** (addresses: clarity "conflates two different files")
   Rephrase to "The two stories therefore touch the same artifact pipeline from different angles (0065 edits the template; 0066 edits the skill that reads it)."

9. **Add 0064 to Related** (addresses: dependency "Canonicalisation sibling 0064 not referenced")
   One-line note that 0064 canonicalised the foreign-reference name (`work_item_id`) that 0066's templates emit; no Blocked-by needed since 0064 is done.

10. **Mark Open Questions as resolved or remove the heading** (addresses: completeness "Open Questions section is empty")
    Add "None — resolved during refinement, see Drafting Notes" or drop the empty section.

11. **Name the template-reading helper as an implementation detail** (addresses: dependency "Implicit dependency on template-reading helper not named")
    Add a Technical Note identifying `config-read-template.sh` (or the canonical loader) and flag any required extension to it as in-scope.

12. **Mention the future visualiser-graph epic as a downstream consumer** (addresses: dependency "Future visualiser-graph epic as a downstream consumer is not named")
    Add a sentence under Blocks or Related noting that the future visualiser-graph epic consumes the review-artifact linkage shapes finalised here.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Work item 0066 is largely clear: scope, dependencies, and the 0065/0066 boundary are explicit, and the four-skills-plus-three-templates structure is well-named throughout. A few mid-grain clarity issues exist around the ambiguous referent for the `target` linkage key in reviews, an unresolved hedge about applicability of `review_number`/`review_pass`, and a sentence in Assumptions whose pronoun and condition are slightly underspecified.

**Strengths**:
- Consistent naming of the four affected skills across every section.
- 0065/0066 boundary restated in both Context and Technical Notes.
- Acronyms and jargon (`ADR-0033`, `linkage vocabulary`, `schema_version`, `producer`) defined or pointed at a referenced document.
- Drafting Notes pre-empt likely reader questions (priority rationale, scope shift, verdict-enum exclusion).

**Findings**:
- 🔵 **minor / high**: `target` linkage key's referent named only parenthetically — implementer could populate `target` with four different value shapes across four skills and still believe the criterion is met. Suggest enumerating the expected value shape per skill or citing 0061's section that pins it.
- 🔵 **minor / high**: "where applicable" for `review_number` and `review_pass` leaves applicability undecided — criterion can be satisfied by emitting the keys everywhere or nowhere. Suggest enumerating which review types carry them.
- 🔵 **minor / medium**: "those four artifact-type bodies" has an ambiguous antecedent in Assumptions — could mean `type` values or template body files. Suggest replacing with the explicit list of `type` values.
- 🔵 **minor / medium**: "does not duplicate the emitted-artifact frontmatter field list" combines a textual claim and a counterfactual one and lacks a precise pass/fail test. Suggest tightening to "no inline YAML frontmatter block; field names may appear only when discussing population logic."
- 🔵 **suggestion / medium**: "the two stories therefore touch the same file from different angles" in Technical Notes — `validate-plan`'s SKILL.md and `templates/validation.md` are two different files. Suggest "the same artifact pipeline."

### Completeness

**Summary**: Work item 0066 is a well-structured Story with all expected sections present and substantively populated. Summary, Context, Requirements, Acceptance Criteria, Dependencies, Assumptions, Technical Notes, Drafting Notes, and References are all filled with kind-appropriate content; frontmatter is complete and valid. The only structural concern is an empty Open Questions section, which is acceptable given the Drafting Notes explicitly record the prior open question as resolved.

**Strengths**:
- Frontmatter is complete and integral: kind=story, status=draft, priority=medium, parent=0057, plus title/date/author/tags.
- Acceptance Criteria contains seven specific, enumerable bullets that closely mirror the Requirements.
- Context explains the motivation and grounds the user decision to mandate template extraction.
- Story-specific completeness needs are met: the 'for whom' (downstream maintainers) and 'why' are explicit.
- Dependencies, Assumptions, and Technical Notes are populated with non-trivial content.

**Findings**:
- 🔵 **minor / high**: Open Questions section is present as a heading but contains no content. The Drafting Notes mention the prior open question was resolved by user decision. Suggest either an explicit 'None — resolved during refinement' marker or remove the empty heading.

### Dependency

**Summary**: Work item 0066's dependency capture is thorough and explicit: upstream blockers (0060 base schema, 0061 linkage vocabulary, 0065 validation.md frontmatter handoff), downstream consumer (0070 corpus migration), and related context (0057 parent epic, ADR-0033) are all named. The 0065 → 0066 ordering constraint around the shared validation.md file is called out in both Technical Notes and Dependencies. Two minor couplings to canonicalisation siblings (0063/0064) and to the future visualiser-graph epic are arguably under-captured but each is transitively reachable.

**Strengths**:
- Upstream blockers named with their precise contributions.
- 0065 → 0066 ordering coupling reinforced with rationale in Technical Notes.
- Downstream consumer 0070 explicitly listed under Blocks.
- ADR-0033 threaded through Requirements/Acceptance Criteria.
- Assumptions captures the scope-expansion trigger explicitly.

**Findings**:
- 🔵 **minor / medium**: Canonicalisation sibling 0064 not referenced even as related — 0064 canonicalised the `work_item_id` foreign-reference name 0066 emits. Suggest adding 0064 to Related with a one-line lineage note.
- 🔵 **suggestion / medium**: Future visualiser-graph epic as downstream consumer not named — parent epic identifies it as the consumer of typed linkage frontmatter on review artifacts. Suggest adding a sentence under Blocks or Related.
- 🔵 **suggestion / low**: Implicit dependency on template-reading helper (`config-read-template.sh`) not named — gaps in helper capabilities may surface mid-implementation.

### Scope

**Summary**: 0066 describes a coherent, well-bounded story: take the four named skills that bake frontmatter inline and rewire them to read from template files, creating three missing review templates along the way. The scope is a single architectural change applied to a finite, explicitly enumerated set of four skills, with a clear cut line versus the sibling 0065 story. Sizing is appropriate for a single story.

**Strengths**:
- Single unifying purpose: move frontmatter out of SKILL.md prose into templates for the four named skills.
- Explicit, finite scope boundary — the four affected skills are named throughout.
- Clear cut line with sibling story 0065 is documented.
- Verdict-enum alignment is explicitly excluded with a follow-up disposition recorded.
- Assumptions section acknowledges the one scope-expansion trigger.

**Findings**:
- 🔵 **suggestion / medium**: Story bundles template creation with skill rewiring across four skills. Defensible cohesion — keep as one story unless the implementing team finds rewirings progress at different rates.

### Testability

**Summary**: The Acceptance Criteria are largely testable: existence of template files, presence of specific base fields, identity-value quoting, and the inline-frontmatter removal can all be verified by file inspection and grep-based checks. However, several criteria lean on phrases like 'where applicable' without defining the trigger conditions, the 'reads its frontmatter from the corresponding template' criterion is not pinned to a concrete observable mechanism, and there is no end-to-end criterion that a generated artifact actually carries populated values for the new fields.

**Strengths**:
- Schema Reference in parent dependency 0065 gives a clear, enumerated list of base fields.
- Most criteria name specific files, specific field names, and a concrete YAML shape.
- Identity-value quoting and 'no duplication' criteria are mechanically verifiable.
- Scope boundaries vs. 0065 are explicitly carved out.

**Findings**:
- 🟡 **major / high**: 'Reads frontmatter from template' (bullets 2 and 3) lacks an observable verification procedure — internal implementation detail without a defined check. Suggest rephrasing to "each SKILL.md invokes the canonical loader and contains no inline YAML frontmatter literal, verified by grep for `^type:`, `^schema_version:`, `^verdict:`, `^result:` in SKILL.md prose returning zero matches."
- 🟡 **major / high**: 'Where applicable' (bullet 5) for `review_number`/`review_pass` is undefined — story does not name which of the three review types must carry these extras. Suggest enumerating the mapping explicitly.
- 🟡 **major / high**: No acceptance criterion verifies a generated artifact's frontmatter values are populated — a skill could leave `{{producer}}` placeholders and still satisfy every existing criterion. Suggest mirroring 0065's populated-values criterion covering `producer`, `schema_version`, `last_updated`, `last_updated_by`, `target`, `reviewer`, `verdict`, `lenses`.
- 🔵 **minor / medium**: 'Does not duplicate' (bullet 4) is a soft check — 'duplication' is undefined. Suggest defining the failure mode concretely.
- 🔵 **minor / medium**: 'target' value shape not pinned to a verifiable form (bullet 5) — admits multiple valid encodings. Suggest inlining the expected encoding per review type or referencing the exact ADR section.
- 🔵 **minor / medium**: Assumption that no other producers emit these frontmatter types is not made testable. Suggest adding a discovery-pass acceptance criterion with a recorded grep command and matched files.

## Re-Review (Pass 2) — 2026-05-31

**Verdict:** APPROVE (upgraded from COMMENT after addressing the two highest-value residual fixes — `pr-review` `target` pinned to `"pr:<pr-number>"`, and the populated-values lifecycle clarified as single-write per pass)

All three major findings from Pass 1 are resolved. The work item is now acceptable for implementation; remaining issues are minor refinements and suggestions that the team can choose to address before or during implementation.

### Previously Identified Issues

- 🟡 **Testability**: 'Reads frontmatter from template' lacks observable verification procedure — **Resolved** (AC bullets 2 and 3 now specify the canonical helper and grep-based zero-match checks for `^type:`, `^schema_version:`, `^verdict:`, `^result:`).
- 🟡 **Testability**: 'Where applicable' for `review_number`/`review_pass` is undefined — **Resolved** (AC bullet 5 now states all three review types carry the five extras).
- 🟡 **Testability**: No criterion verifies a generated artifact's frontmatter values are populated — **Resolved** (new AC bullet requires non-empty values and no unsubstituted template tokens for all relevant fields, with timestamps parsing as ISO-UTC).
- 🔵 **Clarity**: `target` referent named only parenthetically — **Partially resolved** (plan and work-item value shapes pinned to `"plan:<id>"` and `"work-item:<id>"`; `pr-review`'s PR-identifier shape remains underspecified — see new issue below).
- 🔵 **Clarity**: 'where applicable' leaves applicability undecided — **Resolved**.
- 🔵 **Clarity**: 'those four artifact-type bodies' ambiguous antecedent — **Resolved** (Assumptions now lists the explicit `type:` values).
- 🔵 **Clarity**: 'does not duplicate the emitted-artifact frontmatter field list' lacks pass/fail test — **Resolved** (AC bullet 4 now defines the failure mode as inline YAML `key: value` enumeration outside fenced example blocks).
- 🔵 **Completeness**: Open Questions section empty — **Resolved** ('None — resolved' marker added).
- 🔵 **Dependency**: Canonicalisation sibling 0064 not referenced — **Partially resolved** (0064 now under Related with rationale; dependency lens this pass suggests it could be promoted to Blocked-by, but that is non-blocking since 0064 is `done`).
- 🔵 **Testability**: 'Does not duplicate' soft check — **Resolved** (AC bullet 4 tightened).
- 🔵 **Testability**: `target` value shape not pinned — **Partially resolved** (pinned for plan and work-item; PR identifier shape still under-specified).
- 🔵 **Testability**: Discovery-pass assumption not made testable — **Partially resolved** (new AC bullet requires a reproducible discovery pass with grep command and matched files; the concrete grep recipe is left to the implementer).
- 🔵 **Clarity**: 'same file' conflates two files — **Resolved** (reworded to 'same artifact pipeline').
- 🔵 **Dependency**: Future visualiser-graph epic not named — **Partially resolved** (now under Blocks; no work-item ID yet — flagged again this pass).
- 🔵 **Dependency**: Template-reading helper not named — **Partially resolved** (named in Requirements, AC, and Technical Notes as `config-read-template.sh`; not listed under Dependencies — flagged again this pass).
- 🔵 **Scope**: Bundled scope (template creation + skill rewiring) — Acknowledged as defensible; no change.

### New Issues Introduced

- 🔵 **Clarity / Testability (minor)**: PR `target` shape lacks a concrete verifiable pattern — `"plan:<id>"` and `"work-item:<id>"` are pinned, but `pr-review`'s "PR identifier (path-form)" lacks an example or regex.
- 🔵 **Clarity (minor)**: Lifecycle handling of review extras (`reviewer`, `verdict`, `review_pass`) is mentioned only in passing; the populated-values AC implies all extras are filled at generation time, but Technical Notes hedges with "present-but-empty" language — the two should align.
- 🔵 **Dependency (minor)**: `config-read-template.sh` helper coupling, although named in Requirements/AC/Technical Notes, is not surfaced in the Dependencies section; concurrent work touching the helper could collide unnoticed.
- 🔵 **Dependency (minor)**: ADR-0033 and ADR-0034 appear only under References, not under Dependencies' "Blocked by"; mirroring 0065's call-out would make the authoritative-source chain explicit.
- 🔵 **Testability (minor)**: The discovery-pass AC requires "a reproducible grep command and matched files" but does not specify the canonical command; reproducibility depends on whatever grep the verifier runs.
- 🔵 **Testability (minor)**: "No unsubstituted template tokens" lacks a defined token marker syntax (e.g., `{{...}}` vs `<...>`); the regex used by the verifier is implementer's choice.
- 🔵 **Testability (minor)**: "Outside fenced template-example blocks" exception is not defined — which fence syntax (` ```yaml `, ` ```md `, etc.) counts is ambiguous, so the grep-based checks may yield false positives/negatives without a defined pre-filter.
- 🔵 **Testability (minor)**: AC bullets 5 and 6 split plan-validation coverage between extras (bullet 5 names its `target`) and `result` (bullet 6); a verifier checking bullet 5 alone might overlook `result`.
- 🔵 **Completeness (minor)**: Story does not explicitly identify the beneficiary (skill maintainers, downstream visualiser-graph epic, corpus-migration effort); inferable but not stated in Context.
- 🔵 **Scope (suggestion)**: Helper-extension carve-out ("extending it is in-scope") is an open-ended scope-expansion clause; if the helper extension turns out to be substantial, the story could grow without re-decomposition.
- 🔵 **Scope (suggestion)**: "Or any additional producer found is folded into scope" mirrors 0065's discovery-pass clause but leaves the unit of work contingent on a future grep result.

### Assessment

The work item is ready for implementation. All three major testability gaps from Pass 1 are closed, and the remaining issues are second-order refinements — most could be addressed during planning rather than blocking the story. The two clearest residual items worth fixing before implementation are: (a) pinning `pr-review`'s `target` value shape with a concrete example so it is symmetrical with the `"plan:<id>"` / `"work-item:<id>"` examples, and (b) aligning the populated-values AC with Technical Notes on whether review extras are filled at creation or emitted present-but-empty. The remaining minor and suggestion findings are recommendations the team can incorporate or defer at its discretion without affecting deliverability.

