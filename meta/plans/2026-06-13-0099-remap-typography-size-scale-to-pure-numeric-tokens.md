---
type: plan
id: "2026-06-13-0099-remap-typography-size-scale-to-pure-numeric-tokens"
title: "Remap Typography Size Scale To Pure-Numeric Tokens Implementation Plan"
date: "2026-06-13T10:21:57+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0099"
parent: "work-item:0099"
derived_from: ["codebase-research:2026-06-13-0099-remap-typography-size-scale-to-pure-numeric-tokens"]
relates_to: ["work-item:0091", "adr:ADR-0036", "plan:2026-06-02-0090-radius-tokens-consumption"]
tags: [visualiser, design-tokens, typography, refactor, adr]
revision: "3817958f5589153365d2975b3052985f00872411"
repository: "visualisation-system"
last_updated: "2026-06-13T19:13:51+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Remap Typography Size Scale To Pure-Numeric Tokens Implementation Plan

## Overview

Remap the visualiser's typography `--size-*` scale from its three interleaved
naming families (t-shirt tiers, semantic single-purpose names, `-sm`/`-lg`
tweens) to a single **pure-numeric `px×10`** scheme — `--size-110` = 11px,
`--size-145` = 14.5px, `--size-680` = 68px. Every computed px value is
preserved exactly, so the change is visually inert; it is a value-preserving
rename of all 19 tokens plus their ~130 consumers. A same-PR **successor ADR
(ADR-0043)** fully supersedes ADR-0036, carrying forward the font-size
consumption rule, the px-anchoring stance (linking work-item:0091), and a
scale-extension policy adapted to the numeric scheme.

## Current State Analysis

The sub-14px band today interleaves three incompatible naming schemes at every
0.5px step, so the token name no longer communicates its value or ordering
(work item Context). The upper band (`hero`…`sm`) is more regular but is
included for whole-scale consistency.

Verified against the working tree (rev `3817958`, slightly ahead of the
research commit `ad582c8` — work item 0095 landed in between, shifting consumer
counts but nothing structural):

- **Token declarations** — `skills/visualisation/visualise/frontend/src/styles/global.css:161-191`:
  a 12-line convention comment (`:161-172`, references ADR-0036, documents all
  three retired families) followed by 19 literal-`px` declarations (`:173-191`)
  in a single `:root` block. **No `calc()`, no `var()` indirection, no
  dark-mode/media-query overrides** of any `--size-*` token — the declaration
  site is the sole value source.
- **Mirror** — `tokens.ts:179-208`, `TYPOGRAPHY_TOKENS` (`as const`). The size
  keys (`:183-201`) carry **no `--` prefix** (`"size-hero": "68px"`). A sweep
  based on `--size-` misses the mirror entirely — yet the mirror is the source
  the completeness/parity tests read from. **This is the #1 find-replace trap.**
- **Consumers** — 130 `var(--size-*)` references across the `*.module.css`
  files (current per-token counts in Technical Notes). `hero`, `h1`, `h2`,
  `h4`, `body` have **zero** consumers (declared-only scaffolding kept by 0075);
  they are still renamed under the whole-scale decision. No production
  `.ts`/`.tsx` references `--size-*` (confirmed) — no dynamic/computed names.
- **Guardrails** — `migration.test.ts` / `global.test.ts`:
  - `var()`-resolves-to-declared-token (`migration.test.ts:1649-1688`) —
    auto-tracks the declared key set from `Object.keys(TYPOGRAPHY_TOKENS)` etc.
    This is the **rename-completeness oracle**.
  - `global.css ↔ tokens.ts` parity (`global.test.ts:104-119`) — auto-tracks via
    `Object.entries(TYPOGRAPHY_TOKENS)`; order-agnostic.
  - AC5 aggregate `var(--*)` ratchet (`migration.test.ts:1690-1722`,
    `AC5_FLOOR=989`) — name-agnostic, counts the `var(--` prefix; a rename
    preserves the count, no floor bump.
  - font-size literal ban (`migration.test.ts:1949-2006`) — bans raw literals,
    name-agnostic; **but** its lone ADR reference (`:1905-1906`,
    `ADR-0036`) is prose, **not assertion-enforced**, so it must be repointed by
    hand or it silently goes stale.
- **Hardcoded `--size-*` name strings in tests** (a CSS-only sweep misses these;
  they turn the suite red on rename): `migration.test.ts` (1784, 1796, 1817,
  2037-2038, plus comment prose at 1156/1795/1804/1807), `Chip.test.tsx`
  (117/119, 198), `FrontmatterTable.test.tsx` (213-214). **This is the #2
  find-replace trap.**
