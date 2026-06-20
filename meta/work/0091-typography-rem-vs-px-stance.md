---
id: "0091"
title: "Typography rem-vs-px stance review"
date: "2026-05-23T16:30:00+00:00"
author: Toby Clemson
kind: spike
status: ready
priority: low
tags: [design, frontend, tokens, typography, accessibility]
type: work-item
schema_version: 1
last_updated: "2026-06-12T22:08:58+00:00"
last_updated_by: Toby Clemson
relates_to: ["adr:ADR-0036", "work-item:0033", "work-item:0075", "work-item:0090", "work-item:0099", "plan:2026-05-23-0075-typography-size-scale-consumption"]
external_id: PP-113
---

# 0091: Typography rem-vs-px stance review

**Kind**: Spike
**Status**: Ready
**Priority**: Low
**Author**: Toby Clemson

## Summary

Spike to resolve ADR-0036's px-anchored stance for `--size-*` tokens.
The 0075 migration anchors every typography token to px for token-value
determinism, trading away user-controllable default-font-size scaling.
Investigate whether that accessibility trade-off matters for the tool's
software-developer audience and decide whether the tokens should stay
px-anchored, move to rem (heading tiers or family-wide), or adopt a
hybrid that keeps px determinism while recovering user-preference
scaling. This spike *decides* — it does not migrate: its outputs are a
research artefact, a recorded decision (an ADR), and, when a change from
the px status quo is chosen, one or more follow-on work items to
implement it.

## Context

Created alongside the 0075 migration as the durable tracker for the
accessibility trade-off ADR-0036 documents as a known consequence.
Specifically:

- The `MarkdownRenderer` H1 migration (Phase 2 of 0075) changed
  `font-size: 1.75rem` to `var(--size-h3)` (`28px`). At the default
  browser font-size this is computed-identical, but a user who
  customises their browser default font-size loses font-size scaling
  for the H1.
- The full `--size-*` family (post-0075) is px-anchored, so the same
  trade-off applies to every consumer.
- The loss is specific and bounded: px font-size still honours
  browser/page **zoom**, so it satisfies WCAG 1.4.4 Resize Text (AA).
  What px ignores is the browser/OS **default-font-size preference**,
  which only rescales relative units (`rem`/`em`/`%`). Research puts
  the cohort that sets a custom default font-size at ~3% of users
  (Internet Archive data), skewed toward users with visual
  impairments. So this is a best-practice / user-preference question,
  not a strict WCAG conformance failure.

ADR-0036 §Decision documents the px-anchored stance as deliberate, and
§Consequences flags the accessibility trade-off as the principal known
cost. This work item exists so the trade-off is not "shipped and
forgotten".

## Requirements

An exit-criteria-bounded investigation whose job is to reduce
uncertainty and record a decision, not to implement it. Implementation
is handed off to follow-on work item(s) — see *Outputs and exit
criteria*.

### Research questions

- **Q1 — Audience impact.** How likely is the tool's software-developer
  audience to rely on the browser/OS default-font-size preference
  (which px ignores) rather than zoom (which px honours)? Our user base
  is not yet large enough to instrument directly, so estimate from the
  target population: the general-population ~3% custom-default-font-size
  figure (Internet Archive data, per Evan Minto's analysis — confirm the
  underlying dataset and its limits during the research pass) as a
  floor; developer
  behavioural signals (power users skew toward zoom and customise at the
  IDE/OS level); and low-vision / accessibility survey data (e.g.
  WebAIM) for the disabled-developer cohort. Expect a reasoned estimate
  with its sources' limits stated, not a precise figure.
- **Q2 — Option space.** Which stance best balances accessibility and
  token-value determinism: keep px-anchored; rem for heading tiers only
  (`--size-h*` rem, `--size-body` and below px); rem family-wide; or a
  **hybrid** (px-authored tokens emitted as rem, and/or a
  `html { font-size: max(1em, 16px) }` / `clamp()` root anchor, and/or a
  USWDS-style `respect-user-font-size` switch) that keeps px determinism
  while recovering preference-scaling?
- **Q3 — Conformance & cost.** Does the leading candidate satisfy WCAG
  1.4.4 (200% via zoom) and 1.4.10 (reflow at 320 CSS px) without
  regression, keep consumers token-only (0033 CI gate), and preserve
  byte-identical computed sizes at the default font-size? What is its
  implementation cost (per Technical Notes, a unit change is
  declaration-only)?
