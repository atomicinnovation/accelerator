---
date: "2026-05-28T00:00:00+00:00"
type: work-item-review
producer: review-work-item
target: "work-item:0065"
review_number: 1
verdict: COMMENT
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 3
status: complete
id: "0065-update-artifact-templates-to-unified-schema-review-1"
title: "0065-update-artifact-templates-to-unified-schema-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-28T00:00:00+00:00"
last_updated_by: Toby Clemson
---

## Work Item Review: Update All Artifact Templates to Unified Schema

**Verdict:** REVISE

This is a strongly-authored story — well-populated across every section, with unusually precise scope boundaries against its sibling stories (0066, 0067, 0070) and a carefully-defined identity-key convention. The review surfaced no critical issues, but four major findings cluster around one root cause: the story delegates its expected values (the `schema_version` integers, the per-type extras, the in-scope template set, the canonical authority) to external documents without pinning or reproducing them, which leaves several acceptance criteria unverifiable from the document alone. Addressing the schema-authority pinning and enumerating the in-scope template set would resolve most of the findings at once.

### Cross-Cutting Themes

- **Expected values delegated externally, never pinned in-place** (flagged by: clarity, completeness, testability) — The story points at 0060, ADR-0033, and 0057's "Per-artifact extras" as authorities but never reproduces the concrete values (schema_version integers, per-type extras, per-type status vocabularies) nor pins a version. Clarity flags the unresolved 0060-vs-ADR-0033 authority relationship; testability flags that `schema_version` values and hedged per-extras entries can't yield a definitive pass/fail; completeness flags the status vocabulary isn't enumerated. One fix — name a single canonical source and inline the small value tables — neutralises all of these.
- **"Every template" over an unenumerated set** (flagged by: testability, scope) — Criteria are scoped to "every"/"each" template but the in-scope file set is never listed, so completeness can't be conclusively verified and the deliverable size is partly open.

### Findings

#### Major
- 🟡 **Clarity**: Two distinct artifacts both cast as the authoritative schema source (0060 vs ADR-0033)
  **Location**: Summary / Open Questions / Technical Notes
  The work item names 0060 ("decided by 0060", source of `schema_version` values) and ADR-0033 ("schema source-of-truth") as authorities without stating their relationship — a reader can't tell if they're the same decision in two places or two sources, nor which wins on conflict.

- 🟡 **Dependency**: Template-consuming producer skills not named as a coupling
  **Location**: Requirements
  Templates are static scaffolds; the skills that read them must supply values for the new `schema_version`/`last_updated`/`last_updated_by`/provenance fields. The story scopes itself to template files only and never names the consuming creation skills — risking newly-created artifacts inheriting unpopulated values, defeating "born unified."

- 🟡 **Testability**: Scope of "every template under templates/" is not enumerated
  **Location**: Acceptance Criteria
  Criteria 1-6 and 8 are scoped to "every"/"each" template, but the file set is never listed. A verifier can't confirm "every template emits the unified base fields" without an authoritative list — a missed template would still let all criteria be claimed met.

- 🟡 **Testability**: `schema_version` criterion references 0060 values that are neither reproduced nor pinned
  **Location**: Acceptance Criteria
  "Templates include `schema_version` with the value decided in 0060" delegates per-type values to 0060 without reproducing them or pinning a version, so any integer could be argued to satisfy the criterion.

#### Minor
- 🔵 **Clarity**: 0060 and 0061 cited as deciders but never titled in References — listed as bare numbers while 0057 gets a full path.
- 🔵 **Clarity**: "corpus-frontmatter divergence" used without definition in Technical Notes — could read as the authoritative ADR violating its own rule.
- 🔵 **Completeness**: Story does not name the beneficiary whose need is met (the "for whom" a story is expected to identify).
- 🔵 **Completeness**: Status-comment criterion relies on an external, un-enumerated per-type status vocabulary — implementer can't author the comments from this document alone.
- 🔵 **Dependency**: Discovery-pass partitioning shares a producer enumeration with 0066, captured only as "Related" with no note that the partition boundary must be consistent.
- 🔵 **Scope**: Discovery-pass-plus-remediation of arbitrary inline producers is a second, open-shaped concern bundled with the closed template-rewrite task.
- 🔵 **Testability**: Discovery-pass criterion lacks a defined verification procedure for completeness of the enumeration itself.
- 🔵 **Testability**: Per-artifact extras criterion delegates to 0057's list, which has hedged entries ("baseline fields", "where applicable") that don't resolve to a definite field set.
- 🔵 **Testability**: Status-comment criterion verifies presence but not correctness of the documented values.

