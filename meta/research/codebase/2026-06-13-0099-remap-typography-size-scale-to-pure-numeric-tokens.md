---
type: codebase-research
id: "2026-06-13-0099-remap-typography-size-scale-to-pure-numeric-tokens"
title: "Research: Remap Typography Size Scale To Pure-Numeric Tokens (0099)"
date: "2026-06-13T09:39:37+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0099"
parent: "work-item:0099"
relates_to: ["codebase-research:2026-05-23-0075-typography-size-scale-consumption", "codebase-research:2026-06-02-0090-radius-tokens-consumption", "codebase-research:2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown"]
topic: "Remap Typography Size Scale To Pure-Numeric Tokens"
tags: [research, codebase, visualiser, design-tokens, typography, css, adr]
revision: "ad582c8544328b2e39327da23d31d2ff979555b3"
repository: "visualisation-system"
last_updated: "2026-06-13T09:39:37+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Remap Typography Size Scale To Pure-Numeric Tokens (0099)

**Date**: 2026-06-13T09:39:37+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: ad582c8544328b2e39327da23d31d2ff979555b3
**Branch**: HEAD (jj workspace: `workspaces/visualisation-system`)
**Repository**: visualisation-system

## Research Question

What does an implementer of work item 0099 need to know to remap the
visualiser's typography `--size-*` scale to a pure-numeric `--size-<px×10>`
naming scheme (e.g. `--size-110` = 11px, `--size-145` = 14.5px), superseding
ADR-0036 via a new successor ADR? Specifically: where do the tokens live, how do
the four guardrail tests behave under a pure rename, what is the true blast
radius across consumers, how is ADR supersession modelled in this repo, and what
do the precedent work items (0075, 0090) and siblings (0091, 0094) tell us?

## Summary

This is a **mechanical, whole-scale rename** of 19 `--size-*` tokens to a
pure-numeric `px×10` scheme. Every computed px value is preserved, so it is
visually inert. The work divides into four lockstep surfaces — `global.css`
declarations + convention comment, the `tokens.ts` mirror, ~140 CSS consumer
references, and a handful of hardcoded test assertions — plus a same-PR ADR
deliverable (a new ADR-0043 that fully supersedes ADR-0036).

Key findings:

1. **The token surface is clean and self-contained.** All 19 `--size-*` tokens
   are literal `px` values in a single `:root` block
   (`global.css:173-191`), with no `calc()`, no `var()` indirection, and **no
   dark-mode or media-query overrides**. The declaration site is the only place
   values live.

2. **The mirror stores names *without* the `--` prefix.** `TYPOGRAPHY_TOKENS`
   in `tokens.ts:183-201` holds keys like `"size-hero"`, not `"--size-hero"`.
   A naïve `grep '--size-'` **misses the mirror entirely** — yet the mirror is
   the source the completeness test reads from. This is the single biggest
   find-replace trap.

3. **Three of the four named guardrails stay green automatically** under a pure
   rename, *provided* `tokens.ts` keys and every CSS consumer move in lockstep.
   The fourth (the font-size ban) is unaffected by the rename but carries a
   hardcoded `ADR-0036` reference that must be repointed at the successor ADR by
   hand — and is **not** assertion-enforced, so it will silently go stale if
   missed.

4. **The real hazard is ~8 hardcoded `--size-*` name strings in test files**
   (`migration.test.ts`, `Chip.test.tsx`, `FrontmatterTable.test.tsx`) that a
   CSS-only sweep will miss. These turn the suite red on rename and must be
   edited explicitly.

5. **The blast radius is ~140 literal `var(--size-*)` references across 38
   `.module.css` files** (179 total `--size-*` mentions across 43 files
   including definitions, comments, and test assertions). Five upper-band tokens
   (`hero`, `h1`, `h2`, `h4`, `body`) have **zero `var()` consumers** — they are
   defensive scaffolding kept by 0075 and are still renamed under the
   whole-scale decision.

