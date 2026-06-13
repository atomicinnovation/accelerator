---
type: plan
id: "2026-06-13-0110-surface-rcas-in-visualiser"
title: "Surface Root Cause Analyses in the Visualiser Implementation Plan"
date: "2026-06-13T21:15:56+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0110"
parent: "work-item:0110"
derived_from: ["codebase-research:2026-06-13-0110-surface-rcas-in-visualiser-operate-category"]
relates_to: ["plan:2026-05-26-0074-per-doc-type-hues-on-detail-page", "plan:2026-06-09-0082-big-glyph-hero-illustrations", "plan:2026-05-16-0041-library-page-wrapper-and-overview-hub", "plan:2026-06-01-0054-sidebar-search"]
tags: ["visualiser", "rca", "doc-types", "library", "operate"]
revision: "59fd3a3fc675d0605825bb97b9939dde47d61e67"
repository: "build-system"
last_updated: "2026-06-13T23:16:21+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Surface Root Cause Analyses in the Visualiser Implementation Plan

## Overview

Make root cause analyses (RCAs — the `issue-research` doc type) a first-class,
browsable document type in the visualiser, grouped under a new top-level
**Operate** category positioned between Ship and Remember. The RCA type gets its
own listing page, detail page, library-hub card, related-artifacts rows, and
search results — surfaced end-to-end against the authoritative design prototype.

The work is small in code volume but exact in placement: the visualiser is
**server-driven but not data-driven**, so the category structure is a hard-coded
Rust static, and almost every browsable surface is generic over two registries
(server `DocTypeKey` enum, frontend `DocTypeKey` union) plus a handful of
compile-time-exhaustive per-type asset tables. The compiler enforces
completeness — the real work is extending those registries correctly and keeping
four distinct spellings of the same concept in their lanes.

## Current State Analysis

**The four spellings (the central hazard).** One concept, four tokens, each in
its own lane — conflating them is the main risk:

| Spelling | Where it lives | Role |
|---|---|---|
| `issue-research` | frontmatter `type:` discriminator | the artifact's declared type, written into the file |
| `rca` | template stem, `STEM_TO_GLYPH`, config template key | template/registration stem |
| `root-cause-analyses` | frontend glyph-only key, **prototype type key + route**, colour tokens | the browsable doc-type key + URL token |
| `research_issues` | server config `doc_paths` key → `meta/research/issues/` | the on-disk directory the indexer scans |

The server classifies a file's doc type by **which configured directory it lives
in**, not by its frontmatter `type:`. So the wire token / route is
`root-cause-analyses`, the directory key is `research_issues`, and the
frontmatter discriminator stays `issue-research` — all four coexist cleanly.

**What 0096 already landed (verified present, do not re-add):**

- `GLYPH_ONLY_DOC_TYPES = ["root-cause-analyses"]`
  (`frontend/src/components/Glyph/Glyph.constants.ts:13`).
- `DOC_TYPE_TOKEN_KEY` + `DOC_TYPE_COLOR_VAR` both include `root-cause-analyses`
  (`Glyph.constants.ts:36,55`).
- The fishbone `RootCauseAnalysesIcon` + `ICON_COMPONENTS` wiring + framed-tile
  CSS (`Glyph.tsx:39`, `Glyph.module.css:27`).
- `STEM_TO_GLYPH` `rca`/`root-cause-analyses` (`template-tier.ts:64-65`).
- Resolved hex colour tokens `ac-doc-root-cause-analyses` (`#ab2c96`) +
  light/dark fg/bg in `tokens.ts:73-76,92` and `global.css`.
- The config `doc_paths` key `research_issues` → `meta/research/issues/` is
  already wired in `write-visualiser-config.sh:76` and asserted by
  `config_contract.rs:57`. A real RCA artifact already lives at
  `meta/research/issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md`.

**What is missing (everything that makes RCA browsable):**

- Server: `DocTypeKey::RootCauseAnalyses` variant, its match arms, `PHASES`
  Operate tuple, `config_path_key()` → `research_issues` mapping.
- Frontend: `DocTypeKey` union + `DOC_TYPE_KEYS` entry, plural/singular labels,
  numeric `DOC_TYPE_HUE: 310`, BigGlyph hero, `TYPE_COPY` + `EMPTY_TYPE_PLURALS`
  entries, `status-variant` `resolved`/`monitoring` mappings.
- Tests/fixtures across server, frontend unit, and E2E + visual regression.

### Key Discoveries:

- **Exhaustiveness checks force atomicity.** Adding `root-cause-analyses` to the
  frontend union makes every `Record<DocTypeKey, …>` table fail to compile until
  filled (`BIG_GLYPHS` at `BigGlyph.tsx:27-41`, `DOC_TYPE_HUE` at
  `tokens.ts:11-25`, labels at `types.ts:68-101`, `TYPE_COPY`/`EMPTY_TYPE_PLURALS`
  at `empty-descriptions.ts:18,88`). The Rust enum forces all `match` arms in
  `docs.rs`. So per-side changes are inherently atomic.
- **Ordering is load-bearing: frontend before server.** The frontend renders
  whatever `phases` the server emits (`LibraryOverviewHub.tsx:54-81`,
  `build_structure` at `library.rs:161-181`). If the server emitted an Operate
  card before the frontend knew `root-cause-analyses`, hub-card glyph/label
  lookups would resolve `undefined` at runtime. Phase 1 (frontend) is purely
  additive and dormant; Phase 2 (server) activates the feature.
