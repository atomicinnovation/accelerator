---
type: "codebase-research"
id: "2026-09-10-0278-topic-research-visualiser-doc-type-indexer"
title: "Research: Topic-Research Visualiser Doc Type and Indexer (0278)"
date: "2026-09-10T21:47:20+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0278"
parent: "work-item:0278"
topic: "Topic-Research Visualiser Doc Type and Indexer"
tags: ["research", "codebase", "visualiser", "doc-type", "indexer", "corpus", "rename"]
revision: "8eb899d123da8998b50447fcb4a789c97eff3f2c"
repository: "accelerator"
last_updated: "2026-09-10T21:47:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Topic-Research Visualiser Doc Type and Indexer (0278)

**Date**: 2026-09-10T21:47:20+00:00
**Author**: Toby Clemson
**Git Commit**: 8eb899d123da8998b50447fcb4a789c97eff3f2c (jj working-copy commit; not pushed)
**Branch**: detached HEAD (jj-colocated)
**Repository**: accelerator

## Research Question

Research the codebase to support story 0278 —
`meta/work/0278-topic-research-visualiser-doc-type.md` — which registers an
umbrella `topic-research` doc type in the visualiser, indexes each research set
via its `manifest.md`, collapses the manifest's `research_status` onto base
`status`, and renames the `research` doc-type **wire key** to
`codebase-research`. Verify the current state of every named surface, find the
surfaces the work item's file list misses, and surface risks before planning.

## Summary

**The story is buildable as specified, and its mechanics are sound — but the
work item's file list is incomplete in nine places, and two acceptance criteria
appear to conflict with the code they land on.** The three pillars (new
umbrella type, the `research_status` collapse, the wire-key rename) each map
cleanly onto an existing precedent: `DesignInventories` is a working
nested-manifest doc type, the server already reads only the literal `status`
key, and the `m0004` migration already moved linkage/config/`type:` to
`codebase-research` so only the wire token is left.

Two findings change the implementation shape and deserve a decision before
planning:

- ⚠️ **Humanisation mismatch (confirmed).** The title-fallback AC expects
  sentence-case (`model-context-protocol` → "Model context protocol"), but the
  `title_from` cascade falls back to `humanise_slug`, which title-cases each word
  → "Model Context Protocol" (pinned by `slug.rs:415`,
  `"design-token-system"` → `"Design Token System"`). Recommendation: correct the
  AC to the title-cased string and reuse the canonical humaniser, rather than add
  per-type sentence-case logic against the codebase grain.
- ⚠️ **Status-chip fill saturation.** The AC requires each lifecycle chip fill
  to carry HSL saturation ≥ 15%, but the existing `Chip` fills at
  `color-mix(var(--ac-*) 8%, var(--ac-bg))` — 8% of the accent over background,
  which resolves to a near-grey, likely below 15%. Reusing the stock `Chip`
  variants probably fails the required test; a bespoke lifecycle-chip treatment
  is likely needed.

**Nine surfaces the work item does not name but the change forces** (detailed in
"Work-item gaps"): the schema `templates-schema.tsv` mirror; a thirteenth
`Record<DocTypeKey,…>` map (`NON_CANONICAL_PER_KIND`); `cli/corpus/src/slug.rs`;
`DevDesignSystem.tsx`; the `global.test.ts` `var(--atomic-*)` ref-count and
`stageKeys` guards; the `Glyph.module.css` `data-doc-type` selector; and the VR
baseline filename suffix.

**Count arithmetic.** The new variant takes `DocTypeKey::all()` from **14 → 15**
and the linkage-bearing `DOC_TYPES` catalogue from **13 → 14**; the rename
changes no count. The work item's "doc-type count is unchanged (14)" is scoped
to the rename alone — the net post-story `all()` count is 15.

## Detailed Findings

### 1. Rust doc-type registry (`cli/corpus/src/doc_type.rs`)

