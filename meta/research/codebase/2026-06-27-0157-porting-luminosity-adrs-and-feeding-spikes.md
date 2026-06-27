---
type: codebase-research
id: "2026-06-27-0157-porting-luminosity-adrs-and-feeding-spikes"
title: "Research: Porting luminosity ADRs and feeding spikes into Accelerator"
date: "2026-06-27T12:00:42+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0157"
parent: "work-item:0157"
relates_to: ["codebase-research:2026-06-23-0136-shell-scripts-rust-cli-migration-surface"]
topic: "Porting luminosity ADRs and feeding spikes into Accelerator"
tags: [research, codebase, adr, decisions, porting, luminosity, rust-cli, schema, supersession]
revision: "ecd3d7804d72058f35c87f12d549539b829fd6eb"
repository: "accelerator"
last_updated: "2026-06-27T12:00:42+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Porting luminosity ADRs and feeding spikes into Accelerator

**Date**: 2026-06-27T12:00:42+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: ecd3d7804d72058f35c87f12d549539b829fd6eb
**Branch**: HEAD (detached / jj working copy)
**Repository**: accelerator

## Research Question

To prepare an implementation plan for work item
[0157 — Port Luminosity ADRs and Feeding Spikes into Accelerator](../../work/0157-port-luminosity-adrs-and-feeding-spikes.md):
what is the full content and structure of the eleven luminosity ADRs and two
feeding spikes; what local Accelerator schema/convention they must conform to;
where the candidate overlaps with existing local ADRs actually lie; and what
non-obvious editorial, numbering, cross-reference, and supersession work the
port entails?

## Summary

The port is **mechanically straightforward but editorially deep**. The eleven
luminosity ADRs are already accelerator-derived (same template, same
`adr:`/`work-item:` linkage syntax, same body sections), so schema drift is
minor — chiefly identity fields, dates, authorship, and reference rewrites.
**Numbering is unambiguous and confirmed live**: next free local ADR is
**ADR-0045** (`adr-next-number.sh` → `0045`); next free work item is **0158**
(`work-item-next-number.sh` → `0158`). The natural assignment is
**lum ADR-0001…0011 → local ADR-0045…0055** (a uniform +44 shift) and
**lum spike 0002 → local 0158, lum spike 0003 → local 0159**.

But three findings make this **not** a find-and-replace job:

1. **Perspective inversion.** Every luminosity ADR is written *from
   luminosity's vantage point looking at Accelerator* — Accelerator appears as
   external precedent ("adopt Accelerator's proven config model"), as a
   cautionary tale ("Accelerator's bash body … the model ADR-0001 exists to
   leave behind"), or as a comparison repo (`../accelerator`,
   `https://github.com/atomicinnovation/accelerator`). Ported **into**
   Accelerator, those references become self-referential or nonsensical and
   must be rewritten in the first person. This is the single largest editorial
   task and directly contradicts the work item's "minimal edits" framing for a
   subset of the ADRs.

2. **The candidate overlaps in the work item are only partly accurate.**
   - Local **ADR-0001 is *not* "filesystem as message bus"** — it is *"Use
     Filesystem Communication, Agent Separation, and Token Budgets for Context
     Isolation"* and **bundles three concerns**. Only the filesystem-
     communication concern overlaps luminosity ADR-0008. The clean overlap is
     local **ADR-0027** ("Persist structured skill outputs to meta/"). A superset
     that supersedes ADR-0001 wholesale would discard its still-live agent-
     separation and token-budget decisions — so ADR-0001 **cannot be cleanly
     superseded**.
   - Local **ADR-0016 + ADR-0017** genuinely overlap luminosity ADR-0003 (config
     model). But luminosity ADR-0003 **explicitly adopts Accelerator's own
     ADR-0016/0017/0020/0021** as its proven basis — porting it back is nearly
     circular, and the ADR-0016/0017/0020/0021 numbers embedded in its prose are
     **already-correct local references**, not luminosity numbers to shift.

