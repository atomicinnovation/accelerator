---
date: "2026-06-02T14:03:12+00:00"
type: work-item-review
skill: review-work-item
target: "meta/work/0094-inline-code-styling-in-meta-artifact-markdown.md"
work_item_id: "0094"
review_number: 1
verdict: COMMENT
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 2
status: complete
---

## Work Item Review: Inline Code Styling In Meta Artifact Markdown

**Verdict:** COMMENT

This bug work item is strong across every lens: it is tightly scoped to a single
coherent concern (inline `<code>` styling only, fenced blocks explicitly
excluded), structurally complete with all sections substantively populated, and
unusually testable for a visual defect because each property is pinned to a
concrete token or numeric value backed by a divergence table. No critical or
major issues were found. The findings are all minor or suggestion: a recurring
need to resolve the `--ac-stroke-soft` token question, a few acceptance criteria
that mix objective assertions with subjective phrasing, and small internal
inconsistencies. The work item is acceptable as-is, but addressing the
cross-cutting token question and tightening the criteria would make it
implementation-ready with no ambiguity.

### Cross-Cutting Themes

- **Unconfirmed `--ac-stroke-soft` border token** (flagged by: clarity,
  dependency, testability) — Three lenses independently flagged that the border
  requirement depends on a token (`--ac-stroke-soft`) which the Open Questions
  section admits is unconfirmed live. It is stated as a settled requirement in
  Requirements/AC, is not tracked as a conditional blocker in Dependencies, and
  leaves the border-colour AC without a definite target value. This is the
  single most valuable thing to resolve.
- **Acceptance criteria mix objective and subjective phrasing** (flagged by:
  testability, clarity) — Several ACs pair checkable assertions ("in Fira Code")
  with subjective clauses ("visually distinct", "matching the prototype pill")
  or omit the concrete token/value already named in Technical Notes.

### Findings

#### Critical

None.

#### Major

None.

#### Minor

- 🔵 **Dependency**: Unconfirmed `--ac-stroke-soft` token is a latent blocker not reflected in Dependencies
  **Location**: Open Questions / Dependencies
  The border requirement depends on a token that may not exist live; if absent,
  the work stalls until an equivalent live soft-stroke token is identified, yet
  this conditional prerequisite is not tracked in Dependencies.

- 🔵 **Testability**: First AC pairs a checkable assertion with subjective "visually distinct" phrasing
  **Location**: Acceptance Criteria
  "In Fira Code" is verifiable via computed `font-family`, but "visually
  distinct" has no defined threshold and could be argued either way.

- 🔵 **Testability**: "Sunken background" / "pill" criterion omits the concrete target token/value
  **Location**: Acceptance Criteria
  The second AC states the background and pill qualitatively even though
  Technical Notes names the exact token (`--ac-bg-sunken`); a verifier reading
  only the AC list lacks the concrete targets to assert.

- 🔵 **Testability**: Theme-toggle AC lacks concrete per-theme values that constitute "track correctly"
  **Location**: Acceptance Criteria
  "Track the active theme correctly" is currently a tautology — any token-driven
  styling satisfies it regardless of whether the resolved colours are right.

- 🔵 **Testability**: No-regression AC for fenced blocks lacks an explicit baseline reference
  **Location**: Acceptance Criteria
  "Unchanged from current behaviour" is borderline tautological without a
  concrete baseline (snapshot or named properties) to compare against.

- 🔵 **Clarity**: Border-radius target stated two different ways across sections
  **Location**: Requirements
  Requirements say "rounded corners (~`3px`)" (approximate) while Technical Notes
  and the prototype give an exact `3px`; a reader can't tell if 3px is a hard
  target or a ballpark.

- 🔵 **Clarity**: Token availability claim conflicts with the Open Question about it
  **Location**: Open Questions
  The border is stated as a settled requirement while the underlying token's
  existence is questioned only in a separate section; the dependency isn't
  visible at the point the requirement is stated.

