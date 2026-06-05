---
type: work-item-review
id: "0067-create-note-skill-review-1"
title: "Work Item Review: Create `create-note` Skill"
date: "2026-06-05T21:27:02+00:00"
author: "Toby Clemson"
producer: review-work-item
status: complete
target: "work-item:0067"
work_item_id: "0067"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-06-06T08:57:04+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Create `create-note` Skill

**Verdict:** REVISE

This is a strong, densely-populated story — every section is present and substantive, dependencies are reconciled bidirectionally with sibling 0065 and downstream 0070, scope is coherent and well-bounded, and most acceptance criteria name concrete filesystem outcomes grounded in ADR-0033/0034. The REVISE verdict is driven by count, not by any structural defect: five major findings cluster in two areas — a pair of internal/ADR-consistency slips in the linkage prose, and three acceptance criteria whose pass/fail procedure is underdefined (convention-conformance, router-triggering, and an unverified elicitation behaviour). All are addressable with targeted edits; no rework of the underlying design is needed.

### Cross-Cutting Themes

- **Template conformance pinned to the external sibling 0065** (flagged by: clarity, dependency, testability) — Requirements and AC1/AC3 define the template's correctness partly by "matching the conventions applied to the templates in 0065", but 0065 is unlinked in References and its conventions are an external (now-shipped) reference rather than an inline checklist. Three lenses independently flagged this as either ambiguous (clarity), an under-named coupling (dependency), or a non-deterministic verification anchor (testability).
- **Acceptance criteria that defer to undefined external procedures** (flagged by: testability, clarity) — AC6 ("follows existing skill-creation conventions") and AC7 (router triggering) both push the definition of done outside the work item, to an unenumerated convention set or to probabilistic router behaviour, leaving no deterministic check.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Clarity**: `parent` linkage rule stated inconsistently between Requirements and Acceptance Criteria
  **Location**: Requirements / Acceptance Criteria (AC5)
  Requirements say the skill offers `parent` "only when the note is genuinely owned by that artifact" (a property of the note), while AC5 says `parent` is used "when ownership is indicated" (a user signal). It is unclear whether the skill infers ownership, prompts for it, or applies a heuristic — which changes the prompt flow and the linkage written.

- 🟡 **Clarity**: "reserved for the inverse direction" does not match ADR-0034's definitions of `source`/`derived_from`
  **Location**: Requirements
  The parenthetical "(`source` / `derived_from` are reserved for the inverse direction per ADR-0034)" has no clearly named referent — inverse of what edge, pointing from what to what — and ADR-0034 defines `source` as "external/non-meta origin for extracted artifacts" and `derived_from` as the fan-in generative key, neither phrased as an "inverse direction". A reader checking the rationale against the ADR finds it doesn't map onto the ADR's vocabulary.

- 🟡 **Testability**: Convention-conformance criterion (AC6) is unbounded and tautological
  **Location**: Acceptance Criteria (AC6)
  "The skill follows existing skill-creation conventions (see `create-work-item`)" defers the entire definition of done to an unenumerated external skill. A reviewer can always argue some convention is or isn't satisfied, so the criterion yields no definitive pass/fail.

- 🟡 **Testability**: Router-triggering criterion (AC7) specifies probabilistic behaviour without a defined check
  **Location**: Acceptance Criteria (AC7)
  "...named and described in a way that the skill router can trigger it on intent like 'capture a note', 'jot this down'" depends on a probabilistic router with no threshold or enumerated phrase set, so the check rests on subjective judgement rather than a deterministic procedure.

- 🟡 **Testability**: Interactive elicitation behaviour from Requirements has no verifying criterion
  **Location**: Acceptance Criteria
  Requirements state the skill "interactively elicits the note's topic, body content, and any optional tags", but no AC verifies it — every criterion checks the produced file's frontmatter/path/linkage. An implementation that hard-coded or skipped body/tag elicitation could pass every listed criterion.

#### Minor