6. **ADR supersession is well-precedented.** The next ADR id is **ADR-0043**.
   The canonical edge is `supersedes` on the new ADR; the old ADR may carry
   `superseded_by`. 0099 wants a **full** supersession (ADR-0036 → `superseded`),
   which differs from 0090's clause-level *amend* of ADR-0026. An immutability
   tension exists (editing an `accepted` ADR), already documented in a repo note.

7. **The completeness oracle is the `var()`-resolves-to-declared-token test**
   (`migration.test.ts:1649-1688`), which auto-tracks the declared key set from
   `Object.keys(...)` of the token maps. 0099 does **not** add a new CI grep gate
   the way 0090 did; AC1's grep is a manual verification command.

8. **The design-prototype drift guard and visual-regression baselines are
   untouched by construction.** The drift fixture pins only
   `--code-*`/`--tk-*`/`--atomic-*` (no `--size-*`), and the resolved-size specs
   assert computed px (not token names), so a pure rename leaves them green.

## Detailed Findings

### 1. Token declarations and convention comment — `global.css`

File: `skills/visualisation/visualise/frontend/src/styles/global.css`

**The 19 `--size-*` declarations (lines 173-191), verbatim:**

```
173	  --size-hero:        68px;
174	  --size-h1:          48px;
175	  --size-h2:          36px;
176	  --size-h3:          28px;
177	  --size-h4:          26px;
178	  --size-lg:          22px;
179	  --size-body:        20px;
180	  --size-prose:       14.5px;
181	  --size-md:          18px;
182	  --size-sm:          16px;
183	  --size-xs:          14px;
184	  --size-subtitle:    13px;
185	  --size-row:         12.5px;
186	  --size-xxs:         12px;
187	  --size-xxs-sm:      11.5px;
188	  --size-eyebrow:     11px;
189	  --size-3xs-lg:      10.5px;
190	  --size-3xs:         10px;
191	  --size-4xs:         9.5px;
```

- All 19 values are literal `px` lengths. **None use `rem`, `em`, `calc()`, or
  `var()` indirection.** Sub-pixel tokens: `prose` 14.5, `row` 12.5,
  `xxs-sm` 11.5, `3xs-lg` 10.5, `4xs` 9.5.
- They live in the single top-level `:root { … }` block (opens `global.css:76`,
  closes `global.css:338`).
- **No dark-mode or media-query override of any `--size-*` token exists** — a
  full-file grep confirms `--size-` appears only in the comment and these
  declarations. The two dark blocks (`[data-theme="dark"]` at 345-412 and the
  `@media (prefers-color-scheme: dark)` mirror from 418) override only `--ac-*`
  colour/shadow tokens. So the declaration site is the *sole* value source — no
  theme redeclarations to keep in sync.
- **Ordering quirk:** `--size-prose` (14.5px, line 180) sits between `body` (20)
  and `md` (18), not in numeric order. The block is otherwise roughly
  descending. A pure-numeric scheme "sorts numerically", so the implementer may
  *optionally* reorder the block for readability — the work item does not require
  it, and reordering is value-preserving and cosmetic.

**The convention comment to be rewritten (lines 161-172), verbatim:**

```
161	  /*
162	   * font-size consumers: use these tokens — see ADR-0036.
163	   *
164	   * Naming: tier names (e.g. `--size-xxs`, `--size-xs`, `--size-sm`,
165	   * `--size-md`, `--size-body`, `--size-lg`, `--size-h1`, `--size-hero`)
166	   * form the integer-px scale. `--size-eyebrow`, `--size-row`, and
167	   * `--size-subtitle` are semantic single-purpose tokens (each has one
168	   * conceptual consumer in current designs). The `-sm` / `-lg` suffixes
169	   * mark sub-pixel tweens between integer tiers
170	   * (e.g. `--size-xxs-sm` = 11.5px sits between `--size-xxs` 12 and
171	   * `--size-eyebrow` 11).
172	   */
```

This comment (authored by 0075) documents all three retired naming families and
references ADR-0036. **AC2 requires it fully rewritten** to describe the
`--size-<px×10>` encoding (with a whole-step and a half-step example) and to
mention the successor ADR rather than ADR-0036.