- **The category structure is a static, not config/data.** `PHASES` is a
  module-level Rust static (`library.rs:74-111`); "server-driven" means the
  *frontend* reads the structure from the server, not that the structure is
  data-driven. Adding Operate genuinely requires a server code change.
- **`config_contract.rs` needs no change.** The `research_issues` key is already
  in the 13-key `doc_paths` contract (`config_contract.rs:45-50,57`); mapping a
  `DocTypeKey` to it consumes the existing key without adding one.
- **Classification, counts, listing, and search are all generic.** Once a
  `DocTypeKey` exists with a `config_path_key()` pointing at a configured
  directory, `file_driver`/`indexer` classify it, `library_aggregates` count it
  and compute "latest", `docs_list` serves the listing, and `search` includes it
  — with **no per-type code** (`indexer.rs:63-69,640-704`,
  `file_driver.rs:111-121,527`, `api/docs.rs:30-42`, `api/search.rs:95-138`).
- **The prototype RCA BigGlyph ports near-verbatim.** It uses only palette keys
  `deep/stroke/line/fill/accent/white`, all present in `bigPalette.ts:3-11`; the
  only normalisation is `#FFFFFF`→`#ffffff` (already done by the local palette).
  Source: `prototype-full/src/big-glyphs.jsx:349-397`.

### Decision taken during planning (resolves research Open Question 1):

**BigGlyph hero placement — match the prototype.** ACs 4 and 7 literally name the
RCA BigGlyph hero on the *populated detail page*, but the prototype and all 13
existing doc types render the BigGlyph only on the **empty-state** surface; the
populated detail page uses the small framed eyebrow glyph. This plan follows the
prototype: author + register the RCA BigGlyph so it renders on the RCA
empty-state (and recovery) surface, and the populated detail page uses the small
framed eyebrow glyph (hue 310) like every other type. ACs 4 and 7 are reworded
accordingly in the success criteria below.

**Short label "RCA" — not applicable (no registry exists).** Work-item ACs 2 and
7 name a short label "RCA", but the visualiser has **no doc-type short-label
table** (the only `*_SHORT_LABELS` map is `TIER_SHORT_LABELS`, for template
tiers), and the prototype's `short: "RCA"` is rendered only as an *icon-less
fallback* (`icon ? <svg> : <span>{short}</span>`) — RCA has an icon, so its short
label is never shown. The short-label AC is therefore satisfied vacuously; this
plan registers only the plural/singular labels and does **not** introduce a
short-label registry. (Recorded here so the AC checklist reconciliation is
explicit rather than a silent omission.)

Research Open Questions 2 (route token `root-cause-analyses`), 3 (RCA
non-participating in the lifecycle pipeline), and 4 (`config_contract.rs`
unchanged) are taken as settled by the prototype and the verified code.

## Desired End State

Running the visualiser against a repo containing `issue-research` artifacts:

- The library overview hub shows an **Operate** category between Ship and
  Remember, containing a **Root cause analyses** card with the correct count
  (= number of `issue-research` artifacts) and a "latest" preview (most recently
  modified RCA); a zero-count repo shows it identically to other zero-count cards.
- Clicking the card opens the RCA listing (shared list-view layout) rendering all
  RCAs, each row showing `status` (e.g. `resolved`, `monitoring`) via the shared
  status-badge treatment with the correct colours.
- Opening one RCA shows its detail page with the RCA small framed eyebrow glyph
  and hue 310; the RCA empty-state shows the RCA BigGlyph hero.
- An artifact linking to an RCA shows that RCA in related artifacts with the
  correct glyph/hue (310), routing to the RCA detail page.
- Searching matches RCAs, labelled and routed as RCAs.
- `mise run check` and the visualiser test suites pass with new server, frontend
  unit, and E2E coverage.

## What We're NOT Doing

- **Not re-adding 0096's groundwork** — the small glyph, `STEM_TO_GLYPH`,
  colour tokens, and `research_issues` config key already exist and are verified
  correct; this work only adds what is missing.
- **Not adding a BigGlyph hero to the populated detail page** (per the decision
  above — that would be a new pattern for all 13 types; out of scope).
- **Not reconciling unrelated drift** between the current visualiser and the
  prototype — "match the prototype" means parity for the RCA pages specifically.
- **Not changing the frontmatter discriminator** — RCAs are consumed using the
  existing `issue-research` schema (0057) as-is.
- **Not making RCA participate in the lifecycle pipeline / kanban** — it is a peer
  category outside the linear DEFINE→REMEMBER flow.
- **Not changing `config_contract.rs`** — the `research_issues` key is already in
  the contract.

### Deferred follow-ups (surfaced in review; out of scope here)

These are broader than 0110 and would dilute its scope; capture as separate work
rather than expanding this plan:

- **Invert `in_lifecycle()` to an explicit allow-list.** The predicate is
  currently "everyone except Templates", so each new peer/non-lifecycle type must
  remember to opt out or it silently serialises a wrong `inLifecycle: true`. RCA
  is handled explicitly here; flipping the polarity so the safe default is `false`
  is a separate refactor.
- **Single shared source for the `doc_paths` key set.** The launcher
  (`write-visualiser-config.sh`), the E2E harness (`e2e/start-server.mjs`), and the
  smoke test (`api_smoke.rs`) each hand-maintain the key set; they have already
  drifted, and this plan only patches the drift. A shared source (or a parity test)
  would prevent recurrence.