- 🔵 **Clarity / Testability**: Template conformance anchored to "matching 0065" — an unlinked, external referent
  **Location**: Requirements / Acceptance Criteria (AC1, AC3)
  AC1's inline field list is concrete, but the "matching the conventions applied to the templates in 0065" qualifier ties correctness to a sibling story that is named only by number and not linked in References. Verification then depends on resolving 0065 externally, and is only as stable as that artefact. (Merged: clarity flagged the unresolvable referent; testability flagged the non-deterministic verification anchor.)

- 🔵 **Clarity**: Two competing `producer` attributions stated without a single resolved value
  **Location**: Frontmatter: producer / Drafting Notes
  Frontmatter carries `producer: extract-work-items` while Drafting Notes note the enrichment ran through `create-work-item`, so "which skill produced this" is answerable two ways. This is an intentional policy choice (record extraction origin), so no value change is needed — but the rationale could state plainly that `producer` is not expected to reflect later enrichment passes.

- 🔵 **Testability**: "Discoverable via the skill registry" (AC2) lacks a defined verification procedure
  **Location**: Acceptance Criteria (AC2)
  AC2 pairs a concrete clause (the file exists at the path) with an unspecified one — what "discoverable via the skill registry" means as a procedure is not defined, so a verifier has no stated way to confirm the discoverability half.

#### Suggestions

- 🔵 **Completeness**: Story does not explicitly identify the user/persona whose need is met
  **Location**: Summary / Context
  The work item describes what the skill does but never frames the served user in user-story terms (e.g. "as a plugin author, I want to capture a note so that…"). Low-risk given the plugin context, but the "for whom" a story carries is left implicit.

- 🔵 **Dependency**: 0065 template-convention dependency captured as "Related" rather than a precedence coupling
  **Location**: Dependencies
  The note template must mirror 0065's already-shipped conventions, but 0065 appears only under "Related". An implementer scanning Dependencies may not realise the template must conform to a shipped precedent.

- 🔵 **Dependency**: notes path-config prerequisite recorded as an assumption rather than a named coupling
  **Location**: Assumptions
  The skill's deterministic output-path naming depends on the `notes` → `meta/notes` config wiring, captured as an ambient assumption. The assumption reads as already-confirmed, so this is informational — surface it in Dependencies only if the wiring isn't verified.

