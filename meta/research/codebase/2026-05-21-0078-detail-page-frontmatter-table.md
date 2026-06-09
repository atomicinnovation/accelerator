---
date: "2026-05-21T19:46:26+01:00"
researcher: Toby Clemson
revision: "64eca1bf99c3b311862da9df1baf1095b43ca4a7"
repository: accelerator
topic: "Detail-Page Frontmatter Table (work item 0078)"
tags: [research, codebase, frontmatter, detail-page, frontend, visualiser, wiki-links]
status: complete
last_updated: "2026-05-21T00:00:00+00:00"
last_updated_by: Toby Clemson
type: codebase-research
id: "2026-05-21-0078-detail-page-frontmatter-table"
title: "Research: Detail-Page Frontmatter Table (0078)"
author: Toby Clemson
schema_version: 1
relates_to: ["adr:ADR-0026", "codebase-research:2026-05-15-0041-library-page-wrapper-and-overview-hub", "codebase-research:2026-05-16-0041-library-page-wrapper-supplementary", "codebase-research:2026-05-14-0038-generic-chip-component", "codebase-research:2026-05-06-0033-design-token-system", "plan:2026-05-06-0033-design-token-system", "design-gap:2026-05-21-current-app-vs-claude-design-prototype", "work-item:0078"]
derived_from: ["codebase-research:2026-05-15-0041-library-page-wrapper-and-overview-hub", "codebase-research:2026-05-16-0041-library-page-wrapper-supplementary", "codebase-research:2026-05-14-0038-generic-chip-component", "codebase-research:2026-05-06-0033-design-token-system", "design-gap:2026-05-21-current-app-vs-claude-design-prototype"]
---

# Research: Detail-Page Frontmatter Table (0078)

**Date**: 2026-05-21T19:46:26+01:00
**Researcher**: Toby Clemson
**Git Commit**: 64eca1bf99c3b311862da9df1baf1095b43ca4a7
**Branch**: build-system
**Repository**: accelerator

## Research Question

What does the codebase look like today for work item 0078 — adding a CSS-grid
frontmatter table above the markdown body on every detail-page route, with
work-item-ID values auto-linkified? Specifically:

- Where does a parallel `FrontmatterTable` component live and how does it
  follow established conventions?
- How does the existing `FrontmatterChips` render frontmatter values, and
  what parts of that logic can be reused vs. must diverge?
- Where in `LibraryDocView` should the new table mount, and what is the
  layout/grid contract it must respect?
- How does `useWikiLinkResolver` work, and can it linkify bare scalar values
  like `WORK-0041` from frontmatter values?
- Which CSS tokens and typography variables exist for the prototype-style
  appearance (sunken bg, stroke border, Fira Code, muted text)?
- What constraints do related work items (0041, 0084, 0085, 0088) impose?

## Summary

Implementation is straightforward in shape (new component, single mount-point
edit, reuse the parsed frontmatter object) but the **story contains three
factual gaps** vs. the codebase that need correcting before implementation:

1. **CSS variable mismatch.** The story references `--ac-text-muted` and (by
   implication) `--ac-bg-sunken` / `--ac-stroke` as if all three exist. Only
   `--ac-bg-sunken` and `--ac-stroke` exist — the muted-text token in this
   codebase is `--ac-fg-muted` (the `--ac-fg-*` family is the foreground
   namespace; there is no `--ac-text-*` family).
2. **Wiki-link resolver pattern coverage.** The story assumes the existing
   resolver will linkify bare scalars like `WORK-0041` and project-prefixed
   forms like `PROJ-0041`. The resolver as built only matches the
   bracketed wiki-link syntax `[[ADR-NNNN]]` / `[[WORK-ITEM-NNNN]]` — the
   prefix is `WORK-ITEM`, not `WORK`, and bare scalars are not supported.
   The `pattern` returned from `useWikiLinkResolver` is bracket-anchored
   and cannot be reused as-is against scalar frontmatter values.
