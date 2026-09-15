---
type: "codebase-research"
id: "2026-09-12-0278-complete-codebase-research-rename"
title: "Completing the bare-research to codebase-research rename"
date: "2026-09-12T21:24:50+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0278"
parent: "work-item:0278"
relates_to: ["codebase-research:2026-09-08-0277-single-round-web-research-engine", "codebase-research:2026-09-10-0278-topic-research-visualiser-doc-type-indexer"]
topic: "Completing the bare-research to codebase-research rename"
tags: ["research", "codebase", "rename", "doc-type", "visualiser", "corpus"]
revision: "6b59f552dfc580d6b1aeb3226efd060b7d6db904"
repository: "accelerator"
last_updated: "2026-09-12T21:47:47+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Added follow-up research: scope decision (Tier 0-3 + Tier 4 triage), the title-strip migration (m0010), and coordinated template/skill/migration change"
schema_version: 1
---

# Completing the bare-research to codebase-research rename

**Date**: 2026-09-12 21:24 UTC
**Author**: Toby Clemson
**Git Commit**: 6b59f552dfc580d6b1aeb3226efd060b7d6db904
**Branch**: jj change `xwsplpnxmnwz` (off `main`)
**Repository**: accelerator

## Research Question

Work items 0277 (single-round web research engine) and 0278 (topic-research
visualiser doc type) renamed the old "Research" doc type to "Codebase research"
to make room for a new "Topic research" type. Despite an attempt to rename
everywhere, bare `research` references are believed to remain. Map every
surviving `research` reference across the codebase, separate the ones that
denote the codebase-research concept (which must move) from legitimate umbrella
naming (which must stay), and scope the work needed to finish the rename.

## Summary

**Every compiler- or test-enforced surface is already migrated; what remains is
non-enforced.** The Rust doc-type registry, the frontend `DocTypeKey` maps, all
key-derived CSS tokens, the URL route, the parity tables, and the `localStorage`
last-seen rewrite already read `codebase-research`. Nothing that a type-checker,
`parity.rs`, `global.test.ts`, or the public-API snapshot guards is still bare.
That is why `mise run check` passes while bare `research` persists — the residue
lives exactly where no gate looks.

**The 776 bare `research` tokens are mostly load-bearing namespace, not leaks.**
Five distinct namespaces share the word: the codebase-research doc type (moves),
the `meta/research/` umbrella directory and its `research_*` config keys (stay),
the `skills/research/` skill namespace (stays), sibling corpora `research_issues`
/ `research_design_*` (out of scope), and the generic `researcher` agent (stays).
The intellectual core of the plan is the boundary, not a find-replace — a blind
substitution would break the umbrella directory, the config keys, the linkage
parser, and the `cargo-public-api` snapshot.

**The genuine remaining work is small, ranked below, and one item is an actual
error.** `skills/config/configure/SKILL.md:930` tells users the template key is
`research`; it is `codebase-research`, and the same file says so 500 lines
earlier. Everything else is display copy (the artifact's own "Research:" title,
"research document" prose, a handful of frontend labels) and a template-name
namespace lag confined to tests and fixtures — plus two coupled surfaces
(`## Related Research` heading, `hasResearch` field) that must be left alone or
touched with care.

The rename is **code-only**: no `accelerator migrate` migration ever rewrote
`type: research` → `type: codebase-research` (the m0004 migration moved the
config key, template file, and directory; `type:` frontmatter and linkage tokens
were `codebase-research` from birth). So no data migration is owed.

## Detailed Findings

### The five "research" namespaces (the boundary)

Getting this boundary right is the whole task. Every occurrence falls into one
bucket, and only the first moves.

| Namespace | Example | Verdict |
|---|---|---|
| Codebase-research doc type | `wire_str` `research`, label "Research", template stem, artifact title | Move → `codebase-research` / "Codebase research" |
| `meta/research/` umbrella dir | `research_codebase`, `research_topics`, `research_issues`, `research_design_*` config keys + dirs | Stay — groups all corpora under `meta/research/` |
| `skills/research/` skill namespace | parent of `research-codebase`, `research-topic`, `research-issue`, `conduct-spike`, `profiles/`, `outputters/` | Stay — groups the research-flavoured skills |
| Sibling corpora | `issue-research`, `design-inventories`, `design-gaps` | Out of scope |
| Generic agents | `researcher`, `web-search-researcher` | Stay |