3. **A domain mismatch in ADR-0008.** Luminosity ADR-0008 splits the filesystem
   into `meta/` (shared memory) **and `content/`** (shippable marketing
   deliverables — articles, social, ads, imagery, video). Accelerator is a
   plugin, **not a content product** — the `content/` half does not apply and
   must be dropped/rewritten, leaving only the `meta/`-as-message-bus half (which
   is what overlaps ADR-0027).

The four "convention" ADRs the work item flagged for verification
(0004 toolchain, 0005 bash floor, 0006 mise+invoke, 0007 skills-as-product) are
all **net-new as ADRs** — Accelerator documents them only in CLAUDE.md/README
prose, with no existing ADR. So only **two supersets** are forced: config
(lum 0003 ↔ local 0016+0017) and filesystem (lum 0008 ↔ local 0027).

The ADRs cleave into two classes that should be planned differently:
**Class A — "Accelerator already does this"** (0003, 0005, 0006, 0007, 0008):
porting records existing convention/reality; edits are *heavy* (perspective
flip). **Class B — "Accelerator would adopt this" (the Rust-CLI direction)**
(0001, 0002, 0004, 0009, 0010, 0011): forward-looking, imported as `proposed`;
edits are closer to minimal (names/paths). All 11 import as `proposed` per the
work item regardless.

## Detailed Findings

### A. Local target schema — the conformance contract for ported artifacts

Drawn live from ADR-0029/0030/0031/0033/0034/0040. A ported ADR must match the
**live corpus** shape (which follows ADR-0033/0034/0040, *not* ADR-0030's stale
frontmatter list).

**Frontmatter (observed live key order; not alphabetised):**
`id, date, author, status, tags, type, title, schema_version, last_updated,
last_updated_by, <linkage keys>`.
- `type: adr`; `id: "ADR-NNNN"` (always quoted); `title: "ADR-NNNN: <title>"`;
  `schema_version: 1` (bare int); `status: proposed` for all imports
  (ADR-0031: new ADRs always start `proposed`).
- **`producer`**: ADR-0033 lists it as base "where applicable", but the
  **existing local ADRs (0001/0016/0017/0027) omit it**, while the luminosity
  ADRs carry `producer: create-adr`. Decision needed (see Open Questions); the
  safest match-the-local-corpus choice is to **omit `producer`** or set it to a
  porting marker.
- **Linkage keys are omit-when-empty (ADR-0040)** and use the typed form
  (ADR-0034): `parent` (single ref), `relates_to` (list), `supersedes` (list),
  `superseded_by` (single ref). Values are single quoted strings:
  `"adr:ADR-NNNN"`, `"work-item:NNNN"`. `decision_makers` is also omit-when-empty.
- ADRs are **not** code-state-anchored → **no `revision`/`repository`**
  provenance fields.

**Body sections (ADR-0030, required, in order):**
H1 `# ADR-NNNN: <title>` → in-body status block (`**Date**` / `**Status**` /
`**Author**`) → `## Context` → `## Decision Drivers` → `## Considered Options` →
`## Decision` → `## Consequences` (`### Positive` / `### Negative` /
`### Neutral`) → `## References`. The luminosity ADRs **already use exactly this
structure**, so no restructuring is needed — only content edits.

**Immutability (ADR-0031) — the gating mechanic for supersession:**
- Permitted transitions: `proposed → accepted|rejected` (via `review-adr`);
  `accepted → superseded` (via `create-adr --supersedes`);
  `accepted → deprecated` (via `review-adr --deprecate`). `rejected`,
  `superseded`, `deprecated` are terminal.
- **Content edits are only allowed while `proposed`.** A new ADR imported as
  `proposed` is fully editable.
