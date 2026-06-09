---
type: work-item-review
id: "0080-detail-page-header-actions-review-1"
title: "Work Item Review: Detail-Page Header Actions (Open in Editor, Copy Link)"
date: "2026-06-09T15:27:59+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0080"
work_item_id: "0080"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-06-09T18:17:34+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Detail-Page Header Actions (Open in Editor, Copy Link)

**Verdict:** REVISE

The work item is, on its content, unusually strong for a frontend story — named
actors, concrete observable outcomes, a richly populated set of sections, and a
clearly documented decision history. However, it is currently **blocked by an
unresolved version-control merge conflict in the frontmatter** (verified on
disk): a migration to the unified frontmatter shape has collided with a `tags`
edit, leaving the YAML unparseable. Beyond that hard blocker, the strongest
recurring concerns are an ambiguous dependency relationship to work item 0100,
a scope question about bundling two independently-deliverable actions, and
several acceptance criteria that assert outcomes no defined procedure can
verify.

### Cross-Cutting Themes

- **Unresolved frontmatter merge conflict** (flagged by: clarity, completeness,
  dependency, scope) — jj conflict markers (lines 9-22) wrap two competing
  frontmatter shapes (unified-shape migration with `type`/`schema_version`/
  `blocked_by`/`source` vs. a `config`-tag addition). The block is invalid YAML;
  any tooling that parses it will fail or choose arbitrarily. This is the single
  must-fix item.
- **0100's role is ambiguous — precedent or prerequisite?** (flagged by:
  clarity, dependency) — Context, Requirements, Technical Notes, and Drafting
  Notes all describe building on 0100's `visualiser:` config block and env-var
  precedence, yet 0100 is listed only as "Related", not a blocker. A reader
  cannot tell whether 0100's config infrastructure must land first or is merely
  a pattern to re-implement.
- **Acceptance criteria assert unverifiable or under-specified outcomes**
  (flagged by: testability) — the tooltip criterion does not specify required
  content; the VS Code criterion asserts "triggers the OS protocol handler"
  which the work item itself says yields no observable signal; the custom-
  template criterion does not pin down the substitution/encoding rule.
- **Two independent actions bundled with asymmetric weight** (flagged by:
  scope) — "Copy link" is a small frontend-only action; "Open in editor"
  requires standing up an editor-deep-link config subsystem. They share only
  the `Page.actions` slot.

### Findings

#### Critical

- 🔴 **Clarity / Completeness / Dependency / Scope**: Unresolved merge conflict in frontmatter
  **Location**: Frontmatter (lines 9-22)
  The frontmatter contains an unresolved jj merge conflict (`<<<<<<<`,
  `+++++++`, `%%%%%%%`, `>>>>>>>`) wrapping divergent `tags` values and a
  unified-shape migration (`type: work-item`, `schema_version`, `blocked_by`,
  `source`). The YAML cannot be parsed; `tags`, `blocked_by`, `parent`, and
  `source` are all indeterminate until resolved. Verified present on disk.

#### Major

- 🟡 **Dependency**: 0100 config-precedence dependency classified as "Related" but treated as a foundation
  **Location**: Dependencies
  The story introduces new config fields (`visualiser.editor`,
  `visualiser.editor_project`, `ACCELERATOR_VISUALISER_EDITOR`) that plug into
  0100's resolution system, yet 0100 sits under "Related", not `blocked_by`. If
  0100's `visualiser:` block / precedence resolver is a genuine prerequisite,
  scheduling this story first surfaces a hidden blocker; if only a pattern,
  that should be stated.

- 🟡 **Clarity**: 0100 cross-reference inconsistent in role across sections
  **Location**: Summary / Context / Dependencies
  Summary/Context/Drafting Notes treat 0100's config model as a binding
  precedent, while Dependencies lists it only as "Related". State 0100's role
  once, consistently.

- 🟡 **Scope**: Two independently deliverable actions bundled with asymmetric implementation weight
  **Location**: Requirements
  "Copy link" (frontend-only, gated on 0039) and "Open in editor" (an editor
  deep-link subsystem: 14 presets, custom-template parsing, JetBrains project
  resolution, three-layer precedence) share only the `Page.actions` slot.
  Coupling blocks the cheap ready-to-ship action behind the expensive one.