`DocTypeKey` is a closed 14-variant enum (`doc_type.rs:9-24`); `all()` returns
`[Self; 14]` (`doc_type.rs:27-45`); the exhaustiveness test asserts
`variants.len() == 14` after sorting by `wire_str()` (`doc_type.rs:239-245`),
which also pins wire-token uniqueness. Four match methods have **no wildcard
arm** — `config_path_key` (`48-65`), `linkage_type_name` (`71-88`), `label`
(`99-116`), `wire_str` (`168-185`) — so the compiler forces a new arm per
variant. `nested_manifest_filename` (`160-165`) has `_ => None` and `Some(...)`
only for `DesignInventories`.

`DesignInventories` is the exact mirror template. Its predicate profile: not
virtual, in-lifecycle, not in-kanban, not `carries_target_frontmatter`, not
`participates_in_lifecycle`, `nested_manifest_filename() → Some("inventory.md")`
(`doc_type.rs:162`). The one distinguishing arm a mirror must decide is
`nested_manifest_filename()` — for topic-research, `Some("manifest.md")`.

`Research` already has `linkage_type_name() → "codebase-research"`
(`doc_type.rs:78`) and `config_path_key() → "research_codebase"`
(`doc_type.rs:53`); only `wire_str() → "research"` (`doc_type.rs:173`) still
carries the bare token. The Rust identifier `Research` stays, so no call site
(`cluster.rs:191,519,565`, `slug.rs:22`, `library.rs:90`, plus tests) changes.
An invariant test pins `PrDescriptions` as the *only* type where wire, linkage,
and config all differ (`doc_type.rs:262-271`); post-rename `Research` has
wire == linkage (`codebase-research`) and a distinct config key — that test
still holds because it targets `PrDescriptions`, not `Research`.

Count/parity sites to bump for the new variant:

| Site | Location | Current | Action |
|---|---|---|---|
| `all()` array | `doc_type.rs:28-44` | `[Self; 14]` | add element, → 15 |
| Exhaustiveness test | `doc_type.rs:244` | `len == 14` | → 15 |
| `parity.rs` row table | `parity.rs:56-92` | `(kind, wire, cfg)` rows, self-checks `len()` | add row |
| `api_types.rs` | `api_types.rs:32` | `arr.len() == 14` | → 15 |
| `DOC_TYPES` catalogue | `catalogue.rs:70-84` | 13 rows | add row |
| `DOC_TYPES` count | `catalogue.rs:275` | `len == 13` | → 14 |
| `doc_type_single_source.rs` | `:15-36` | derives from `all()` | auto-covered if linkage- + path-bearing |

`server/src/doc_type_view.rs:35-53` and `api/types.rs:14-23` iterate `all()` and
need **no** per-variant edit. `PATH_KEYS` already declares `research_topics`
(`catalogue.rs:64-67`) and `TEMPLATE_KEYS` already has the five
`topic-research-*` template keys (`catalogue.rs:100-104`) — 0277 scaffolding.

### 2. Visualiser server indexing (`file_driver.rs`, `indexer.rs`)

Nested-manifest indexing is switched entirely by
`DocTypeKey::nested_manifest_filename()`. `LocalFileDriver::list`
(`file_driver.rs:243-322`) reads that at line 258; when `Some(manifest_name)`,
the branch at `290-309` descends **directories only**, skips any child whose
name `starts_with('.')` (the `.…tmp` in-flight convention, `298-304`), and
pushes `<slug-dir>/<manifest_name>` only if that file exists (`305-308`).
Exactly one entry per set directory, keyed on the manifest filename. The flat
branch (`310-318`) emits each top-level `*.md`.