- **The supersede edge cannot be applied while the superset ADR is `proposed`.**
  `accepted → superseded` requires the superseding ADR to be accepted first
  (and the supersession is enacted via `create-adr --supersedes`, which writes
  `superseded_by` on the old ADR and `supersedes` on the new). So the
  supersession of local ADR-0016/0017/0027 is a **deferred, post-acceptance
  step**, exactly as the work item states.

**ID allocation (verified):**
- `skills/decisions/scripts/adr-next-number.sh` → globs `ADR-NNNN*`, takes
  high-water-mark + 1, `printf "%04d"`. Live output: **`0045`**. Supports
  `--count`.
- `skills/work/scripts/work-item-next-number.sh` → pattern-driven
  (`work.id_pattern`, default `{number:04d}`), high-water-mark + 1. Live output:
  **`0158`**. Supports `--count`.
- File naming: `ADR-NNNN-description.md` (ADR-0029) and
  `<full-id>-kebab-slug.md` for work items.

### B. The luminosity source set — content, linkage, and provenance

All eleven ADRs are `status: accepted`, `author: Toby Clemson`,
`producer: create-adr`, `schema_version: 1`, dated 2026-06-24…26. Linkage uses
`parent: "work-item:NNNN"` and `relates_to: ["adr:ADR-NNNN", …]`. **None carries
`supersedes` or `superseded_by`.**