- 🔵 **Completeness**: No explicit reproduction steps for the bug
  **Location**: Context
  The input/action/expected/actual elements are implied across Summary, Context,
  and Technical Notes but never presented as one reproducible scenario.

- 🔵 **Dependency**: Related work items not differentiated into ordering, blocking, or thematic adjacency
  **Location**: Dependencies
  All four siblings are labelled "Related" with differing coupling hints; it is
  left unstated whether any sequencing exists (e.g. token churn from 0076).

#### Suggestions

- 🔵 **Testability**: Unresolved border-token question leaves the border AC's target value undetermined
  **Location**: Open Questions
  Until the token is pinned, the border-colour assertion has no single definite
  expected value, only an as-yet-unnamed equivalent.

- 🔵 **Dependency**: Design prototype as a shared source-of-truth artefact is not tracked as a coupling
  **Location**: References
  All target values derive from `prototype-standalone.html`; whether that
  prototype is frozen or still evolving is not captured, risking silent drift.

- 🔵 **Clarity**: "The live rule" vs "the prototype rule" relies on the reader tracking two selectors
  **Location**: Context
  Bare later references require the reader to keep the two selector identities
  mapped across four sections; short labels or relying on the table would help.

### Strengths

- ✅ Tightly scoped and genuinely atomic — a single CSS rule adjustment, correctly filed as a bug, with fenced blocks explicitly and consistently excluded across every section.
- ✅ Structurally complete: every section is present and substantively populated, and frontmatter (kind=bug, status=draft, priority=medium, tags, IDs) is fully and correctly filled.
- ✅ The Technical Notes divergence table makes each requirement traceable to a specific property delta and doubles as a verification checklist and the bug's "actual outcome".
- ✅ Every property under change is pinned to a concrete value or named token, so verification can assert against computed styles rather than eyeballing.
- ✅ The subject ("inline code spans") is held constant throughout, with no shifting referents, and the reframe from "not rendered" to "mis-styled" is recorded in Drafting Notes.
- ✅ The one unverified prerequisite (`--ac-stroke-soft`) is surfaced transparently as an Open Question rather than silently assumed.

### Recommended Changes

1. **Resolve or formally track the `--ac-stroke-soft` token question** (addresses: "Unconfirmed `--ac-stroke-soft` token is a latent blocker", "Token availability claim conflicts with the Open Question", "Unresolved border-token question leaves the border AC's target value undetermined")
   Confirm whether `--ac-stroke-soft` exists in the live `global.css`. If it does, fold the answer into Requirements/AC/Technical Notes and remove the Open Question. If it doesn't, name the live soft-stroke equivalent (or note a token must be provisioned) and record it as a conditional dependency. This single resolution clears the dominant cross-cutting theme.

2. **Inline concrete target values into the acceptance criteria** (addresses: the three testability AC findings)
   Replace subjective clauses with the objective assertions already present in Technical Notes: computed `font-family` resolves through `--ac-font-mono`; `background-color` resolves to `--ac-bg-sunken`; `border` is `1px solid <soft-stroke token>`; `border-radius` is `3px`. For the theme-toggle AC, specify that dark mode changes the computed `background-color`/`border-color` to the dark variants. For the fenced-block AC, anchor to the `:not(pre code)` mechanism plus a before/after comparison of `font-family`, `font-size`, and `background`.

3. **Make the border-radius target consistent** (addresses: "Border-radius target stated two different ways")
   Commit to `3px` in Requirements to match Technical Notes and the prototype, dropping the `~` tilde — or state the tolerance explicitly if approximation is genuinely acceptable.

4. **Add a short reproduction scenario** (addresses: "No explicit reproduction steps for the bug")
   State as one unit: input = artifact body with an inline `code` span, action = view it in the meta-artifact markdown view, expected = monospace pill matching the prototype, actual = Inter body font at 14px with no border.