- **ADR lifecycle** — `ADR-0031:73-80`: `accepted → superseded` via
  `create-adr --supersedes` writes `status` + `superseded_by` atomically; this
  *is* the one permitted edit on an accepted ADR (distinct from the
  body-content immutability tension noted for ADR-0026). The next ADR id is
  **ADR-0043**. `ADR-0039` (border-radius) is the structural model: px-encoded
  ladder + scale-extension + value-mutation policies, with a comment
  disambiguating the px suffix from the ordinal `--sp-N` suffix.

## Desired End State

Every `--size-*` token — declaration, mirror key, and consumer — uses the
pure-numeric `--size-<px×10>` scheme; the retired names exist nowhere as
*names*. The `global.css` convention comment describes the numeric encoding and
references ADR-0043. ADR-0043 is accepted and fully supersedes ADR-0036
(`status: superseded`, `superseded_by: adr:ADR-0043`), carrying forward the
font-size consumption rule, the px-anchoring Neutral consequence linking
work-item:0091, and a numeric scale-extension policy. The full vitest +
Playwright suites pass; screenshot baselines are byte-identical because no px
value changes.

**Verify the end state**: AC1/AC2 greps return zero matches; `mise run
test:unit:frontend` and `mise run test:e2e:visualiser` are green; ADR-0043 and
ADR-0036's frontmatter reflect the supersession.

### Key Discoveries:

- The mirror keys carry **no `--` prefix** (`tokens.ts:183-201`) — the single
  biggest omission risk. The completeness and parity tests read the declared
  key set from this mirror.
- Two retired names are **substrings** of two others: `--size-3xs` ⊂
  `--size-3xs-lg`, and `--size-xxs` ⊂ `--size-xxs-sm`. A naïve global replace of
  the shorter name corrupts the longer one. Anchor each replacement on its
  delimiter (`)` for `var(--size-NAME)` consumers, `:` for declarations, `":`
  for mirror keys) — this is collision-safe regardless of order.
- The `var()`-resolves-to-declared test (`migration.test.ts:1649-1688`) is the
  executable completeness gate: any stale `var(--size-OLD)`, or a renamed
  declaration with a surviving old consumer, fails it. No edit to this block is
  required — it self-adjusts to the mirror's keys.
- The font-size ban's `ADR-0036` reference (`migration.test.ts:1905`) is the
  **only** ADR-0036 reference in either test file and is **not**
  assertion-enforced — it must be repointed explicitly (Phase 1).
- `migration.test.ts:2027` `"--size-foo: 11px;"` is an intentional fictional
  placeholder in `NEGATIVE_LITERAL` — **leave it untouched**.

## What We're NOT Doing

- **No semantic alias layer.** Consumers reference numeric tokens directly; the
  retired semantic names (`eyebrow`/`row`/`subtitle`/`prose`) disappear as
  names.
- **No value changes.** Pure rename; every computed px is preserved. No new
  tokens, no removed values.
- **No unit change.** px-vs-rem is work-item:0091's axis; this changes names
  only. The px-anchoring trade-off is *carried forward* in ADR-0043, not
  re-decided.
- **No per-surface Playwright regression spec** of the kind 0090 added for its
  *value* migration. AC4 guarantees byte-identity "by construction"; the
  existing resolved-size specs (value-pinned) and screenshot baselines are
  secondary confirmation.
- **No new CI grep gate.** Unlike 0090, the standing gate is the in-test
  `var()`-resolves oracle; AC1/AC2 greps are manual verification commands.
- **No changes to non-size typography tokens** (`--lh-*`, `--tracking-caps`) or
  any other token family.
- **No edits to ADR-0036's body.** Only its frontmatter status transition
  (handled by the create-adr lifecycle).

## Implementation Approach