- 🟡 **Testability**: Tooltip criterion asserts content without specifying it
  **Location**: Acceptance Criteria
  "tooltip states what configuration is required" never specifies the expected
  text or which config keys must be named, so no substring assertion can give a
  definitive pass/fail.

- 🟡 **Testability**: "Triggers the OS protocol handler" is not verifiable
  **Location**: Acceptance Criteria
  The VS Code criterion ends with "clicking it triggers the OS protocol
  handler", but Assumptions states the browser gives no success/failure signal.
  The clause mixes a verifiable part (href form) with an unverifiable one.

- 🟡 **Testability**: Custom-template criterion under-specifies the verbatim/substitution rule
  **Location**: Acceptance Criteria
  The criterion does not define the preset-vs-custom recognition rule nor
  whether `{abs}`/`{rel}` are percent-encoded on substitution (unlike the VS
  Code criterion, which mandates encoding). Two implementers could produce
  different both-arguably-passing outputs.

#### Minor

- 🔵 **Completeness**: Source-path provenance asserted in Assumptions but not specified as a requirement
  **Location**: Requirements / Assumptions
  "Open in editor" depends on the document's absolute/workspace-relative source
  path being available to the route, captured only as an Assumption. Unclear
  whether plumbing that path is in scope or a prerequisite.

- 🔵 **Dependency**: Work item 0035 referenced with no stated coupling
  **Location**: References
  0035 appears only in "Related: 0039, 0041, 0035, 0100" and nowhere in the
  body. Every other referenced item has its coupling explained; 0035 is an
  orphan reference.

- 🔵 **Clarity**: Editor-family list and preset list use overlapping but unreconciled groupings
  **Location**: Requirements
  Family overview lists JetBrains by product name (IntelliJ, WebStorm); the
  config bullet lists them by preset key (`idea`, `web-storm`). The reader must
  infer the IntelliJ→`idea` mapping.

- 🔵 **Scope**: Editor configuration subsystem is a separable concern from the header-action feature
  **Location**: Requirements
  The dominant body of work in the "Open in editor" half is a reusable config
  capability conceptually distinct from "render a button in the header".

- 🔵 **Scope**: Source design-gap cautions against collapsing related drifts into one item
  **Location**: References / Source design-gap
  The source DetailHeaderActions entry scopes only button-wiring; this story
  has expanded into a config-subsystem build the source did not call for.

- 🔵 **Testability**: Preset lists tested by example, not exhaustively bounded
  **Location**: Acceptance Criteria
  Criteria say "a VS Code-family editor" / "a JetBrains preset" but don't say
  whether every preset's scheme/tag mapping must be verified or just one per
  family.

- 🔵 **Testability**: Styling criterion lacks a defined pass/fail procedure
  **Location**: Acceptance Criteria
  "follow the existing `TopbarIconButton` styling precedent" is subjective
  unless tied to a defined check (e.g. reuses the component; computed colours
  resolve to `--ac-*` tokens).

- 🔵 **Testability**: Copy-link fallback path has no acceptance criterion
  **Location**: Acceptance Criteria
  The `document.execCommand('copy')` fallback is specified in Requirements but
  no criterion exercises it; it could ship broken without failing any criterion.

#### Suggestions

- 🔵 **Clarity**: "without being guaranteed scope" phrasing is unclear
  **Location**: Requirements / Drafting Notes
  Replace with explicit wording such as "without being officially supported
  presets in this story".

### Strengths

- ✅ Actors and triggers are named throughout — the visualiser user, the
  route/server, and the browser are each identified, so responsibility is never
  obscured by passive voice.
- ✅ Outcomes are stated as concrete, inspectable states: each acceptance
  criterion specifies the exact anchor `href` form rather than a vague "opens
  the editor".
- ✅ Every expected section for a story is present and substantively populated,
  with a user-story Summary, motivating Context, and decision history in
  Drafting Notes.