`build_entry` (`indexer.rs:1247-1371`) derives the slug from the parent
directory name re-suffixed `.md` when nested (`1258-1267`), then
`corpus::slug::derive` (`1311`). Title uses the three-layer cascade
`frontmatter::title_from` (`frontmatter.rs:122-148`): frontmatter `title`
(`127-137`) → first body `# ` H1 (`138-144`) → `humanise_slug(filename_stem)`
(`145-147`). Status is **not** special-cased — the whole frontmatter map is
copied verbatim (`1333-1347`); the only `status` read is generic, in
`extract_facet_value` (`indexer.rs:72-105`, `status` at `78-84`). **Nothing
reads `research_status` anywhere in the server**, confirming the collapse is a
document/producer concern only.

The `design-inventories` precedent is proven end-to-end by
`design_inventories_indexed_from_nested_directories` (`indexer.rs:1955-2012`):
two real dirs, one `.…tmp` dot dir ignored, one dir missing `inventory.md`
ignored → 2 entries; slugs from parent dirs, titles from frontmatter.

### 3. Frontend doc-type registry and maps (`frontend/src/`)

`DocTypeKey` is a 14-member union (`api/types.ts:4-18`) mirrored at runtime by
`DOC_TYPE_KEYS` (`:24-39`); the router's `isDocTypeKey` (`:42-44`) drives
route-segment validation. **The wire key IS the URL segment** — `router.ts`
`parseParams` (`104-114`) calls `isDocTypeKey(raw.type)` with no slug map, so
`/library/research/<slug>` becomes `/library/codebase-research/<slug>`
automatically once the union carries the new key, and the old path 404s to
`/library`.

Every total `Record<DocTypeKey,…>` (and `Record<GlyphDocType,…>`, currently
identical) map — a new key is a **compile error until added**:

| # | Map | File:line |
|---|---|---|
| 1 | `DOC_TYPE_LABELS` | `api/types.ts:70` |
| 2 | `DOC_TYPE_LABELS_SINGULAR` | `api/types.ts:90` |
| 3 | `DOC_TYPE_HUE` | `styles/tokens.ts:11` |
| 4 | `TYPE_COPY` | `routes/library/empty-descriptions.ts:18` |
| 5 | `EMPTY_TYPE_PLURALS` | `routes/library/empty-descriptions.ts:94` |
| 6 | `DOC_TYPE_TOKEN_KEY` | `components/Glyph/Glyph.constants.ts:26` |
| 7 | `DOC_TYPE_COLOR_VAR` | `components/Glyph/Glyph.constants.ts:45` |
| 8 | `ICON_COMPONENTS` | `components/Glyph/Glyph.tsx:25` |
| 9 | `BIG_GLYPHS` | `components/BigGlyph/BigGlyph.tsx:28` |
| 10 | `DETAIL_ROUTE_SLUGS` | `tests/lib/detail-route-slugs.ts:20` |
| 11 | `DETAIL_ROUTE_RENDERS_ARTICLE` | `tests/lib/detail-route-slugs.ts:43` |
| 12 | `DOC_TYPE_LABELS`/`_SINGULAR` (dup of 1/2) | — |
| 13 | **`NON_CANONICAL_PER_KIND`** | `components/FrontmatterChips/FrontmatterChips.test.tsx:437` |

Map 13 is **not named in the work item** (see gaps). Derived maps
`EMPTY_DESCRIPTIONS` (`empty-descriptions.ts:113-119`) and `EXPECTED_COLOR`
(`tests/lib/expected-colours.ts:56-70`) are `Object.fromEntries(...)`-built —
not literal-checked, so a missing key is a silent `undefined`; they auto-cover
a new key once their *source* (`TYPE_COPY`, `DOC_TYPE_TOKEN_KEY` + a resolvable
token) is updated. `Partial<Record<DocTypeKey,…>>` maps (`SeenMap`,
`LibrarySelection`, test-only worked-example maps) need no per-key row.

Last-seen `localStorage`: `use-unseen-doc-types.ts:40-48` has the exact
`prs → pr-descriptions` one-shot rewrite precedent, applied in `parseStored()`
*before* the `isDocTypeKey` filter at `:53`. Mirror it for
`research → codebase-research`, in the same location, or the renamed type reads
as all-unseen once.