5. **Clarify the coupling strength of the related work items** (addresses: "Related work items not differentiated", "Design prototype as a shared source-of-truth artefact")
   Confirm explicitly that 0076/0088/0089/0095 are purely thematic with no ordering constraint (or reclassify any that introduce shared tokens/selectors), and note whether the cited prototype is a frozen snapshot or a living reference.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: This work item is unusually clear: it reframes the defect precisely (styling gap, not parsing failure), names the live and prototype CSS rules explicitly, and backs every requirement with a concrete property-by-property comparison table. Referents are mostly unambiguous because the subject ("inline code") stays constant throughout. The only clarity weaknesses are a couple of internal numeric inconsistencies and one undefined token referenced as if confirmed elsewhere.

**Strengths**:
- The subject ("inline code spans") is held constant across Summary, Context, Requirements, and Acceptance Criteria, so there are no shifting referents.
- The Technical Notes divergence table makes each requirement traceable to a specific property delta, eliminating guesswork about what "correct styling" means.
- The bug is precisely scoped — fenced code blocks are explicitly excluded and the :not(pre code) mechanism is named — so "inline code" never ambiguously bleeds into "all code".
- Drafting Notes record the reframe from "not rendered" to "rendered with incorrect styling", removing a potential reader misinterpretation of the defect.

**Findings**:
- [minor, high] Requirements — Border-radius target stated two different ways across sections: The target border-radius is stated as approximately 3px in Requirements ("~`3px`") and as exactly 3px in Technical Notes (prototype `3px`). The tilde implies an approximate value while the prototype source is exact, leaving a reader unsure whether 3px is a hard target or a ballpark.
- [minor, medium] Open Questions — Token availability claim in Technical Notes conflicts with the Open Question about it: Open Questions asks whether `--ac-stroke-soft` exists live, but Requirements/AC state the soft `1px` border as a settled requirement and Technical Notes lists the token without flagging the uncertainty inline. A reader may treat the border requirement as fully specified and miss the gap.
- [suggestion, low] Context — "The live rule" versus "the prototype rule" relies on the reader tracking two selectors: Later bare phrases require the reader to keep the two selector identities mapped throughout four sections. Optionally label them once and reuse short labels, or rely on the table as the single source of truth.

### Completeness

**Summary**: This bug work item is structurally complete and densely populated: every expected section is present and substantive, and frontmatter is fully and correctly populated. The Context section thoroughly explains the styling divergence and its forces. The one notable gap for a bug kind is the absence of explicit, structured reproduction steps — the information is implied across the prose but not stated as a reproducible scenario.

**Strengths**:
- Frontmatter is complete and correct: kind=bug, status=draft, priority=medium, with tags and IDs populated.
- The Summary states the defect unambiguously as a single phenomenon.
- Context distinguishes a styling gap from a parsing failure and enumerates exactly which properties diverge and why.
- Acceptance Criteria contains five specific, scenario-framed bullets covering the fix and the non-regression case.
- Optional sections (Open Questions, Dependencies, Assumptions, Technical Notes) are all populated with genuinely relevant content.

**Findings**:
- [minor, medium] Context — No explicit reproduction steps for the bug: As a bug, the work item would benefit from explicit reproduction steps stated as a single unit — input (artifact body with inline code), action (rendering in the visualiser), expected (Fira Code pill per prototype), actual (Inter font, no border, 14px). These are scattered implicitly across Summary, Context, and Technical Notes but never presented as one reproducible scenario.

### Dependency

**Summary**: This bug work item is well-scoped to a single CSS styling concern and has an explicit Dependencies section listing four related work items plus a References section. The implied couplings are largely captured: the work depends on theme tokens already confirmed to exist live, with the one unconfirmed token (`--ac-stroke-soft`) honestly surfaced as an Open Question. The main gap is that the unconfirmed token represents a latent blocker whose resolution path is not captured as an explicit dependency, and the related items are labelled "Related" without distinguishing ordering/blocking from thematic adjacency.