- **A `/api/types` keys == `DOC_TYPE_KEYS` contract test.** The server enum and
  frontend union are coupled only by convention; nothing fails at CI time if they
  diverge (a mismatch resolves to a runtime `undefined` glyph/label). A cheap
  cross-stack assertion would convert that into a build failure.

## Implementation Approach

Three phases, each a compiler-forced-atomic, independently mergeable unit that
leaves `mise run check` green. They merge in order — **frontend (dormant) →
server (activates) → cross-stack tests** — because the frontend must understand
the type before the server emits it. TDD is applied within each phase: the
contract assertions (frontend unit tests, server structure tests, E2E
navigation) are written/updated to express the new behaviour first, then the
production code is added until they pass.

---

## Phase 1: Frontend — promote RCA to a browsable doc type (dormant)

### Overview

Make the frontend understand `root-cause-analyses` as a real `DocTypeKey`,
filling every exhaustive per-type table and authoring the BigGlyph hero. After
this phase the RCA route resolves but is unreachable via the UI (the server emits
no Operate card yet), so nothing is user-visibly broken — the change is additive
and dormant.

### Changes Required:

#### 1. Doc-type union, runtime mirror, and labels

**File**: `skills/visualisation/visualise/frontend/src/api/types.ts`
**Changes**: Add `"root-cause-analyses"` to the `DocTypeKey` union (`:4-17`), the
`DOC_TYPE_KEYS` runtime array (`:23-37`), `DOC_TYPE_LABELS` (`:68-82`), and
`DOC_TYPE_LABELS_SINGULAR` (`:87-101`).

```typescript
// DocTypeKey union — add:
  | "root-cause-analyses"
// DOC_TYPE_KEYS — add (order: alongside the others, before "templates"):
  "root-cause-analyses",
// DOC_TYPE_LABELS:
  "root-cause-analyses": "Root cause analyses",
// DOC_TYPE_LABELS_SINGULAR:
  "root-cause-analyses": "Root cause analysis",
```

#### 2. Numeric hue

**File**: `skills/visualisation/visualise/frontend/src/styles/tokens.ts`
**Changes**: Add the RCA entry to `DOC_TYPE_HUE` (`:11-25`). This is the upstream
colour-identity source the BigGlyph palette and empty-state gradient derive from.

```typescript
  "root-cause-analyses": 310,
```

Note: the `global.test.ts` WCAG-contrast suite iterates physical `DOC_TYPE_KEYS`,
so once RCA joins the union it **auto-asserts** `ac-doc-root-cause-analyses`
clears ≥3:1 against `--ac-bg` in both themes. The 0096 tokens (`#ab2c96` light)
almost certainly pass, but confirm before relying on them — a failure would turn
Phase 1's `mise run check` red with no other warning.

#### 3. Graduate out of the glyph-only escape hatch

**File**: `skills/visualisation/visualise/frontend/src/components/Glyph/Glyph.constants.ts`
**Changes**: Remove `"root-cause-analyses"` from `GLYPH_ONLY_DOC_TYPES` (`:13`) so
it becomes empty (`[] as const`), and update the explanatory comment. The
`DOC_TYPE_TOKEN_KEY` / `DOC_TYPE_COLOR_VAR` entries stay (now reached via the
union side of `GlyphDocType`). Confirm `GlyphDocType` still type-checks with an
empty `GLYPH_ONLY_DOC_TYPES`. With the array empty, `GlyphDocType` collapses to
`DocTypeKey | never` (≡ `DocTypeKey`) — keep the `GlyphDocType` apparatus but
replace the old comment with a one-line note that the empty list is **intentional
headroom** for future glyph-only keys, so a future reader does not treat the
indirection as dead and remove it (collapsing it back is a deliberate non-goal).

**File**: `skills/visualisation/visualise/frontend/src/components/Glyph/Glyph.test.tsx`
**Changes**: Once RCA joins `DOC_TYPE_KEYS` it is covered by the
`describe.each(DOC_TYPE_KEYS)` matrix, so the bespoke glyph-only special-case
block (`:205-215`, "`root-cause-analyses` is a glyph-only key — not in
`DOC_TYPE_KEYS`") is now contractually false and duplicative — remove it (or
rewrite its comment), and bump the matrix count comment if present.

#### 4. Empty-state copy

**File**: `skills/visualisation/visualise/frontend/src/routes/library/empty-descriptions.ts`
**Changes**: Add RCA entries to `TYPE_COPY` (`:18`) and `EMPTY_TYPE_PLURALS`
(`:88`).

```typescript
// TYPE_COPY:
  "root-cause-analyses": {
    purpose:
      "Hypothesis-driven investigations tracing a production issue to its root cause.",
    path: "meta/research/issues/",
    hue: DOC_TYPE_HUE["root-cause-analyses"],
  },
// EMPTY_TYPE_PLURALS:
  "root-cause-analyses": "root cause analyses",
```

#### 5. Status-variant colours

**File**: `skills/visualisation/visualise/frontend/src/api/status-variant.ts`
**Changes**: Add `resolved` to the `GREEN` set and `monitoring` to the `INDIGO`
set (`:4-20`), so RCA listing rows render the prototype's colours rather than
neutral grey.

```typescript
const GREEN = new Set([..., "resolved"]);
const INDIGO = new Set([..., "monitoring"]);
```