3. **`work.id_pattern` is not exposed to the frontend.** Only
   `defaultProjectCode` is surfaced via `/api/work-item/config`; the actual
   `id_pattern` format string lives server-side only and is consumed for
   filename parsing, not for client-side linkification.

These three points are not blockers — they're shape corrections for the
story. The cleanest implementation path is for `FrontmatterTable` to (a)
use `--ac-fg-muted` for the dimmed em-dash, (b) build its own bare-ID regex
from `defaultProjectCode` (the same source the resolver uses), and (c)
invoke the existing `resolver(prefix, id)` function with the captured
groups — reusing the resolution layer while owning the matching layer.

Everything else is well-supported: the component folder convention is
clear, the mount point in `LibraryDocView` is mechanical, the parsed
frontmatter object is already exposed (`Record<string, unknown>`), and the
CSS tokens cover sunken background, stroke border, and Fira Code.

## Detailed Findings

### FrontmatterChips — the existing parallel component

**Files** (flat `<Name>/<Name>.{tsx,module.css,test.tsx}` convention, no
`index.ts`, no `types.ts`):
- `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx`
- `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.module.css`
- `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.test.tsx`

**Prop signature** is a discriminated union on `state`
(`FrontmatterChips.tsx:5-8`):
- `{ state: 'absent' }`
- `{ state: 'malformed' }`
- `{ state: 'parsed'; frontmatter: Record<string, unknown> }`

There is no shared `Frontmatter` type — `IndexEntry.frontmatter` is
declared as `Record<string, unknown>` at `src/api/types.ts:73`, with a
sibling `frontmatterState: 'parsed' | 'absent' | 'malformed'` at line 74.

**`formatChipValue` logic** (`FrontmatterChips.tsx:10-14`):
```text
if Array.isArray(value)        -> value.join(', ')      // [] -> ''
else if typeof === 'object'
     && value !== null         -> JSON.stringify(value) // raw, no replacer
else                           -> String(value)         // 0, false render
```

**Skip rules** (`FrontmatterChips.tsx:26-30`): drops entries whose value is
`null`, `undefined`, or `''`. **Does NOT drop** empty arrays, empty
objects, `0`, `false`, or whitespace-only strings. Note the empty-array
quirk: `[].join(', ')` returns `''`, but the filter checks the raw value
before `formatChipValue` runs — so an empty array is kept and renders as
an empty-string chip. The table will need explicit empty-array handling
(render dimmed em-dash) since the story's AC says empty arrays should
render as dashes, not as empty chips.

**Iteration order** (`FrontmatterChips.tsx:26`): `Object.entries(...)` —
pure source order. The work item's "source order" requirement is the JS
default; no sort or whitelist exists today.

**Wiki-link resolver usage**: none. `FrontmatterChips` does not import or
invoke `useWikiLinkResolver` — values are rendered as plain text inside
`Chip`. The table will be the first frontmatter-related component to
linkify scalars.

**Status-aware chip variant**: `isStatusKey(key)` is case-insensitive
literal match on `"status"`; `statusToChipVariant(value)` maps normalised
values to `green | indigo | amber | red | neutral`
(`src/api/status-variant.ts:3-26`). The table doesn't need this — the
story does not call for status-coloured rows.

### LibraryDocView — mount point and layout

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx`

**JSX shape** (lines 78-130):
- `title` ← `entry.title` (line 78)
- `subtitle` ← `<FrontmatterChips frontmatter={…} state={…} />` (lines 79-84)
- `body` ← `<article className={styles.article}>` (line 86) containing:
  - `<div className={styles.aside}>` with related artifacts (lines 87-110)
  - conditional malformed-frontmatter banner (lines 112-117) — uses
    `grid-column: 1 / -1` to span the article grid
  - `<div className={styles.body}>` wrapping `<MarkdownRenderer …>`
    (lines 119-121)

The article uses CSS grid `grid-template-areas: "body aside"` with a
fixed `260px` aside and `1fr` body (`LibraryDocView.module.css:1-6`).
There is **no max-width** on the body div itself — the markdown's own
`max-width: 720px` lives inside `MarkdownRenderer.module.css:2`.

**`useWikiLinkResolver` is already invoked at line 46**:
```ts
const { resolver: resolveWikiLink, pattern: wikiLinkPattern } =
  useWikiLinkResolver()