### What is already done (enforced surfaces)

These were verified as fully migrated; listing them scopes the work by exclusion.

- **Rust registry** — `doc_type.rs` `wire_str()` (`codebase-research`), `label()`
  ("Codebase research"), `linkage_type_name()`, the `(type,kind)` schema rows,
  `cluster.rs` completeness `present` token, `catalogue.rs` `DOC_TYPES` row and
  `TEMPLATE_KEYS`, `template_shape.rs`/`schema.rs` source-type vocab.
- **Visualiser server** — `clusters.rs:502` present token, `parity.rs:63` wire
  field, the checked-in topic-research fixture set, `api_types.rs` count.
- **Frontend** — `DocTypeKey` union, `DOC_TYPE_KEYS`, every `Record<DocTypeKey,…>`
  map key, `DOC_TYPE_LABELS`/`_SINGULAR` ("Codebase research"), all `--ac-doc-*`
  / `--ac-doc-bg-*` / `--ac-stage-*` tokens, `data-doc-type`/`data-stage`
  attributes, the dynamic route (`/library/codebase-research/<slug>`),
  `pipeline-step-parity` `CANONICAL_PRESENT_ORDER`, and the one-shot
  `localStorage` `research`→`codebase-research` rewrite in
  `use-unseen-doc-types.ts:52-54` (present and tested).

### Remaining work, tiered

**Tier 0 — factual error (fix regardless of scope decisions).**

| file:line | current | should be |
|---|---|---|
| `skills/config/configure/SKILL.md:930` | template key `research` | `codebase-research` (self-contradicts `:396` and `configuration.md:82`) |

**Tier 1 — the artifact's own identity.** The codebase-research doc type is now
labelled "Codebase research", but every document it emits still titles itself
"Research:" and the skill calls its output a "research document". This is the
most visible remaining leak — it reproduces on every new codebase-research doc.

- `templates/codebase-research.md:4` (frontmatter `title:`), `:22` (body H1),
  `:59` (related-docs prose). `:58` `## Related Research` is Tier 5 (coupled).
- `skills/research/research-codebase/SKILL.md:137` (sets the artifact `title:`),
  and the own-artifact "research document(s)" nouns at
  `:110,123,164,170,192,205,210,224,226,227`.
- `skills/config/configure/SKILL.md:877,884,100` (template filename example +
  template-type lists).
- Hand-written doc-type enumerations: `docs-site/…/configuration.md:70`,
  `docs-site/…/visualiser.md:13`.
- ⚠️ Each `SKILL.md` has a generated mirror under
  `docs-site/src/content/docs/reference/skills/…`; fix the source, regenerate the
  mirror (do not hand-edit both). The generated `docs-site/.astro/data-store.json`
  is a build cache — ignore it.

**Tier 2 — template-name namespace lag (tests/fixtures only; production works).**
The real template is `templates/codebase-research.md`; the config key is
`templates.codebase-research`. The frontend resolves its glyph correctly by
tokenising `codebase-research` on dashes and matching the `research` sub-token
(`STEM_TO_GLYPH['research']` in `template-tier.ts:40`, documented in its comment)
— so this is **not** a production bug, only a naming inconsistency where a bare
`research` template name still appears.

- `template-tier.ts:40` — key `research` maps to `codebase-research`. Renaming
  the key to `codebase-research` still resolves (whole-name match) and is
  clearer; no other template carries a bare `research` token.
- Server synthetic-template test seeds: `cli/visualiser/server/src/templates.rs:442,446`,
  `tests/common/mod.rs:56`, `tests/api_related.rs:37`,
  `tests/api_work_item_pattern.rs:42,197`.
- Server fixture `cli/visualiser/server/tests/fixtures/templates/research.md`
  (its stem is consumed as a live template name by `api_smoke.rs`).
- Frontend assertions `LibraryTemplatesIndex.test.tsx:53,77,197,267-269`.

**Tier 3 — frontend display copy.**