Two phases, both landing in a **single PR** (per the work item's same-PR
artefact coupling — the ADR "must land with the rename rather than trailing
it"), sequenced **Phase 1 → Phase 2** and each individually green so they are
reviewable/revertible as discrete commits:

1. **Phase 1 — Successor ADR + supersede ADR-0036 + repoint the authoritative
   reference.** Authored via the `create-adr`/`review-adr` skills (interactive,
   per the chosen approach), plus a one-line comment repoint. Pure docs +
   comment; touches no token names, so the suite stays green. Establishes the
   ADR-0043 id that Phase 2's `global.css` comment must cite.
2. **Phase 2 — Mechanical whole-scale rename (atomic).** All declarations
   (reordered numerically), the convention comment, the mirror, all ~130
   consumers, and every hardcoded test string in **one** change. The rename
   cannot be sub-split: the `var()`-resolves invariant forbids any intermediate
   state where a renamed declaration coexists with an old consumer (or vice
   versa).

**TDD posture.** The rename is value-preserving, so the existing guardrails are
the executable spec rather than newly-authored tests. Where natural, edit the
hardcoded test assertions to the *target* names first (they go red against the
old CSS), then perform the find-replace to bring them — together with the
auto-tracking `var()`-resolves and parity gates — to green.

## Phase 1: Successor ADR (ADR-0043) + supersede ADR-0036

### Overview

Create ADR-0043 documenting the pure-numeric scheme and fully superseding
ADR-0036, via the `create-adr --supersedes` → `review-adr` flow. Repoint the
single authoritative ADR reference in the test suite. This phase is performed
interactively (not by automated plan edits) and is independently green and
mergeable.

### Changes Required:

#### 1. Author ADR-0043 via the create-adr skill

Run `/accelerator:create-adr` with `--supersedes ADR-0036` (then
`/accelerator:review-adr` to accept). The lifecycle skill writes ADR-0036's
`status: superseded` + `superseded_by: adr:ADR-0043` atomically (`ADR-0031:76`)
— do **not** hand-edit ADR-0036's body. Model the structure on
`ADR-0039-border-radius-consumption-rule.md`.

> **ADR id**: `ADR-0043` is the *assumed* next id (ADR-0042 is current latest).
> `create-adr` allocates the real id at creation — if it differs, substitute the
> actual id everywhere the literal `ADR-0043` appears (below, in Phase 2 §2's
> `global.css` comment, and in the Phase 1 §2 test repoint), and re-run the
> Phase 1 grep ACs against the allocated id.

ADR-0043 must contain:

- **Title**: e.g. "ADR-0043: Pure-numeric typography size-token naming".
- **Context**: ADR-0036's scale interleaves three naming families (t-shirt
  tiers, semantic single-purpose, `-sm`/`-lg` tweens) at every 0.5px step, so a
  token name no longer communicates its value or ordering. Remap to `px×10`.
- **Decision**:
  - The pure-numeric `--size-<px×10>` naming scheme (no zero-padding;
    variable-width names; sorts numerically).
  - **Carry forward the font-size consumption rule** verbatim-in-spirit: every
    `font-size` in current-app CSS must resolve to a `var(--size-*)` token; no
    `px`/`rem`/`em` literals (incl. `font:` shorthand). ADR-0043 becomes the
    authoritative typography consumption ADR.
  - **Scale-extension policy adapted to numeric** (mirror ADR-0039's "extend
    the ladder, not tolerance-band substitution"): any 0.5px step is now
    trivially nameable under `px×10`; new steps still require design-review
    sign-off rather than ad-hoc literals. State this explicitly rather than
    silently dropping ADR-0036's `-sm`/`-lg` tween policy. **Ease-of-naming is
    not ease-of-admission**: px×10 removes the awkward-name friction that
    previously discouraged off-tier half-steps, so the prose design-review
    policy is now the *sole* guard against scale proliferation.
  - **Value-mutation policy** (mirror ADR-0039): because the name encodes the
    value, a value change is made by adding a new step and re-pointing
    consumers, never by mutating a token's value in place.
  - **Escape valve**: the `FONT_SIZE_LITERAL_EXCEPTIONS` array carried forward
    **with its rationale** — ADR-0036's "Why a separate array from EXCEPTIONS"
    distinction (EXCEPTIONS = the steady-state per-occurrence admission ledger;
    `FONT_SIZE_LITERAL_EXCEPTIONS` = the near-empty category-level escape valve)
    must survive, not just the array, so the two-ledger policy intent is not
    lost. AC8 keeps both guards live.
- **Scope of supersession**: ADR-0043 **fully** supersedes ADR-0036 — unlike
  ADR-0039's *partial* supersession of ADR-0026 (state the granularity
  explicitly: ADR-0036 is entirely re-expressed here, so a clean full transition
  to `superseded` is correct, whereas ADR-0026 retained non-typography clauses).
  Also state that ADR-0043 assumes governance of the ADR-0026 typography clauses
  ADR-0036 previously superseded (ADR-0026 §2 Typography + §3 em-relative /
  heading-size rows), so the live chain ADR-0026 → ADR-0036 → ADR-0043 stays
  discoverable from the live end — ADR-0036's body is immutable and cannot be
  updated to point forward. ADR-0043 keeps `supersedes: ["adr:ADR-0036"]` only
  (it does **not** fully supersede ADR-0026, which retains non-typography
  clauses); unlike ADR-0039 — which carried a direct `supersedes: [adr:ADR-0026]`
  edge for its radius row — ADR-0043 records the ADR-0026 typography-clause
  governance via this prose plus an `adr:ADR-0026` `relates_to` edge (a
  first-class link from the live node, surviving any superseded-node pruning,
  without over-claiming a full ADR-0026 supersession). Call out this deliberate
  divergence from the ADR-0039 model explicitly so it is not silent.