**Out-of-scope neighbours** (do NOT touch): the `--lh-*` line-height tokens
(`global.css:192-196`, unitless), `--tracking-caps` (line 197, an `em` value),
the font-family tokens `--ac-font-*` (157-159), and the `--sp-*` spacing scale
(199-210).

### 2. The `tokens.ts` mirror — `TYPOGRAPHY_TOKENS`

File: `skills/visualisation/visualise/frontend/src/styles/tokens.ts`

`TYPOGRAPHY_TOKENS` is a hand-maintained `as const` object literal at
`tokens.ts:179-208`. The `--size-*` portion (lines 183-201) stores **keys
without the `--` prefix**:

```
183	  "size-hero": "68px",
184	  "size-h1": "48px",
…
198	  "size-eyebrow": "11px",
…
201	  "size-4xs": "9.5px",
```

- The object also holds `ac-font-*` (180-182) before the size block and `lh-*` +
  `tracking-caps` (202-207) after. The derived type
  `TypographyToken = keyof typeof TYPOGRAPHY_TOKENS` is at `tokens.ts:393`
  (exported but currently unconsumed as an annotation).
- It is a **hand-maintained mirror validated against `global.css` at test time**
  — not generated, and `global.css` is not generated from it.
- **Nothing constructs `--size-*` names dynamically** except the generic
  `Object.keys`/`Object.entries` loops in the tests (which self-adjust to
  whatever keys exist). Every other reference is a literal name. A mechanical
  per-token find-replace is therefore complete *if* it covers the mirror keys,
  the CSS declarations, all `var()` consumers, and the hardcoded test strings.

**Critical trap:** because the mirror keys carry no `--` prefix, a sweep based on
`rg -- '--size-'` will **not** touch `tokens.ts`. The completeness test
(`migration.test.ts`) and the parity test (`global.test.ts`) both read the
declared key set from this mirror, so a missed mirror key produces a confusing
"declared vs consumed" mismatch.

### 3. The four guardrails + parity — `migration.test.ts` / `global.test.ts`

Files: `…/src/styles/migration.test.ts`, `…/src/styles/global.test.ts`

| Guard | Location | Auto-tracks? | Manual edit needed for a pure rename? |
|---|---|---|---|
| **var() resolves to declared token** | `migration.test.ts:1649-1688` | Yes — set built from `Object.keys(TYPOGRAPHY_TOKENS)` etc. at `:1650-1661` | No (just rename mirror key + all CSS consumers together) |
| **font-size ban / ADR-0036 reference** | `migration.test.ts:1905-1906` (comment), gate at `:1949-2006` | n/a — bans literals, not names | **Yes** — repoint the `ADR-0036` comment at `:1905` to the successor ADR. *Not* assertion-enforced; will silently go stale if missed |
| **EXCEPTIONS hygiene** | `migration.test.ts:1734-1765` | Yes — keys on literals + file paths | No (stale `"--size-lg"` prose in a `reason` at `:1156` is cosmetic, unvalidated) |
| **AC5 aggregate `var(--*)` ratchet** | `migration.test.ts:1690-1722` | Yes — counts the `var(--` prefix, name-agnostic | No (`AC5_FLOOR = 989`, unchanged by a rename) |
| **global.css ↔ tokens.ts parity** | `global.test.ts:104-119` | Yes — loops `Object.entries(TYPOGRAPHY_TOKENS)` | No (rename mirror key + `global.css` declaration together) |

Mechanism detail:

- **Completeness oracle** (`migration.test.ts:1649-1688`): builds a `declared`
  Set from `Object.keys(...)` of all ten token maps (including
  `TYPOGRAPHY_TOKENS` at `:1653`), extracts every `var(--NAME)` from globbed CSS
  (`VAR_REF_RE` at `:32`), and asserts any name not in `declared` (and not in a
  per-file `LOCAL_CUSTOM_PROPS` allow-list, which has no `--size-*` entries) is
  empty. **This is the rename-completeness gate**: a stale `var(--size-OLD)`
  left in any CSS file, or a renamed declaration with a surviving old consumer,
  fails here. No edit to this block is required.