- ✅ The config-precedence ordering (env > personal > project) and the JetBrains
  project-name fallback (explicit override → workspace basename) are specified
  deterministically.
- ✅ The two hard UI blockers (0041 Page.actions, 0039 Toaster) are consistently
  captured across the body and annotated inline at the requirements that consume
  them.
- ✅ Scope boundaries are explicitly stated ("Other detail routes are out of
  scope"), with the narrowing recorded in Drafting Notes.

### Recommended Changes

1. **Resolve the frontmatter merge conflict** (addresses: Unresolved merge
   conflict in frontmatter) — Take the unified-shape side (`id`, `type:
   work-item`, `kind`, `schema_version`, `last_updated`, `blocked_by`, `source`)
   and merge in the `config` tag, producing `tags: [design, frontend,
   detail-page, config]`. Remove all conflict markers and confirm the YAML
   parses. This unblocks everything else.

2. **State 0100's role once, consistently** (addresses: 0100 classified as
   "Related" but treated as foundation; 0100 cross-reference inconsistent) —
   Decide whether this story depends on 0100's config infrastructure (promote to
   `blocked_by`) or merely follows its pattern with independently-built plumbing
   (keep "Related" and add a one-line note that no 0100 code is consumed). Align
   Summary, Context, and Dependencies.

3. **Tighten the unverifiable acceptance criteria** (addresses: tooltip content
   unspecified; OS-handler clause unverifiable; custom-template rule
   under-specified) — Name the config key the tooltip must mention; drop the
   "triggers the OS protocol handler" clause (keep the href assertion); add a
   concrete custom-template example pinning the substitution/encoding behaviour.

4. **Decide the scope split** (addresses: two actions bundled; config subsystem
   separable; source-gap caution) — Either split into "Copy link" and "Open in
   editor" stories (and/or extract the editor-config subsystem into its own
   item), or add an explicit note justifying why the config subsystem is pulled
   into this story rather than tracked separately.

5. **Close the remaining minor gaps** (addresses: source-path provenance; 0035
   orphan reference; fallback criterion; styling/preset testability) — Clarify
   whether source-path plumbing is in scope; explain or remove 0035; add a
   fallback acceptance criterion; reframe the styling criterion as a defined
   check.

## Per-Lens Results

### Clarity

**Summary**: Generally well-written with unambiguous referents, consistent actor
identification, and clear concrete outcomes. The most serious problem is the
unresolved frontmatter merge conflict; a secondary mismatch exists in how 0100's
title and dependency role are stated across sections.

**Strengths**:
- Actors and triggers named throughout (the visualiser user, the route/server,
  the browser).
- Outcomes stated as concrete observable states (exact anchor href forms).
- The custom-template-vs-preset disambiguation rule is stated explicitly.
- The `{abs}`/`{rel}` placeholder vocabulary is defined inline where introduced.

**Findings**:
- 🔴 critical (high) — Frontmatter: tags — Unresolved merge conflict leaves
  frontmatter with two interpretations. Conflict markers wrap divergent `tags`
  and frontmatter shapes; no single authoritative metadata.
- 🟡 major (medium) — Summary — 0100 cross-reference inconsistent in title and
  role across sections (binding precedent vs. "Related").
- 🔵 minor (medium) — Requirements — Editor-family list (product names) and
  preset list (config keys) are overlapping but unreconciled.
- 🔵 minor (low) — Requirements — "without being guaranteed scope" is not
  self-explanatory; reword.

### Completeness

**Summary**: A well-developed story with substantive, kind-appropriate content
in every expected section. The single serious defect is the unresolved
frontmatter merge conflict breaking frontmatter integrity. Otherwise the item
gives a reader nearly everything needed to act.

**Strengths**:
- Every expected section present and substantively populated.
- Summary is a clear user-story sentence with user, actions, and integration
  point.
- Context explains motivation and grounds the config approach in 0100.
- Eight specific acceptance criteria covering both buttons, all editor families,
  templates, precedence, and the disabled state.
- Drafting Notes captures resolved decisions.

**Findings**:
- 🔴 critical (high) — Frontmatter: tags/parent — Unresolved merge conflict;
  `tags`, `parent`, `blocked_by`, `source` indeterminate; tooling will choke.
