---
date: "2026-05-24T00:00:00Z"
type: work-item-review
skill: review-work-item
target: "meta/work/0074-per-doc-type-hues-on-detail-page.md"
work_item_id: "0074"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 3
status: complete
---

## Work Item Review: Per-Doc-Type Hues on Detail Page

**Verdict:** REVISE

The work item is structurally complete, well-scoped, and clearly motivated, with strong alignment between Summary, Requirements, and Acceptance Criteria, and explicit carve-outs to sibling work items. However, two major testability issues block implementation readiness: AC #2 defers the aside related-row tint surface to implementation (no fixed pass/fail boundary), and AC #3's "visual regression check" is underspecified (no named routes, fixtures, or comparison technique). Several minor clarity, dependency, and testability gaps cluster around the same unresolved decisions — coordination with 0079, the 13-vs-12 hue-map mismatch, and the undefined "virtual" doc-type concept — and resolving these tightens the work item.

### Cross-Cutting Themes

- **Aside related-row surface and 0079 coordination unresolved** (flagged by: scope, dependency, testability) — The aside-row tint is one of two in-scope surfaces, but the choice of tinted element is deferred and the sequencing relative to 0079 (which redesigns the same aside region) is captured only as an open question. Both threaten deterministic verification and scheduling.
- **AC #3 "visual regression check" lacks a verification procedure** (flagged by: clarity, testability) — Both lenses noted that the criterion's actor, mechanism, and target surfaces are unspecified, leaving the no-regression guard non-verifiable.
- **13-vs-12 hue-map mismatch and "virtual" doc-type key concept unresolved** (flagged by: clarity, testability) — The terminology is introduced without a canonical definition or count framing, and the unresolved fallback behaviour blocks tester enumeration of the route matrix.

### Findings

#### Critical
*(none)*