```
Both values are passed into `MarkdownRenderer` at line 120. A
`FrontmatterTable` can either receive `resolveWikiLink` as a prop from
the same call site, or call the hook itself (React Query dedupes — the
work-item-config query is shared).

**Recommended mount point**: insert between line 117 (end of the malformed
banner) and line 119 (`<div className={styles.body}>` opening), as a new
sibling inside the article grid. The new node either lives inside the
existing `.body` wrapper (above the markdown), or in its own wrapper with
`grid-area: body` so it stays in the body column. The story says
"between page header and markdown body" — the cleanest reading is
**inside `.body`, above the markdown**, so the table inherits the body
column.

### useWikiLinkResolver — what it does and doesn't do

**Files**:
- `frontend/src/api/use-wiki-link-resolver.ts` — the React hook
- `frontend/src/api/wiki-links.ts` — pure index/pattern/resolve module
- `frontend/src/components/MarkdownRenderer/wiki-link-plugin.ts` — remark plugin

**Return signature** (`use-wiki-link-resolver.ts:26-29, 79`):
```ts
{ resolver: Resolver; pattern: RegExp }
```

**Resolver signature** (`wiki-link-plugin.ts:15-18`):
```ts
type Resolver = (prefix: 'ADR' | 'WORK-ITEM', id: string) => ResolverResult
```
`ResolverResult` is `{ kind: 'resolved'; href; title } | { kind:
'unresolved' } | { kind: 'pending' }`. It takes a **pre-parsed**
`(prefix, id)` pair, not a raw token string.

**Pattern coverage** (`wiki-links.ts:28-33`):
```
/\[\[(ADR|WORK-ITEM)-(\d+|<PROJ>-\d+|\d+)\]\]/g
```
- **Bracket-anchored only.** Bare `WORK-0041` will not match.
- **Prefix is `WORK-ITEM`**, not `WORK`. The story's references to
  `WORK-####` need to be reframed as `WORK-ITEM-####`.
- Project-prefixed form is `[[WORK-ITEM-PROJ-0042]]` when
  `defaultProjectCode` is set (`wiki-links.ts:26-27` — project codes
  containing hyphens are explicitly out of scope).

**Substring matching inside text nodes is supported** — the remark plugin
iterates `pattern.exec(value)` over a text node's full string and
splices pre/post-match slices as plain text
(`wiki-link-plugin.ts:48-110`). So `see [[WORK-ITEM-0041]] for context`
linkifies the bracketed span only. But this only works because the
bracket markers anchor the regex unambiguously; a bare-ID pattern
needs lookarounds or word boundaries to avoid eating substrings of
unrelated text.

**Pattern source**: `buildWikiLinkPattern(defaultProjectCode)` builds the
regex from a single config field, fetched from
`GET /api/work-item/config` (`use-wiki-link-resolver.ts:20-24`,
`use-wiki-link-resolver.ts:52-56`).

**Implication for the table**: the work item's claim that "the table
reuses the same resolver as the markdown body, not a separate regex" is
half right. The **resolution layer** (`resolver(prefix, id)`) is
reusable. The **matching layer** (the bracket-anchored `pattern`) is
not — the table needs its own regex for bare scalars. The
"consistent, even if incomplete" fallback in the story's Assumptions
section is fine in principle, but the bracket-only resolver would
match **zero** bare frontmatter values, which is plainly broken. The
table must own its bare-ID regex.

### work.id_pattern — what's exposed to the frontend

**Server endpoint**: `GET /api/work-item/config`
(`server/src/api/work_item_config.rs:15-24`, registered at
`server/src/api/mod.rs:44`).

**Response shape**: `{ defaultProjectCode?: string | null }` only. The
`id_pattern` format string (`{number:04d}`, `{project}-{number:04d}`)
and the `scan_regex` for filename parsing live server-side at
`server/src/config.rs:49-99` and are not exposed.