- 🔵 minor (medium) — Requirements/Assumptions — Source-path provenance asserted
  in Assumptions but not specified as a requirement or surfaced as a dependency.

### Dependency

**Summary**: The two hard UI blockers (0041, 0039) are captured well across
frontmatter and the Dependencies section. The principal gap is 0100: the body
describes building on its config machinery, yet it is classified only as
"Related". 0035 is listed with no explanation, and the merge conflict corrupts
the `blocked_by` field.

**Strengths**:
- 0041 and 0039 consistently captured across `blocked_by`, Dependencies, and
  Requirements.
- Each requirement consuming a dependency is annotated inline ("consumes 0039").
- External protocol-handler couplings named, with best-effort nature in
  Assumptions.
- `Blocks: none` correctly stated.

**Findings**:
- 🟡 major (high) — Dependencies — 0100 config-precedence dependency classified
  "Related" but treated as a foundation the story builds on.
- 🔵 minor (medium) — References — 0035 referenced with no stated coupling
  (orphan reference).
- 🔵 minor (high) — Frontmatter: blocked_by — Merge conflict corrupts the
  structured `blocked_by` source of truth.

### Scope

**Summary**: Anchored on a single UI slot, giving surface coherence, but it
bundles two functionally independent actions with wildly asymmetric
implementation weight. The editor-config subsystem is the dominant cost and a
separable concern, making the "story" arguably over-scoped.

**Strengths**:
- Scope narrowing to LibraryDocView is explicitly recorded and restated.
- Both actions share a real coherence anchor (the Page.actions slot).
- The bundling traces to the source design-gap's own grouping.
- Previously-open config-format and JetBrains-project questions are resolved.

**Findings**:
- 🟡 major (medium) — Requirements — Two independently deliverable actions
  bundled, with asymmetric implementation weight.
- 🔵 minor (medium) — Requirements — Editor configuration subsystem is a
  separable concern from the header-action feature.
- 🔵 minor (medium) — References/Source design-gap — Source cautions against
  collapsing related drifts; story expanded beyond the source paragraph.
- 🔵 minor (low) — Frontmatter: tags — Merge conflict's divergent `config` tag
  corroborates the cross-domain (frontend + config) straddle.

### Testability

**Summary**: Acceptance criteria are unusually strong — mostly observable
Given/When/Then pairs with concrete outputs. The main gaps are a tooltip
criterion that asserts content without specifying it, an OS-handler assertion no
check can confirm, and an under-specified custom-template substitution rule.

**Strengths**:
- Most criteria pair a precondition with an exact, inspectable href output.
- The config-precedence criterion names the exact ordering.
- The JetBrains project-name fallback is fully specified.
- The disabled-state criterion ties an unconfigured precondition to an
  observable outcome.

**Findings**:
- 🟡 major (high) — Acceptance Criteria — Tooltip criterion asserts content
  without specifying it.
- 🟡 major (high) — Acceptance Criteria — "Triggers the OS protocol handler" is
  not verifiable by a defined procedure.
- 🟡 major (medium) — Acceptance Criteria — Custom-template criterion
  under-specifies the verbatim/substitution and encoding rule.
- 🔵 minor (medium) — Acceptance Criteria — Preset lists tested by example, not
  exhaustively bounded.
- 🔵 minor (medium) — Acceptance Criteria — Styling criterion lacks a defined
  pass/fail procedure.
- 🔵 minor (medium) — Acceptance Criteria — Copy-link `execCommand` fallback path
  has no acceptance criterion.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-06-09

**Verdict:** COMMENT

All critical and major findings from pass 1 are resolved or have been
downgraded to acceptable. The fresh pass found no critical or major issues;
the remaining items are minor polish and optional precision improvements. The
work item is acceptable for implementation as-is.

### Previously Identified Issues

- 🔴 **Clarity/Completeness/Dependency/Scope**: Unresolved merge conflict in
  frontmatter — **Resolved**. Conflict resolved, workspace un-stale'd;
  frontmatter is clean unified shape with the `config` tag preserved. No
  conflict markers remain.