- **Font-size ban** (`migration.test.ts:1949-2006`): `FONT_SIZE_LITERAL_RE` +
  `FONT_SHORTHAND_RE` ban raw `font-size`/`font:` numeric literals. It is the
  "AUTHORITATIVE IMPLEMENTATION of ADR-0036's font-size consumption rule"
  (comment `:1905-1906`) — **the only ADR-0036 reference in either test file**.
  A pure rename touches no literals, so the gate stays green, but **AC8 requires
  this comment repointed to the successor ADR** as part of the work.
- **AC5 ratchet** (`migration.test.ts:1690-1722`): sums `var(--` occurrences in
  `*.module.css` only; `AC5_FLOOR = 989`, `AC5_TARGET = 300`, slack 0. A
  `var(--size-OLD)` → `var(--size-NEW)` rewrite preserves the `var(--` prefix, so
  the count is unchanged — no floor bump.
- **Parity** (`global.test.ts:104-119`): for each `[name, value]` in
  `TYPOGRAPHY_TOKENS`, asserts `readCssVar(name, "root")` (regex-extracted from
  `global.css`, `:42-54`) equals the mirror value. Auto-tracks renamed keys; but
  if the mirror key is renamed and the `global.css` declaration is not (or vice
  versa), `readCssVar` returns `null` and the assertion fails.

### 4. The real hazard — hardcoded `--size-*` name strings in tests

These are **not** part of the four named guardrails but live in the same suite
and turn red on rename. A CSS-only sweep misses them:

- `migration.test.ts:1783-1785` — `expect(css).toContain("var(--size-prose)")`
- `migration.test.ts:1795-1797` — `expect(css).toContain("var(--size-xxs-sm)")`
- `migration.test.ts:1804-1812` — asserts a block does **NOT** contain
  `var(--size-xs)` (negative assertion)
- `migration.test.ts:1813-1819` — `expect(css).toContain("var(--size-eyebrow)")`
- `migration.test.ts:2037-2038` — regex *fixtures* using `var(--size-xxs)` as a
  literal test string (these exercise the font regex and would still pass, but
  reference an old name and should be updated for consistency)