**Parent / provenance grouping (important — neither parent is being ported):**
- **lum ADR-0001…0008** → `parent: "work-item:0004"` (luminosity story "record
  existing implicit architecture decisions as ADRs").
- **lum ADR-0009…0011** → `parent: "work-item:0005"` (luminosity story "record
  spike-dependent architecture decisions as ADRs").
- Work items 0004 and 0005 are **not** in scope to port, so the ported ADRs'
  `parent` must be dropped or repointed (Open Question).

**Internal ADR cross-reference graph (luminosity numbering):**
- 0002→0001; 0003→0001,0002; 0004→0001,0002; 0005→0004,0002,0003,0001;
  0006→0001,0004,0005; 0007→0001; 0008→0007,0001; 0009→0001,0002,0004;
  0010→0001,0002,0009; 0011→0004,0006,0007,0008.
- Plus **in-prose ordinal references** ("the 5th ADR in this set", "decision 9",
  "decision 10/11") that also need rewriting to local ADR numbers.

**Feeding spikes (both `status: done`, `kind: spike`, research-only — no
prototype):**
- **lum 0002** "Modular Rust CLI Architecture & Hexagonal Workspace Layout"
  (~373 lines; `parent: work-item:0001`, `blocks: [0005,0007,0008]`; **no
  `external_id`, no `relates_to`**). Feeds **lum ADR-0009 (hexagon) and
  ADR-0010 (modular CLI/launcher)** — split of its Recommendation §1–§4.
- **lum 0003** "Skill Evaluation Framework Selection" (~316 lines;
  `parent: work-item:0001`, `blocks: [0005,0010]`; same key set). Feeds
  **lum ADR-0011 (Inspect harness)**.
- Both record outcomes **in the work item itself** (short `## Spike Outcome` +
  long `## Recommendation`). Their `parent`/`blocks` all point at luminosity
  work items not being ported → drop or repoint on port (Open Question).
- Date-sensitive facts to preserve verbatim as historical record: promptfoo
  "acquired by OpenAI (9 Mar 2026)"; sigstore-verification "archived May 2026".

### C. Per-ADR port classification and edit burden

| lum ADR | Title | Class | Overlap → supersedes | Edit burden |
|---|---|---|---|---|
| 0001 | Skills-vs-CLI Division of Labour | B (CLI direction) | net-new | **Heavy** — Accelerator is the cautionary precedent in the original; must reframe in first person |
| 0002 | Zero-Setup Static-Binary Distribution | B | net-new (spike-fed) | Medium — names/paths; Accelerator pipeline cited as "ported cleanly" |
| 0003 | Multi-Level Userspace Configuration Model | A (already done) | **lum 0003 → local 0016 + 0017** | **Heavy** — near-circular; embedded Accelerator ADR refs (0016/0017/0020/0021) stay as-is |
| 0004 | Three-Toolchain Split | A/B mix | net-new | **Heavy** — "three" → **four** toolchains (add TS/React); Rust-as-product is forward-looking |
| 0005 | Bash 3.2 Compatibility Floor | A | net-new | Medium — "only `.sh` is the linter" is false for Accelerator (226 `.sh` files) |
| 0006 | mise + invoke Task Runner | A | net-new | Light–medium — matches Accelerator reality; tool list edits |
| 0007 | Skills as the Product | A | net-new (README/CLAUDE.md have the concept, no ADR) | Medium — "inherited from Accelerator" reframed; keep min-version fact |
| 0008 | Filesystem as Message Bus & Knowledge Corpus | A | **lum 0008 → local 0027** (+ partial 0001) | **Heavy** — drop `content/` marketing half; keep `meta/` half |
| 0009 | Thin CLI over Hexagonal Core | B | net-new (spike-fed by 0002) | Medium — names/crates; tooling (cargo-deny/pup) |
| 0010 | Git-Style Modular CLI of On-Demand Static Binaries | B | net-new (spike-fed by 0002) | Medium — `luminosity` command/crate names |
| 0011 | Inspect as the Skill-Evaluation Harness | B | net-new (spike-fed by 0003) | Medium — `configure`-skill target; three-vs-four-toolchain note |

### D. Recommended ID-assignment and reference-rewrite map

A uniform **+44 shift** maps luminosity ADR numbers to free local numbers, which
keeps the rewrite mechanical *for luminosity self-references only*:

| lum ADR | → local ADR | lum `relates_to` (luminosity #s) | → rewritten local `relates_to` |
|---|---|---|---|
| 0001 | **ADR-0045** | — | — |
| 0002 | **ADR-0046** | 0001 | adr:ADR-0045 |
| 0003 | **ADR-0047** | 0001, 0002 | adr:ADR-0045, adr:ADR-0046 (+ supersedes local 0016, 0017) |
| 0004 | **ADR-0048** | 0001, 0002 | adr:ADR-0045, adr:ADR-0046 |
| 0005 | **ADR-0049** | 0004, 0002, 0003, 0001 | adr:ADR-0048, 0046, 0047, 0045 |
| 0006 | **ADR-0050** | 0001, 0004, 0005 | adr:ADR-0045, 0048, 0049 |
| 0007 | **ADR-0051** | 0001 | adr:ADR-0045 |
| 0008 | **ADR-0052** | 0007, 0001 | adr:ADR-0051, 0045 (+ supersedes local 0027) |
| 0009 | **ADR-0053** | 0001, 0002, 0004 | adr:ADR-0045, 0046, 0048 |
| 0010 | **ADR-0054** | 0001, 0002, 0009 | adr:ADR-0045, 0046, 0053 |
| 0011 | **ADR-0055** | 0004, 0006, 0007, 0008 | adr:ADR-0048, 0050, 0051, 0052 |

Spikes: **lum 0002 → local 0158**, **lum 0003 → local 0159** (allocate with
`work-item-next-number.sh --count 2`; the script confirms 0158 is next). Repoint
the spike-fed ADRs' provenance:
- ADR-0053 (was lum 0009) and ADR-0054 (was lum 0010) → primary provenance
  **work-item:0158**; luminosity original
  `https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0002-…`
  as secondary.
- ADR-0055 (was lum 0011) → primary provenance **work-item:0159**; luminosity
  `…/0003-…` as secondary.

> **Critical rewrite caveat:** the +44 shift applies **only to luminosity's own
> ADR numbers**. Luminosity ADR-0003's prose cites **Accelerator's** ADR-0016,
> 0017, 0020, 0021 — these are *already* correct local references (confirmed:
> all four files exist locally with matching titles) and must **not** be shifted.
> The implementer must read each ADR and classify every `ADR-NNNN` mention as
> either a luminosity self-reference (shift +44) or an Accelerator reference
> (leave as-is). This is per-ADR judgement, not sed.

### E. Supersession plan (only two supersets are forced)

**Superset 1 — Configuration model:** new **ADR-0047** (ported lum 0003) is
authored as the superset of {lum 0003, local ADR-0016, local ADR-0017}. Once
ADR-0047 is **accepted** (via `review-adr`), enact
`accepted → superseded` on local 0016 and 0017 via `create-adr --supersedes`
semantics. Note local ADR-0016 is already *partially* superseded on file paths
(`.claude/accelerator*.md` → `.accelerator/config*.md` per work item 0031), and
its body says "a full superseding ADR is forthcoming" — **ADR-0047 is that
forthcoming superseder**, which is a clean fit.

**Superset 2 — Filesystem as message bus:** new **ADR-0052** (ported lum 0008)
is authored as the superset of {lum 0008 (meta-half only), local ADR-0027}.
Supersede local **ADR-0027** once ADR-0052 is accepted.
- **Do *not* supersede local ADR-0001.** Its filesystem-communication concern
  overlaps, but it also carries live agent-separation and token-budget
  decisions that ADR-0052 does not subsume. Link ADR-0052 → ADR-0001 via
  `relates_to`, not `supersedes`. (Flag for confirmation — see Open Questions.)

All supersede edges are **deferred to post-acceptance**; at import time every
ported ADR is `proposed` and carries no `superseded_by`/`supersedes` edge to a
local ADR yet (the superset ADRs may pre-declare `supersedes` of 0016/0017/0027
in prose, but the enforced edge only lands at acceptance).

### F. Luminosity-reference rewrite inventory (the editorial core)

Recurring tokens to edit, by category:
- **Repo/product name:** "Luminosity" → "Accelerator" (or first-person "we"/
  "this plugin") throughout.
- **Self/comparison references to Accelerator:** `../accelerator`,
  `https://github.com/atomicinnovation/accelerator` — these flip from *external
  precedent* to *first person*. Sentences like "Reuses Accelerator's proven
  model; this repo already operates this way" become "This repo already operates
  this way." Per work item, the **luminosity** origin must appear as a full
  `https://github.com/atomicinnovation/luminosity/…` URL (secondary reference),
  while accelerator self-references are dropped or made first-person.
- **Directory/command naming:** `.luminosity/` → `.accelerator/`; `luminosity`
  CLI command → the future Accelerator CLI name (undecided — see 0136 research);
  `luminosity-<sub>` crates.
- **Toolchain count:** lum 0004 says **three** (Python/Shell/Rust); Accelerator
  has **four** (add TypeScript/React frontend per CLAUDE.md). lum 0011 repeats
  the "three languages / no fourth toolchain" claim — must be reconciled with
  Accelerator's reality (the visualiser already adds a fourth).
- **Repo-specific facts that are false for Accelerator:** lum 0005's "the only
  `.sh` file is the linter itself" (Accelerator has ~226 `.sh` files — see
  related 0136 research); lum 0001's "deterministic logic grew into a large body
  of bash scripts" is now *literally Accelerator's own history*.
- **Work-item path references:** every `meta/work/000N-…md` (epics 0001, stories
  0004/0005, slices 0007/0008/0009/0010/0011) is a luminosity path → drop,
  repoint to ported spikes (0158/0159) where applicable, or render as a full
  luminosity GitHub URL.
- **Domain specifics (lum 0008):** `content/articles|social|ads|imagery|video`,
  marketing artefact types, "run Luminosity alongside Accelerator" coexistence
  reasoning — all drop on port.

### G. Schema-drift checklist (what actually differs, field by field)

Because the luminosity ADRs are accelerator-derived, drift is small:
- `status: accepted` → **`proposed`** (all 11). Body status block
  `**Status**: Accepted` → `Proposed` (kept in sync; `adr-read-status.sh` reads
  frontmatter as authoritative).
- `date` / `last_updated` → **porting context** (2026-06-27T…). `author` /
  `last_updated_by` → porting author.
- `parent: "work-item:0004"|"work-item:0005"` → **drop or repoint** (targets not
  ported).
- `relates_to` → **rewritten per §D map** (luminosity self-refs +44; Accelerator
  refs unchanged).
- `producer: create-adr` → **decision needed** (local ADRs omit it).
- `id`/`title` → renumbered.
- Spikes: `status: done` (Open Question on whether to reset); `parent`/`blocks`
  → drop/repoint; **add nothing** for `external_id`/`relates_to` (omit-when-empty
  — they were absent upstream and stay absent unless we add a luminosity
  `source`).
- Add a typed `source`/full-URL secondary reference to the luminosity original on
  every ported artifact per the work item.

## Code References

- `skills/decisions/scripts/adr-next-number.sh:58-71` — ADR high-water-mark
  allocation (live output `0045`).
- `skills/work/scripts/work-item-next-number.sh:104-140` — pattern-driven
  work-item allocation (live output `0158`).
- `skills/decisions/create-adr/SKILL.md:55-61,148-189,245-258` — ID assignment,
  frontmatter substitution, `status: proposed`, file naming, dual status fields.
- `meta/decisions/ADR-0029-sequential-adr-identifiers.md:56-61` — `ADR-NNNN`
  naming, never-reused numbering.
- `meta/decisions/ADR-0030-adr-template.md:59-77` — required body sections +
  in-body status block (frontmatter list here is stale).
- `meta/decisions/ADR-0031-skill-level-adr-immutability.md:62-80,101-102` —
  transition table; new ADRs start `proposed`; edits only while `proposed`.
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md:119-145,198-225` —
  base fields; ADRs not code-state-anchored (no provenance); `decision_makers`
  extra.
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md:52-87` — linkage keys,
  cardinality, `"doc-type:id"` reference shape.
- `meta/decisions/ADR-0040-…:94-138` — omit-when-empty classification; typed
  form is canonical for new writes.
- `meta/decisions/ADR-0001-context-isolation-principles.md` — *not* a pure
  filesystem-bus ADR; bundles agent separation + token budgets (supersession
  hazard).
- `meta/decisions/ADR-0016-userspace-configuration-model.md` — config model;
  body status "Active — file paths superseded in part"; awaits full superseder.
- `meta/decisions/ADR-0017-configuration-extension-points.md` — extends 0016;
  `relates_to: ["adr:ADR-0016", …]`.
- `meta/decisions/ADR-0027-persist-structured-skill-outputs-to-meta.md` — the
  clean overlap for lum ADR-0008; `relates_to` cites ADR-0001/0006/0008/0019.
- `../luminosity/meta/decisions/ADR-0001…0011-*.md` — the source set.
- `../luminosity/meta/work/0002-…md`, `0003-…md` — the feeding spikes.

## Architecture Insights

- **The luminosity ADR set is Accelerator's own architecture, reflected back.**
  Luminosity bootstrapped from Accelerator and recorded — as fresh, coherent
  ADRs — decisions Accelerator made implicitly and never formalised (config
  model, bash floor, mise+invoke, skills-as-product, filesystem-as-bus) plus the
  Rust-CLI direction Accelerator is *now* researching (0136). Porting them back
  is less "import foreign decisions" and more "reclaim our own decision record,
  re-pointed at first person." This reframing is why the edits are heavy where
  the work item expected them light.
- **Two ADR classes need different planning.** Class A (already-true conventions)
  port as *records of existing reality* and collide with existing ADRs/prose;
  Class B (Rust-CLI direction) port as *proposed future direction* gated on the
  0136 migration actually being adopted. The work item's Assumption (Accelerator
  is committed to the Rust-CLI direction) is load-bearing for Class B — if it is
  not settled, those four/six ADRs describe an unadopted architecture.
- **Immutability sequences the work.** Imports are `proposed` (editable);
  supersession of local 0016/0017/0027 is a *second*, post-acceptance phase via
  `review-adr` then `create-adr --supersedes`. The plan must phase: (1) author
  all ports + spikes as proposed with correct cross-refs; (2) `review-adr` →
  accept the supersets; (3) apply supersede edges. Steps 2–3 cannot precede a
  human acceptance gate.
- **Reference rewriting is two-layered:** luminosity self-references shift +44;
  embedded Accelerator references are already local and must be preserved. A
  blind mechanical shift corrupts ADR-0047's config lineage.
- **`meta/` is shared, single-rooted.** lum ADR-0008 contemplated `meta/`
  shared between Accelerator and Luminosity in consumer repos; in Accelerator
  itself there is one `meta/`, so the coexistence reasoning is dropped, not
  ported.

## Historical Context

- `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`
  — the staged Rust-CLI migration research this port's Class B ADRs align with;
  the bare-path invocation contract, distribution precedent, and toolchain
  inventory there are the Accelerator-side facts the ported CLI ADRs must be
  reconciled against.
- `meta/decisions/ADR-0016/0017/0020/0021` — Accelerator's *existing* config
  model and extension points, which luminosity ADR-0003 explicitly adopted and
  cites. Confirmed present locally with matching titles.
- `meta/work/0031-consolidate-accelerator-owned-files-under-accelerator.md`
  (referenced by ADR-0016) — the partial supersession of ADR-0016's file paths;
  the ported config superset (ADR-0047) completes it.

## Related Research

- `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`
  (parent epic 0136 — Rust CLI migration surface).

## Open Questions

1. **`producer` on ported ADRs.** Local ADRs omit `producer`; luminosity ADRs
   carry `producer: create-adr`. Match the local corpus (omit), or set a porting
   marker? (Leaning: omit, to match the live local ADRs read by the corpus
   validator.)
2. **`parent` on ported ADRs.** Luminosity parents (work-item:0004/0005) are not
   ported. Drop entirely (omit-when-empty), or repoint all ported ADRs' `parent`
   at this porting work item (work-item:0157) as provenance? (Leaning: drop, and
   record luminosity origin in References as a full URL; spike-fed ADRs instead
   gain a `relates_to`/References pointer to the ported spike 0158/0159.)
3. **ADR-0001 supersession.** Confirm ADR-0001 is *not* superseded by the ported
   filesystem ADR (ADR-0052) — only ADR-0027 is — because ADR-0001 bundles
   live, non-overlapping decisions. (Strongly recommended; contradicts the work
   item's literal "0001 + 0027" pairing.)
4. **Spike status.** Import the two spikes as `done` (faithful luminosity
   history) or reset to `draft`/`abandoned` (the effort did not occur here)?
   Work item draft assumes `done`. (Leaning: `done` with a clear "ported from
   luminosity" note, since the research content and outcomes are real.)
5. **Spike `parent`/`blocks`.** All targets are luminosity work items not being
   ported — drop both keys (omit-when-empty) and capture luminosity lineage via
   `source`/References full URL?
6. **Toolchain count in ADR-0004/0011.** Edit "three toolchains / no fourth" to
   Accelerator's four (incl. TS/React)? This changes the ADR's reasoning
   slightly (the eval ADR's "no fourth language toolchain" argument weakens).
   Confirm the intended framing.
7. **Class B viability gate.** Are the six Rust-CLI-direction ADRs (0045/0046/
   0048/0053/0054/0055) imported now as `proposed` even though Accelerator has
   not committed to the migration, or held until 0136 is decided? (Work item
   Assumptions says import; confirm.)
8. **CLI command name.** lum 0010/0009 name the `luminosity` command/crates;
   Accelerator's future CLI name is undecided (0136 open question). Use a
   placeholder, or defer naming and note it inline?