#### Suggestions
- 🔵 **Clarity**: `id`-is-a-base-field framing is split across three Requirements bullets; could be stated once up front.
- 🔵 **Dependency**: 0060/0064 listed as plain "Blocked by" though the convention is a retroactive correction now living in ADR-0033 — annotate the entries to point at ADR-0033.
- 🔵 **Scope**: Status-comment documentation is tangential to the schema rewrite; fine as natural batching, no action needed.

### Strengths
- ✅ The own-`id` / foreign-`<type>_id` convention is defined precisely on first use and reinforced with concrete examples consistently across all sections.
- ✅ Scope boundaries against sibling work items are unusually precise — 0066 (four named review skills), 0067 (note template), 0070 (corpus migration) each carved out by name.
- ✅ All four upstream blockers (0060, 0061, 0063, 0064) are named with file-level rationale; downstream consumer 0070 is captured as Blocks.
- ✅ Every section a story needs is present and densely populated; Open Questions is explicitly resolved-to-none with rationale rather than left blank.
- ✅ Eight specific acceptance criteria each map to a Requirements bullet; negative criteria (no `git_commit`/`branch`, no `<type>_id` for own identity) are precise, greppable checks.
- ✅ The discovery pass is bounded with a defined null outcome ("record that it was run and found nothing").

### Recommended Changes

1. **Pin a single canonical schema authority and inline the concrete values** (addresses: 0060-vs-ADR-0033 authority, `schema_version` not pinned, per-extras hedged, status vocabulary not enumerated)
   State once that 0060 produced ADR-0033 and ADR-0033 is the source-of-truth, then cite it consistently. Add a small table to the story: artifact type → `schema_version` integer → per-type extras → valid status values. This makes the delegating acceptance criteria verifiable from the document alone.

2. **Enumerate the in-scope template files** (addresses: "every template" unverifiable, open-shaped scope)
   Add a checklist of the specific (non-note) template paths under `templates/` that this story must touch, so "every template" becomes a fixed, fully-coverable set.

3. **Clarify the template-consuming-skill boundary** (addresses: producer skills not named as coupling)
   Either name the creation skills that must populate the new fields as in-scope (with a matching acceptance criterion that produced artifacts carry populated values), or add a Dependencies note stating field-population in consuming skills is covered elsewhere and out of this story's scope.

4. **Note the 0066 partition is a shared boundary** (addresses: discovery-pass coordination with 0066)
   Add a one-line Dependencies note that the inline-producer partition is shared with 0066 and the four-skill cut line is the agreed boundary, so the two discovery passes reconcile to the same set.

5. **Tidy minor clarity items** (addresses: References titles, "corpus-frontmatter divergence", `id` base-field framing)
   Add titles/paths for 0060 and 0061 in References; briefly define "corpus-frontmatter divergence" and that ADR-0033's own frontmatter is 0070's job; state once up front that `id` is always a base field.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is largely internally consistent and uses its key domain terms with care, defining them on first use and reinforcing them with examples. The most significant clarity gap is the unexplained relationship between two artifacts both cast as the authoritative schema source — 0060 (the deciding work item for the base schema and source of `schema_version` values) and ADR-0033 (the "schema source-of-truth"). A reader cannot tell whether these are the same decision recorded in two places or two distinct sources, and the document never references 0060/0061 by title.

**Strengths**:
- The own-identity (`id`) vs foreign-reference (`<type>_id`) convention is defined precisely on first use and reinforced with concrete examples consistently across Summary, Requirements, Acceptance Criteria, and Technical Notes.
- Scope boundaries are drawn unambiguously through explicit exclusion: the four review/validation skills are named in full every time, and the note template's exclusion is stated with its destination (0067).
- The three producer shapes are enumerated explicitly in Technical Notes.