- `Chip.test.tsx:117-119` — asserts `var(--size-3xs-lg)`
- `Chip.test.tsx:198` — asserts `var(--size-xxs-sm)` (via `new RegExp(...)`)
- `FrontmatterTable.test.tsx:213-214` — asserts `var(--size-xxs-sm)`
- Comments only (cosmetic, won't fail): `typography-resolved-sizes.spec.ts`
  (lines 6, 132-133), `inline-code-resolved-styles.spec.ts:99`,
  `migration.test.ts:1156` (the stale `--size-lg` `reason` prose)

**Prefix guard:** `VAR_FALLBACK_RE` (`migration.test.ts:33,38`) matches the
family *prefix* `size-` (alongside `ac-`, `sp-`, `radius-`, …), not individual
names. A per-token rename that **keeps the `size-` prefix** (which `px×10` does)
leaves this regex untouched. Only a prefix change would require editing it.

### 5. Blast radius — consumers across the frontend

Confined to `skills/visualisation/visualise/frontend/`:

- **139 literal `var(--size-*)` occurrences across 38 `.module.css` files.**
- **179 total `--size-*` mentions across 43 files** (adds the `global.css`
  declarations, the `tokens.ts` mirror keys, comments, and test assertions).

Per-token literal `var()` consumption (definitions excluded):

| Token | `var()` refs | Notes |
|---|---|---|
| `--size-hero` | 0 | defined only (defensive scaffolding) |
| `--size-h1` | 0 | defined only |
| `--size-h2` | 0 | defined only |
| `--size-h3` | 2 | `Page.module.css:48`, `MarkdownRenderer.module.css:15` |
| `--size-h4` | 0 | defined only — **flagged in 0099 Assumptions as originally missed; in scope** |
| `--size-lg` | 2 | `EmptyState`, `NoResultsPanel` |
| `--size-body` | 0 | defined only |
| `--size-md` | 2 | `TopbarIconButton`, `MarkdownRenderer` |
| `--size-sm` | 6 | KanbanCardShowcase, MarkdownRenderer, LibraryDocView, Brand, NoResultsPanel, LifecycleIndex |
| `--size-prose` | 1 (+test) | `MarkdownRenderer.module.css:6` |
| `--size-xs` | 17 (+test) | widely used across showcases, kanban, library, lifecycle |
| `--size-subtitle` | 11 | kanban, Page, library, RelatedCluster, Sidebar, lifecycle |
| `--size-row` | 5 | KanbanColumn, FilterPill, SortPill, Sidebar |
| `--size-xxs` | 21 | the broadest consumer set |
| `--size-xxs-sm` | 12 (+test) | EmptyState, MarkdownRenderer, FrontmatterTable, Chip, Sidebar, lifecycle |
| `--size-eyebrow` | 13 (+test) | kanban, library, Page, FilterPill, Pipeline, Sidebar, lifecycle — **0094's table-cell consumer to be swept up** |
| `--size-3xs-lg` | 16 (+test) | kanban, ActivityFeed, Chip, FilterPill, Sidebar, lifecycle |
| `--size-3xs` | 4 | FilterPill, Brand, Sidebar |
| `--size-4xs` | 4 | WorkKindBadge, Pipeline, Sidebar |

Heaviest files: `Sidebar.module.css`, the `lifecycle/*` routes,
`FilterPill.module.css`, `MarkdownRenderer.module.css`. The five zero-consumer
upper-band tokens (`hero`, `h1`, `h2`, `h4`, `body`) are still renamed under the
whole-scale decision (0099 Assumptions).

### 6. ADR-0036 and the supersession lifecycle

File: `meta/decisions/ADR-0036-typography-font-size-consumption-rule.md`

Frontmatter: `id: "ADR-0036"`, `status: accepted`,
`supersedes: ["adr:ADR-0026"]`, `relates_to` includes `work-item:0075` and
`work-item:0091`. No `superseded_by` (it is the *superseding* ADR).

- **Decision** (lines 54-97): every `font-size` in current-app CSS must resolve
  to a `var(--size-*)` token; no `px`/`rem`/`em` literals (incl. `font:`
  shorthand). Scale-extension policy (70-75): new sub-pixel `-sm`/`-lg` tweens
  need design-review sign-off. Escape valve: a `FONT_SIZE_LITERAL_EXCEPTIONS`
  array beside the vitest ban.
- **Neutral consequence — px-anchoring (lines 121-128):** `--size-*` tokens are
  "intentionally px-anchored", trading user-controllable root-font scaling for
  token-value determinism; "A future review of the px-vs-rem stance remains
  open; see … work item 0091." **0099 AC7 requires the successor ADR to
  reproduce this Neutral consequence verbatim-in-spirit, still-open, linking
  `work-item:0091`** — the px-vs-rem trade-off must survive the rename.
- Authoritative enforcement is `src/styles/migration.test.ts` (ADR body lines
  88-90, 103-104).

**Supersession conventions** (from the ADR directory):

- The directory holds ADR-0001…ADR-0042 (no gaps). **The next id is ADR-0043.**
- The canonical, always-written edge is `supersedes` on the **new** ADR
  (`supersedes: ["adr:ADR-0036"]`). The **old** ADR *may* carry
  `superseded_by: "adr:ADR-0043"` — the inverse is derivable from `supersedes`.
- **Full vs partial supersession determines the old ADR's status.** ADR-0036
  *partially* superseded ADR-0026 (typography clauses only), so ADR-0026 stayed
  `status: accepted`. 0099 wants a **full** supersession (AC6: "ADR-0036 is
  marked superseded"), so ADR-0036 should transition to `status: superseded`
  with `superseded_by: "adr:ADR-0043"`.
- **Immutability tension:** ADR-0031 forbids content edits to a non-`proposed`
  ADR, and ADR-0034 argues the new ADR's `supersedes` is "the only writable
  channel". Yet ADR-0026 *was* edited in place to carry `superseded_by` + an
  in-body blockquote. This exact tension is documented in
  `meta/notes/2026-06-03-adr-0026-body-edited-in-place-breaks-immutability.md`.
  The safe reading: the successor ADR's `supersedes` edge is authoritative;
  flipping ADR-0036's `status` to `superseded` and adding `superseded_by` is the
  lifecycle-prescribed transition for a full supersession (ADR-0031 lines 73-80).
- ADR conventions are themselves ADRs (no standalone template file):
  ADR-0029 (sequential ids), ADR-0030 (template: Context, Decision Drivers,
  Considered Options, Decision, Consequences[Positive/Negative/Neutral],
  References), ADR-0031 (immutability/lifecycle), ADR-0033 (base schema),
  ADR-0034 (typed linkage), ADR-0040, ADR-0042.

### 7. Precedents and siblings

- **0075 — progenitor** (`meta/work/0075-typography-size-scale-consumption.md`,
  done). Created ADR-0036 and the mixed naming 0099 fixes. Added five tokens
  (`4xs`/`3xs` numeric tiers; `eyebrow`/`row`/`subtitle` semantic) and renamed
  the chip tokens to tween names (`--size-chip` → `--size-3xs-lg`, `--size-chip-md`
  → `--size-xxs-sm`), layering the third scheme. Kept dead tokens
  `hero`/`h1`/`h2`/`h4` as defensive scaffolding. Authored the `global.css`
  convention comment 0099 must rewrite.
- **0090 — rename + ADR + grep-gate precedent**
  (`meta/work/0090-radius-tokens-consumption.md`, done; plan
  `meta/plans/2026-06-02-0090-radius-tokens-consumption.md`). Renamed the radius
  t-shirt scale to a px-encoded ladder (`--radius-<px>` + semantic `--radius-pill`
  /`--radius-full`), via an **Authorised Deviation** in the plan that overrode
  the work item's literal AC text. Migration-first/enforcement-last, single
  atomic PR. **Key difference from 0099:** 0090 *amended* ADR-0026 §3 (clause
  level) AND created ADR-0039; 0099 does a *full* supersession of ADR-0036.
  0090 also added a Playwright `getComputedStyle` regression spec; **0099
  deliberately does NOT** add per-surface inspection (AC4 guarantees byte-identity
  "by construction"). The radius end-state is the model:
  `global.css:212-227` (declarations + comment `:212-217`),
  `tokens.ts:224-235` (`RADIUS_TOKENS`). The radius comment disambiguates its
  px-suffix from the ordinal `--sp-N` suffix — a pattern 0099's new comment
  should mirror.
- **0091 — px-vs-rem sibling** (`meta/work/0091-typography-rem-vs-px-stance.md`,
  status `ready`, spike). Decides the *unit* axis; orthogonal to 0099's *naming*
  axis. Expects to **amend** 0099's successor ADR (not fork a second successor) —
  conditional: chains off the successor ADR if 0099 has landed, else off ADR-0036
  directly. 0091's eventual unit change is small and disjoint (~19 declarations,
  not the ~140 consumers). **Stale-name warning:** after 0099 lands, the token
  names 0091 cites (`--size-h3`, `--size-prose`, `--size-body`) are stale — 0091
  must refresh by value or name.
- **0094 — originating bug** (plan
  `meta/plans/2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown.md`).
  Its table-cell rule `.markdown td code { font-size: var(--size-eyebrow); }`
  (11px) added a second consumer to the "single-purpose" `--size-eyebrow`,
  which proved no local rename could be clean → spun out 0099 (review Resolution
  lines 473-495). 0094 is **decoupled**: it consumes `--size-eyebrow` by name and
  needs no rework when 0099 renames that reference with the rest.

### 8. Untouched-by-construction surfaces

- **Design-prototype token-drift guard:**
  `src/styles/prototype-tokens.fixture.test.ts` against fixture
  `src/styles/fixtures/prototype-tokens.json`. Scope comment (lines 72-73) limits
  it to `--code-*`/`--tk-*`/`--atomic-*`. The fixture JSON has **zero `size`
  matches** (confirmed). The rename does not touch it — the expected/correct
  state.
- **Visual-regression baselines:** specs in `tests/visual-regression/`, baselines
  in `tests/visual-regression/__screenshots__/<spec>.spec.ts-snapshots/` with
  darwin/linux encoded in the filename suffix
  (`…-visual-regression-darwin.png` / `…-linux.png`). Resolved-value specs
  (`typography-resolved-sizes.spec.ts`, `*-resolved-*.spec.ts`) assert computed
  px via `getComputedStyle` — **they pin values, not token names**, so a pure
  rename leaves them green. Screenshot baselines stay byte-identical because no
  px value changes. (Per project memory, linux baselines lag darwin — but a
  value-preserving rename produces no pixel diff regardless.)

## Code References

- `skills/visualisation/visualise/frontend/src/styles/global.css:161-172` — convention comment to rewrite (AC2)
- `skills/visualisation/visualise/frontend/src/styles/global.css:173-191` — the 19 `--size-*` declarations
- `skills/visualisation/visualise/frontend/src/styles/global.css:192-197` — out-of-scope `--lh-*` / `--tracking-caps`
- `skills/visualisation/visualise/frontend/src/styles/global.css:212-227` — radius ladder end-state (the model)
- `skills/visualisation/visualise/frontend/src/styles/tokens.ts:179-208` — `TYPOGRAPHY_TOKENS` (size keys at 183-201, **no `--` prefix**)
- `skills/visualisation/visualise/frontend/src/styles/tokens.ts:224-235` — `RADIUS_TOKENS` (mirror precedent)
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:1649-1688` — var()-resolves-to-declared-token completeness gate
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:1690-1722` — AC5 aggregate `var(--*)` ratchet (`AC5_FLOOR = 989`)
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:1734-1765` — EXCEPTIONS hygiene
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:1905-1906` — ADR-0036 reference to repoint (AC8)
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:1949-2006` — font-size ban gate
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:1783-1819, 2037-2038` — hardcoded `--size-*` assertions/fixtures
- `skills/visualisation/visualise/frontend/src/styles/global.test.ts:104-119` — tokens.ts ↔ global.css parity
- `skills/visualisation/visualise/frontend/src/components/Chip/Chip.test.tsx:117-119, 198` — hardcoded `--size-3xs-lg` / `--size-xxs-sm`
- `skills/visualisation/visualise/frontend/src/components/FrontmatterTable/FrontmatterTable.test.tsx:213-214` — hardcoded `--size-xxs-sm`
- `skills/visualisation/visualise/frontend/src/styles/prototype-tokens.fixture.test.ts` + `src/styles/fixtures/prototype-tokens.json` — drift guard (no `--size-*`)
- `skills/visualisation/visualise/frontend/tests/visual-regression/typography-resolved-sizes.spec.ts` — resolved-px spec (value-pinned)
- `meta/decisions/ADR-0036-typography-font-size-consumption-rule.md:121-128` — px-anchoring Neutral consequence (carry forward → AC7)

## Architecture Insights

- **The mirror is the source of truth for the test suite, not `global.css`.** The
  completeness and parity tests read the declared key set from
  `Object.keys(TYPOGRAPHY_TOKENS)` in `tokens.ts`. `global.css` is validated
  *against* the mirror. This inverts the intuitive "CSS is canonical" model and
  is why the `--`-less mirror keys are the most dangerous omission.
- **Enforcement is a Vitest harness, not a CI grep step.** All consumption rules
  live in `migration.test.ts`, run by `mise run test → test:unit:frontend →
  vitest run`. There is no standalone ripgrep CI job. 0099's AC1/AC2 greps are
  *manual verification commands*, not standing gates; the standing gate is the
  var()-resolves test. This differs from how 0090's AC3/AC4 were *described*
  (rg sweeps) but matches how they were *implemented* (in-test regex).
- **"Byte-identical by construction" is the correct verification posture.** Each
  renamed token declares the exact px its old name carried (line-checkable
  against 0099's mapping table), and the var()-resolves gate guarantees every
  consumer resolves to one of those renamed tokens. No consumer's resolved px can
  change, so per-surface screenshot inspection is secondary — 0099 rightly omits
  the per-selector Playwright spec that 0090 added for a *value* migration.
- **Full vs partial supersession is the one genuine design decision left.** 0099
  has resolved encoding, scope, and alias-layer; the implementer must still
  enact a *full* supersession of ADR-0036 (status → `superseded`,
  `superseded_by` added), navigating the documented immutability tension, and
  ensure the successor ADR carries forward (a) the categorical font-size rule,
  (b) the px-anchoring Neutral consequence linking 0091, and arguably (c) a
  scale-extension policy adapted to the numeric scheme.
- **The successor ADR is the linchpin, not a follow-up.** It is a same-PR
  deliverable: it owns the `supersedes` edge, hosts the px-anchoring carry-forward
  that 0091 will later amend, and is where `migration.test.ts`'s authoritative
  font-size-rule reference must repoint.

## Historical Context

- `meta/work/0075-typography-size-scale-consumption.md` — created ADR-0036 and
  the three-scheme mixed naming this remap fixes; authored the convention comment.
- `meta/work/0090-radius-tokens-consumption.md` +
  `meta/plans/2026-06-02-0090-radius-tokens-consumption.md` — the rename + ADR +
  grep-gate precedent; the "Authorised Deviation" pattern for overriding literal
  AC text; the px-encoded radius ladder as the end-state model.
- `meta/work/0091-typography-rem-vs-px-stance.md` — the unit-axis sibling that
  will amend 0099's successor ADR; source of the px-anchoring obligation (AC7).
- `meta/plans/2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown.md`
  and `meta/reviews/plans/2026-06-02-0094-…-review-1.md` — the originating plan
  and the re-review pass 2 / Resolution (lines 473-495) that spun out 0099.
- `meta/decisions/ADR-0036-typography-font-size-consumption-rule.md` — to be
  superseded.
- `meta/notes/2026-06-03-adr-0026-body-edited-in-place-breaks-immutability.md` —
  documents the in-place-edit immutability tension relevant to marking ADR-0036
  superseded.
- `meta/reviews/work/0099-remap-typography-size-scale-to-pure-numeric-tokens-review-1.md`
  — the existing work-item review (verdict COMMENT).

## Related Research

- `meta/research/codebase/2026-05-23-0075-typography-size-scale-consumption.md`
  — prior investigation of the same `tokens.ts`/`global.css` size surface.
- `meta/research/codebase/2026-06-02-0090-radius-tokens-consumption.md` — the
  parallel rename-migration research.
- `meta/research/codebase/2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown.md`
  — research for the originating bug.
- `meta/research/codebase/2026-05-06-0033-design-token-system.md` — origin of the
  design-token system and original `--size-*` shape.

## Open Questions

- **Full-supersession mechanics under immutability.** 0099 AC6 mandates marking
  ADR-0036 `superseded`. The repo precedent (ADR-0026) was edited in place
  despite being `accepted`, and a note flags this as breaking ADR-0031
  immutability. The implementer (or the create-adr/review-adr skill flow) must
  decide whether to (a) follow the lifecycle transition and edit ADR-0036's
  frontmatter status + `superseded_by`, or (b) rely solely on the successor ADR's
  `supersedes` edge and leave ADR-0036 frontmatter untouched. The AC text implies
  (a); the immutability ADRs lean (b). Worth confirming with the author before
  implementation.
- **Optional declaration reordering.** Whether to reorder the `global.css` block
  into numeric order (the new scheme "sorts numerically") or preserve the current
  order to minimise diff noise. Cosmetic; value-preserving either way.
- **Scale-extension policy carry-forward.** ADR-0036's `-sm`/`-lg` tween policy
  (lines 70-75) is arguably obsoleted by the numeric scheme (any half-step is now
  trivially nameable). The 0094 review pass 3 suggested carrying *some*
  scale-extension policy forward — the successor ADR should state how new
  half-steps are added under `px×10` rather than dropping the policy silently.