### 4. Frontend styling: colour tokens, glyphs, status chips

**Colour tokens are hand-authored literals duplicated in two files and, in CSS,
three blocks.** `DOC_TYPE_HUE` (`tokens.ts:11-26`) is the numeric source
(`research: 28`, `design-inventories: 185`). Three token families —
`--ac-doc-*` (light fg), `--ac-doc-bg-*` (bg tint), `--ac-stage-*` (pipeline
accent, absent for non-pipeline types like `design-inventories`) — live in
`tokens.ts` (light `62-105`, dark `110-178`) **and** `global.css` across
`:root` (`76-347`), `[data-theme="dark"]` (`354-421`), and
`@media (prefers-color-scheme: dark)` (`427-490`). Dark collapses `--ac-doc-*`
to `#ffffff` and `--ac-doc-bg-*` to `#1d2030`. `rgb(28,146,51)` = `#1c9233`;
adding it means a `DOC_TYPE_HUE` entry, a light fg/bg pair + dark pair, in all
duplicated locations, plus `Glyph.constants.ts` entries.

`global.test.ts` guards: per-key contrast ≥ 3:1 for every
`isPhysicalDocTypeKey` in both themes (`380-397`), TS↔CSS parity (`80-94`),
dark-block mirror parity (`148-194`), and — easily missed — exact
`var(--atomic-X)` **ref-count** assertions per block (`root:9`, `data-dark:17`,
`media-dark:17`, at `325-327`) that change when dark tokens are added as
`var(--atomic-white)`. **No hue-separation test exists** — the AC's ≥15°
claim (104°/53°/37°) is satisfied by construction, not automatically.

Glyphs are per-key SVG components. Small: `ICON_COMPONENTS`
(`Glyph.tsx:25-40`) → a `<g fill="none" stroke="currentColor" strokeWidth="1.25"
…>` in a 24×24 `viewBox`, coloured via `color: var(--ac-doc-<key>)` on the
`<svg>` (`ResearchIcon.tsx`, `DesignInventoriesIcon.tsx` are drop-in
templates). Big: `BIG_GLYPHS` (`BigGlyph.tsx:28-43`) → a **render function**
`(p: BigPalette) => ReactElement` in an **80×80** viewBox, coloured from the
seven-tone `bigPalette(hue)` (`bigPalette.ts:27-37`), not `currentColor`.

**Status chips are a separate, doc-type-agnostic word lexicon.**
`api/status-variant.ts` maps a normalised `status` word to one of six
`ChipVariant`s (`neutral | indigo | green | amber | red | violet`,
`Chip.tsx:4-10`); rendered on the card at `LibraryTypeView.tsx:285-288`. Of the
five lifecycle states, **only `complete` is mapped today** (→ green); the other
four fall through to `neutral` (grey). Five non-grey variants exist, so a
five-way mapping is available — but chip fills are
`color-mix(var(--ac-*) 8%, var(--ac-bg))` (`Chip.module.css:33-52`), and no
chip-fill-contrast or saturation test exists (`status-variant.test.ts`,
`StatusBadge.test.tsx` test only the word→variant mapping). See the risk below.

### 5. Schema and the 0277-delta (`schema.rs` and co-change sites)

The schema table is `[SchemaRow; 18]` (`schema.rs:19`), resolved by
`row_for(type, kind)` with an exact-then-default fallback (`246-255`).
Topic-research has **five** rows (`189-239`) — `manifest`, `brief`, `outline`,
`finding`, `synthesis`; there is **no `report` row** (deferred to 0281,
`topic_research_kinds_each_resolve_to_a_distinct_row` at `360-369` iterates
exactly those five and asserts `row_for("topic-research","")` is `None`). The
manifest row (`189-203`):

```rust
SchemaRow {
    linkage_type: "topic-research",
    kind: "manifest",
    code_state_anchored: false,
    extras: &["slug", "research_status", "round_count", "finding_count", "primary"],
    status_vocab: &["complete"],
    forbidden_own_id_keys: &[],
    typed_linkage_keys: &["parent", "relates_to"],
},
```