**Findings**:
- 🟡 major (high): **Two distinct artifacts both cast as the authoritative schema source (0060 vs ADR-0033)** — Location: Summary / Open Questions / Technical Notes. The Summary says the schema was "decided by 0060" while Open Questions/Technical Notes call ADR-0033 the "schema source-of-truth"; the relationship is never stated, so an implementer can't tell which is canonical. Suggestion: state the relationship once and refer to a single canonical source consistently.
- 🔵 minor (medium): **0060 and 0061 cited as deciders but never titled in References** — Location: References. They appear only as bare numbers while 0057 gets a full path. Suggestion: add titles and paths.
- 🔵 minor (medium): **"corpus-frontmatter divergence" used without definition** — Location: Technical Notes. The term is introduced without definition and the reasoning (existing files are 0070's job) is compressed, risking a reader mistaking it for a contradiction. Suggestion: define the term and state ADR-0033's own frontmatter is migrated by 0070.
- 🔵 suggestion (low): **`id`-base-field framing split across three bullets** — Location: Requirements. Could be stated once up front that `id` is always a base field so per-extras lists read as strictly additive.

### Completeness

**Summary**: Work item 0065 is a well-populated story with substantive content in every section a story of this kind needs. Frontmatter is complete and valid. The only completeness gaps are minor: the story-kind expectation of identifying the beneficiary is implicit rather than stated, and one acceptance criterion (status-value comments) depends on per-type vocabulary that is referenced but not enumerated in-place.

**Strengths**:
- All expected sections present and substantively populated.
- Eight specific acceptance criteria each correspond to a Requirements bullet.
- Context clearly explains the why and demarcates scope against 0066 and 0067.
- Frontmatter complete and internally consistent.
- Open Questions explicitly resolved-to-none with rationale; Drafting Notes records how each prior question was settled.

**Findings**:
- 🔵 minor (medium): **Story does not name the beneficiary whose need is met** — Location: Summary. The "for whom" a story should identify is left implicit. Suggestion: add a "so that…" clause naming the beneficiary.
- 🔵 minor (low): **Status-comment criterion relies on an external, un-enumerated vocabulary** — Location: Acceptance Criteria. An implementer cannot author the status comments from this document alone. Suggestion: reference or inline the authoritative per-type status vocabulary.

### Dependency

**Summary**: The work item has a thorough Dependencies section naming all four upstream blockers with file-level rationale, the downstream consumer (0070), and the coordinating sibling (0066). The strongest gap is an uncaptured coupling to the skills that consume these templates: changing templates to emit new fields implies the producing skills must supply those values, yet no such dependency is named. A secondary concern is that the discovery-pass partitioning shares a producer enumeration with 0066, only loosely captured as "Related".

**Strengths**:
- All four upstream blockers named with the specific files each touches.
- Downstream consumer 0070 captured as Blocks.
- ADR-0033's authoritative status and the 0060/0064 retroactive-correction nuance spelled out in Technical Notes.
- Scope boundary with 0066 and 0067 stated repeatedly.

**Findings**:
- 🟡 major (medium): **Template-consuming producer skills not named as a coupling** — Location: Requirements. The skill that reads each template must supply values for the new fields; if not updated alongside, new artifacts inherit unpopulated values. Suggestion: name the consuming skills as in-scope (with an acceptance criterion) or add a Dependencies note that field-population is covered elsewhere.
- 🔵 minor (medium): **Discovery-pass partition shared with 0066 only captured as "Related"** — Location: Dependencies. If the two stories disagree on the boundary a producer could be claimed by both or neither. Suggestion: note the four-skill cut line is the agreed boundary.
- 🔵 suggestion (low): **0060/0064 listed as plain "Blocked by" despite retroactive correction** — Location: Technical Notes. The dependency is on the corrected convention (now in ADR-0033), not the as-shipped wording. Suggestion: annotate the entries to point at ADR-0033.

### Scope

**Summary**: Work item 0065 is a well-bounded story that carves out the template-based-producer slice of epic 0057's broader mandate, with explicit boundaries against sibling stories. It does bundle two distinguishable threads — rewriting existing `templates/` files and a discovery-pass-plus-migration of arbitrary non-review inline producers — but the coupling is deliberate, justified by a shared schema goal, and the discovery pass is bounded with a defined null outcome. The declared kind (story) is appropriate.

**Strengths**:
- Boundaries against sibling work items are explicit and unusually precise.
- The discovery pass is bounded with a defined stopping point and null outcome.
- Scope coheres around a single unifying purpose without drift across Summary/Requirements/Acceptance Criteria.

**Findings**:
- 🔵 minor (medium): **Discovery-pass-plus-remediation is a second, open-shaped concern bundled with the template rewrite** — Location: Requirements. A single complex inline producer found could materially expand delivered work. Suggestion: pre-commit to spinning a non-trivial find into a follow-up, or note the size assumption (expected zero/trivial) explicitly.
- 🔵 suggestion (low): **Status-comment documentation is tangential** — Location: Requirements. Orthogonal to the schema rewrite but small and co-located; no action needed if treated as natural batching.

### Testability

**Summary**: The acceptance criteria are mostly verifiable through mechanical inspection of template files against the schema authority, and several correctly point to an external authority. However, the heavy use of "every"/"all" over an unenumerated set of templates, the open-ended discovery-pass criterion, and the criterion checking `schema_version` against per-type values not reproduced here leave verification underspecified.

**Strengths**:
- Most criteria define a concrete, inspectable outcome (presence/absence of named fields, quoting of identity values).
- Criteria appropriately defer canonical field lists to a single authority rather than restating and risking drift.
- Negative criteria ("no longer emit git_commit or branch", "no template uses <type>_id for its own identity") are precise, falsifiable grep checks.
- The discovery-pass criterion includes an explicit empty-result fallback.

**Findings**:
- 🟡 major (high): **Scope of "every template under templates/" is not enumerated** — Location: Acceptance Criteria. A verifier can't confirm "every template" without an authoritative file list. Suggestion: enumerate the in-scope template paths as a checklist.
- 🟡 major (high): **`schema_version` criterion references 0060 values that are neither reproduced nor pinned** — Location: Acceptance Criteria. Any integer could be argued to satisfy it. Suggestion: inline a type → integer table or reference a version-locked section.
- 🔵 minor (medium): **Discovery-pass criterion lacks a defined verification procedure for completeness of enumeration** — Location: Acceptance Criteria. The grep hint is not a pinned check. Suggestion: promote a concrete enumeration procedure (patterns used, files searched) into the criterion.
- 🔵 minor (medium): **Per-artifact extras criterion delegates to a hedged external list** — Location: Acceptance Criteria. 0057's list has entries ("baseline fields", "where applicable") that don't resolve to a definite set. Suggestion: state the exact expected extras per template or note which inherit a judgement set.
- 🔵 minor (medium): **Status-comment criterion verifies presence but not correctness** — Location: Requirements. A wrong/incomplete status list would still pass a presence-only check. Suggestion: reference or inline the canonical per-type status sets.

## Re-Review (Pass 2) — 2026-05-28

**Verdict:** REVISE (all four pass-1 majors resolved; four new majors surfaced, then remediated in a follow-up edit — see Assessment)

### Previously Identified Issues
- 🟡 **Clarity**: 0060 vs ADR-0033 authority unresolved — **Resolved**. Summary now states 0060 produced ADR-0033, which is named the single source-of-truth throughout; References and Schema Reference cite it consistently.
- 🟡 **Dependency**: Template-consuming producer skills not named as a coupling — **Resolved**. A requirement and acceptance criterion now cover the consuming skill supplying the new field values.
- 🟡 **Testability**: "Every template" not enumerated — **Resolved**. Schema Reference table enumerates the in-scope templates by filename and criteria reference them explicitly.
- 🟡 **Testability**: `schema_version` not pinned — **Resolved**. Pinned to `1` for every type per ADR-0033 §Schema versioning.
- 🔵 Minors (References titles, "corpus-frontmatter divergence", beneficiary, `id`-base-field framing, 0066 partition, per-extras hedging) — **Resolved** via the pass-1 edits.

### New Issues Introduced
Pass 2 surfaced four new majors, all introduced or exposed by the pass-1 edits:
- 🔴 **Clarity** (medium): `validation.md` listed as in-scope while `validate-plan` is named an excluded inline skill — the same producer appeared on both sides of the cut line. **Root cause was a real error**: `validation.md` is a body-only report template with no frontmatter, and plan-validation frontmatter is emitted inline by `validate-plan` (0066's). **Remediated**: `validation.md` removed from scope (9 → 8 templates), with an explicit exclusion note.
- 🟡 **Testability** (medium): Two per-type extras cells non-determinate ("baseline fields", "align field-name conventions only"). **Remediated**: the `validation.md`/"baseline fields" row is gone; `design-inventory` extras now enumerate its concrete domain fields.
- 🟡 **Testability** (medium): Status-comment criterion had no source for "verbatim". **Remediated**: criterion reframed to a reproducible diff against the pre-change vocabulary.
- 🟡 **Testability** (high): Discovery-pass "found none" escape clause unfalsifiable. **Remediated**: criterion reframed to require a reproducible grep command + matched-file list, and to assert the discovered set excludes exactly the four named skills.
- 🔵 **Dependency** (medium): 0066 coupling filed as bidirectional but only "Related". **Remediated**: clarified the cut line is statically fixed (four named skills), so no execution ordering/handshake is needed.

### Remaining (minor / by-design, not blocking)
- 🔵 **Scope**: the "absorb any non-review inline producer" clause and folded-in consuming-skill edits keep the story's size partly discovery-dependent — defensible and bounded by the static cut line; left as-is.
- 🔵 **Clarity**: "0061's sibling ADR" is cited by task number, not ADR number (the ADR may not be numbered yet) — forward reference, acceptable.

### Assessment
All four original majors and all four pass-2 majors have been addressed; the most important was a genuine scoping error (`validation.md` has no frontmatter). The work item is now self-contained against ADR-0033 with an enumerated eight-template set and reproducible verification procedures. Remaining items are minor and by-design. The work item is ready for implementation pending a confirmatory pass on the latest edits if desired.

## Re-Review (Pass 3) — 2026-05-28

**Verdict:** COMMENT (no critical or major findings; acceptable as-is)

Scope note: between pass 2 and pass 3 the user corrected the `validation.md` decision — it should *gain* frontmatter in this story rather than be excluded. 0065 was updated back to nine templates (validation.md gains a frontmatter block), and sibling 0066 was rescoped from "rewrite inline frontmatter" to "move frontmatter into templates" (creating the three missing review templates and rewiring all four skills), with a new 0065→0066 ordering dependency. Pass 3 reviewed this updated state across all five lenses.

### Previously Identified Issues (pass-2 majors)
- 🔴 **Clarity**: validation.md double-ownership — **Resolved differently than pass 2**: rather than excluding validation.md, the 0065/0066 split is now drawn by *concern* (0065 owns the template's frontmatter block; 0066 rewires validate-plan to read it), captured reciprocally in both stories.
- 🟡 **Testability** (non-determinate extras, status-comment source, discovery-pass completeness) — **Remain resolved**: extras enumerated, status check anchored to the pre-edit VCS revision, discovery pass reproducible.
- 🔵 **Dependency** (0066 coupling) — **Resolved**: now an explicit bidirectional Blocks/Blocked-by between 0065 and 0066, verified consistent in both files.

### Pass-3 Findings (all minor / suggestion — none blocking)
- 🔵 **Clarity** (minor): consuming-skill criterion swept in validation.md, whose skill 0066 rewires — **remediated** (validation.md explicitly excepted in the requirement and AC9).
- 🔵 **Testability** (minor): "non-placeholder" undefined; status-comment baseline unnamed — **remediated** (AC9 now defines the failure shape; AC8 names the pre-edit VCS revision).
- 🔵 **Dependency** (minor/suggestion): discovery pass may surface an unrecorded coupling; 0061's specific role in classifying `target` could be named — left as optional polish.
- 🔵 **Scope** (minor/suggestion): the open-ended discovery-pass clause and folded-in consuming-skill edits keep size partly elastic — judged a genuinely indivisible coupling, left by design.
- 🔵 **Testability** (suggestion): attach the recorded discovered-producer list to the closing note for audit — optional.
- ✅ **Completeness**: no findings — every section present, populated, frontmatter valid.

### Assessment
The work item is implementation-ready. Three pass-3 nits I introduced or that sharpened verifiability have been remediated in place (validation.md exception, non-placeholder definition, VCS-revision baseline). The residual items are explicitly by-design (elastic discovery scope, indivisible template+skill coupling) or optional polish (recording the discovered-producer list, naming 0061's specific role). No critical or major issues remain across any lens, and the 0065/0066 boundary is consistent in both work items.
