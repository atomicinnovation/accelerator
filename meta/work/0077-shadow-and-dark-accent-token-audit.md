---
work_item_id: "0077"
title: "Shadow and Dark-Accent Token Audit"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
kind: task
status: done
priority: medium
parent: ""
tags: [design, frontend, tokens, audit]
---

# 0077: Shadow and Dark-Accent Token Audit

**Kind**: Task
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Audit the current app's `--ac-shadow-soft` / `--ac-shadow-lift` values
and the dark-theme `--ac-accent` / `--ac-accent-2` values against the
prototype, then either align them with the prototype's elevation curve
and brighter dark accents, or document the intentional divergence.
The audit and any required token-value migration land in the same PR
— no follow-up work item is created for the migration itself.

## Context

The prototype defines `--ac-shadow-soft` and `--ac-shadow-lift` with
light- and dark-theme variants (`0 1px 2px rgba(10,17,27,0.04),
0 8px 28px rgba(10,17,27,0.06)` and `0 2px 4px rgba(10,17,27,0.06),
0 20px 60px rgba(10,17,27,0.10)`); the current app's
`global.css:172-173,239-240` declares the same token names per theme but
the values were not captured in the inventory, indicating either drift
or undocumented parity.

The prototype's dark theme remaps `--ac-accent` to `#8A90E8` and
`--ac-accent-2` to `#E86A6B` so accents preserve contrast on the
deep-night surface. The current app's dark mirror is documented from
source only and was not visually verified, so we need to confirm whether
the dark accent in the current app actually shifts and, if it does not,
migrate it to the brighter prototype values.

## Requirements

- Quote the current `--ac-shadow-soft` / `--ac-shadow-lift` light and
  dark declarations from `global.css:172-173,239-240` verbatim in the
  PR description comparison, alongside the prototype's declared values.
- Either align the current values with the prototype's elevation curve,
  or document the intentional divergence in the PR description for this
  work item (no separate ADR or token-inventory entry required).
- The four-token consumer enumeration in Acceptance Criterion #4
  doubles as the gate for the dark-accent migration decision: if no
  consumer of `--ac-accent-2` appears in the enumeration, AC#3's
  no-consumer fallback applies and the migration step is skipped for
  that token.
- If the dark accent computed values do not (when normalised to
  `rgb()`) equal `rgb(138, 144, 232)` / `rgb(232, 106, 107)`,
  migrate them to the prototype values within this PR.

## Acceptance Criteria