**Strengths**:
- Upstream token dependencies are explicitly enumerated and verified live (`--ac-font-mono`, `--ac-bg-sunken`, `--radius-sm`, `--size-xs`, `--sp-1`), so no hidden prerequisite surprises for confirmed tokens.
- The one unverified prerequisite (`--ac-stroke-soft`) is surfaced transparently rather than silently assumed.
- Fenced-code-block scoping is explicitly called out as a no-regress constraint.
- A Dependencies section is present and lists four related work items, giving reviewers visibility into the surrounding cluster.

**Findings**:
- [minor, high] Open Questions — Unconfirmed `--ac-stroke-soft` token is a latent blocker not reflected in Dependencies: If the token is absent, the work cannot satisfy the border requirement until an equivalent is identified or introduced — a genuine upstream prerequisite not named as a conditional blocker. An implementer could stall mid-task or silently substitute an ad-hoc value.
- [minor, medium] Dependencies — Related work items not differentiated into ordering, blocking, or thematic adjacency: All four siblings are "Related" with differing coupling hints; no sequencing is stated. If a sibling introduces/relocates a consumed token, an uncaptured ordering constraint could cause rework.
- [minor, low] References — Design prototype as a shared source-of-truth artefact is not tracked as a coupling: All target values derive from the prototype HTML; whether it is frozen or evolving is not captured, risking silent design drift if it is later revised.

### Scope

**Summary**: This is a tightly scoped, atomic bug work item addressing a single coherent concern: aligning inline-code styling in the meta-artifact markdown renderer with the design prototype. All requirements, acceptance criteria, and the summary describe the same narrow scope, and fenced code blocks are explicitly and consistently excluded across every section. The declared `bug` kind fits well, and the boundaries are crisply stated.

**Strengths**:
- Summary, Requirements, Acceptance Criteria, and Technical Notes all describe the same scope — inline code span styling only — with no drift.
- Scope boundary is explicit and reinforced in multiple sections: fenced blocks deliberately excluded via the `:not(pre code)` scoping.
- The work is genuinely atomic — a single CSS rule adjustment — and correctly filed as a bug rather than inflated or split per property.
- Related work is referenced rather than absorbed, keeping scope from bleeding into adjacent concerns.

**Findings**: None.

### Testability

**Summary**: This bug specification is unusually testable for a visual styling defect: each requirement maps to a concrete token or numeric value and the Technical Notes provide a property-by-property divergence table that doubles as a verification checklist. The main testability gaps are that several acceptance criteria mix objectively-checkable assertions with subjective visual phrasing, and the theme-toggle and "sunken background" criteria omit the concrete target values a verifier would assert against.

**Strengths**:
- Every property under change is pinned to a concrete value or named token, so a verifier can assert against computed styles rather than eyeballing.
- The Technical Notes divergence table enumerates prototype-vs-live values per property, functioning as a precise verification checklist and supplying the "actual broken outcome".
- The no-regression boundary is specified mechanically via the `:not(pre code)` selector and a dedicated AC.
- Bug framing includes the exact trigger and current actual behaviour.

**Findings**:
- [minor, high] Acceptance Criteria — First AC pairs a checkable assertion with subjective "visually distinct" phrasing: "In Fira Code" is verifiable via computed `font-family`, but "visually distinct" has no defined threshold. Drop the clause or replace with the objective assertion already implied.
- [minor, medium] Acceptance Criteria — "Sunken background"/"pill" criterion omits the concrete target token/value: The background and pill are stated qualitatively even though Technical Notes names `--ac-bg-sunken`. Inline the concrete targets into the AC.
- [minor, medium] Acceptance Criteria — Theme-toggle AC lacks the concrete per-theme values that constitute "track correctly": "Track correctly" is a tautology — any token-driven styling satisfies it regardless of whether resolved colours are right. Specify the observable per-theme check.
- [minor, medium] Acceptance Criteria — No-regression AC for fenced blocks lacks an explicit baseline reference: "Unchanged" is borderline tautological without a concrete baseline. Anchor to the `:not(pre code)` mechanism plus a before/after property comparison.
- [suggestion, medium] Open Questions — Unresolved border-token question leaves the border AC's target value undetermined: Until the token is pinned, the border-colour portion cannot be verified against a single definite value. Resolve the question and record the chosen token name.