The collapse: `status_vocab` → the five lifecycle states, drop `research_status`
from `extras`. **This row is mirrored in
`cli/corpus/src/frontmatter_validation/templates-schema.tsv:15`**, and
`every_row_matches_templates_schema_tsv` (`schema.rs:405-454`) asserts the
`status_vocab` and `extras` columns agree — so the TSV must co-change or the
test fails. The work item names `schema.rs` but not the TSV mirror.

The template `templates/topic-research-manifest.md` carries `status: "complete"`
(line 8) and `research_status: "briefed"` (line 11) plus a body "Research
status" line (line 35); base `status` becomes the lifecycle (`briefed`) and the
`research_status` line goes. The skill `skills/research/research-topic/SKILL.md`
reads/writes `research_status` at the `outline`/`conduct` preconditions
(`67-69`) and the four transition writes (`brief` `102-104`, `outline`
`117-118`, `conduct` `159`, `synthesise` `174-176`) — all move to base
`status`. The corpus-cli fixture
`cli/corpus-cli/tests/fixtures/topic-research-set/manifest.md` carries
`status: "complete"` + `research_status: "synthesised"` → base
`status: synthesised`, drop `research_status`.

### 6. Pipeline `completeness.present` vocabulary — the rename lockstep

Three strings contain "research" in three separate namespaces; only the
**present/wire token** moves. Move/stay map:

| Site | Location | String | Namespace | Move? |
|---|---|---|---|---|
| `wire_str` (root) | `doc_type.rs:173` | `"research"` | doc-type wire key | **move** |
| `STAGE_PUSH_ORDER` | `cluster.rs:137` | `"research"` literal | present token | **move** |
| `STAGE_PUSH_ORDER` | `cluster.rs:137` | `c.has_research` closure | completeness flag | stay |
| server test mirror | `clusters.rs:502` | `"research".to_string()` | present token | **move** |
| `parity.rs` | `parity.rs:61` | `"research"` (mid field) | `wire_str` pin | **move** |
| `parity.rs` | `parity.rs:61` | `"research_codebase"` | config key | stay |
| TS union/keys/labels | `types.ts:8,28,74,94` | `"research"` | DocTypeKey | **move** |
| pipeline step | `types.ts:308` | `docType: "research"` | doc-type token | **move** |
| parity test | `pipeline-step-parity.test.ts:15` | `"research"` | `CANONICAL_PRESENT_ORDER` | **move** |
| dev harness | `DevDesignSystem.tsx:732` | `has("research")` | present-token check | **move** |
| flag fields | `cluster.rs:52,550,565`; `clusters.rs:15,32`; `types.ts:238,282,306` | `has_research`/`hasResearch` | completeness flag | stay |

`STAGE_PUSH_ORDER` (`cluster.rs:134-147`) is the source of truth; it feeds
`Completeness.present` (`cluster.rs:580-585`). The cross-language parity test
`pipeline-step-parity.test.ts:32-44` compares `CANONICAL_PRESENT_ORDER`
(literal) against `LIFECYCLE_PIPELINE_STEPS.map(s => s.docType)` (typed
`DocTypeKey`), so `research` sits on both sides of one `toEqual` — the five sites
must move together. `DevDesignSystem.tsx:732` (`hasResearch: has("research")`)
is the one present-token check the work item does not name.

### 7. VR baselines, fixtures, and the Docker harness

`fixture-coverage.spec.ts`
(`frontend/tests/resolved-styles/fixture-coverage.spec.ts`, 29 lines) iterates
`DOC_TYPE_KEYS`, emitting a listing test (`/library/<type>`, asserts an `h1`)
and a detail test (`/library/<type>/<slug>` from `DETAIL_ROUTE_SLUGS`, asserts
`<article>` or, for `templates`, the templates layout). So each type needs a
`DOC_TYPE_KEYS` entry, `DETAIL_ROUTE_SLUGS` + `DETAIL_ROUTE_RENDERS_ARTICLE`
entries (both compiler-checked), **and** a checked-in server fixture whose
derived slug matches, or the detail `toBeVisible()` times out.