- `frontend/src/api/types.ts:313` — `LIFECYCLE_PIPELINE_STEPS` caption
  `label: "Research"` (the last bare "Research" display label; canonical labels
  already read "Codebase research"). Paired: `:314` placeholder `"no research
  yet"` and its test `LifecycleClusterView.test.tsx:155` (`/^No research yet$/i`).
- `frontend/src/routes/library/empty-descriptions.ts:104` — `EMPTY_TYPE_PLURALS`
  value `"research notes"`.
- `frontend/README.md:3` — prose doc-type listing.

**Tier 4 — prose labels across skills and docs (large, low-risk, judgement).**
Display labels resolving `research_codebase` ("**Research directory**",
"**Research:**") and "research document" nouns, spread across `research-codebase`,
`config/init`, `visualise`, `extract-adrs`, `extract-work-items`, `create-plan`,
`review-plan`, `stress-test-plan`, `create-note`, `create-work-item`,
`conduct-spike` SKILLs; the `plan.md`/`adr.md`/`work-item.md` cross-reference
labels; and ~15 `docs-site` narrative pages. Full file:line lists in the
per-area audits; representative anchors: `research-codebase/SKILL.md:24`,
`extract-adrs/SKILL.md:7,27,46`, `templates/plan.md:124`, `templates/work-item.md:84`,
`docs-site/…/workflow.md:30`, `docs-site/…/getting-started.mdx:11`. Pure
verb/phase usage ("research the codebase", the "research → plan → implement"
spine, the generic `researcher`) is umbrella and stays.

**Tier 5 — coupled surfaces (leave alone, or change with care).**

- ⚠️ `## Related Research` heading is **parser-coupled**. `cli/corpus/src/linkage.rs:52`
  (`SECTIONS[3]`) and `:364` parse this literal heading to detect `derived_from`
  links, and it groups **both** codebase-research and issue-research links — it is
  an umbrella heading. Changing it means changing the parser token, every
  template that emits it (`templates/codebase-research.md:58`, `plan.md`, `adr.md`),
  and the corpus-cli/goldens in lockstep. Recommend **keep**.
- ⚠️ `hasResearch` completeness field is **deliberately kept per 0278** and spans
  layers: `cli/corpus/src/cluster.rs` (`has_research`), `server/src/clusters.rs:15,32`,
  `frontend/src/api/types.ts:243`, and the `public-api.txt` snapshot. Renaming
  shifts the wire JSON and the public-API snapshot. Recommend **keep** unless a
  deeper consistency pass is explicitly wanted.

**Tier 6 — cosmetic test-local identifiers (optional).** Behaviourally inert:
`server/src/clusters.rs` test fn/vars/comments/`"Research"` title literal
(`:763,765,770,791,795,802,803,809,812,814`), `Pipeline.test.tsx` `research`/
`researchConnector` locals, `Sidebar.test.tsx` `researchLink`, `corpus/src/linkage.rs:906`
test var. Rename only if aligning descriptive naming; consistent with keeping the
`DocTypeKey::Research` identifier.

### The boundary — do not touch

| Surface | Why it stays |
|---|---|
| Rust variant `DocTypeKey::Research` + all call sites + `public-api.txt` | Renaming shifts `cargo-public-api` and every call site (0278 decision) |
| Config keys `research_codebase`/`research_topics`/`research_issues`/`research_design_*` | Umbrella `meta/research/` bindings, moved by m0004 |
| `meta/research/` dir, `skills/research/` namespace | Group all corpora / research skills |
| Glyph identifiers `ResearchIcon`, `ResearchBigGlyph` | Component unchanged; only map keys moved |
| `DETAIL_ROUTE_SLUGS` value `2026-01-01-first-research`, hue `28` | Separate namespaces, unchanged by the rename |
| Migrations `m0004`/`m0006`/… + fixtures + `CHANGELOG.md` | Immutable applied history |
| `OBSOLETE_LEGACY_KEYS` `research_status` (`schema.rs:317`) | Must stay to reject the collapsed legacy key |

### Adjacent follow-ups (related, not "code" leaks)

- **Sibling work items carry stale vocabulary.** 0279/0281/0282 still say
  `research_status`, `research_kind`, and "set handle", superseded by ADR-0067
  (`kind`), the 0278 status collapse (base `status`), and ADR-0068 ("slug").
  These are `meta/work/` documents, not code.