## Re-Review (Pass 2) — 2026-06-02T14:03:12+00:00

**Verdict:** COMMENT

Re-ran the four lenses that had findings (clarity, completeness, dependency,
testability; scope had none in pass 1). Every pass-1 finding is resolved:
`--ac-stroke-soft` was verified present in the live `global.css` and folded
into the requirement, acceptance criteria, and Technical Notes; the ACs now
assert computed-style values; border-radius is consistently `3px`; a
Reproduction scenario was added; and the related work items are explicitly
classified as non-blocking thematic adjacencies. Completeness and dependency
returned zero findings. Three new minor findings emerged — all small AC
completeness gaps that the tightening exposed, none critical or major.

### Previously Identified Issues
- 🔵 **Dependency**: `--ac-stroke-soft` latent blocker not in Dependencies — Resolved (token confirmed live; open question removed; Drafting Notes records resolution)
- 🔵 **Testability**: AC1 "visually distinct" subjective phrasing — Resolved (now asserts computed `font-family` resolves through `--ac-font-mono`, differing from prose)
- 🔵 **Testability**: "Sunken background"/"pill" omits concrete token — Resolved (AC now names `--ac-bg-sunken`, `1px solid var(--ac-stroke-soft)`, `3px`)
- 🔵 **Testability**: Theme-toggle AC tautological — Resolved (now asserts a light→dark change in `background-color`/`border-color`)
- 🔵 **Testability**: No-regression AC lacks baseline — Resolved (now anchors to `:not(pre code)` retention + before/after property comparison)
- 🔵 **Clarity**: Border-radius stated two ways (`~3px` vs `3px`) — Resolved (consistently `3px`)
- 🔵 **Clarity**: Border requirement vs open-question conflict — Resolved (open question gone; requirement is now settled)
- 🔵 **Completeness**: No explicit reproduction steps — Resolved (Reproduction block added to Context)
- 🔵 **Dependency**: Related items not differentiated by coupling — Resolved (stated as non-blocking thematic adjacencies)
- 🔵 **Testability**: Border-token target undetermined (suggestion) — Resolved (token + values recorded)
- 🔵 **Dependency**: Prototype frozen-vs-living not tracked (suggestion) — Resolved (References note it is a frozen dated snapshot)
- 🔵 **Clarity**: Live-rule/prototype-rule labelling (suggestion) — Not addressed (judged low value; selectors are named on first use and the table is the source of truth)

### New Issues Introduced
- 🔵 **Clarity** (minor): Font-size AC folds the table-cell case (`11px`) into a parenthetical rather than its own Given, leaving the in-table expected value open to two readings.
- 🔵 **Testability** (minor): Dark-mode AC asserts the values *change* but states no resolved dark value for `--ac-bg-sunken`, so a wrong-token regression could still pass.
- 🔵 **Testability** (minor): `1px 5px` padding is in Requirements/Technical Notes but no AC asserts computed `padding`, so it could go unverified at done-time.

### Assessment
The work item is materially stronger than pass 1 — all substantive findings are
resolved and two lenses are now clean. The three new findings are minor AC
completeness gaps (table-cell Given, dark `--ac-bg-sunken` value, padding
assertion) that would take one more small edit to close. The work item is
acceptable for implementation as-is; closing the three would make the
acceptance criteria fully exhaustive.