VR baselines live under
`frontend/tests/visual-regression/__screenshots__/{spec}-snapshots/`. The
project suffix is mandatory — real filenames are
`<docType>-<size>-<theme>-visual-regression.png` (glyph, sizes 16/24/32/48 ×
light/dark = 8) and `<docType>-<theme>-visual-regression.png` (big-glyph ×
2) = **10 per type**. The work item's shorthand (`research-16-dark.png`) omits
the `-visual-regression` suffix. The 10 `research-*-visual-regression.png` files
rename to `codebase-research-*-visual-regression.png` (pixel-identical — the
specs name from `DOC_TYPE_KEYS`). Specs
`dev-design-system-glyph.spec.ts` / `-big-glyph.spec.ts` enumerate
`SIZES × themes × DOC_TYPE_KEYS`, capturing per-`data-testid` cells that
`DevDesignSystem.tsx` must render.

VR runs **only** through Docker: `mise run test:e2e:visualiser:docker:update`
(regenerate, `mise.toml:415-418`) and `:docker` (compare, `410-413`) →
`playwright.docker.config.ts` (`testDir: ./tests/visual-regression`). Native
`mise run test` → `test:e2e:visualiser` uses the default `playwright.config.ts`
(`testDir: ./e2e` + `./tests/resolved-styles`) and never emits screenshots.

Server fixtures sit at `server/tests/fixtures/meta/research/` (flat `research`
top-level, nested `design-inventories/<slug>/inventory.md`). A topic-research
set belongs at `.../meta/research/topics/<slug>/manifest.md` (catalogue default
`research_topics → meta/research/topics` is already present,
`catalogue.rs:64-67`). ⚠️ The `findings/` + `reports/` subdirectory shape the
work item's fixture describes has **no precedent** — but it is harmless: `list()`
only joins `<slug>/manifest.md`, never descending, so sub-documents are ignored
by construction (which is exactly what the fixture is meant to prove).

## Work-item gaps and corrections

Surfaces the change forces that the work item's Requirements/References do not
name:

- **`templates-schema.tsv:15`** — TSV mirror of the schema manifest row;
  `every_row_matches_templates_schema_tsv` fails if it diverges from the
  collapsed `schema.rs` row.
- **`NON_CANONICAL_PER_KIND`** (`FrontmatterChips.test.tsx:437`) — a total
  `Record<DocTypeKey,…>`; new key = compile error; rename must move its
  `research` key.
- **`cli/corpus/src/slug.rs`** — nested slug derivation; `Research`/
  `DesignInventories` sit in the `strip_prefix_date_and_optional_id` arm
  (`slug.rs:21-30`). `TopicResearch` needs a deliberate arm choice (its slug
  dirs are subject names like `model-context-protocol`, not date-prefixed).
- **`DevDesignSystem.tsx:732`** — `hasResearch: has("research")` present-token
  check moves on rename.
- **`global.test.ts` `var(--atomic-*)` ref-counts** (`325-327`) and the
  hard-coded **`stageKeys`** list (`405`, contains literal `"research"`) — both
  need hand edits.
- **`Glyph.module.css:18`** — `.frame[data-doc-type="research"]` selector; if
  not renamed, the framed background tint silently drops after the key rename.
- **VR baseline `-visual-regression` suffix** — the rename/add must use the full
  on-disk filename, not the work item's shorthand.
- **Count target** — post-story `all()` is `[Self; 15]` and `api_types.rs` len
  is 15; `DOC_TYPES` is 14. The "unchanged (14)" phrasing is rename-scoped.