Note: `statusToVariant` is **doc-type-agnostic** — these verbs colour the status
column of *every* doc type, not only RCAs. That is intentional (the sets are a
shared status lexicon, matching the prototype's `StatusBadge`); a code comment at
the new entries should record this so they are not later mistaken for RCA-private
and "cleaned up".

**File**: `skills/visualisation/visualise/frontend/src/api/status-variant.test.ts`
**Changes** (TDD — write first): Add `"resolved"` to the green `it.each` block and
`"monitoring"` to the indigo `it.each` block (the exhaustive per-status tables),
so a wrong-set placement fails a unit test rather than only surfacing as a wrong
colour in the listing. This is the cheapest mutation-resistant guard for the
status-colour requirement.

#### 6. BigGlyph hero (author + register)

**File** (new): `skills/visualisation/visualise/frontend/src/components/BigGlyph/icons/RootCauseAnalysesBigGlyph.tsx`
**Changes**: Port the fishbone illustration from
`prototype-full/src/big-glyphs.jsx:349-397` as a `BigGlyphDraw`
(`(p: BigPalette) => ReactElement`) returning the inner `<g>`. Match the existing
icon files' shape (e.g. `DesignGapsBigGlyph.tsx`). All palette keys used
(`deep/stroke/line/fill/accent/white`) exist in `bigPalette.ts:3-11`.

**File**: `skills/visualisation/visualise/frontend/src/components/BigGlyph/BigGlyph.tsx`
**Changes**: Import `RootCauseAnalysesBigGlyph` and add it to `BIG_GLYPHS`
(`:27-41`); update the "13"/"thirteen" comments (`:1,21,105`) to 14.

```typescript
  "root-cause-analyses": RootCauseAnalysesBigGlyph,
```

**Count-comment sweep (all files).** Promoting RCA invalidates hard-coded
`13`/`12`/`thirteen`/`twelve` doc-type counts in comments across the frontend,
which the exhaustiveness checks do **not** catch. Sweep and fix (prefer rewording
them **count-free** — e.g. "one per `DocTypeKey`" — to stop the recurrence):
`bigPalette.ts:16` ("thirteen illustration call sites"),
`BigGlyph.test.tsx:167`, `Glyph.tsx:1,23,75` ("all 13 server doc-type keys"),
`tokens.ts:59,133,151` ("all twelve/12 …"), `global.css:369,384`,
`router.ts:151,160` (the `/glyph-showcase` / `/big-glyph-showcase` descriptions),
and the frontend README's `/glyph-showcase` line ("all 12 doc-type Glyphs").

Also update every comment (and test name/comment) that frames
`root-cause-analyses` as a glyph-only / non-browsable / "not a server doc type"
key — all now false after Phase 2: `Glyph.constants.ts:9,13`, `Glyph.tsx:23-25,75-77`,
`tokens.ts:73-76`, **`global.css:119`** ("Glyph-only doc type (not a server
DocTypeKey)"), **`template-tier.ts:30-33,62-63`** ("…which is not a browsable
server doc type" / "Root-cause analyses: glyph-only (not a server doc type)"), and
**`LibraryTemplatesIndex.test.tsx:93-97`** (the test name + comment "is not a
server DocTypeKey" — keep the assertion, which is about the `rca` *template* row,
but reword its rationale).

Two caveats: (1) use a **case-insensitive** enumerating grep —
`grep -rEni 'thirteen|twelve|all 1[23]|glyph-only' src` — so capitalised
"Glyph-only" comments (`global.css:119`, `Glyph.constants.ts:9`) are caught.
(2) The tables do **not** all hold the same count (`DOC_TYPE_HUE` has 13 entries →
14; the per-doc-type fg/bg colour tokens are 12 because templates borrows
`--ac-fg-muted`; `DOC_TYPE_KEYS`/`BIG_GLYPHS` are 13 → 14), so phrase each
count-free comment against the specific table it annotates rather than assuming a
single shared number.

#### 7. Frontend unit tests (TDD — write first)

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryOverviewHub.test.tsx`
**Changes**: Extend the mock `baseStructure` with an Operate phase containing the
RCA `LibraryDocType`; assert the rendered card links to
`/library/root-cause-analyses` (mirror the existing `href` assertion at `:99-107`).
Add a **zero-count** case (Operate phase with the RCA doc type at `count: 0`,
`latest: null`) asserting the card renders identically to other zero-count cards
(count 0, no "latest" preview) — the automated guard for work-item AC #2, which
is otherwise only manually verified and is no longer covered by the server
zero-count test now that the seeded fixture is non-zero.

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.test.tsx`
**Changes**: Add a case that renders the RCA listing from a mocked
`LibraryStructureResponse` + `IndexEntry[]` (statuses `resolved`/`monitoring`),
asserting the RCA label appears and the status column renders the **specific**
chip variants — `resolved` → green, `monitoring` → indigo (assert the variant
data-attribute/class, not mere chip presence), so the test fails if a verb falls
through to neutral grey.

**File**: `skills/visualisation/visualise/frontend/src/components/BigGlyph/BigGlyph.test.tsx`
**Changes**: The dispatch-collision guard iterates `DOC_TYPE_KEYS` — confirm it
now covers 14 referentially-distinct draw functions. Two explicit `13` literals
must become `14`: `expect(Object.keys(BIG_GLYPHS).length).toBe(13)` (`:69`) and
`expect(new Set(Object.values(BIG_GLYPHS)).size).toBe(13)` (`:81`).

**File**: `skills/visualisation/visualise/frontend/src/routes/glyph-showcase/GlyphShowcase.test.tsx`
**Changes**: The showcase renders one Glyph per `DOC_TYPE_KEYS` entry; update the
SVG-count assertion `:9` `39` → `42` (14 doc types × 3 themes). _(The
`/glyph-showcase` route is still live on this tree, retained as a VR fixture.)_

**File**: `skills/visualisation/visualise/frontend/src/routes/big-glyph-showcase/BigGlyphShowcase.test.tsx`
**Changes**: Update the SVG-count assertion `:9` `13` → `14`. _(The
`/big-glyph-showcase` route is still live on this tree, retained as a VR
fixture — the Phase 1/3 manual-verification steps that reference it are valid.)_

### Success Criteria:

#### Automated Verification:

- [x] Frontend type-check passes (exhaustive Records filled): `mise run frontend:check`
- [x] Frontend lint + format pass: `mise run frontend:check`
- [x] Frontend unit tests pass, including new RCA cases — hub card (populated +
      zero-count), listing, `status-variant`, `RelatedArtifacts`,
      `SearchResultsPanel`, BigGlyph dispatch, and the bumped showcase counts
      (`GlyphShowcase` 42, `BigGlyphShowcase` 14): `mise run test:unit:frontend`
      (2446 tests pass)
- [x] `GLYPH_ONLY_DOC_TYPES` is empty and `GlyphDocType` still type-checks
      (covered by the build).

#### Manual Verification:

- [ ] Navigating directly to `/library/root-cause-analyses` resolves to the
      listing route (empty, since the server emits no RCAs yet) without a console
      error — confirming the route is dormant, not broken.
- [ ] The `/big-glyph-showcase` dev page renders the RCA fishbone hero at hue 310
      in both light and dark themes.

---

## Phase 2: Server — emit the Operate category and RCA doc type

### Overview

Add `DocTypeKey::RootCauseAnalyses`, fill every match arm, map it to the existing
`research_issues` config directory, and insert the Operate tuple into `PHASES`
between Ship and Remember. This activates the whole feature: counts, latest
preview, listing, related artifacts, and search all light up generically, and the
frontend (Phase 1) renders the Operate card.

### Changes Required:

#### 1. The `DocTypeKey` enum and its match arms

**File**: `skills/visualisation/visualise/server/src/docs.rs`
**Changes**:
- Add `RootCauseAnalyses` to the enum (`:6-20`).
- Add it to `all()` and bump `[DocTypeKey; 13]` → `14` (`:23-39`).
- `config_path_key()` → `Some("research_issues")` (`:41-62`).
- `label()` → `"Root cause analyses"` (`:64-80`).
- `wire_str()` → `"root-cause-analyses"` (`:143-159`).
- Lifecycle predicates: leave `RootCauseAnalyses` **out** of
  `participates_in_lifecycle`, `in_kanban`, `is_virtual`,
  `carries_target_frontmatter`, `nested_manifest_filename`.
- `in_lifecycle()`: **return `false` explicitly** for `RootCauseAnalyses` rather
  than letting it fall through the default-`true` arm (only Templates is excluded
  today). Operate is a peer category *outside* the linear DEFINE→REMEMBER flow, so
  emitting `inLifecycle: true` would make the serialised flag a lie. This is safe:
  the linear pipeline placement is governed by `participates_in_lifecycle` (the
  frontend pipeline view drives off a hard-coded step list and the sidebar
  discards the `inLifecycle` flag), so flipping it to `false` does not remove RCA
  from any surface — it only makes the flag match the design intent. Add a one-line
  comment recording why, and a `docs.rs` test asserting RCA's `in_lifecycle()` ==
  `false` and `participates_in_lifecycle()` == `false` so the choice is
  intentional rather than incidental. _(If the implementer finds an unexpected
  consumer that relies on `inLifecycle` for a peer type, fall back to keeping the
  default-`true` arm with a comment + the same assertion — but the explicit
  `false` is preferred.)_

```rust
// enum:
    RootCauseAnalyses,
// all() — add the variant and change the return type to [DocTypeKey; 14]:
    DocTypeKey::RootCauseAnalyses,
// config_path_key:
    DocTypeKey::RootCauseAnalyses => Some("research_issues"),
// label:
    DocTypeKey::RootCauseAnalyses => "Root cause analyses",
// wire_str:
    DocTypeKey::RootCauseAnalyses => "root-cause-analyses",
```

**Classification assumption (no code change).** File→doc-type classification is a
`path.starts_with(root)` match over the `doc_paths` map (with non-deterministic
iteration order). This is collision-free for the default layout because
`research_issues` (`meta/research/issues`) and `research_codebase`
(`meta/research/codebase`) are disjoint siblings. It would only misclassify if a
user *override* nested one `research_*` root under the other — out of scope here,
but worth knowing the correctness of RCA classification rests on the two
`research_*` roots staying non-nested.

#### 2. The Operate phase

**File**: `skills/visualisation/visualise/server/src/api/library.rs`
**Changes**: Insert the Operate tuple into `PHASES` (`:77-111`) **between** the
`ship` and `remember` entries.

```rust
    (
        "operate",
        "Operate",
        &[DocTypeKey::RootCauseAnalyses],
    ),
```

#### 3. Server tests (TDD — update assertions first)

**File**: `skills/visualisation/visualise/server/tests/api_library_structure.rs`
**Changes**: Update the canonical phase-id assertion (`:35`) to
`["define", "discover", "build", "ship", "operate", "remember"]`. Add a test
asserting the Operate phase contains a `root-cause-analyses` doc type with the
expected count from the seeded fixture (mirror the decisions count test at
`:40-63`). Keep the Operate fixture to a **single** seeded RCA so the `latest`
preview is unambiguous — `library_aggregates` tie-breaks equal `modified_at` on
the lexically-smaller `rel_path`, so any future second same-tick RCA would make a
`latest`-title assertion order-dependent; if more are ever seeded, set distinct
mtimes via the existing `set_mtime_ms` helper.

**File**: `skills/visualisation/visualise/server/tests/common/mod.rs`
**Changes**: In `seeded_cfg`, create the `meta/research/issues` directory (the
`research_issues` key is already inserted at `:73`) and write a fixture RCA file
with `status: resolved` so the count/latest assertions are non-zero. Give the RCA
a stable slug/title so the related-artifacts and search assertions can match it.
Also seed (or extend) one existing artifact with an inbound link to the RCA so the
related-artifacts test (below) has a link to assert.

**`seeded_cfg` is shared — guard against collateral breakage.** This helper is
compiled into every server integration binary (`api_search`, `api_related`,
`api_library_structure`, `api_types`, …), so the new file can perturb suites that
assert on totals/ordering (e.g. `api_search`'s cross-type match tests,
`api_related`'s `linked_count_equals_sum_of_related_lists`). Choose an RCA
title/slug that is provably **non-colliding** with existing query tokens, and
treat "the *entire* `cargo test` server suite stays green after the `seeded_cfg`
edit" as a success criterion — not just the new RCA cases.

```rust
let research_issues = meta.join("research/issues");
// add to the create_dir_all loop, then:
std::fs::write(
    research_issues.join("2026-06-10-example-rca.md"),
    "---\ntitle: \"Example RCA\"\ntype: issue-research\nstatus: resolved\n---\n# body\n",
).unwrap();
// and an artifact that links to the RCA (for the related-artifacts assertion):
// the linkage MUST use a channel the server's related resolver actually
// surfaces (related.rs / indexer.rs) — confirm before coding (see the
// linkage-channel note under api_related.rs below), not a generic relates_to ref.
```

**Pre-flight: confirm the related-artifacts linkage channel (do this first).**
The server's related resolver surfaces an inbound entry only via specific
channels (a review `target:`, or work-item cross-refs `work_item_id:` / `parent:`
/ `related:` that canonicalise to a work-item ID) — a generic
`relates_to: ["issue-research:…"]` on a non-work-item, non-review artifact may
resolve to **nothing**. Before writing the AC #5 fixtures/tests, verify which
linkage actually makes an RCA appear in `/api/related` (read `related.rs` +
`indexer.rs`). If a resolvable inbound channel to an RCA exists, seed the linking
artifact using *that* form; if none does, this is a **feature discovery** — note
that AC #5's server half is not constructible, cover the related-artifacts
rendering via the frontend `RelatedArtifacts.test.tsx` synthetic-`IndexEntry`
case + E2E only, and flag whether the resolver needs extending to surface RCAs.

**File**: `skills/visualisation/visualise/server/tests/api_types.rs`
**Changes**: Update the `/api/types` length assertion at `:32` `13` → `14` (note:
`:13` is the test *name*, not the assertion). Rename the test function (and any other count-named
test, e.g. in the `docs.rs` module) away from `...thirteen...` to describe the
invariant rather than encoding a magic number. Also add an RCA property assertion
mirroring the `decisions` block — that the `root-cause-analyses` entry is
`virtual: false`, `inLifecycle: false`, `count: 1` (matching the now-seeded
fixture), and has a non-null string `dirPath` — so a mis-mapped
`config_path_key()`, a wrong lifecycle flag, or an un-indexed fixture is caught at
the wire boundary rather than passing a bare length bump.

**File**: `skills/visualisation/visualise/server/tests/api_smoke.rs`
**Changes**: The smoke test independently asserts the `/api/types` length — update
`assert_eq!(t["types"].as_array().unwrap().len(), 13)` (`:102`, comment at `:93`)
`13` → `14`. (Omitting this leaves the Phase 2 `mise run check` gate red.) Its
hand-rolled `doc_paths` map (`:18-31`) has drifted from the launcher; add
`("research_issues", "research/issues")` so the RCA type reports a real `dirPath`
rather than null (a bare length bump otherwise emits RCA with `dirPath: null`,
`count: 0`).

**File**: `skills/visualisation/visualise/server/src/docs.rs` (test module)
**Changes**: Update the count-13 assertions to 14 at **all three** sites:
`all()`-length (`:296-298`), the `:249-251` count, and
`describe_types_populates_dir_paths_from_config` (`:376`,
`assert_eq!(types.len(), 13)`). Extend the kebab-case / wire_str round-trip pair
lists (`:219-233`) with the RCA entry. (These fail to compile/pass until updated
— TDD by construction.)

#### 4. Server tests for the cross-cutting ACs (related artifacts + search)

These two work-item ACs (#5, #6) only become exercisable once the variant
exists, and both have cheap server-level seams — add them here rather than
leaving them to manual/E2E verification.

**File**: `skills/visualisation/visualise/server/tests/api_related.rs`
**Changes**: *Contingent on the linkage-channel pre-flight above.* If an RCA can
be surfaced as a related entry, add a test seeding an artifact that links to the
seeded RCA via the confirmed channel and asserting the RCA appears in the
related-artifacts response with `docType` `root-cause-analyses` and the RCA's
slug, mirroring the existing typed-linkage scenarios in this file. If no inbound
channel resolves to an RCA, **skip this server test** and rely on the frontend
`RelatedArtifacts.test.tsx` case + E2E for AC #5 (recording the gap). _(The
glyph/hue/route are frontend-derived either way.)_

**File**: `skills/visualisation/visualise/server/tests/api_search.rs`
**Changes**: Add a test querying the seeded RCA's title and asserting a result
row with `docType` `root-cause-analyses` and a non-empty `slug` — confirming the
auto-inclusion preconditions (non-virtual, `config_path_key` mapped, non-`None`
slug) actually hold for RCA. _(Covers AC #6; today this file only exercises
plans/decisions.)_

#### 5. Frontend unit tests for the cross-cutting ACs

**File**: `skills/visualisation/visualise/frontend/src/components/RelatedArtifacts/RelatedArtifacts.test.tsx`
**Changes**: Add a case with a `type: "root-cause-analyses"` `IndexEntry`
(`makeIndexEntry`), asserting the row links to `/library/root-cause-analyses/<slug>`
and renders the RCA singular label — the frontend half of AC #5.

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/SearchResultsPanel.test.tsx`
**Changes**: Add a case with an RCA search result, asserting the row uses the RCA
singular label and the `/library/root-cause-analyses/$fileSlug` route — the
frontend half of AC #6.

### Success Criteria:

#### Automated Verification:

- [x] Server compiles with the new variant (all match arms filled):
      `cd skills/visualisation/visualise/server && cargo build` (also forced
      arms in `slug.rs`, `cluster_key.rs`, `clusters.rs` beyond those the plan
      enumerated)
- [x] Server tests pass, including the updated phase-id list, RCA count/latest,
      the RCA `/api/types` property assertions, `api_related`/`api_search` RCA
      cases, and the bumped counts in `api_smoke.rs` and the `docs.rs` test
      module: `cargo test` (534 pass)
- [x] `config_contract.rs` passes **unchanged** (the `research_issues` key is
      already in the 13-key contract).
- [x] Full read-only gate passes: `mise run check` (each component green
      standalone; aggregate run hits the known pyrefly-vs-`deps:install:node`
      node_modules race, not a real finding)

#### Manual Verification:

- [ ] With the visualiser running against this repo (which has a real RCA at
      `meta/research/issues/`), the library hub shows an **Operate** category
      between Ship and Remember with a **Root cause analyses** card whose count
      and latest preview are correct.
- [ ] Clicking the card opens the RCA listing; each row shows `status` with the
      correct colour (`resolved`→green, `monitoring`→indigo).
- [ ] Opening an RCA shows the detail page with the small framed eyebrow glyph and
      hue 310; an artifact that links to it shows the RCA in related artifacts;
      searching its title returns it labelled/routed as an RCA.

---

## Phase 3: Cross-stack tests — E2E navigation + visual regression

### Overview

Add the end-to-end navigation test and visual-regression coverage that exercise
the live server + frontend together, plus the committed fixtures they depend on.
Purely additive, test-only — independently mergeable.

### Changes Required:

#### 1. E2E fixture wiring

**File**: `skills/visualisation/visualise/frontend/e2e/start-server.mjs`
**Changes**: Add `research_issues: join(fixturesDir, "research/issues")` to the
`docPaths` object (`:62-75`) — it is currently absent (this E2E config has drifted
from the launcher's 13-key set in `write-visualiser-config.sh`). This edit must
land **with or before** any E2E that expects RCAs; otherwise the live E2E server
has no configured RCA directory and the feature is server-active but
E2E-invisible. Worth a comment that `start-server.mjs`'s `docPaths` should track
`write-visualiser-config.sh`'s key set to prevent future drift.

**File** (new): `skills/visualisation/visualise/server/tests/fixtures/meta/research/issues/2026-06-10-example-rca.md`
**Changes**: Commit a fixture RCA (frontmatter `type: issue-research`,
`status: resolved`) mirroring
`server/tests/fixtures/meta/research/design-gaps/2026-05-26-example-gap.md`. Keep
its frontmatter **byte-identical** to the inline RCA seeded in `common/mod.rs`
(Phase 2) — both encode the same `type`/`status` contract that the count,
status-colour, and detail assertions depend on, so they must be updated together.

**File** (new or existing fixture): an artifact whose frontmatter links to the RCA
**Changes**: To exercise the related-artifacts AC, add/extend a committed fixture
(e.g. a work item or plan) with a typed-linkage ref to the RCA so the
related-artifacts row renders in E2E and the detail page.

#### 2. E2E navigation test

**File**: `skills/visualisation/visualise/frontend/e2e/navigation.spec.ts`
**Changes**: Add a case mirroring `:3-36`: `goto("/library/root-cause-analyses")`
→ assert `[role="table"]` → click a row → assert the detail URL
`/library/root-cause-analyses/<slug>`. Beyond routing, assert the
RCA-distinguishing behaviour so these ACs are guarded cross-stack rather than only
by VR pixel-diff: (a) on the listing, that a row's **status cell** shows the
expected badge text/variant for the committed fixture (`resolved`) — AC #3; and
(b) if the AC #5 linkage pre-flight (Phase 2) confirmed a resolvable inbound link,
navigate to the linking artifact's detail page and assert the RCA appears as a
**related-artifacts link** routing to `/library/root-cause-analyses/<slug>` —
AC #5. Reuse the same committed fixtures; skip (b) only if the linkage channel was
found non-constructible.

#### 3. Visual regression

**File**: the VR `ROUTES` array under
`skills/visualisation/visualise/frontend/tests/visual-regression/`
**Changes**: Add the RCA listing and detail routes (and, if the VR set covers
empty states, the RCA empty-state for the BigGlyph hero). Generate **darwin**
baselines locally; regenerate **linux** baselines via the "Update visual
regression baselines" CI workflow (a `GITHUB_TOKEN` push will not re-trigger Main
CI — a known handoff).

### Success Criteria:

#### Automated Verification:

- [x] E2E suite passes, including the RCA navigation case and its status-cell
      assertion (full Playwright suite exits 0). The related-artifacts assertion
      is covered server-side (`api_related`) + frontend-unit
      (`RelatedArtifacts.test.tsx`) instead of E2E — an E2E inbound-link fixture
      would have pulled an RCA into the balanced `ac2-coverage` cluster the
      aside-row VR spec counts on.
- [x] Visual-regression specs pass against committed **darwin** baselines (VR
      project exits 0): new RCA listing/detail routes, the RCA glyph/BigGlyph
      showcase cells, plus the existing showcase cells which legitimately
      reflow when the 14th doc type is added.
- [x] Full gate passes: `mise run check` (see Phase 2 note re the
      pyrefly-vs-node_modules race; Phase 3 adds only tests/fixtures/baselines).

#### Manual Verification:

- [ ] Linux VR baselines regenerated via the CI workflow and committed (darwin
      generated locally) — the only cross-platform handoff.
- [x] The RCA detail screenshot (`library-doc-view-rca`) shows the small framed
      eyebrow glyph (hue 310), and the fishbone BigGlyph hero is captured by
      `big-glyph-showcase` (`root-cause-analyses-{light,dark}`) — confirming the
      BigGlyph-placement decision is honoured. (No empty-state screenshot: the
      RCA fixture dir is non-empty, so the hero is VR-covered via the showcase
      rather than the empty-state surface.)

---

## Testing Strategy

### Unit Tests:

- **Frontend (Vitest):** RCA library hub card links to
  `/library/root-cause-analyses` (populated **and** zero-count — AC #2); RCA
  listing renders with `resolved`/`monitoring` status chips in green/indigo;
  `status-variant.test.ts` covers `resolved`→green / `monitoring`→indigo;
  `RelatedArtifacts.test.tsx` routes an RCA row to the RCA detail page (AC #5);
  `SearchResultsPanel.test.tsx` labels/routes an RCA result (AC #6); BigGlyph
  dispatch covers 14 distinct draws.
- **Server (Rust):** `PHASES` emits Operate between Ship and Remember; the RCA
  doc type appears with correct count/latest from the seeded fixture (single
  fixture → deterministic `latest`); `/api/types` emits RCA as
  `virtual:false` with a `dirPath`, and RCA is `in_lifecycle:false`;
  `DocTypeKey::all()` has 14 variants; `wire_str`/serde round-trips for the new
  variant; `/api/types` length is 14; `api_related.rs` surfaces the RCA from a
  linking artifact (AC #5); `api_search.rs` returns the RCA for a title query
  (AC #6).

### Integration / Cross-stack Tests:

- **E2E (Playwright):** library hub → Operate/RCA card → listing → detail
  navigation against committed fixtures; related-artifacts row routes to the RCA
  detail page.

### Manual Testing Steps:

1. Run the visualiser against this repo; confirm the Operate category and RCA card
   (count + latest) on the hub.
2. Click through to the listing; verify status colours.
3. Open an RCA; verify eyebrow glyph + hue 310; verify the empty-state BigGlyph on
   a repo (or fixture dir) with no RCAs.
4. Verify a linking artifact's related-artifacts row and a search query both
   surface the RCA, labelled/routed correctly.

## Performance Considerations

None. The new type flows through the existing in-memory indexer and generic
endpoints with no new scans or per-request work; counts/latest are computed in the
same two-pass aggregate already run for every type.

## Migration Notes

No data migration. The `research_issues` config key and on-disk directory already
exist; this work makes the server consume them. No user-facing config change is
required (the launcher already emits `research_issues`).

## References

- Original work item: `meta/work/0110-surface-root-cause-analyses-in-visualiser.md`
- Research: `meta/research/codebase/2026-06-13-0110-surface-rcas-in-visualiser-operate-category.md`
- Authoritative design: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full`
  (`src/data.jsx:18,25-32,54`, `src/ui.jsx:77-99,303`, `src/big-glyphs.jsx:349-397`)
- Server registry seam: `skills/visualisation/visualise/server/src/docs.rs:4-167`,
  `src/api/library.rs:74-111`
- Frontend registry seam: `frontend/src/api/types.ts:4-101`,
  `src/styles/tokens.ts:11-25`, `src/components/BigGlyph/BigGlyph.tsx:27-41`,
  `src/components/Glyph/Glyph.constants.ts:13`,
  `src/api/status-variant.ts:4-33`, `src/routes/library/empty-descriptions.ts:18,88`
- Related prior work: 0041 (server-driven library), 0074 (per-doc-type hues),
  0054 (search), 0057 (`issue-research` schema), 0082 (BigGlyph set), 0093/0096
  (`rca` stem + glyph registration)