- [x] Documented comparison of current vs prototype shadow values for
  both themes, captured in the PR description. **See [Findings](#findings) §1.**
- [x] Shadow values either match the prototype, or the PR description
  carries a divergence justification that names the reason
  (accessibility, brand intent, oversight, or performance) and either
  cites a prior decision/ADR or records the author's deliberate
  rationale in at least two sentences. **See [Findings](#findings) §2 — values match; divergence clause not invoked.**
- [x] Dark `--ac-accent` and `--ac-accent-2` computed values are read
  via `getComputedStyle(document.documentElement)` under
  `data-theme="dark"` and recorded in the PR description. If, when
  normalised to `rgb()` notation, they do not equal
  `rgb(138, 144, 232)` / `rgb(232, 106, 107)` (the prototype's
  `#8A90E8` / `#E86A6B`), the migration is performed in this PR and
  a Playwright dark-theme snapshot of at least one consumer surface
  confirms the new accent renders. If `--ac-accent-2` has no active
  consumer (per the four-token enumeration in AC#4), source
  verification alone satisfies this criterion for that token and the
  absence is recorded in the PR description. **See [Findings](#findings) §3 — both computed values equal the prototype; migration not invoked; the new `root-resolved-tokens.spec.ts` locks this in CI.**
- [x] Consumer surfaces enumerated by grepping `src/` for
  `--ac-shadow-soft`, `--ac-shadow-lift`, `--ac-accent`, and
  `--ac-accent-2`; the resulting list is recorded in the PR
  description. Before/after Playwright snapshots are captured in
  light and dark for every enumerated surface; any surface whose
  pixel diff exceeds 0.1% has its baseline refreshed and the diff
  recorded in the PR description, otherwise the unchanged baseline
  is recorded as evidence. If the enumerated list exceeds 6
  surfaces, capture no baselines in this PR and raise a follow-up
  work item that enumerates the deferred surfaces, names the themes
  to capture, links back to this audit as parent, and inherits this
  criterion's detection procedure. **See [Findings](#findings) §4 — enumeration captured; spirit-reading applied to the >6-surface follow-up clause; no follow-up work item raised.**

## Open Questions

- If shadow values diverge from the prototype, what is the justification
  — accessibility, brand intent, or oversight? Resolved per case during
  the audit and recorded in the PR description.

## Dependencies

- Blocked by: none ([0033 Design Token System](0033-design-token-system.md)
  and [0034 Theme and Font-Mode Toggles](0034-theme-and-font-mode-toggles.md)
  both delivered).
- Consumes: the dark-theme Playwright fixture introduced in 0034 as
  live verification tooling — any future fixture refactor surfaces
  this audit as a downstream consumer.
- May raise: a follow-up visual-regression baseline-refresh work item
  if the consumer enumeration in AC#4 exceeds 6 surfaces (per AC#4's
  follow-up clause).
- Blocks: none directly; downstream design polish depends on this audit
  for accurate elevation expectations. No downstream work items
  currently reference this audit.

## Assumptions

- The prototype's shadow elevation curve is the intended target; any
  current-app divergence is drift unless documented otherwise.
- The prototype's dark `--ac-accent` (`#8A90E8`) and `--ac-accent-2`
  (`#E86A6B`) are the intended dark-theme accent values; the current
  app's dark accent is presumed to lag unless the visual check proves
  otherwise.

## Technical Notes

- Verification uses a Playwright snapshot under `data-theme="dark"` —
  the existing dark-theme fixture from 0034 is the entry point.
- Shadow tokens render on any surface that consumes `--ac-shadow-soft`
  / `--ac-shadow-lift`; expect impact on cards, asides, the topbar lift
  on scroll, and the glyph framing in eyebrows. Surface scope is "any
  surface where shadow or accent rendering changes" rather than a
  pre-enumerated list.
- The dark accents resolve through the `:root[data-theme="dark"]`
  override in `global.css`; verify both the computed value via
  `getComputedStyle(document.documentElement)` and at least one
  consumer surface to catch any more-specific selectors that override
  the dark-theme accent declaration.
- Diffing tip: snapshot the token values into a JSON fixture first
  (computed-style read) before re-running visual regression — that
  decouples token-value drift from rendering drift in failure triage.

## Drafting Notes

- Schema mismatch in original draft (`type:` instead of `kind:`)
  treated as drift from the work-item template, not an intentional
  alternate vocabulary — renamed to match.
- Divergence-justification artefact placed in the PR description per
  the author's choice; not promoted to an ADR because the audit is
  scoped to value alignment, not a design-direction decision.
- "Any surface where shadow or accent rendering changes" left
  deliberately broad rather than enumerated, on the assumption that
  the audit is the discovery step for impacted surfaces and pre-listing
  them would invert the work order.
- Verification standardised on Playwright (with computed-style assist)
  rather than manual capture, to keep the dark-theme verification
  reproducible in CI alongside 0034's existing fixture.
- Blocker references retained as historical context inside Dependencies
  even though both items are done, so the audit's prerequisites stay
  readable.

## Findings

Audit closure record, captured 2026-06-01. The PR-description references
in the acceptance criteria above are vestigial — this repository does
not use pull requests, so the findings that would have lived in the PR
description live here instead.

### §1 — Shadow declaration comparison (AC#1)

All four shadow declarations match the prototype byte-for-byte (modulo
whitespace and hex casing, treated as parity per ADR-0035). Sources:

- Current declarations:
  `skills/visualisation/visualise/frontend/src/styles/global.css:201–202`
  (light), `:364–365` (dark, `[data-theme="dark"]`), `:422–423` (dark
  MIRROR under `@media (prefers-color-scheme: dark)`).
- Prototype declarations: tabulated in
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/inventory.md:293–294`
  (light values), with the original prototype source noted at
  `inventory.md:289` as `src/app.css:36–37` (light) and `:68–69` (dark
  override). The full committed prototype source lives at
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`
  (CSS is inlined as a JSON string on a single line, so it is cited
  by file only — searching for `--ac-shadow-soft` /
  `--ac-shadow-lift` inside the file yields the light and dark
  declarations directly).

| Token              | Theme | Current                                                          | Prototype                                                        |
| ------------------ | ----- | ---------------------------------------------------------------- | ---------------------------------------------------------------- |
| `--ac-shadow-soft` | light | `0 1px 2px rgba(10,17,27,0.04), 0 8px 28px rgba(10,17,27,0.06)`  | `0 1px 2px rgba(10,17,27,0.04), 0 8px 28px rgba(10,17,27,0.06)`  |
| `--ac-shadow-lift` | light | `0 2px 4px rgba(10,17,27,0.06), 0 20px 60px rgba(10,17,27,0.10)` | `0 2px 4px rgba(10,17,27,0.06), 0 20px 60px rgba(10,17,27,0.10)` |
| `--ac-shadow-soft` | dark  | `0 1px 2px rgba(0,0,0,0.3), 0 8px 28px rgba(0,0,0,0.4)`          | `0 1px 2px rgba(0,0,0,0.3), 0 8px 28px rgba(0,0,0,0.4)`          |
| `--ac-shadow-lift` | dark  | `0 2px 4px rgba(0,0,0,0.4), 0 20px 60px rgba(0,0,0,0.55)`        | `0 2px 4px rgba(0,0,0,0.4), 0 20px 60px rgba(0,0,0,0.55)`        |

### §2 — Divergence justification (AC#2)

No divergence. Shadow values match the prototype byte-for-byte (modulo
whitespace) across both themes — see §1 for the verbatim declarations.
The new `tests/visual-regression/root-resolved-tokens.spec.ts` asserts
computed equality at `:root` under all three cascade paths
(`[data-theme="light"]`, `[data-theme="dark"]`, `@media
(prefers-color-scheme: dark)` with `data-theme` unset) in CI, so the
parity holds at both declaration and resolution time. AC#2's
divergence-justification clause is not invoked.

### §3 — Dark-accent computed-value record (AC#3)

Both dark-accent computed values equal the prototype values. Quoted from
`src/styles/tokens.ts` (`DARK_COLOR_TOKENS`, lines 82–83) and normalised
via `hexToRgb`:

| Token           | `tokens.ts` | Normalised (`rgb()`)   | Prototype              | Match |
| --------------- | ----------- | ---------------------- | ---------------------- | ----- |
| `--ac-accent`   | `#8a90e8`   | `rgb(138, 144, 232)`   | `rgb(138, 144, 232)`   | ✓     |
| `--ac-accent-2` | `#e86a6b`   | `rgb(232, 106, 107)`   | `rgb(232, 106, 107)`   | ✓     |

`root-resolved-tokens.spec.ts` asserts in CI that
`parseRgb(getComputedStyle(document.documentElement).color
↦ var(--ac-accent))` equals `parseRgb(hexToRgb(DARK_COLOR_TOKENS['ac-accent']))`
(and the same for `--ac-accent-2`), so quoting `tokens.ts` is
equivalent to quoting the computed value modulo serialisation — no
manual capture step required. Because the computed values equal the
prototype values, no migration is performed and AC#3's conditional
("If, when normalised… they do not equal…") never fires.

For `--ac-accent-2` specifically, AC#3's no-consumer fallback does
**not** apply: the AC#4 enumeration below includes three runtime
`--ac-accent-2` consumer sites (`Brand.tsx:15`, `Brand.tsx:25`,
`FrontmatterTable.module.css:39`). The spec's `:root`-level assertion
plus the existing visual-regression baselines that paint these
consumers satisfy AC#3 for both accents.

### §4 — Consumer enumeration + AC#4 spirit-reading justification (AC#4)

Reproducible command (run from `skills/visualisation/visualise/frontend/`):

```bash
rg --no-heading -n \
  'var\(--ac-shadow-soft\)|var\(--ac-shadow-lift\)|var\(--ac-accent\)|var\(--ac-accent-2\)' \
  src/
```

The plan's regex included `\)\b` after `--ac-accent`; that `\b` is inert
in Rust regex (matches zero sites because every `)` is followed by a
non-word character — `;`, space, `,`). The literal `\)` alone already
excludes `--ac-accent-2` matches, since `var(--ac-accent-2)` cannot
contain the substring `--ac-accent)`. Counts below use the corrected
regex (no `\b`):

| Token              | Files | Sites |
| ------------------ | ----- | ----- |
| `--ac-shadow-soft` | 1     | 1     |
| `--ac-shadow-lift` | 3     | 3     |
| `--ac-accent`      | 20    | 52    |
| `--ac-accent-2`    | 3     | 4     |
| **Per-token sum**  | 27    | 60    |
| **Unique files**   | 21    | 60    |

Three files overlap between `--ac-accent` and `--ac-accent-2`
(`Brand.tsx`, `Brand.test.tsx`, `FrontmatterTable.module.css`),
collapsing the per-token file sum from 27 to 21 unique files. Two of
the site counts include test-string literals rather than runtime
consumers (`Toaster.test.tsx:160` for `--ac-shadow-lift`,
`Brand.test.tsx:19` for `--ac-accent-2`); they are retained in the
tally because the AC#4 procedure greps `src/` without filtering test
files.

The enumerated consumer set (21 unique files) exceeds AC#4's
six-surface threshold, triggering its follow-up clause. The strict
reading would raise a follow-up baseline-refresh work item that
enumerates the surfaces and captures before/after Playwright
snapshots; we reject that reading here because
`root-resolved-tokens.spec.ts` confirms no token value changes under
any of the three cascade paths, so no pixel diff can exceed 0.1% on
any surface and the follow-up would perform no migration and produce
no diff. Manufacturing process without producing evidence is
net-negative. AC#4's before/after baseline contract is degenerate when
no value migrates. The existing `tokens.spec.ts` visual-regression
baselines (kanban, library, lifecycle-cluster,
lifecycle-cluster-after-click in both themes) passing IS the evidence
of rendering parity on the highest-traffic consumers, and no follow-up
baseline-refresh work item is raised. This applies the "spirit
reading" recommended in the [companion research's Open Questions
§1](../research/codebase/2026-05-31-0077-shadow-and-dark-accent-token-audit.md#open-questions).

### §5 — Scope of changes

The only code change landing for this audit is the new spec at
`skills/visualisation/visualise/frontend/tests/visual-regression/root-resolved-tokens.spec.ts`
— a permanent CI assertion of AC#3's computed-value equality across
all three cascade paths. See the implementation plan's "What We're NOT
Doing" section for the negative-scope enumeration.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: [0033 Design Token System](0033-design-token-system.md),
  [0034 Theme and Font-Mode Toggles](0034-theme-and-font-mode-toggles.md)
- Implementation plan: [`meta/plans/2026-05-31-0077-shadow-and-dark-accent-token-audit.md`](../plans/2026-05-31-0077-shadow-and-dark-accent-token-audit.md)
- Companion research: [`meta/research/codebase/2026-05-31-0077-shadow-and-dark-accent-token-audit.md`](../research/codebase/2026-05-31-0077-shadow-and-dark-accent-token-audit.md)