**Default fallback**: if `work_item` is `None`, the endpoint returns
`{}` (or `defaultProjectCode: null`). Bash-layer defaults at
`scripts/config-defaults.sh:81-85`: `work.id_pattern = "{number:04d}"`,
`work.default_project_code = ""`.

The table can read `defaultProjectCode` from the same query the
resolver uses (`queryKeys.workItemConfig()` at
`src/api/query-keys.ts:42`), or piggy-back by exporting a small helper
from `wiki-links.ts`. Either way, the matching pattern for bare scalars
is something like:

```
/^(WORK-ITEM|ADR)-(?:(<PROJECT>)-)?(\d{4,})$/    // scalar value match
```
or, for embedded matches inside free text:
```
\b(WORK-ITEM|ADR)-(?:(<PROJECT>)-)?(\d{4,})\b
```

### CSS tokens and typography

**File**: `frontend/src/styles/global.css` (sourced from
`frontend/src/styles/tokens.ts`).

**Tokens used by the story (corrected)**:
- `--ac-bg-sunken` — exists. Light: `#f4f6fa` (line 73). Dark:
  `#070b12` (lines 191, 253).
- `--ac-stroke` — exists. Light: `rgba(32, 34, 49, 0.10)` (line 83).
  Dark: `rgba(255, 255, 255, 0.08)` (lines 201, 263).
- `--ac-text-muted` — **does not exist**. Use `--ac-fg-muted` instead.
  Light: `#5f6378` (line 81). Dark: `#a0a5b8` (lines 199, 261).

**Fira Code** (`global.css:127-129`):
- `--ac-font-display: "Sora", system-ui, sans-serif;`
- `--ac-font-body: "Inter", system-ui, sans-serif;`
- `--ac-font-mono: "Fira Code", ui-monospace, monospace;`

Font-face declarations at `global.css:54-66`; bundled files in
`frontend/public/fonts/`. There's also a "mono everywhere" mode that
overrides `display` and `body` to `var(--ac-font-mono)` under
`[data-font-mode="mono"]` (`global.css:309-310`); the table inherits
this behaviour for free because Fira Code is the mono var.

**Font size**: the story specifies `11.5px` Fira Code. The current
size scale uses `--size-*` tokens but none of them maps to 11.5 — so
this is a literal that will be flagged by the migration-test scanner
(`src/styles/migration.test.ts`) unless added to its allowlist. ADR-0026
(`meta/decisions/ADR-0026-css-design-token-application-conventions.md`)
covers the convention; 0088 is the in-flight work that will resolve
literals into shared variables.

**Markdown body width cap**: `max-width: 720px` literal in
`MarkdownRenderer.module.css:2` — already flagged in the design-gap
research and is the target of 0088. Until 0088 ships, the table can
match by setting the same literal (acknowledging the duplication).

**CSS Modules conventions**:
- Colocated `.module.css` per component.
- camelCase class names (e.g. `.libraryHeading`, `.searchRow`), not
  kebab-case, not BEM.
- Variants applied as additional camelCase classes.
- Global stylesheets only at `src/styles/global.css` and
  `src/styles/wiki-links.global.css`.

### Related work items

- **0041 (done)** — Provides the `Page` wrapper (eyebrow, h1, subtitle
  slot, actions, content). `LibraryDocView` already uses it. The
  "between page header and markdown body" slot the table targets is
  inside `Page`'s content (i.e. inside the article grid).
  File: `meta/work/0041-library-page-wrapper-and-overview-hub.md`.
- **0084 (draft)** — Caps the chip strip to four canonical keys
  (`status`, `verdict`, `date`, `author`); explicitly defers everything
  else to 0078's table. Confirms the story's pairing logic.
  File: `meta/work/0084-detail-page-chip-strip-cap.md`.
- **0085 (draft)** — Humanises the H1 via `frontmatter.title` ||
  humanised slug. The H1 sits directly above the table in the page
  layout. No data dependency.
  File: `meta/work/0085-humanise-detail-page-h1.md`.