- **Q4 — ADR coordination with 0099.** 0099 (pure-numeric rename)
  already supersedes ADR-0036 on the *naming* axis. Confirm this spike's
  decision chains off 0099's successor ADR on the *unit* axis rather
  than forking a second direct successor to ADR-0036, and that 0099's
  ADR carries the px-anchoring stance forward as still-open.

### Time-box

- No hard deadline; completion is defined by the exit criteria below.
- Soft effort guideline: ~1–2 days for the Q1–Q4 research pass. This is a
  prompt to reconsider the question's value if the research overruns, not
  a hard stop, and reflects the low priority and the bounded (~3%-cohort)
  trade-off. The exit criteria remain the authoritative definition of
  done.

### Outputs and exit criteria

The spike is complete when:

1. Q1–Q4 are answered in a research artefact under `meta/research/`,
   with each source's limits cited; and
2. a stance is chosen and recorded — either a new ADR (superseding/
   amending the then-current typography ADR, coordinated with 0099 so
   the ADRs chain rather than fork) or, if deferred, a child work item
   capturing the explicit reasoning and what new signal would unblock
   it; and
3. if a change from the px status quo is decided, **one or more
   follow-on work items are raised to implement it** (e.g. the
   token-definition unit change, a root `max()`/`clamp()` anchor,
   build-time rem emission), linked from this spike. If "keep px" is
   decided, that is stated explicitly and no implementation item is
   needed.

## Acceptance Criteria

- [ ] **AC1.** Research questions Q1–Q4 are answered in a research
  artefact under `meta/research/`. Each question has a written answer;
  Q1 cites at least the three named source classes (general-population
  floor, developer behavioural signal, low-vision/accessibility survey)
  and states each source's limitation; Q3 records a pass/fail against
  each of its four named sub-conditions (WCAG 1.4.4 via zoom, WCAG
  1.4.10 reflow at 320 CSS px, the 0033 token-only gate, and
  byte-identical default-size computed values).
- [ ] **AC2.** A stance is chosen from the option space (keep px / rem
  headings / rem family-wide / hybrid). The recorded rationale states
  (a) the Q1 estimate value/range it relied on and (b) the Q3 pass/fail
  result for each sub-condition, and explains why the chosen option is
  preferred over each rejected option.