- **Fixture config wiring** — the frontend e2e server (`start-server.mjs:82-85`)
  remaps only `research_codebase`; `topics` resolves via catalogue default, so
  no explicit remap is needed, but confirm the fixture root the coverage spec
  reads includes `meta/research/topics/`.

## Risks and open questions

- ⚠️ **Humanisation: sentence-case vs title-case (resolved).** AC-6 asserts
  `model-context-protocol` → "Model context protocol" (first letter only). The
  `title_from` fallback uses `humanise_slug` (`slug.rs:160-198`), which
  title-cases each word → "Model Context Protocol" — confirmed by reading the
  code and its test (`slug.rs:415`). Two further AC divergences: `humanise_slug`
  splits on `-` only (not `_`, as the AC states) and strips one leading date/ID
  prefix; both are moot for `-`-delimited subject slugs. **Recommendation:**
  revise AC-6 to the title-cased string and reuse the canonical humaniser; do not
  add per-type sentence-case logic. The "neither title nor H1" fixture's test
  will assert "Model Context Protocol" from the real code as written.
- ⚠️ **Status-chip fill saturation ≥ 15%.** The stock `Chip` fill is
  `color-mix(accent 8%, bg)` — likely below the required saturation. A bespoke
  lifecycle-chip fill (or a reinterpretation of "fill") is probably required;
  the mandated five-state saturation/contrast unit test does not exist yet.
- ❓ **`report`-kind fixture doc with no schema row.** The work item's fixture
  set includes `reports/<slug>.md`, but no `report` schema row exists (0281).
  Server indexing ignores it (only `manifest.md` is walked), so it is safe there;
  confirm no whole-corpus frontmatter validation runs over the visualiser
  fixtures that would reject a `kind: report` document.
- ❓ **Hue separation is unguarded.** No test enforces the ≥15° inter-type hue
  distance; AC-13 is a by-construction argument, not an automated check.

## Code References

- `cli/corpus/src/doc_type.rs:9-45` — `DocTypeKey` enum, `all()`, exhaustiveness
- `cli/corpus/src/doc_type.rs:160-185` — `nested_manifest_filename`, `wire_str`
- `cli/corpus/src/slug.rs:21-30,160-167` — nested slug derive, `humanise_slug`
- `cli/corpus/src/cluster.rs:134-147,580-585` — `STAGE_PUSH_ORDER`, `present`
- `cli/corpus/src/frontmatter_validation/schema.rs:189-239` — topic-research rows
- `cli/corpus/src/frontmatter_validation/templates-schema.tsv:15` — TSV mirror
- `cli/config/src/catalogue.rs:64-67,70-84,275` — path key, `DOC_TYPES`, count
- `cli/visualiser/server/src/file_driver.rs:243-322` — `list` nested/flat branch
- `cli/visualiser/server/src/indexer.rs:1247-1371` — `build_entry`; `:72-105` status facet
- `cli/visualiser/server/src/frontmatter.rs:122-148` — `title_from` cascade
- `cli/visualiser/server/tests/parity.rs:56-97` — wire/config pins
- `cli/visualiser/server/src/clusters.rs:482-510` — present-order test mirror
- `cli/visualiser/frontend/src/api/types.ts:4-39,70-105,292-362` — union, keys, maps, pipeline steps
- `cli/visualiser/frontend/src/api/use-unseen-doc-types.ts:40-48` — localStorage rewrite precedent
- `cli/visualiser/frontend/src/api/pipeline-step-parity.test.ts:13-44` — cross-language parity
- `cli/visualiser/frontend/src/styles/tokens.ts:11-178` — hue + token families
- `cli/visualiser/frontend/src/styles/global.css:76-490` — three theme blocks
- `cli/visualiser/frontend/src/styles/global.test.ts:80-426` — parity + contrast + ref-count guards
- `cli/visualiser/frontend/src/components/Glyph/` and `components/BigGlyph/` — glyph dispatch + templates
- `cli/visualiser/frontend/src/api/status-variant.ts`, `components/Chip/Chip.module.css:25-52` — chip lexicon + fills
- `cli/visualiser/frontend/tests/resolved-styles/fixture-coverage.spec.ts` — per-type coverage
- `cli/visualiser/frontend/tests/lib/detail-route-slugs.ts:20-55` — slug + article maps
- `cli/visualiser/frontend/tests/visual-regression/__screenshots__/` — VR baselines
- `mise.toml:405-426`, `tasks/test/e2e.py` — VR Docker tasks
- `templates/topic-research-manifest.md`, `skills/research/research-topic/SKILL.md`, `cli/corpus-cli/tests/fixtures/topic-research-set/manifest.md` — 0277-delta targets