- **0088 (draft)** — Markdown body width and text-size
  harmonisation. Decides canonical width and exposes it via CSS
  variable so every markdown surface (including the new table) reads
  the same value. Until 0088 ships the table mirrors the current 720px
  literal.
  File: `meta/work/0088-markdown-body-width-harmonisation.md`.

### Component-folder convention summary (for the new FrontmatterTable)

The new component should live at:
```
skills/visualisation/visualise/frontend/src/components/FrontmatterTable/
  FrontmatterTable.tsx
  FrontmatterTable.module.css
  FrontmatterTable.test.tsx
```
- No `index.ts`, no `types.ts` (matches every other sibling).
- camelCase classnames in the CSS module.
- Discriminated-union prop on `state: 'parsed' | 'absent' | 'malformed'`
  to match `FrontmatterChips` — or accept that the table can render
  only when `state === 'parsed'` and let `LibraryDocView` gate the
  mount. The simpler shape is the latter, since the malformed banner
  already lives at `LibraryDocView.tsx:112-117`.
- Reuse `Record<string, unknown>` for the input — no need for a new
  shared `Frontmatter` type.

## Code References

- `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx:5-38` — prop union, formatter, skip rules, render
- `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.module.css:1-15` — chip container + malformed banner classes
- `skills/visualisation/visualise/frontend/src/api/types.ts:64-74` — `IndexEntry.frontmatter` shape (`Record<string, unknown>`) and `frontmatterState` literal union
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:46` — `useWikiLinkResolver()` already invoked
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:79-84` — current `FrontmatterChips` subtitle render
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:117-121` — recommended insertion point between malformed banner and markdown body
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.module.css:1-6` — article grid (260px aside, 1fr body)
- `skills/visualisation/visualise/frontend/src/api/use-wiki-link-resolver.ts:20-79` — hook fetches config and memoises `{ resolver, pattern }`
- `skills/visualisation/visualise/frontend/src/api/wiki-links.ts:28-33` — `buildWikiLinkPattern` (bracket-anchored, `WORK-ITEM` prefix)
- `skills/visualisation/visualise/frontend/src/api/wiki-links.ts:110-129` — pure `resolveWikiLink(prefix, id, index)`
- `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/wiki-link-plugin.ts:10-110` — `Resolver`/`ResolverResult` types and remark text-node splicing
- `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css:1-2` — `max-width: 720px` literal (current body width cap)
- `skills/visualisation/visualise/frontend/src/styles/global.css:71-93` — `--ac-bg-sunken`, `--ac-stroke`, `--ac-fg-muted` light values
- `skills/visualisation/visualise/frontend/src/styles/global.css:189-272` — dark-theme equivalents
- `skills/visualisation/visualise/frontend/src/styles/global.css:127-129` — `--ac-font-mono: "Fira Code", ...`
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts` — allowlist for CSS literals (will need an entry for the new module if it carries any flagged literals)
- `skills/visualisation/visualise/server/src/api/work_item_config.rs:10-24` — `/api/work-item/config` exposes `defaultProjectCode` only
- `skills/visualisation/visualise/server/src/config.rs:49-99` — server-side `WorkItemConfig` (not exposed to client)
- `scripts/config-defaults.sh:81-85` — bash-layer default `work.id_pattern`

## Architecture Insights

- **Prop shape vs. typed contract.** The codebase deliberately leaves
  `frontmatter` as `Record<string, unknown>` rather than typing per-kind
  schemas. The table inherits this — keys are unknown at compile time,
  so all type narrowing happens at render time.
- **Resolution / matching split.** The wiki-link system separates
  pattern matching (in the remark plugin) from token resolution (the
  index-lookup `Resolver`). The plugin owns "given this string, where
  are the link spans?"; the resolver owns "given this `(prefix, id)`,
  does it exist?". The table inherits this split: it implements its
  own matching pass over scalar values, then defers to the existing
  `Resolver` for the lookup.
- **Source-order frontmatter iteration is implicit.** Both the YAML
  parser (server-side) and `Object.entries` (client-side) preserve
  insertion order. No code enforces this — the story's "source order"
  AC is satisfied by accident-of-runtime. Worth a test that pins
  iteration order against a fixture.
- **CSS literals are tracked.** `src/styles/migration.test.ts` scans
  for hard-coded widths/colours and gates them via allowlist. Any
  literal in the new module (e.g. `11.5px`, `12px 14px`, the body
  width mirror) will need to be either tokenised or allowlisted.
- **Page wrapper owns chrome padding.** Since 0041, `Page` is the
  canonical horizontal-padding owner; the table is inside `Page`'s
  content, so it doesn't need its own outer padding — only the
  prototype's `12px 14px` inner padding.

## Historical Context

- `meta/decisions/ADR-0026-css-design-token-application-conventions.md`
  — Conventions ADR governing token vs. literal usage (relevant
  because the prototype values `11.5px`, `12px 14px`, sunken bg, etc.
  are partly tokens and partly literals).
- `meta/research/codebase/2026-05-15-0041-library-page-wrapper-and-overview-hub.md`
  — Original research behind 0041's `Page` wrapper.
- `meta/research/codebase/2026-05-16-0041-library-page-wrapper-supplementary.md`
  — Supplementary research after slot-contract review.
- `meta/research/codebase/2026-05-14-0038-generic-chip-component.md`
  — Research on the `Chip` primitive that `FrontmatterChips` composes.
- `meta/research/codebase/2026-05-06-0033-design-token-system.md` and
  `meta/plans/2026-05-06-0033-design-token-system.md` — Original
  design-token system (where `--ac-bg-sunken` etc. were introduced).
- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
  — Source design-gap analysis behind 0078, 0084, 0085, 0088. Lines
  ~119 and ~358 flag the `720px` body-width literal explicitly.
- `meta/reviews/work/0078-detail-page-frontmatter-table-review-1.md`
  — In-progress review of the story (working copy at time of research).

## Related Research

- `meta/research/codebase/2026-05-15-0041-library-page-wrapper-and-overview-hub.md`
- `meta/research/codebase/2026-05-16-0041-library-page-wrapper-supplementary.md`
- `meta/research/codebase/2026-05-14-0038-generic-chip-component.md`
- `meta/research/codebase/2026-05-06-0033-design-token-system.md`
- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`