#### Major
- 🟡 **Testability**: Aside related-row criterion defers verification surface to implementation
  **Location**: Acceptance Criteria (#2)
  AC #2 says aside related-row entries 'carry a per-doc-type tint' but leaves icon/border/background to implementation. Without a fixed surface, a verifier cannot deterministically check the criterion — any of those choices, or none, could be claimed compliant.

- 🟡 **Testability**: 'Visual regression check' is not a specific verification procedure
  **Location**: Acceptance Criteria (#3)
  AC #3 requires sidebar and library hub rendering to be 'unchanged (visual regression check)' but does not name the routes, snapshot fixtures, or comparison technique. 'Unchanged' is ambiguous across multiple sidebar/library views per doc-type.

#### Minor
- 🔵 **Clarity**: 'Virtual' doc-type key concept is undefined
  **Location**: Open Questions / Assumptions
  Open Questions refers to "any 'virtual' doc-type keys" and Assumptions distinguishes "non-virtual `DocTypeKey` values", but 'virtual' is never defined or linked to a definition.

- 🔵 **Clarity**: 13-vs-12 hue-map mismatch lacks a single canonical referent
  **Location**: Assumptions / Drafting Notes
  Three phrasings ("thirteen-entry HSL map", "twelve non-virtual values plus one currently unmapped key", "13-vs-12 mismatch") describe the same gap with subtly different framings. A reader cannot tell which side is over-counted.

- 🔵 **Clarity**: Passive 'visual regression check' obscures the actor
  **Location**: Acceptance Criteria (third bullet)
  The criterion does not name who performs the check or what establishes the baseline (automated diff vs manual reviewer vs CSS inspection).

- 🔵 **Clarity**: 'aside related rows' introduced before the aside surface is named
  **Location**: Summary
  The Summary mentions 'aside related rows' parenthetically before introducing the aside region, leaving a reader unfamiliar with the visualiser unable to resolve the term on first read.

- 🔵 **Dependency**: 0041 (Library Page Wrapper) is referenced as the source eyebrow pattern but not listed as a dependency
  **Location**: Dependencies
  Technical Notes states the page eyebrow pattern is established by 0041 and should be mirrored, but 0041 does not appear in Dependencies or References — the coupling is invisible at planning time.

- 🔵 **Dependency**: 0079 ordering constraint is captured as an open question rather than a sequencing dependency
  **Location**: Open Questions
  The 0079 ordering decision is surfaced as a question rather than a dependency declaration; depending on resolution, one of the two stories needs a Blocked-by entry to prevent merge collision on the aside related-row surface.

- 🔵 **Testability**: Doc-type coverage scope not enumerated in AC #1
  **Location**: Acceptance Criteria (#1)
  AC #1 does not specify which doc types must be covered, and the 13-vs-12 mismatch plus the virtual-key fallback question leave testers unable to enumerate the route matrix or confirm fallback behaviour.

- 🔵 **Testability**: Eyebrow tint target (icon, label, or both) not specified
  **Location**: Acceptance Criteria (#1)
  AC #1 says the eyebrow 'uses the `--ac-doc-<key>` token' but Open Question 2 acknowledges the target element (icon vs label vs both) is undecided. As written, two visually distinct implementations could both pass.

#### Suggestions
- 🔵 **Scope**: Coordination with 0079 left as an open question rather than a sequencing decision
  **Location**: Open Questions / Dependencies
  If 0079 lands first or in parallel with a different aside structure, the aside-row half of this story may need to be redone. Either resolve sequencing or split the aside-row tint into a follow-up.

- 🔵 **Scope**: Visual regression check for unchanged surfaces is an implicit second deliverable
  **Location**: Acceptance Criteria
  AC #3 adds a verification surface (sidebar + library hub) beyond the change surface (detail page). Minor — flagging only because verification scope exceeds change scope.

### Strengths
- ✅ Token identifiers (`--ac-doc-<key>`, `--ac-doc-bg-<key>`) and file paths (`src/styles/global.css:98-124`) are stated unambiguously and used consistently across sections.
- ✅ Scope boundaries are restated identically in Summary, Requirements, and Drafting Notes — hero illustration / BigGlyph tint owned by 0082, eyebrow sizing owned by 0075 — eliminating contradiction and preventing scope creep.
- ✅ Summary, Requirements, and Acceptance Criteria are tightly aligned on the same two surfaces (eyebrow, aside related rows).
- ✅ User story names a concrete actor ('viewer of a project document') and outcome ('identify the document's type at a glance').
- ✅ Dependencies section is populated with concrete work item ids (0037 Blocked by, 0082 Blocks, 0073/0075/0079 Related), making upstream/downstream chains visible at planning time.
- ✅ Open Questions and Assumptions sections capture concrete unresolved decisions rather than being left empty.
- ✅ Frontmatter is complete and well-formed; story-kind content (user, motivation, acceptance criteria) is appropriately substantive.

### Recommended Changes

1. **Fix AC #2 — pin or parameterise the aside related-row tint surface** (addresses: Aside related-row criterion defers verification surface to implementation; Coordination with 0079)
   Either resolve Open Question 1 and name the concrete surface (e.g., "left-border accent uses `--ac-doc-<key>` and background uses `--ac-doc-bg-<key>`"), or restate AC #2 as "at least one of {icon, border, background} on each aside related row is set from `--ac-doc-<key>`/`--ac-doc-bg-<key>` matching the linked doc's type" so a tester can enumerate the chosen surface.

2. **Fix AC #3 — name the verification procedure** (addresses: 'Visual regression check' is not a specific verification procedure; Passive 'visual regression check' obscures the actor)
   Restate as e.g. "For each of the 12 non-virtual doc-type keys, the existing Playwright screenshot tests for sidebar and library hub still pass with zero pixel diff" — or whatever the actual regression harness is — so the criterion names the test surface, actor, and tolerance explicitly.

3. **Resolve the 0079 sequencing decision** (addresses: 0079 ordering constraint is captured as an open question; Coordination with 0079 left as an open question)
   Promote the 0079 ordering decision to a pre-start blocker — either add 0079 to "Blocked by" or capture an explicit sequencing note in Dependencies — so the schedule cannot be set until ordering is resolved. Alternatively, split the aside-row tint into a follow-up that lands after 0079.

4. **Define 'virtual doc-type key' and reconcile the 13-vs-12 count** (addresses: 'Virtual' doc-type key concept is undefined; 13-vs-12 hue-map mismatch lacks a single canonical referent; Doc-type coverage scope not enumerated in AC #1)
   In Context (or a glossary footnote in Assumptions), define 'virtual doc-type key' on first use and state once whether the prototype has one entry the app lacks or vice versa — naming the specific key if known. Then add an explicit coverage list (or count) to AC #1, plus a sub-criterion for unmapped/virtual fallback once Open Question 3 is resolved.

5. **Pin the eyebrow tint target element(s)** (addresses: Eyebrow tint target not specified)
   Resolve Open Question 2 and name the target in AC #1, e.g. "the eyebrow icon and label both use `color: var(--ac-doc-<key>)`".

6. **Add 0041 to Dependencies** (addresses: 0041 referenced as source eyebrow pattern but not listed as a dependency)
   Add 0041 to Dependencies — as 'Blocked by' if 0041 must merge first, or 'Related' if it is already done and merely informs the pattern — so the planner sees the eyebrow-pattern lineage at sprint-planning time.

7. **Clarify aside terminology in Summary** (addresses: 'aside related rows' introduced before the aside surface is named)
   In Summary or early in Context, briefly identify the aside region (e.g., "the detail page's right-hand aside region listing related documents") so the term resolves without forward references.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item communicates its intent with high precision: token names, file references, and scope boundaries are explicit, and the actor/outcome pattern in Acceptance Criteria is consistent. There are a few minor ambiguities — most notably the unexplained 'virtual' doc-type key concept, an unresolved 13-vs-12 hue-map count that surfaces in multiple sections without a clear referent, and a passive AC phrasing that hides the actor for the regression check.

**Strengths**:
- Token identifiers and file paths are stated unambiguously and used consistently.
- Scope boundaries are restated identically across Summary, Requirements, and Drafting Notes.
- The user story names a concrete actor and a concrete outcome.

**Findings**:
- 🔵 minor / high — **'Virtual' doc-type key concept is undefined** (Open Questions / Assumptions): 'Virtual doc-type keys' is used in Open Questions and Assumptions but never defined or linked. Implementers may interpret the term differently, leading to divergent fallback behaviour. Suggestion: define on first use, or link to where `DocTypeKey` enumerates virtual vs non-virtual entries.
- 🔵 minor / high — **13-vs-12 hue-map mismatch lacks a single canonical referent** (Assumptions / Drafting Notes): Three phrasings describe the same gap with subtly different framings. A reader cannot tell which side is over-counted. Suggestion: state once, in Context or Assumptions, whether the prototype has one entry the app lacks or vice versa — naming the specific key if known.
- 🔵 minor / medium — **Passive 'visual regression check' obscures the actor** (Acceptance Criteria third bullet): The criterion does not name who performs the check or what establishes the baseline. Suggestion: name the actor/mechanism (e.g., Playwright visual regression suite or manual review against pre-change screenshots).
- 🔵 minor / medium — **'aside related rows' introduced before the aside surface is named** (Summary): The term 'aside' is used parenthetically before being introduced. Suggestion: briefly identify the aside region so the term resolves without forward references.

### Completeness

**Summary**: The story is structurally complete: all expected sections are populated with substantive content, and frontmatter fields are present and recognised. Kind-appropriate story content is present — user/viewer identified, motivation explained, and three specific acceptance criteria define done. No critical completeness gaps were identified.

**Strengths**:
- Summary identifies user, capability, and motivation.
- Context explains current state and prototype reference point.
- Acceptance Criteria contains three specific bullets covering both surfaces and the non-regression check.
- Open Questions and Assumptions are populated with concrete unresolved decisions.
- Frontmatter is complete and well-formed.

**Findings**: *(none)*

### Dependency

**Summary**: The work item captures the most important couplings cleanly: 0037 as the upstream blocker delivering the tokens to be consumed, 0082 as a downstream consumer that is explicitly blocked, and 0079/0073/0075 named as related work. Two gaps worth flagging: 0041 (Library Page Wrapper) is named in Technical Notes as the source of the eyebrow pattern but is not listed in Dependencies, and the ordering question with 0079 is surfaced only as an Open Question rather than captured as an explicit sequencing constraint.

**Strengths**:
- Blocked-by / Blocks / Related are all populated with concrete work item ids.
- The downstream consumer 0082 is explicitly captured with rationale.
- 0073 is surfaced as Related with an accurate description of its role.
- 0075 (eyebrow sizing) is signposted as tracked separately, avoiding scope creep.

**Findings**:
- 🔵 minor / high — **0041 referenced as source eyebrow pattern but not listed as a dependency** (Dependencies): Technical Notes references 0041 as establishing the eyebrow pattern, but 0041 does not appear in Dependencies or References. Suggestion: add 0041 as Blocked by or Related depending on its status.
- 🔵 minor / high — **0079 ordering constraint is captured as an open question rather than a sequencing dependency** (Open Questions): The 0079 ordering decision should be a pre-start blocker or explicit sequencing note in Dependencies, not just an open question, to prevent scheduling collision on the aside related-row surface.

### Scope

**Summary**: Work item 0074 is a well-scoped, atomic story describing a single coherent unit of work: surfacing existing per-doc-type tokens on the detail page. The scope is explicitly bounded (hero illustration carved out to 0082, hero sizing to 0075, tokens themselves delivered by 0037), and the Summary, Requirements, and Acceptance Criteria are aligned on the same two surfaces. Sizing is appropriate for a story kind.

**Strengths**:
- Summary, Requirements, and Acceptance Criteria are tightly aligned on the same two surfaces.
- Out-of-scope items are explicitly carved out and assigned to sibling work items.
- Single bounded context: one frontend page in one app.
- Story kind matches the scope.

**Findings**:
- 🔵 suggestion / medium — **Coordination with 0079 left as an open question rather than a sequencing decision** (Open Questions / Dependencies): If 0079 lands first or in parallel with a different aside structure, the aside-row half of this story may need to be redone. Suggestion: resolve sequencing or split the aside-row tint into a follow-up after 0079.
- 🔵 suggestion / low — **Visual regression check for unchanged surfaces is an implicit second deliverable** (Acceptance Criteria): The no-regression guard adds verification surface (sidebar + library hub) beyond the change surface (detail page). Minor — flagging only because verification scope exceeds change scope.

### Testability

**Summary**: The acceptance criteria are mostly testable — eyebrow tint and sidebar/library-hub non-regression are observable behaviours. However, the aside related-row criterion intentionally defers the verification surface to implementation, and the third criterion uses underspecified 'visual regression check' wording without naming routes or comparison method. Doc-type coverage scope is also left implicit, which weakens deterministic verification.

**Strengths**:
- AC #1 names the specific token and surface tying the verification to an observable CSS value.
- AC #3 explicitly calls out a no-regression check on sidebar and library hub.
- Open Questions section acknowledges that the aside-row tint surface is unsettled.
- Out-of-scope items are explicitly excluded, narrowing the verification surface.

**Findings**:
- 🟡 major / high — **Aside related-row criterion defers verification surface to implementation** (AC #2): Without a fixed surface, a verifier cannot deterministically check the criterion. Suggestion: either pin the surface or restate as 'at least one of {icon, border, background} is set from `--ac-doc-<key>`/`--ac-doc-bg-<key>`'.
- 🟡 major / high — **'Visual regression check' is not a specific verification procedure** (AC #3): The criterion does not name routes, snapshot fixtures, or comparison technique. Suggestion: restate to name the test surface (e.g., 'For each of the 12 non-virtual doc-type keys, the existing Playwright screenshot tests for sidebar and library hub still pass with zero pixel diff').
- 🔵 minor / high — **Doc-type coverage scope not enumerated in AC #1** (AC #1): AC #1 does not specify which doc types must be covered, and the 13-vs-12 mismatch plus virtual-key fallback question leave testers unable to enumerate the route matrix. Suggestion: add an explicit list/count plus a fallback sub-criterion.
- 🔵 minor / medium — **Eyebrow tint target (icon, label, or both) not specified** (AC #1): As written, applying the token to any single eyebrow element would satisfy the criterion. Suggestion: resolve Open Question 2 and name the target element(s).

## Re-Review (Pass 2) — 2026-05-24

**Verdict:** REVISE

Lenses re-run: clarity, dependency, scope, testability (completeness skipped — pass-1 had no findings).

### Previously Identified Issues

- ✅ **Testability**: Aside related-row criterion defers verification surface (AC #2) — **Resolved** (pinned to row icon only).
- 🟡 **Testability**: 'Visual regression check' is not a specific verification procedure (AC #3) — **Partially resolved** (named Playwright pattern; baseline strategy and selector scope still vague — see new findings).
- ✅ **Clarity**: 'Virtual' doc-type key concept is undefined — **Resolved** (defined in Context).
- ✅ **Clarity**: 13-vs-12 hue-map mismatch framing — **Resolved** (reconciled in Context).
- ✅ **Clarity**: Passive 'visual regression check' obscures the actor — **Resolved**.
- ✅ **Clarity**: 'aside related rows' introduced before aside surface is named — **Resolved**.
- ✅ **Dependency**: 0041 referenced but not listed as a dependency — **Resolved** (added to Related).
- ✅ **Dependency**: 0079 ordering captured as open question — **Resolved** (promoted to Blocks).
- 🟡 **Testability**: Doc-type coverage scope not enumerated in AC #1 — **Partially resolved** (enumerated as count; specific routes/fixtures still implicit — see new findings).
- ✅ **Testability**: Eyebrow tint target not specified — **Resolved** (icon only).
- ✅ **Scope**: Coordination with 0079 left as open question — **Resolved** (now Blocks).
- 🔵 **Scope**: Visual regression check as implicit second deliverable — **Still present** (now a suggestion only).

### New Issues Introduced

- 🟡 **Testability/Clarity**: `templates` fallback wording 'or equivalent neutral' (introduced in AC #1/#2 during pass-1 edits) admits multiple passing colours — self-inflicted regression.
- 🟡 **Testability**: Detail-page route/fixture per doc type not specified — verifiers must guess which fixtures exercise each of the 13 keys.
- 🔵 **Testability**: Eyebrow icon and row icon selectors not named (no test handles or DOM identifiers).
- 🔵 **Testability**: AC #2 'background and borders unchanged' lacks captured baseline.
- 🔵 **Testability**: AC #3 spec scope 'elements that previously consumed `--ac-doc-<key>`' is unbounded — requires reverse-engineering.
- 🔵 **Testability**: Eyebrow label 'existing neutral text colour' not named as a specific token.
- 🔵 **Clarity**: AC #3 baseline strategy (hard-coded values vs captured run vs current tokens) not stated.
- 🔵 **Dependency**: 0075 (eyebrow sizing) sequencing relative to this story not stated.
- 🔵 **Dependency**: 0073 (brand-layer palette) value-stability assumption not stated.
- 🔵 **Clarity**: 'Eyebrow' jargon used in Summary before being anchored.
- 🔵 **Clarity**: Prototype 13-entry hue map only partially listed (relationship to `templates` not stated explicitly).

### Assessment

Pass-1's two major findings are resolved structurally, and the work item is materially tighter. However, the "or equivalent neutral" hedge introduced during pass-1 edits is a new major regression, and a second major (per-doc-type fixture/route enumeration) was surfaced. Verdict remains REVISE. The remaining new findings cluster on testability precision (selectors, baselines, fixture routes) and dependency-stability notes (0075, 0073).

Pass-3 edits (applied immediately after this re-review, before a further pass would be run) addressed all of the above: neutral fallback pinned to `var(--ac-text-muted)` exactly; fixture coverage gaps enumerated against `server/tests/fixtures/meta/`; component file paths and slots named (`[data-slot="eyebrow"]`, `RelatedArtifacts.tsx`); AC #2 and AC #3 baselines made explicit (computed values captured at story start from current `--ac-doc-*` declarations); 0075 and 0073 independence/stability notes added to Dependencies. A follow-up re-review is recommended to confirm.

## Re-Review (Pass 3) — 2026-05-24

**Verdict:** APPROVE

Lenses re-run: clarity, dependency, scope, testability.

### Previously Identified Issues

- ✅ **Testability**: 'or equivalent neutral' fallback — **Resolved** (pinned to `var(--ac-text-muted)` exactly).
- ✅ **Testability**: Per-doc-type fixture route not specified — **Resolved** (fixture path enumerated; missing fixtures named in Technical Notes).
- ✅ **Testability**: Eyebrow/row icon selectors not named — **Resolved** (`[data-slot="eyebrow"]`, `RelatedArtifacts.tsx` referenced).
- ✅ **Testability**: AC #2 'background/borders unchanged' baseline — **Resolved** (asserted as identical computed values pre/post).
- ✅ **Testability**: Eyebrow label colour token — **Resolved** (computed `color` identity assertion).
- ✅ **Testability**: AC #3 spec scope unbounded — **Resolved** (`TypeGlyph` and `StageTile` named explicitly).
- ✅ **Testability**: Eyebrow label colour token not named — **Resolved**.
- ✅ **Clarity**: AC #3 baseline strategy — **Resolved** (literal RGB constants from current declarations, not live stylesheet).
- ✅ **Clarity**: 'Eyebrow' jargon — **Resolved** (Summary anchors to 0041).
- ✅ **Clarity**: Prototype 13-entry hue map / `templates` relationship — **Resolved**.
- ✅ **Dependency**: 0075 sequencing — **Resolved** (independence stated).
- ✅ **Dependency**: 0073 stability — **Resolved** (baseline-at-story-start strategy).

### New Issues Identified at Pass 3 (all minor or suggestions)

- 🔵 **Testability**: AC #1/#2 used `color: var(--ac-doc-<key>)` form, which would not match a browser's resolved RGB computed-style serialisation — pinpointing a likely implementation-time bug.
- 🔵 **Testability**: AC #2 lacked the same enumeration clause as AC #1.
- 🔵 **Testability**: AC #1 verification mechanism (automated vs manual) not stated.
- 🔵 **Clarity**: AC #3 baseline phrasing could be read as "compute from live CSS" vs "embed literal constants".
- 🔵 **Dependency**: Fixture-creation prerequisite captured in Technical Notes but not in Dependencies.
- 🔵 (6 suggestions): acronyms undefined; Summary/Context duplication; passive "in scope" phrasing; e2e reference specs as pattern dependency; prototype source placement; AC #3 framing as non-regression.

### Pass-3 Edits Applied

All five minors addressed in a single structural rewrite of Acceptance Criteria:

1. **AC #1/#2/#3 assertion form harmonised** — all three criteria now assert computed `color` equals literal RGB strings captured from `--ac-doc-*` at story start, eliminating the `var()`-vs-resolved-RGB mismatch.
2. **AC #2 coverage enumeration** — now requires at least one related-row fixture per non-virtual type plus one for the virtual `templates` key.
3. **AC #1 verification mechanism** — preamble to Acceptance Criteria states all three are verified by a single Playwright e2e spec.
4. **AC #3 baseline phrasing** — preamble explicitly states the spec "embeds literal RGB string constants captured … at story start; it does not read those values from the live stylesheet at test time".
5. **Fixtures as in-scope prerequisite** — new Dependencies entry "In-scope prerequisites" names the three missing fixture types and the potential `RelatedArtifacts` icon introduction.

### Assessment

Verdict closed at **APPROVE**. The work item is implementation-ready. The remaining six suggestions (acronym expansions, Summary/Context paragraph deduplication, passive phrasing in Requirements, e2e reference specs as a Related dependency, prototype-source placement, AC #3 framing) are stylistic polish that an attentive implementer or planner can address opportunistically.