- 🔵 **Scope**: Two deliverables (template + skill) bundled in one story — deliberate, no change needed
  **Location**: Requirements
  The story ships both `templates/note.md` and the `create-note` skill. The two are tightly coupled (the skill is the template's sole consumer) and the work item explicitly defends shipping them together. Recorded only to confirm the bundling was a deliberate scoping decision.

### Strengths

- ✅ Every structurally expected section for a story is present and substantively populated, with former open questions explicitly resolved in Assumptions and Technical Notes — no dangling unknowns.
- ✅ Actors are consistently named ("the skill" acts; "the user" supplies input) and outcomes are stated as observable system states (a file at a precise path, populated frontmatter keys, omitted empty slots) rather than vague properties.
- ✅ Dependency capture is unusually thorough and accurate: upstream blockers (0060, 0061) named with rationale and verified `done`, downstream consumer (0070) recorded under Blocks, and the 0065 handoff reconciled in both directions.
- ✅ Scope is coherent and crisply bounded — `list-notes`/`show-note`, existing-note migration, and a sequential allocator are each explicitly excluded — and the `story` sizing is reasoned.
- ✅ The strongest acceptance criteria (AC1 field enumeration, AC4 filename pattern, AC5 linkage ref shape) give a verifier concrete, deterministic checks grounded in ADR-0033/0034.

### Recommended Changes

1. **Align the `parent`-vs-`relates_to` selection rule across Requirements and AC5** (addresses: "`parent` linkage rule stated inconsistently")
   Pick one phrasing that names *who* determines ownership and the concrete trigger — e.g. "the skill writes `parent` only when the user confirms the related artifact owns the note; otherwise `relates_to`" — and use it in both sections.

2. **Restate the `source`/`derived_from` exclusion in ADR-0034's own terms** (addresses: "'reserved for the inverse direction'")
   Replace "reserved for the inverse direction" with the actual relationship: for a note, *other* artifacts point to it via `source`/`derived_from` (extraction origin), so a note does not write those keys itself.

3. **Make AC6 enumerate the conventions it requires** (addresses: "Convention-conformance criterion is unbounded")
   Replace the holistic "follows existing skill-creation conventions" with a checkable list, e.g. "SKILL.md carries `allowed-tools` frontmatter; uses a conversational elicitation flow; derives the output path deterministically."

4. **Pin AC7 to a deterministic artefact** (addresses: "Router-triggering criterion is probabilistic")
   Anchor the check to something verifiable — e.g. "the name/description contain the keywords 'note' and 'capture'", or an enumerated list of test phrases that must each route to the skill.

5. **Add an acceptance criterion for interactive elicitation** (addresses: "Interactive elicitation behaviour has no verifying criterion")
   e.g. "Running the skill prompts for topic, body content, and optional tags, and the resulting note's body and `tags` frontmatter reflect the values the user supplied."

6. **Promote the inline AC1 field list to the authoritative checklist; demote "matching 0065" to a non-normative note and link 0065** (addresses: "Template conformance anchored to 'matching 0065'", "0065 captured as Related")
   Treat the enumerated fields as the pass/fail source of truth, add a resolvable reference to 0065, and note in the 0065 Dependencies entry that it is the (shipped) convention precedent.

7. **Optional polish** (addresses: AC2 discoverability, producer attribution, persona framing)
   Define the AC2 discoverability check concretely; add a one-line note that `producer` records extraction origin by policy; optionally add a one-line user-persona framing to the Summary.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is generally clear, with well-named actors (the skill, the user) and concrete, observable outcomes throughout Requirements and Acceptance Criteria. The main clarity weaknesses are an internally inconsistent characterisation of the `parent` linkage rule between Requirements and Acceptance Criteria, an ambiguous "reserved for the inverse direction" justification that does not match how ADR-0034 actually defines `source`/`derived_from`, and a few specialist terms that lean heavily on the linked ADRs.

**Strengths**:
- Actors are consistently named: "the skill" performs writing/eliciting/linking actions and "the user" supplies topic/body/tags.
- Outcomes are stated as observable system states (a file at a precise path, populated frontmatter keys, omitted empty slots), not vague desired properties.
- Specialist terms ("unified base schema", "provenance bundle", "typed-linkage slots", "omit-when-empty") are each anchored to ADR-0033 or ADR-0034, which are linked in References.
- Assumptions and Technical Notes explicitly record which open questions were resolved and how.

**Findings**:
- 🟡 major (high) — Requirements / AC: `parent` linkage rule stated inconsistently — "genuinely owned by" (a property of the note) vs "ownership is indicated" (a user signal); unclear whether the skill infers, prompts, or applies a heuristic.
- 🟡 major (high) — Requirements: "reserved for the inverse direction" has no named referent and does not match ADR-0034's definitions of `source` (external/non-meta origin) and `derived_from` (fan-in generative key).
- 🔵 minor (medium) — Requirements: "the same field shapes and conventions as the templates updated in 0065" relies on an external, unlinked referent (0065 not pathed in References).
- 🔵 minor (medium) — Frontmatter: producer: two competing producer attributions (`extract-work-items` in frontmatter vs `create-work-item` ran the enrichment); intentional, so no value change needed, but could be stated plainly.

### Completeness

**Summary**: Work item 0067 is a thoroughly populated story with substantive Summary, Context, Requirements, seven Acceptance Criteria, Dependencies, Assumptions, Technical Notes, and Drafting Notes — all internally consistent with its source epic (0057) and the two referenced ADRs. Every section a story needs is present and densely filled, with former open questions explicitly resolved. The only completeness gap is a minor one: the story never explicitly names the served user/persona.

**Strengths**:
- All structurally expected sections for a story are present and substantively populated.
- Summary is a single, unambiguous action statement naming exactly what is built and what category it serves.
- Context explains motivation rather than restating the summary.
- Seven specific acceptance criteria define "done" across template, discoverability, output, filename, linkage, conventions, and routing.
- Former open questions are each explicitly resolved and recorded in Assumptions and Technical Notes.
- Requirements are grounded in the referenced ADRs, giving an implementer concrete field shapes.

**Findings**:
- 🔵 suggestion (medium) — Acceptance Criteria: story does not explicitly identify the user whose need is met (no "as a [persona], I want…" framing).

### Dependency

**Summary**: The work item has unusually thorough and accurate dependency capture: upstream blockers (0060, 0061) named with rationale, the downstream consumer (0070) recorded under Blocks, and the template/skill handoff from sibling 0065 reconciled in both directions. All four dependency claims verify against the referenced documents — 0060 and 0061 are `done`, and 0065 independently confirms the note-template move. The only gaps are soft upstream couplings (the `notes` path config and 0065's template conventions) treated as ambient rather than named.

**Strengths**:
- Upstream blockers named with the specific capability each provides; both verify as `done`, so the story is genuinely startable.
- Downstream consumer (0070) captured under Blocks with a concrete reason.
- The 0065 coupling is reconciled bidirectionally and confirmed in 0065's own Drafting Notes.
- ADR couplings (0033, 0034) named in References and threaded through Requirements and AC.

**Findings**:
- 🔵 suggestion (medium) — Dependencies: 0065 template-convention dependency captured as "Related" rather than a precedence coupling (and 0065 is `done`, so the reference is stable).
- 🔵 suggestion (medium) — Assumptions: notes path-config prerequisite recorded as an ambient assumption rather than a named upstream coupling; reads as already-confirmed.

### Scope

**Summary**: Work item 0067 describes a coherent, well-bounded story: ship the `create-note` skill together with the single `templates/note.md` template it consumes. The two deliverables are tightly coupled, the boundaries are explicit and well-defended (list/show, existing-note migration, and a sequential allocator are all called out as out of scope), and the story sits correctly as a child of epic 0057. Sizing fits the declared `story` kind.

**Strengths**:
- Tight coupling justified: the template and its sole consumer ship together, with an explicit rationale for moving the template out of 0065.
- Boundaries stated crisply: `list-notes`/`show-note`, existing-note migration (deferred to 0070), and a sequential allocator are all explicitly excluded.
- Correctly positioned within epic 0057, which names a `create-note` skill and the `note` artifact extras in its decomposition.
- Sizing fits `story` and the choice is reasoned in Drafting Notes.

**Findings**:
- 🔵 suggestion (medium) — Requirements: two deliverables (template + skill) bundled in one story; deliberate and correct (the skill is the template's sole consumer), recorded only to confirm the coupling was intentional. No change needed.

### Testability

**Summary**: For a story this work item is unusually well-grounded — most Acceptance Criteria name concrete, observable filesystem outcomes that a verifier could check deterministically. The weakest criteria are the convention-conformance and router-triggering ones, which lean on undefined external checklists or probabilistic behaviour, and one Requirements behaviour (interactive elicitation) has no corresponding criterion.

**Strengths**:
- AC1 enumerates the exact frontmatter the template must emit, giving a field-by-field checklist grounded in ADR-0033/0034.
- AC4 specifies the output path pattern precisely, including date source and slug derivation.
- AC5 defines the linkage outcome with a concrete ref shape, the relates_to-vs-parent rule, and omit-when-empty.

**Findings**:
- 🟡 major (high) — Acceptance Criteria (AC6): convention-conformance criterion is unbounded and tautological; defers the definition of done to an unenumerated external skill.
- 🟡 major (high) — Acceptance Criteria (AC7): router-triggering criterion specifies probabilistic behaviour without a defined threshold or enumerated phrase set.
- 🟡 major (medium) — Acceptance Criteria: interactive elicitation behaviour from Requirements has no verifying criterion; an implementation that skipped elicitation could pass every listed AC.
- 🔵 minor (medium) — Acceptance Criteria (AC2): "discoverable via the skill registry" lacks a defined verification procedure.
- 🔵 minor (medium) — Acceptance Criteria (AC1/AC3): template conformance pinned to a moving sibling ("matching 0065") rather than enumerated values.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-06-05

**Verdict:** COMMENT

Re-ran clarity, completeness, dependency, and testability after the edits (scope skipped — its sole finding was "no change needed"). **All five major findings from Pass 1 are resolved or downgraded to minor; zero major or critical findings remain**, so the verdict moves from REVISE to COMMENT. The work item is ready for implementation; the residual items are optional polish.

### Previously Identified Issues

- 🟡 **Clarity**: `parent` linkage rule inconsistent between Requirements and AC5 — **Resolved**. Both sections now state `relates_to` by default, `parent` only on confirmed user ownership; clarity flagged the linkage direction as a strength ("internally consistent with ADR-0034").
- 🟡 **Clarity**: "reserved for the inverse direction" mismatch with ADR-0034 — **Resolved**. Restated in ADR-0034's own terms (the *other* artifact writes `source`/`derived_from` back to the note); confirmed consistent with ADR-0034's type-pair table.
- 🟡 **Testability**: AC6 convention-conformance unbounded/tautological — **Resolved**. Now enumerates `allowed-tools` frontmatter, conversational flow, deterministic path; testability cited the anti-tautology clauses as a strength.
- 🟡 **Testability**: AC7 router-triggering probabilistic — **Partially resolved**. Now AC8 with required keywords + three named test phrases; the keyword half is mechanically checkable, but testability still flags the routing-threshold half as underspecified (downgraded major → minor).
- 🟡 **Testability**: interactive elicitation had no verifying criterion — **Resolved**. New elicitation AC added with an explicit fail condition.
- 🔵 **Clarity**: "matching 0065" unlinked referent — **Resolved**. Inline field list designated authoritative; 0065 demoted to a non-normative cross-check.
- 🔵 **Clarity**: two competing `producer` attributions — **Resolved**. Drafting Notes now state `producer` records the originating skill by policy.
- 🔵 **Testability**: AC2 discoverability had no procedure — **Partially resolved**. Now requires well-formed `name`/`description` + appears in registry listing; testability notes the enumeration *surface* is still unnamed (remains a low-confidence minor).
- 🔵 **Completeness**: no persona framing — **Resolved**. Summary now names the intended user (plugin author/contributor); completeness returned zero findings.
- 🔵 **Dependency**: 0065 captured as "Related" not a precedence coupling — **Resolved**. Now marked the shipped convention precedent; dependency cited it as a strength.
- 🔵 **Dependency**: notes path-config as an ambient assumption — **Still present** (deliberately not changed; reads as already-confirmed). Dependency re-flagged it as an optional minor.

### New Issues Introduced

- 🔵 **Clarity** (minor): the rewritten linkage bullet now packs four rules into one dense sentence — recoverable but forces re-reading; suggest splitting note-emits vs other-artifact-emits into two bullets.
- 🔵 **Testability** (minor): the explicit "a note never writes `source`/`derived_from`" statement added to Requirements has no corresponding AC — suggest extending AC to assert non-emission.
- 🔵 **Dependency** (minor): the `create-work-item` convention coupling and the extraction-origin counterpart (other skills writing `source`/`derived_from` back to notes) are named in prose but not surfaced in Dependencies.

### Assessment

The work item is now in good shape and ready to move toward planning. No blocking (major/critical) issues remain. The residual minors are all optional polish — the most worthwhile being a one-line AC asserting `source`/`derived_from` are never emitted (closing the loop on the negative requirement) and splitting the dense linkage bullet for readability. None need to block promotion from `draft`.

## Re-Review (Pass 3) — 2026-06-06

**Verdict:** APPROVE

All residual minors from Pass 2 have since been applied to the work item: the dense linkage bullet was split into emit/non-emit bullets; an acceptance criterion asserting `source`/`derived_from` non-emission was added; the AC8 routing criterion was split into a mechanical keyword check plus a routing clause with a stated dispatch threshold; AC2 now names the enumeration surface; and the `create-work-item` convention coupling, the `notes` path-config prerequisite, and the extraction-origin counterpart are surfaced in Dependencies. No open findings remain across any lens.

The work item is approved and has been transitioned `draft → ready`. It is ready to move into planning.