## Open Questions

1. **Should the table mount only when `frontmatterState === 'parsed'`,
   or should it also render in the `'malformed'` / `'absent'` cases?**
   The malformed-banner already lives in `LibraryDocView` and would
   double-message. Recommend: render the table only on `'parsed'`;
   leave the existing banner alone.
2. **Where should the bare-ID regex live?** Two options:
   (a) inside `FrontmatterTable.tsx` as a private helper; or
   (b) factored into `frontend/src/api/wiki-links.ts` as a sibling to
   `buildWikiLinkPattern`. (b) is more reusable but expands the
   resolver's surface. Recommend: start in the component, lift if a
   second consumer appears.
3. **Project-prefixed pattern matching for non-numeric project codes.**
   Existing comment at `wiki-links.ts:26-27` explicitly excludes
   project codes containing hyphens; the table should match that
   limitation rather than inventing a more permissive scheme.
4. **Story corrections.** Three pre-implementation edits to 0078:
   - Replace `--ac-text-muted` with `--ac-fg-muted` in the AC and
     Technical Notes sections.
   - Replace `WORK-####` references with `WORK-ITEM-####` (or note
     explicitly that the table introduces a `WORK-####` short-form
     beyond what the resolver supports — but this would be new
     behaviour, not a reuse).
   - Add a note that `work.id_pattern` is not currently exposed to
     the frontend; the table reads `defaultProjectCode` instead.
5. **Empty-array rendering precedent.** The chips strip's current
   behaviour for `[]` is "render empty chip" (a bug — should likely
   skip). The table's AC of "render dimmed em-dash for empty arrays"
   is correct in isolation but means table and chips diverge on this
   case in a third dimension beyond the documented two. Worth
   acknowledging in Drafting Notes.