- **Consequences → Negative**: record the one-time vocabulary-relearning cost of
  abandoning the t-shirt / semantic names (mirroring ADR-0039's equivalent),
  with the `global.css` comment cited as the self-introducing mitigation. Record
  the discoverability trade-off honestly: numeric names trade intent-discovery
  ("which token do I want?") for value-recovery — acceptable because the retired
  semantic names already misdescribed most consumers. Cite 2-3 (e.g.
  `--size-subtitle` 13px on toast bodies and sidebar search rows; `--size-eyebrow`
  11px on count indicators; a `WorkItemCard` rule whose own comment admits it
  picked "the nearest token"), reframing the change as removing a *false* signal
  (mirroring ADR-0039's `--radius-block` analysis).
- **Consequences → Neutral (AC7)**: restate ADR-0036's px-anchoring stance as
  **still-open**, linking `work-item:0091` — the px-vs-rem trade-off survives
  the rename rather than being silently re-decided. work-item:0091 will later
  resolve the unit axis, likely by *superseding* ADR-0043 with a further
  successor ADR (**not** amending it — per ADR-0031 an accepted ADR is immutable
  and can only be superseded, so the unit decision re-states the consumption
  rule in its successor).
- **Frontmatter**: `supersedes: ["adr:ADR-0036"]`; `relates_to` to include
  `adr:ADR-0026` (a first-class live-node edge for the typography-clause
  provenance — see Scope of supersession), `work-item:0099`, `work-item:0075`,
  `work-item:0091` (and ADR-0030/0031/0034 per template convention). `create-adr`
  writes the frontmatter; when hand-verifying, check against the on-disk `id:` /
  `supersedes:` / `relates_to:` shape used by ADR-0036/0039, not ADR-0030's prose
  field names.
- **References**: ADR-0036 (superseded), ADR-0039 (sibling consumption-rule
  whose shape is mirrored), ADR-0030/0031/0034, work items 0099/0075/0091.

#### 2. Repoint the authoritative ADR reference in the test suite

**File**: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`
**Changes**: at `:1905-1906`, change the comment "AUTHORITATIVE IMPLEMENTATION
of ADR-0036's font-size consumption rule" → "…of ADR-0043's…" (use the real
allocated id if not ADR-0043). This reference is prose (not assertion-enforced);
the font-size ban itself is name-agnostic and stays green. Pointing it at
ADR-0043 is valid immediately — ADR-0043 carries the consumption rule, which
holds independent of token names. **Touch only the ADR id on this line** —
leave the surrounding comment's AC2 sweep guidance (`:1918-1920`) and the
`FONT_SIZE_LITERAL_RE` / `FONT_SHORTHAND_RE` regexes unchanged (they are
token-name-agnostic; the in-same-commit re-derivation obligation at `:1915` is
*not* triggered because no regex is structurally edited).

### Success Criteria:

#### Automated Verification:

- [ ] Full vitest suite stays green (no token names changed yet):
      `mise run test:unit:frontend`
- [ ] `migration.test.ts` no longer references `ADR-0036`:
      `grep -n 'ADR-0036' skills/visualisation/visualise/frontend/src/styles/migration.test.ts` returns nothing
- [ ] `migration.test.ts` now references the successor ADR (positive check, not
      just ADR-0036 absence):
      `grep -n 'ADR-0043' skills/visualisation/visualise/frontend/src/styles/migration.test.ts` returns the repointed comment line
- [ ] ADR-0043 exists and is accepted; `grep -n 'status:' meta/decisions/ADR-0043-*.md` shows `accepted`
- [ ] ADR-0036 marked superseded: `grep -nE 'status: superseded|superseded_by: "adr:ADR-0043"' meta/decisions/ADR-0036-typography-font-size-consumption-rule.md` shows both

#### Manual Verification:

- [ ] ADR-0043 reads coherently against the ADR-0030 template and the ADR-0039
      model (Context, Decision Drivers, Considered Options, Decision,
      Consequences[Positive/Negative/Neutral], References).
- [ ] AC5: ADR-0043 created and supersedes ADR-0036.
- [ ] AC6: ADR-0036 marked superseded per the lifecycle; ADR-0043 owns the
      `supersedes` edge; ADR-0036's body is unedited.
- [ ] AC7: ADR-0043's Neutral consequence restates px-anchoring as still-open
      and links `work-item:0091`.
- [ ] ADR-0043 carries forward the font-size consumption rule and a numeric
      scale-extension policy (not silently dropped).

---

## Phase 2: Mechanical whole-scale rename (atomic)

### Overview

Rename all 19 `--size-*` tokens to `px×10` across every surface in one atomic
change: declarations (reordered numerically), convention comment, mirror, all
~130 consumers, and every hardcoded test string. Value-preserving; the
auto-tracking guardrails are the completeness oracle.

### The full rename mapping (preserve every px value):

| px | old token | new token |
|---|---|---|
| 68 | `--size-hero` | `--size-680` |
| 48 | `--size-h1` | `--size-480` |
| 36 | `--size-h2` | `--size-360` |
| 28 | `--size-h3` | `--size-280` |
| 26 | `--size-h4` | `--size-260` |
| 22 | `--size-lg` | `--size-220` |
| 20 | `--size-body` | `--size-200` |
| 18 | `--size-md` | `--size-180` |
| 16 | `--size-sm` | `--size-160` |
| 14.5 | `--size-prose` | `--size-145` |
| 14 | `--size-xs` | `--size-140` |
| 13 | `--size-subtitle` | `--size-130` |
| 12.5 | `--size-row` | `--size-125` |
| 12 | `--size-xxs` | `--size-120` |
| 11.5 | `--size-xxs-sm` | `--size-115` |
| 11 | `--size-eyebrow` | `--size-110` |
| 10.5 | `--size-3xs-lg` | `--size-105` |
| 10 | `--size-3xs` | `--size-100` |
| 9.5 | `--size-4xs` | `--size-95` |

> **Substring trap**: replace `--size-3xs-lg`/`--size-xxs-sm` (and their mirror
> keys) **before** `--size-3xs`/`--size-xxs`, or anchor each replace on its
> delimiter (`)`, `:`, or `":`) — which is collision-safe in any order.

### Changes Required:

#### 1. global.css declarations — reorder numerically + rename

**File**: `skills/visualisation/visualise/frontend/src/styles/global.css:173-191`
**Changes**: replace the 19 declarations with the numerically-sorted,
renamed block (descending by value; fixes the current `--size-prose`
out-of-order quirk):

```css
  --size-680:         68px;
  --size-480:         48px;
  --size-360:         36px;
  --size-280:         28px;
  --size-260:         26px;
  --size-220:         22px;
  --size-200:         20px;
  --size-180:         18px;
  --size-160:         16px;
  --size-145:         14.5px;
  --size-140:         14px;
  --size-130:         13px;
  --size-125:         12.5px;
  --size-120:         12px;
  --size-115:         11.5px;
  --size-110:         11px;
  --size-105:         10.5px;
  --size-100:         10px;
  --size-95:          9.5px;
```

> **Alignment**: the Biome CSS formatter is disabled (formatting `global.css`
> desyncs it from the `tokens.ts` mirror and breaks the CSS-source-asserting
> tests), so this value-column alignment is hand-maintained. Keep it internally
> consistent — the new block aligns values one column left of the retired block
> because every new name is shorter than the retired `--size-subtitle`.

#### 2. global.css convention comment (AC2)

**File**: `skills/visualisation/visualise/frontend/src/styles/global.css:161-172`
**Changes**: replace with a comment describing the `px×10` encoding, with a
whole-step and a half-step example, referencing ADR-0043, and disambiguating the
suffix from the ordinal `--sp-N` suffix (mirroring ADR-0039's radius comment).
It must contain **no** retired `--size-<name>` token name:

```css
  /*
   * font-size consumers: use these tokens — see ADR-0043. Rule: every
   * `font-size` in current-app CSS must resolve to one of these tokens; no
   * `px`/`rem`/`em` literals (including `font:` shorthand).
   *
   * Naming: every size token is pure-numeric, encoding its px value ×10
   * with no zero-padding — `--size-<px×10>`. Whole-step example: 11px →
   * `--size-110`. Half-step example: 14.5px → `--size-145` (so the smallest
   * token `--size-95` is 9.5px, NOT 95px). Names are therefore variable-width
   * — `--size-95` (9.5px, two digits) sits alongside `--size-110` (11px,
   * three digits) — and sort numerically by name. Any 0.5px step is nameable
   * without a new naming family.
   * NB: the ×10 suffix is a px-derived value, NOT the ordinal step index used
   * by the sibling `--sp-N` scale, NOR the ×1 px value used by the sibling
   * `--radius-N` scale: 12px is `--size-120`, `--sp-3`, and `--radius-12`
   * respectively — three numeric-suffix families, three numbering bases.
   */
```

#### 3. tokens.ts mirror — rename keys (no `--` prefix!) + reorder to match

**File**: `skills/visualisation/visualise/frontend/src/styles/tokens.ts:183-201`
**Changes**: rename each `"size-<old>"` key to `"size-<px×10>"` (preserving the
`"68px"` etc. values), and reorder to match the `global.css` block for
readability (parity is order-agnostic, so this is cosmetic but consistent):

```ts
  "size-680": "68px",
  "size-480": "48px",
  "size-360": "36px",
  "size-280": "28px",
  "size-260": "26px",
  "size-220": "22px",
  "size-200": "20px",
  "size-180": "18px",
  "size-160": "16px",
  "size-145": "14.5px",
  "size-140": "14px",
  "size-130": "13px",
  "size-125": "12.5px",
  "size-120": "12px",
  "size-115": "11.5px",
  "size-110": "11px",
  "size-105": "10.5px",
  "size-100": "10px",
  "size-95": "9.5px",
```

#### 4. CSS consumers — mechanical per-token rename (~130 refs)

**Files**: 38 `*.module.css` files under
`skills/visualisation/visualise/frontend/src/`.
**Changes**: replace every `var(--size-<old>)` with `var(--size-<px×10>)` per
the mapping. **Anchor each replacement on its closing `)`** — e.g. find
`var(--size-3xs)` → `var(--size-100)` and find `var(--size-3xs-lg)` →
`var(--size-105)`; the `)` anchor makes the substring pair collision-safe in
*either* order. This anchored form is **mandatory**, not one option among
several — an un-anchored shorter-first replace would corrupt `--size-3xs-lg`
into `--size-100-lg`. The per-token consumer counts below are
**non-authoritative orientation only** (they predate work-item:0095 and read
like a checklist but are not one) — re-count at implementation time, and treat
the green `var()`-resolves oracle plus the src-wide retired-name grep as the
sole completeness evidence:

| old token → new | refs | old token → new | refs |
|---|---|---|---|
| `xxs` → `120` | 25 | `row` → `125` | 5 |
| `3xs-lg` → `105` | 23 | `4xs` → `95` | 4 |
| `xs` → `140` | 19 | `3xs` → `100` | 4 |
| `eyebrow` → `110` | 15 | `md` → `180` | 2 |
| `xxs-sm` → `115` | 12 | `lg` → `220` | 2 |
| `subtitle` → `130` | 10 | `h3` → `280` | 2 |
| `sm` → `160` | 6 | `prose` → `145` | 1 |

`hero`/`h1`/`h2`/`h4`/`body` have zero `var()` consumers (renamed at declaration
only). Heaviest files: `Sidebar.module.css`, the `routes/lifecycle/*` modules,
`FilterPill.module.css`, `MarkdownRenderer.module.css`.

#### 5. Hardcoded test strings (the #2 trap — a CSS sweep misses these)

**File**: `migration.test.ts`
- `:1783-1784` — assertion + description `var(--size-prose)` → `var(--size-145)`
- `:1795` — comment "not --size-xs" → "not --size-140"
- `:1796` — `var(--size-xxs-sm)` → `var(--size-115)`
- `:1804` — comment "no longer sizes off --size-xs" → "--size-140"
- `:1807` — comment "legitimate var(--size-xs)" → "var(--size-140)"
- `:1811` — negative assertion `not.toContain("var(--size-xs)")` → `("var(--size-140)")`
- `:1817` — `var(--size-eyebrow)` → `var(--size-110)`
- `:1156` — `reason` prose "equals --size-lg" → "equals --size-220" (cosmetic, unvalidated)
- `:2037-2038` — fixtures `var(--size-xxs)` → `var(--size-120)` (regex-neutral; updated for consistency)
- **Leave `:2027` `"--size-foo: 11px;"` untouched** (intentional fictional placeholder)

**File**: `components/Chip/Chip.test.tsx`
- `:117` description "binds base font-size to --size-3xs-lg" → "--size-105"
- `:119` regex `var\(--size-3xs-lg\)` → `var\(--size-105\)`
- `:198` regex `var\\(--size-xxs-sm\\)` → `var\\(--size-115\\)`

**File**: `components/FrontmatterTable/FrontmatterTable.test.tsx`
- `:213` description "uses --size-xxs-sm" → "--size-115"
- `:214` regex `var\(--size-xxs-sm\)` → `var\(--size-115\)`

**File**: `styles/testing/extractAcDeclarations.test.ts` — **required** (under
`src/`, so in scope of the src-wide retired-name grep; not optional)
- `:35` — `--size-md: 18px;` → `--size-180: 18px;` (a non-`--ac-*` "skip"
  fixture; keeps a real token name so the AC's zero-match grep stays clean)

#### 6. Optional consistency edits (no functional effect)

These two live under `tests/` — *outside* the src-wide retired-name grep's root
— so they are genuinely optional (skipping them leaves stale names only in spec
comments, which no gate checks). `extractAcDeclarations.test.ts` is **not** here
— it is under `src/` and is a required edit (see §5).

- `tests/visual-regression/typography-resolved-sizes.spec.ts:132-133` — comment
  `--size-subtitle 13, --size-3xs-lg 10.5, --size-4xs 9.5` →
  `--size-130 13, --size-105 10.5, --size-95 9.5`.
- `tests/visual-regression/inline-code-resolved-styles.spec.ts:99` — comment
  `var(--size-xs)` → `var(--size-140)`.

#### 7. Add a px×10 value-preservation assertion (closes the AC4 gap)

**File**: `skills/visualisation/visualise/frontend/src/styles/global.test.ts`,
co-located with the existing `global.css ↔ tokens.ts` parity block.
**Changes**: add an invariant that, **for each size token**, decodes its numeric
name and asserts it equals the declared value, so a value typo *duplicated*
across `global.css` and the mirror (which the parity test alone would pass)
turns the suite red. `TYPOGRAPHY_TOKENS` **also contains non-size keys**
(`ac-font-*`, `lh-*`, `tracking-caps`), so the loop **must** filter to `size-N`
keys first — an unfiltered `Number(key.slice(5))` is `NaN` for those and would
fail the test on a *correct* rename:

```ts
describe("size tokens decode to their declared px value", () => {
  const sizeEntries = Object.entries(TYPOGRAPHY_TOKENS).filter(([k]) =>
    /^size-\d+$/.test(k),
  );
  it.each(sizeEntries)("%s decodes to px×10 of its name", (key, value) => {
    const px = Number(key.slice("size-".length)) / 10;
    expect(Number.parseFloat(value)).toBe(px); // value e.g. "14.5px"
    expect(value).toBe(`${px}px`); // and no stray unit / precision drift
  });
});
```

This is the only new test the rename adds; it keeps the "guardrails-are-the-spec"
posture and makes value-preservation machine-checkable rather than
diff-review-dependent. It **composes with the parity gate** (parity proves
mirror == CSS; this proves mirror == decoded-name; transitively CSS ==
decoded-name), so co-locating the two keeps that dependency legible. It does
**not** catch a consumer pointed at the wrong-but-declared token — that residual
gap is accepted; the value-pinned Playwright resolved-size specs remain the
secondary net for it.

### Success Criteria:

#### Automated Verification:

- [ ] **AC1** — no old size-token names declared:
      `rg -nP -- '^\s*--size-(?![0-9]+\s*:)[\w-]+\s*:' skills/visualisation/visualise/frontend/src/styles/global.css`
      returns zero matches. (The regex flags any declared `--size-` token whose
      name is not `--size-<digits>:` — sufficient for this rename set; it does
      not assert full-name numericness in the general case.)
      **Path note**: all verification commands here use repo-root-relative paths
      (`skills/…/frontend/src/…`), the runnable form since mise tasks run from
      the repo root — this intentionally supersedes the work item's
      frontend-relative `src/…` AC paths.
- [ ] **AC2** — convention comment carries no retired name:
      `rg -nP -- '--size-(hero|h1|h2|h3|h4|lg|body|md|sm|prose|xs|subtitle|row|xxs|xxs-sm|eyebrow|3xs-lg|3xs|4xs)\b' skills/visualisation/visualise/frontend/src/styles/global.css`
      returns zero matches.
- [ ] No retired name survives anywhere in the frontend `src` (declarations,
      consumers, tests):
      `rg -nP -- '--size-(hero|h1|h2|h3|h4|lg|body|md|sm|prose|xs|subtitle|row|xxs|xxs-sm|eyebrow|3xs-lg|3xs|4xs)\b' skills/visualisation/visualise/frontend/src`
      returns zero matches. (The alternation lists only retired names, so the
      intentional `--size-foo` placeholder at `migration.test.ts:2027` cannot
      match — no exclusion clause needed; just leave `:2027` untouched per §5.)
- [ ] **AC3** — `var()`-resolves-to-declared-token passes (no stale refs);
      **AC8** — font-size ban, EXCEPTIONS hygiene, and AC5 ratchet all green and
      executed (not skipped/`.only`/deleted): `mise run test:unit:frontend`
- [ ] Value-preservation invariant (§7) passes — each `size-<px×10>` mirror key
      decodes to its declared px value: `mise run test:unit:frontend`
- [ ] Lint, format, and types pass: `mise run frontend:check`
- [ ] **AC4** — Playwright suite green incl. value-pinned resolved-size specs;
      screenshot baselines byte-identical: `mise run test:e2e:visualiser`

#### Manual Verification:

- [ ] **AC4** — spot-check 2-3 surfaces (e.g. a Chip, a FrontmatterTable cell,
      a Sidebar label) render at identical sizes to before.
- [ ] The `global.css` block reads cleanly in numeric order; the convention
      comment states the encoding with both a whole-step and a half-step
      example and cites ADR-0043.
- [ ] Diff review confirms the change is a pure rename (no value drift, no
      stray token introduced).

---

## Testing Strategy

### Unit / integration (vitest — `mise run test:unit:frontend`):

- The `var()`-resolves-to-declared-token gate (`migration.test.ts:1649-1688`) is
  the **primary completeness oracle**: it auto-tracks the mirror's keys, so a
  missed consumer or missed mirror key fails here with a "declared vs consumed"
  mismatch.
- `global.css ↔ tokens.ts` parity (`global.test.ts:104-119`) catches a
  declaration/mirror divergence (e.g. renamed in one but not the other).
- Component suites (`Chip.test.tsx`, `FrontmatterTable.test.tsx`) and the
  hardcoded `migration.test.ts` assertions catch the test-string trap.
- AC5 ratchet, font-size ban, EXCEPTIONS hygiene must stay green and enabled.
- **New**: the px×10 value-preservation invariant (Phase 2 §7) decodes each
  numeric **size**-token name (filtering out the non-size `ac-font-*`/`lh-*`/
  `tracking-caps` keys) and asserts it equals its declared px value — catching a
  value typo duplicated across `global.css` and the mirror that the parity test
  alone would pass.

### End-to-end / visual (Playwright — `mise run test:e2e:visualiser`):

- Value-pinned resolved-size specs (`typography-resolved-sizes.spec.ts`,
  `*-resolved-*.spec.ts`) assert computed px via `getComputedStyle` — they pin
  values, not names, so they confirm byte-identity.
- Screenshot baselines remain byte-identical (no px changes). Per project
  memory, linux baselines lag darwin, but a value-preserving rename produces no
  pixel diff regardless.

### Manual testing steps:

1. Run the visualiser locally and confirm typography looks unchanged on a
   Kanban card, a library doc view, and a lifecycle route.
2. Inspect a renamed consumer in devtools — confirm `font-size` resolves to the
   numeric token and the same computed px.

## Performance Considerations

None. A token rename has no runtime cost; the computed CSS is identical.

## Migration Notes

- **Atomicity**: Phase 2 must land as one change — the `var()`-resolves
  invariant has no green intermediate for a partial rename, and the work item
  rejects a transitional alias layer.
- **Same-PR coupling**: Phases 1 and 2 ship together (work item Dependencies),
  Phase 1 first so the ADR-0043 id exists for the `global.css` comment.
- **Recovery**: jj/VCS revert of the PR is the rollback path; no data migration.
- **Downstream**: after this lands, the names work-item:0091 cites
  (`--size-h3`, `--size-prose`, `--size-body`) are stale — 0091 must refresh by
  value or new name. 0091 **already self-documents** this in its own Technical
  Notes (it defers the refresh to itself), so this PR is **not** expected to edit
  0091. 0091's unit decision will *supersede* ADR-0043 with a successor ADR (not
  amend it — per ADR-0031 immutability), per the reconciliation in Phase 1.
- **Follow-up (out of scope)**: the font-size-ban gate's link to its governing
  ADR is unchecked prose, so it can silently rot on future supersessions; a
  later hardening could assert the cited ADR id is not a `superseded` one.

## References

- Work item: `meta/work/0099-remap-typography-size-scale-to-pure-numeric-tokens.md`
- Research: `meta/research/codebase/2026-06-13-0099-remap-typography-size-scale-to-pure-numeric-tokens.md`
- ADR to supersede: `meta/decisions/ADR-0036-typography-font-size-consumption-rule.md`
- Structural model for ADR-0043: `meta/decisions/ADR-0039-border-radius-consumption-rule.md`
- ADR lifecycle: `meta/decisions/ADR-0031-skill-level-adr-immutability.md:73-80`
- Rename precedent: `meta/plans/2026-06-02-0090-radius-tokens-consumption.md`
- Tokens: `skills/visualisation/visualise/frontend/src/styles/global.css:161-191`,
  `…/src/styles/tokens.ts:179-208`
- Guardrails: `…/src/styles/migration.test.ts:1649-1722,1905-2006`,
  `…/src/styles/global.test.ts:104-119`
- Unit-axis sibling: `meta/work/0091-typography-rem-vs-px-stance.md`