- [ ] **AC3.** The decision is recorded in a new ADR
  superseding/amending the then-current typography ADR (ADR-0036 or
  0099's successor) — or, if deferred, a child work item captures the
  explicit reasoning. The ADRs chain rather than fork, verifiable by:
  the new ADR's supersedes/amends link points at the then-current
  typography ADR (0099's successor if 0099 has landed, else ADR-0036),
  and no two ADRs claim to be direct successors of the same predecessor
  on the unit axis.
- [ ] **AC4.** If a change from the px status quo is decided, one or
  more follow-on implementation work items are raised and linked from
  this spike (token-definition unit change, root anchor, rem emission,
  consumer/migration work as needed). If "keep px" is decided, that is
  stated explicitly and no implementation item is required.

## Dependencies

- **Upstream (resolved).** 0075 (typography size-scale consumption) has
  **landed** (status: done), so the px-anchored stance is now in
  production and available for evaluation. This spike is no longer
  blocked.
- **Coordinates with 0099** (remap typography size scale to pure-numeric
  tokens), which supersedes ADR-0036 and renames the same scale. Preferred
  ordering is to sequence after 0099, but this is not guaranteed: 0099 is
  decided and ready yet not yet landed, so this spike may reach its
  decision first — hence the conditional ADR-chaining fallback below. This
  spike changes only the token unit, a declaration-only follow-on that
  does not collide with 0099's consumer-rename. See Requirement 4 and Open
  Questions.
- **Constraint — 0033's CI gate.** Any chosen stance must pass 0033's
  inline-literal CI gate (no inline px/rem outside `--size-*`
  definitions) and preserve byte-identical computed sizes at the default
  font-size (the 0075/0099 determinism requirement). This gate can
  invalidate candidate options (e.g. rem family-wide, build-time
  emission), so it is a binding upstream constraint on the decision, not
  just background. See Q3 and Technical Notes.
- **Conditional prerequisite — 0099's successor ADR.** AC3 records the
  decision against the then-current typography ADR. If 0099 has landed
  (the preferred case), this spike's ADR chains off 0099's successor ADR —
  0099's delivery is assumed to produce that ADR; if it does not, or if
  0099 has not landed when this spike decides, this spike chains off
  ADR-0036 directly. See Q4 and Open Questions.
- **Blocks (conditional, downstream).** If a change from the px status
  quo is decided, this spike gates one or more follow-on implementation
  work items (token-definition unit change, root `max()`/`clamp()`
  anchor, build-time rem emission, consumer/migration work as needed),
  raised and linked per AC4. These are not yet raised; the coupling is
  noted here so the work the decision unblocks is visible before the
  spike closes.

## Assumptions

- The accessibility regression may not materially affect our users;
  this spike should not presume the answer and should seek real
  signal before deciding.
- Browser-level zoom remains the dominant scaling mechanism and px
  honours it; the loss is confined to the browser/OS default-font-size
  preference (~3% of users per Internet Archive data) — a bounded but
  non-trivial cohort that skews toward users with visual impairments.
- The tool's audience is software developers, who as power users
  plausibly rely on browser/OS zoom more than the general population —
  which would make the px trade-off lower-risk for our users — but this
  is a hypothesis the baseline research should evidence, not assume.

## Open Questions

- Who owns the **unit-axis** ADR — this spike or 0099? 0099 already owns
  the *naming*-axis successor to ADR-0036 (settled); the open question is
  only which ADR records the px-vs-rem *unit* decision. If 0099 lands
  first, does its successor ADR pre-empt this decision, or should this
  spike's outcome be an input that 0099's successor implements?
- Once the user base is large enough, should we instrument users'
  computed root/default font-size as an analytics dimension to replace
  the developer-population estimate with real signal — and is there a
  surface to carry that dimension?
- Must body copy specifically remain preference-scalable even if UI
  chrome and display headings stay px-anchored (Comeau's "right tool
  per purpose" model)?

## Technical Notes

- The accessibility loss is narrow: px font-size honours page zoom
  (satisfying WCAG 1.4.4) but ignores the browser/OS default-font-size
  preference, which only scales rem/em/%. So px is AA-conformant; the
  concern is best-practice and the default-font-size cohort, not a
  conformance failure.
- Hybrid patterns let px determinism coexist with preference-scaling:
  tokens authored in px but emitted as rem at build; an
  `html { font-size: max(1em, 16px) }` or `clamp()` root anchor
  (OddBird, 2025) where the px value is a floor and a larger user
  default wins; or a USWDS-style `respect-user-font-size` opt-in.
- Constraint: 0033 established a CI gate banning inline px/rem literals
  — the `--size-*` tokens are the single source of truth. Any approach
  must keep consumers token-only and preserve byte-identical computed
  sizes at the default (0075's determinism requirement; 0099 reasserts
  "byte-identical computed font sizes before and after").
- Current consumers to re-check: `MarkdownRenderer` H1
  (`var(--size-h3)` = 28px, formerly 1.75rem); the `.markdown` prose
  tokens added in 0088 (`--size-prose: 14.5px`); the eleven-step
  `--size-*` scale from 0033.
- Blast radius of a unit change is small and disjoint from 0099's
  rename: switching px→rem (or to a hybrid) edits only the ~19 token
  *definitions* in `global.css` (and the `tokens.ts` mirror), not the
  ~100 `var(--size-*)` consumer sites. The font-size literal gate in
  `migration.test.ts` matches `font-size:` declarations / `font:`
  shorthand only, so a `rem` value in a `--size-*:` definition is
  gate-exempt — the definitions are the sanctioned home for the literal.
- 0099 renames the tokens this review references by name (`--size-h3`,
  `--size-prose`, `--size-body`, …) to pure-numeric. After 0099 lands
  those names are stale here — cite tokens by value, or refresh the
  names once 0099 has merged.

## Drafting Notes

- Reframed back to a **spike** (reversing the earlier task framing) per
  the author's direction: `kind` is `spike`, Requirements are
  restructured around research questions + exit criteria, and the
  deliverables now explicitly include follow-on implementation work
  item(s), not just an ADR — the spike decides, it does not migrate.
- Not hard-time-boxed: bounded by the exit criteria instead, consistent
  with the earlier "no time-box" preference. A duration can be added if
  wanted.
- Removed the non-template `## Decisions` section (it held only "None
  yet — this is a spike"); the decision outcome is captured by the
  Acceptance Criteria and Requirements.
- Cleared `blocked_by: work-item:0075` and rewrote Dependencies — 0075
  is done, so the stance is in production and the item is unblocked.
- Body header `Kind`/`Status` tracks the frontmatter (`Spike`/`Draft`).
- Added a fourth "hybrid" option (AC2 / Requirements) from
  best-practice research; the original three options omitted the
  approach most authorities now favour for reconciling determinism
  with preference-scaling.
- Surfaced coordination with 0099 (which already proposes superseding
  ADR-0036) as a Requirement and Open Question, since both pivot on
  ADR-0036's future.
- Corrected the implicit premise that px is an accessibility failure:
  research shows px passes WCAG 1.4.4 via zoom; the loss is the
  default-font-size preference only.
- Status left at `draft` (unchanged); not transitioned despite the
  blocker landing.
- Reframed Requirement 1 / AC1 from "instrument our own users" to
  "estimate a developer-population baseline": the user base is not yet
  large enough to instrument meaningfully, and the tool targets
  software developers, so a triangulated target-audience estimate is
  the more decision-relevant signal. Direct developer-specific
  default-font-size data is expected to be sparse, so the requirement
  asks for a reasoned estimate, not a measurement; own-user
  instrumentation is demoted to a future-validation step (Open
  Questions).
- Addressed review-1 findings
  (`meta/reviews/work/0091-typography-rem-vs-px-stance-review-1.md`):
  resolved the time-box wording (preamble is now "exit-criteria-bounded"
  and the Time-box section adds a soft ~1–2 day effort guideline, not a
  hard stop); added the 0033 CI-gate constraint, the conditional
  0099-successor-ADR prerequisite, and a conditional Blocks note for the
  follow-on items to Dependencies (and added 0033/0090 to `relates_to`);
  hardened AC1/AC2/AC3 into objectively checkable conditions; aligned the
  ~3% figure's attribution to "Internet Archive" throughout; scoped the
  ADR-ownership Open Question to the unit axis; and standardised body
  self-reference to "this spike".
- Re-review (pass 2) caught that the review-1 Dependencies rewrite had
  introduced a contradiction — "sequence after 0099" alongside the
  "if 0099 has not landed" fallback branch. Reconciled by framing
  sequence-after-0099 as the *preferred-but-not-guaranteed* ordering
  (0099 is ready but unlanded) with the conditional ADR-chaining as the
  fallback, and made the 0099-successor-ADR existence trigger explicit.

## References

- `meta/decisions/ADR-0036-typography-font-size-consumption-rule.md`
  — origin of the px-anchored stance.
- `meta/work/0075-typography-size-scale-consumption.md` — landed the
  migration this work item evaluates (status: done).
- `meta/plans/2026-05-23-0075-typography-size-scale-consumption.md` —
  plan that introduced the px-anchored stance.
- Related work items:
  - `meta/work/0099-remap-typography-size-scale-to-pure-numeric-tokens.md`
    — proposes superseding ADR-0036 and remapping the same `--size-*`
    scale; must be coordinated with this review.
  - `meta/work/0033-design-token-system.md` — introduced the
    `--size-*` scale and the CI gate banning inline px/rem literals.
  - `meta/work/0088-markdown-body-width-harmonisation.md` — migrated
    `.markdown` rem font-sizes onto px `--size-*` tokens.
  - `meta/work/0090-radius-tokens-consumption.md` — parallel token
    precedent; name-checks this work item.
- External research:
  - W3C — Understanding SC 1.4.4 Resize Text:
    https://www.w3.org/WAI/WCAG21/Understanding/resize-text
  - W3C WAI — Page Structure / Styling tutorial:
    https://www.w3.org/WAI/tutorials/page-structure/styling/
  - GOV.UK Design System — Type scale:
    https://design-system.service.gov.uk/styles/type-scale/
  - USWDS — Font-size design tokens:
    https://designsystem.digital.gov/design-tokens/typesetting/font-size/
  - OddBird — Designing for User Font-size and Zoom (2025):
    https://www.oddbird.net/2025/07/22/size-preferences/
  - Josh Comeau — The Surprising Truth About Pixels and Accessibility:
    https://www.joshwcomeau.com/css/surprising-truth-about-pixels-and-accessibility/
  - CSS-Tricks — Accessible Font Sizing, Explained:
    https://css-tricks.com/accessible-font-sizing-explained/
  - Evan Minto — Pixels vs. Ems: Users DO Change Font Size:
    https://medium.com/@vamptvo/pixels-vs-ems-users-do-change-font-size-5cfb20831773
    (the general-population ~3% default-font-size figure; the AC1 floor).
- Target-audience baseline sources (for AC1):
  - WebAIM — Survey of Users with Low Vision #2 (2018):
    https://webaim.org/projects/lowvisionsurvey2/ — browser/OS usage and
    proficiency of the low-vision cohort; informs the disabled-developer
    slice of the baseline.
  - Stack Overflow — 2025 Developer Survey:
    https://survey.stackoverflow.co/2025/ — characterises the target
    developer audience (OS/browser/tooling). Note: does not cover
    default-font-size directly; used for audience profiling, not as a
    font-size signal.