## Architecture Insights

- **Single-source enum, hand-synced parity.** `DocTypeKey` (Rust) and the TS
  union are kept in step by self-guarding tests (`parity.rs`, `global.test.ts`,
  `pipeline-step-parity.test.ts`), not codegen. The compiler catches missing
  total-`Record` keys; the tests catch value/token drift across languages.
- **Nested-manifest indexing is a one-method switch.**
  `nested_manifest_filename()` alone flips both `list` enumeration and
  `build_entry` slug/title derivation, so a new nested type is genuinely a mirror
  of `DesignInventories` plus its identity (label/wire/colour/glyph).
- **Three "research" namespaces.** The wire key (moves), the
  `has_research`/`hasResearch` completeness flag (stays), and the
  `research_codebase` config key (stays) only ever touch at pairing sites
  (`cluster.rs:137`, `DevDesignSystem.tsx:732`). Conflating them is the rename's
  main footgun.
- **Colour is doubly and triply duplicated by design** (ADR-0035): TS mirror +
  three CSS theme blocks, guarded by parity and ref-count tests — the reason a
  new token or a rename is more edit sites than it first appears.

## Historical Context

- `cli/migrate/src/migrations/m0004.rs` — the historical restructure that moved
  `research` → `codebase-research` for linkage name, config key, directory, and
  the user-overridable template; this is why only the wire token remains.
- `meta/decisions/ADR-0067-kind-discriminated-corpus-schema-and-templates.md` —
  the umbrella-type-plus-`kind` model this registration follows.
- `meta/decisions/ADR-0068-general-slug-resolution-in-the-corpus-cli.md` —
  directly relevant to the `slug.rs` derivation decision for the new type.
- `meta/work/0277-single-round-web-research-engine.md` — the co-land engine that
  landed the topic-research corpus/config scaffolding this story builds on.
- `meta/work/0121-topic-research-skillset.md` — parent epic; its artifact
  contract still asserts `research_status` and must move to base `status` in
  lockstep (recorded as a follow-up).
- `meta/reviews/work/0278-topic-research-visualiser-doc-type-review-1.md`,
  `…-review-2.md` — prior work-item reviews that shaped the current draft.

## Related Research

- `meta/research/codebase/2026-09-08-0277-single-round-web-research-engine.md`
- `meta/research/codebase/2026-08-06-0195-accelerator-corpus-cli-implementation-surface.md`
- `meta/research/codebase/2026-05-11-0056-restructure-meta-research-into-subject-subcategories.md`
- `meta/research/design-gaps/2026-09-10-current-app-vs-claude-design-prototype.md`
- Prototype:
  `meta/research/design-inventories/2026-09-10-174311-claude-design-prototype/prototype-full`

## Open Questions

1. ~~Does `humanise_slug` title-case each word?~~ **Resolved:** yes, it
   title-cases (`slug.rs:415`). Recommendation is to revise AC-6 to
   "Model Context Protocol" and reuse the canonical humaniser.
2. Can the five lifecycle chips meet ≥15% fill saturation with the stock `Chip`
   `color-mix(… 8%)` fill, or is a bespoke chip fill required?
3. Is any whole-corpus frontmatter validation run over the visualiser fixtures
   that would reject the `reports/<slug>.md` `kind: report` document before 0281
   adds its schema row?