- 🟡 **Dependency**: 0100 classified "Related" but treated as a foundation —
  **Resolved**. Dependency lens now praises the rigour: 0100 is explicitly a
  sibling, not a prerequisite, with reasoning in Context and Drafting Notes.
- 🟡 **Clarity**: 0100 cross-reference inconsistent across sections —
  **Resolved**. Role stated consistently in Summary, Context, and Dependencies.
- 🟡 **Scope**: Two independent actions bundled with asymmetric weight —
  **Resolved** (downgraded to acceptable minor). Bundling is a documented,
  defensible decision aligned with the source design-gap; scope lens states
  "no change required" with an extraction path recorded.
- 🟡 **Testability**: Tooltip criterion asserts content without specifying it —
  **Partially resolved**. AC now names the `visualiser.editor` field /
  `ACCELERATOR_VISUALISER_EDITOR` env var; a minor residual remains (pin the
  exact substring to assert).
- 🟡 **Testability**: "Triggers the OS protocol handler" not verifiable —
  **Resolved**. Clause dropped; AC now asserts only the verifiable `href`.
- 🟡 **Testability**: Custom-template substitution under-specified —
  **Resolved**. AC now states the detection rule, percent-encoding, and a
  worked example.
- 🔵 **Completeness**: Source-path provenance only an Assumption — **Resolved**.
  Now an explicit in-scope Requirement.
- 🔵 **Dependency**: 0035 orphan reference — **Resolved**. Explained in
  Dependencies (TopbarIconButton styling precedent).
- 🔵 **Testability**: Copy-link fallback path had no AC — **Resolved**. AC
  added.
- 🔵 **Testability**: Preset lists tested by example, not bounded — **Resolved**.
  Table-driven preset AC added.
- 🔵 **Testability**: Styling criterion subjective — **Partially resolved**.
  Reframed as renders-via-`TopbarIconButton` + computed `--ac-*` colours; minor
  residual (enumerate which properties).
- 🔵 **Clarity**: Family-vs-preset list mismatch — **Resolved**. JetBrains
  presets now annotated with product names.
- 🔵 **Clarity**: "guaranteed scope" phrasing — **Resolved**. Reworded.

### New Issues Introduced

None. No regressions were introduced by the edits. The fresh pass surfaced
several deeper minor observations (not present as findings in pass 1):

- 🔵 **Clarity**: Template placeholders (`{abs}`, `{rel}`, `{scheme}`, `{tag}`,
  `{project}`) used in Requirements before being defined together — a
  consolidated glossary at first use would help.
- 🔵 **Clarity**: Custom-template detection rule stated in three places with
  slightly different wording; state it once as an ordered rule (preset-key
  match first, else custom).
- 🔵 **Dependency**: The pre-existing `visualiser:` config plumbing is relied on
  but not captured as a Dependencies entry (the work item/ADR that shipped
  `visualiser.binary`).
- 🔵 **Dependency**: Whether the route/server already emits `{abs}`/`{rel}` is
  unconfirmed — possible server-side prerequisite framed as in-scope.
- 🔵 **Testability**: Tooltip exact substring not pinned; AC10 colour properties
  not enumerated; no AC for preset-vs-custom disambiguation; JetBrains
  `{rel}` encoding lacks a worked example.

### Assessment

The work item is ready for implementation. The verdict moves from **REVISE** to
**COMMENT**: nothing blocks planning, and the remaining minor observations are
optional precision improvements that can be folded in during `/create-plan` or a
light refinement pass if desired.

### Verdict update — 2026-06-09

All pass-2 minor observations were subsequently folded into the work item
(consolidated placeholder glossary and a single ordered preset-vs-custom
detection rule; terminology standardised; pre-existing config plumbing captured
as a tracked dependency and the server-side path-exposure scope sharpened; ACs
tightened with an exact tooltip substring, enumerated `--ac-*` colour
properties, a preset-vs-custom disambiguation criterion, a worked JetBrains
`{rel}` example, and an anchor-element criterion). With those addressed, the
verdict is raised to **APPROVE**.