- **Epic 0121 artifact-contract reconciliation** is a recorded 0278 follow-up
  (manifest frontmatter row + `research_status` transition table + "never infer
  set progress" rule).

## Code References

- `cli/corpus/src/doc_type.rs:81,108,179` — `linkage_type_name`/`label`/`wire_str`, all migrated.
- `cli/config/src/catalogue.rs:89` — `templates.codebase-research` (canonical template key).
- `cli/visualiser/frontend/src/routes/library/template-tier.ts:40` — `STEM_TO_GLYPH['research'] → codebase-research` (live via token match).
- `cli/visualiser/frontend/src/api/types.ts:313-314` — bare `"Research"` caption + `"no research yet"` placeholder.
- `cli/visualiser/frontend/src/api/use-unseen-doc-types.ts:52-54` — the last-seen rewrite (done).
- `cli/visualiser/server/src/templates.rs:442,446` + `tests/fixtures/templates/research.md` — stale template name in tests/fixtures.
- `cli/corpus/src/linkage.rs:52,364` — `## Related Research` parser token (coupled).
- `skills/config/configure/SKILL.md:930` — factual error (template key).
- `templates/codebase-research.md:4,22,58,59` — artifact title/H1/heading/prose.
- `skills/research/research-codebase/SKILL.md:137` + own-artifact nouns — artifact identity.

## Architecture Insights

- **Rust↔TS parity is hand-synced, so the guards are per-language.** `parity.rs`
  and `global.test.ts` each pin their own side; the rename passed both because it
  moved keys, not counts. Nothing cross-checks display copy or template names,
  which is precisely where the residue survives.
- **Template identity is resolved by dash-token matching, not whole names.** The
  frontend deliberately keys glyphs on sub-tokens (`research` inside
  `codebase-research`), so the template can be renamed on disk without a matching
  frontend map entry. This is why production works despite the naming lag — and
  why the lag is invisible to tests.
- **The linkage parser binds prose headings to behaviour.** `## Related Research`
  is not decorative; it is a scanned token that also serves issue-research. Prose
  and parser are coupled, so "just renaming a heading" is a multi-file change.
- **`type:`/linkage were never the wire key.** The wire token is a pure
  serialisation concern in `DocTypeKey`; because m0004 set `type:` frontmatter to
  `codebase-research` from the start, the wire rename needs no data migration.

## Historical Context

- `meta/research/codebase/2026-09-10-0278-topic-research-visualiser-doc-type-indexer.md`
  — established the wire-token-only nature of the rename and pre-identified
  surfaces the work item missed (the `templates-schema.tsv` row, `NON_CANONICAL_PER_KIND`,
  `slug.rs`, `DevDesignSystem.tsx`, `global.test.ts` counts, the `Glyph.module.css`
  selector). Those enforced surfaces are now migrated.
- `meta/research/codebase/2026-09-08-0277-single-round-web-research-engine.md`
  — the `(type, kind)` schema generalisation and the `research_kind` → `kind`
  rename (ADR-0067), and the general `corpus resolve --type <type> <slug>` (ADR-0068).
- `meta/decisions/ADR-0067-kind-discriminated-corpus-schema-and-templates.md`,
  `meta/decisions/ADR-0068-general-slug-resolution-in-the-corpus-cli.md` — the
  accepted decisions behind the sibling work items' now-stale vocabulary.
- `cli/migrate/src/migrations/m0004.rs` (work item 0056,
  `restructure-meta-research-into-subject-subcategories`) — the historical move
  of `paths.research`→`research_codebase`, `templates/research.md`→`codebase-research.md`,
  and top-level `meta/research/*` into `meta/research/codebase/`. Immutable.

## Related Research

- `meta/research/codebase/2026-09-10-0278-topic-research-visualiser-doc-type-indexer.md`
- `meta/research/codebase/2026-09-08-0277-single-round-web-research-engine.md`
- `meta/research/codebase/2026-05-11-0056-restructure-meta-research-into-subject-subcategories.md`

## Open Questions

These are scope decisions for the plan, not unknowns in the code.

1. **How far does "thorough" go?** Tier 0–3 (error, artifact identity,
   template-name consistency, frontend copy) is a bounded, low-risk change and
   the recommended default. Tier 4 (all prose "Research directory" / "research
   document" labels across skills and docs) is a much larger consistency pass —
   include it or leave the umbrella-adjacent prose as-is?
2. **Rename the artifact's own title?** Should new codebase-research docs title
   themselves "Codebase research: {topic}" (H1 + frontmatter `title:`) instead of
   "Research: {topic}"? This is the most visible leak but changes the convention
   for every future doc (existing on-disk docs keep their titles).
3. **Confirm the two coupled keeps.** Leave `## Related Research` (parser-coupled,
   umbrella over codebase+issue) and `hasResearch` (cross-layer + public-API) as
   deliberately kept? Recommended: yes.
4. **In scope for this effort or separate?** The stale sibling work items
   (0279/0281/0282) and the epic-0121 contract reconciliation are `meta/`
   documents, not code — fold into this plan or track separately?

## Follow-up Research 2026-09-12 21:47 UTC

Author decisions on the four open questions, and the research they trigger.

### Scope decided

| Question | Decision |
|---|---|
| How far | Tier 0-3 in full; **assess** Tier 4 for obvious cases only |
| Artifact title | **Strip** the `Research: ` prefix (not rename to "Codebase research:"), and migrate existing docs to match |
| Coupled keeps | Confirmed — `## Related Research` and `hasResearch` stay |
| Adjacent docs | Track separately — 0279/0281/0282 and epic 0121 are out of this plan |

The title decision reverses the earlier "no migration owed" finding: stripping
existing on-disk docs requires a new `accelerator migrate` migration. It is a
data change, distinct from the wire rename (which remains code-only).

### Tier 4 assessment — the obvious-migrate set

Two clusters are genuinely ambiguous now that two research corpora coexist and
each resolves to a *specific* corpus; the rest is umbrella/verb prose that reads
correctly and stays.

**Migrate (obvious):**

- **`**Research directory**` labels that resolve `research_codebase`.** Each sits
  in a list beside `**Plans directory**`, `**Decisions directory**`, etc., and
  now names one of two research directories (`research_topics` is the other). →
  "Codebase research directory". At `research-codebase/SKILL.md:24`,
  `config/init/SKILL.md:22`, `visualise/SKILL.md:16`, `extract-adrs/SKILL.md:27`,
  and the `extract-work-items` source labels (+ generated mirrors).
- **Template cross-reference labels pointing at `meta/research/codebase/`.** The
  path is codebase-specific but the label is bare: `templates/plan.md:124`
  ("Related research"), `templates/work-item.md:84` ("Research:"),
  `templates/adr.md:62` ("Related research"). → "Codebase research" / "Related
  codebase research". The path already commits to the corpus; the label should
  match.

**Leave (umbrella / verb / generic — reads fine):**

- The skill H1 `# Research Codebase` (imperative brand title) and the "research →
  plan → implement" phase spine.
- Verb usage ("research the codebase", "conduct research", "spawn research
  tasks").
- Generic doc-type lists in narrative docs and in `create-plan`/`create-work-item`
  prose that mean research artefacts broadly, and `pr-description.md:38`
  ("work item, plan, or research document").
- `extract-adrs` `argument-hint`/`description` — user-facing input guidance;
  borderline, defer.
- Tags (`["research", "codebase", …]`) and non-coupled section headings
  (`## Research Question`, `## Follow-up Research`).

### The title-strip migration — mechanics

**Next migration: `0010-strip-research-title-prefix`** (highest current is
`Migration0009`). It is a new module inside the existing `cli/migrate/` library
crate — not a new crate or sub-binary, so neither `tasks/README.md` registration
checklist applies beyond the library-crate public-API discipline.

Registration — three edits, mirroring the m0006 shape (the closest precedent, a
frontmatter-value + body-label rewrite over `research_codebase`):

1. `cli/migrate/src/migrations/mod.rs` — `pub mod m0010;`.
2. `cli/migrate/src/registry.rs` — `use crate::migrations::m0010::Migration0010;`
   and a trailing `MigrationEntry::Mechanical(Box::new(Migration0010)),` in
   `registry()`.
3. `cli/migrate/src/migrations/m0010.rs` — `struct Migration0010` implementing
   `MigrationMeta` (`id`/`description`) + `Migration` (`apply`).

Selection and rewrite:

- **Select by `paths.research_codebase`** (default `meta/research/codebase/`) via
  `ctx.config_value` + `ctx.list_md_files`, exactly as m0006 walks its corpora.
  This key is the canonical codebase-research selector and naturally excludes
  `research_topics` and `research_issues`. Mutate through `ctx.write` (the only
  manifest-tracked path).
- **Frontmatter:** strip `Research: ` only when the title value starts with it,
  inside the quotes — `title: "Research: X"` → `title: "X"`. Reuse m0006's
  quote-aware helpers (`is_double_quoted`/`semantic_inner`). On-disk reality:
  177/180 titles are exactly `title: "Research: …"` (double-quoted, uniform); the
  3 exceptions are embedded template examples in doc bodies, which the migration
  never touches (it rewrites only the document's own frontmatter block).
- ⚠️ **Body H1:** rewrite `# Research: X` → `# X` **anchored** to the pre-first-`## `
  region (m0006's `extract_pre_h2` / `!saw_first_h2` guard) and prefix-matched at
  line start, so it never touches `#`-comment lines inside code fences (the corpus
  is full of them) or later headings.
- **Idempotent** by construction: a second run finds no prefix and no-ops
  (byte-stable), the tested property every mechanical migration must hold.

Files the migration forces to change:

- **Source (4):** the two registration files, the new `m0010.rs`, and the
  regenerated public-API snapshot `cli/migrate/tests/fixtures/public-api.txt`
  (`pub mod migrations` re-exports the struct) via `mise run public-api:update`.
- **Tests:** a new `cli/migrate-cli/tests/migration_0010.rs` (apply +
  idempotency), plus the hardcoded full-ledger `already_applied()` strings that
  must gain the `0010-…` id — `migration_0001.rs` through `0007`, `0009`,
  `skip_unskip_unapply.rs`, `no_pending_and_help.rs`, `full_registry_e2e.rs`
  (its `applied:`/`pending:` counts shift), `dirty_tree_preflight.rs`; review
  `migration_0008.rs` and `list_and_decisions_file.rs` (partial ledgers — 0010
  becomes newly pending).
- **No new `fixtures/0010/` tree** — those fixtures are retired bash-golden
  provenance the Rust tests do not replay; `regenerate.sh` is inert.
- **No visualiser-fixture edits** — the fixtures under
  `cli/visualiser/server/tests/fixtures/meta/research/` do not use the `Research: `
  prefix, and the `migrate-byte-equiv` golden has no `research/codebase/` tree.

### The change is three coordinated parts

The template and skill stop *emitting* the prefix; the migration strips it from
what already *exists*.

1. **Template** — `templates/codebase-research.md:4` `title:` and `:22` H1 drop
   the `Research: ` prefix (new docs are born clean).
2. **Skill** — `skills/research/research-codebase/SKILL.md:137` sets the artifact
   `title:` without the prefix (regenerate its docs-site mirror).
3. **Migration** — `m0010` strips the prefix from the ~177 existing corpus docs.

⚠️ This very research document and its two predecessors carry `Research: ` titles
and will be stripped by `m0010` — intended and consistent. ⚠️ Running `m0010` on
the live repo rewrites real corpus files, so `mise run` will show a dirty tree
afterwards; that is the migration working, not a preflight fault.

### Net remaining-work shape for the plan

- Tier 0: the `configure/SKILL.md:930` factual fix.
- Tier 1: template/skill artifact identity + the `m0010` migration (title strip).
- Tier 2: template-name consistency in tests/fixtures + `STEM_TO_GLYPH` key.
- Tier 3: the three frontend copy strings.
- Tier 4 (obvious only): the `**Research directory**` labels + three template
  cross-reference labels.
- Keep: `## Related Research`, `hasResearch`, and the whole boundary table above.
- Every touched `SKILL.md` regenerates its `docs-site` mirror; close with
  `mise run fix && mise run check`.
