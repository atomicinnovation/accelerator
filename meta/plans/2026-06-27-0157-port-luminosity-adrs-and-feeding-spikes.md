---
type: plan
id: "2026-06-27-0157-port-luminosity-adrs-and-feeding-spikes"
title: "Port Luminosity ADRs and Feeding Spikes Implementation Plan"
date: "2026-06-27T12:23:42+00:00"
author: "Toby Clemson"
producer: create-plan
status: draft
work_item_id: "work-item:0157"
parent: "work-item:0157"
derived_from: ["codebase-research:2026-06-27-0157-porting-luminosity-adrs-and-feeding-spikes"]
tags: [adr, decisions, porting, luminosity, rust-cli, spikes, supersession]
revision: "d88eba092cb9225c6e9727b8999181048c7104d6"
repository: "accelerator"
last_updated: "2026-06-27T12:23:42+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Port Luminosity ADRs and Feeding Spikes Implementation Plan

## Overview

Port the eleven `accepted` ADRs and two `done` feeding spikes from the
[luminosity](https://github.com/atomicinnovation/luminosity) project into this
repository's `meta/decisions/` and `meta/work/`, renumbered to the next free
local IDs, reframed from luminosity's vantage point into Accelerator's first
person, and conformed to this repo's unified schema. The luminosity set is, in
effect, **Accelerator's own architecture reflected back from a downstream
repo** — porting it reclaims an already-reasoned decision record rather than
re-deriving each decision.

The port is mechanically simple (same template, same linkage syntax) but
**editorially deep**: every ADR is written from luminosity looking *at*
Accelerator (as external precedent, cautionary tale, or comparison repo), and
those references must be rewritten in the first person when the ADRs live
*inside* Accelerator.

## Current State Analysis

- **Local corpus ends at ADR-0044** and work item 0157. Verified live:
  `adr-next-number.sh` → `0045` (→ `0055` for 11); `work-item-next-number.sh`
  → `0158` (→ `0159` for 2).
- **Luminosity source is checked out** at `../luminosity` with all 11 ADRs
  (`ADR-0001`…`ADR-0011`, all `accepted`, `producer: create-adr`) and both
  spikes (`0002`, `0003`, both `done`, `kind: spike`).
- **The conformance contract** is ADR-0029 (sequential IDs), ADR-0030 (template
  body sections), ADR-0031 (immutability / status transitions), ADR-0033
  (unified base frontmatter), ADR-0034 (typed-linkage vocabulary), ADR-0040
  (omit-when-empty). The luminosity ADRs already follow ADR-0030's body
  structure, so no restructuring is needed — only content edits.
- **The corpus validator is the real gate, and it is _not_ wired into
  `mise run check`.** `scripts/validate-corpus-frontmatter.sh meta/`
  (whole-corpus mode) is what enforces per-type required keys, status
  vocabulary, typed-`doc-type:id` shape, and **referential integrity**
  (`DANGLING-REF` — every typed-linkage ref must resolve to a real artifact).
  It runs only via test fixtures and at migration time, never against the live
  tree as a CI gate. **Baseline confirmed green** (`exit 0`) on the current
  corpus — that is the invariant the port must preserve.

### Key Discoveries:

- **Perspective inversion is the largest task.** lum ADR-0001 treats
  Accelerator's bash body as "the model ADR-0001 exists to leave behind"; lum
  ADR-0003 says "adopt Accelerator's proven config model"; references to
  `../accelerator` and `https://github.com/atomicinnovation/accelerator` are
  *external precedent* that become first-person self-reference once ported.
  (research §F, §C; `2026-06-27-0157-…:271-301`)
- **Reference rewriting is two-layered.** A uniform **+44 shift** maps
  luminosity ADR numbers to local (lum 0001→local 0045 … lum 0011→local 0055),
  but it applies **only to luminosity's own ADR self-references**. lum ADR-0003's
  prose cites **Accelerator's** ADR-0016/0017/0020/0021 — these are *already
  correct local references* (all four files confirmed present) and must **not**
  shift. Classifying each `ADR-NNNN` mention is per-ADR judgement, not `sed`.
  (research §D, "Critical rewrite caveat", `…:239-245`)
- **Only two supersets are forced** (research §E):
  - **Config:** local ADR-0047 (ported lum 0003) supersedes local ADR-0016 +
    ADR-0017. ADR-0016 already says "a full superseding ADR is forthcoming" —
    ADR-0047 is that superseder.
  - **Filesystem:** local ADR-0052 (ported lum 0008) supersedes local ADR-0027
    **only**. ADR-0001 is **not** superseded — it bundles live agent-separation
    and token-budget decisions ADR-0052 does not subsume; it is linked via
    `relates_to`. *(Confirmed with requester.)*
- **lum ADR-0008 has a domain mismatch.** It splits the filesystem into `meta/`
  (shared memory) **and `content/`** (marketing deliverables — articles,
  social, ads, imagery, video). Accelerator is a plugin, not a content product;
  the `content/` half is dropped on port, leaving the `meta/`-as-bus half that
  overlaps ADR-0027. (research §A.3, §F)
- **Supersession is recorded bilaterally, matching the live corpus and the
  sanctioned tooling.** The superseding ADR carries `supersedes: ["adr:ADR-NNNN",
  …]` and the old ADR carries `status: superseded` **plus** `superseded_by:
  "adr:ADR-MMMM"`. This is exactly what `create-adr --supersedes` writes
  (`create-adr/SKILL.md:194-196`) and what the live precedent shows: **ADR-0036**
  carries `status: superseded` + `superseded_by: "adr:ADR-0043"` while
  **ADR-0043** carries `supersedes: ["adr:ADR-0036"]`. On the old ADR, **only
  these two frontmatter fields change** (`status`, and the *added* `superseded_by`
  key); no body content is touched — including the in-body `**Status**:` block,
  which stays as-authored (ADR-0036's body still reads `**Status**: Accepted`).
- **`supersedes:` is referential-integrity-checked; `superseded_by` is _not_.**
  The schema lists `parent supersedes relates_to` as the `adr` linkage keys, so
  `validate-corpus-frontmatter.sh` resolves every `supersedes:` ref (a stale/typo'd
  target fails `DANGLING-REF`). `superseded_by` is **not** an `adr` linkage key,
  so the validator neither shape-checks nor resolves it — a typo'd `superseded_by`
  target passes green. The validator also performs **no symmetry or status
  check**. So both the `supersedes` target *list* (does ADR-0047 name both 0016
  and 0017?) and the `superseded_by` target *value* must be pinned by explicit
  grep assertions, not left to the validator.
- **Immutability sequences the supersession** (ADR-0031): the 11 imports are
  `proposed` (fully editable); the `supersedes:` edge is authored on the proposed
  superset in Phase 2, but the old ADRs' `accepted → superseded` flip is only
  legal once the superset is itself `accepted`. So the supersession enactment
  (status flip + `superseded_by`) is a **deferred Phase 3**, gated on a human
  accepting ADR-0047/0052 via `/review-adr`. The sanctioned `create-adr
  --supersedes` mechanism cannot be reused as-is here (it mints a *new* ADR; the
  supersets already exist from Phase 2), so Phase 3 reproduces that mechanism's
  exact frontmatter changes (`status: superseded` + `superseded_by`) by hand —
  the same end-state, owner-sanctioned. If a superset is instead **rejected**,
  the old ADR is simply never flipped (stays `accepted`, no `superseded_by`) and
  the corpus remains coherent — no contingency edits.

## Desired End State

- All 11 luminosity ADRs exist as `meta/decisions/ADR-0045-…` … `ADR-0055-…`,
  status `proposed`, schema-conformant, internally cross-referenced by their
  **local** numbers, reframed to first person, with every luminosity origin
  cited as a full `https://github.com/atomicinnovation/luminosity/…` URL in the
  prose `## References` section.
- Both feeding spikes exist as `meta/work/0158-…` and `meta/work/0159-…`, status
  `done`, research and outcomes intact, luminosity origin cited as a full URL.
- Genuinely spike-derived ADRs (0053, 0054 ← spike 0158; 0055 ← spike 0159)
  reference the locally-ported spike as primary provenance (`relates_to` + a
  References note) and the luminosity original as a full-URL secondary reference.
  ADR-0046 (distribution) `relates_to` spike 0158 as a contributing input but
  attributes primary provenance to Accelerator's existing distribution pipeline.
- Two supersets declare the edge: **ADR-0047** carries `supersedes: ["adr:ADR-0016",
  "adr:ADR-0017"]` and **ADR-0052** carries `supersedes: ["adr:ADR-0027"]`. Once
  those supersets are accepted (Phase 3), the pre-existing **ADR-0016/0017** carry
  `status: superseded` + `superseded_by: "adr:ADR-0047"` and **ADR-0027** carries
  `status: superseded` + `superseded_by: "adr:ADR-0052"` — the bilateral pattern
  matching ADR-0036/0043. Only those two frontmatter fields change on each old
  ADR; no body content (including the in-body `**Status**` block) is touched.
- **`bash scripts/validate-corpus-frontmatter.sh meta/` exits 0** at every phase
  boundary, and `mise run check` stays green.

## What We're NOT Doing

- **Not implementing** the architecture the Class-B ADRs describe (Rust CLI,
  hexagonal core, static-binary distribution, Inspect harness). These record
  decisions; building against them is separate downstream work.
- **Not re-running** the spikes — captured results are imported verbatim, not
  regenerated.
- **Not recording** Accelerator conventions as new ADRs beyond the two supersets
  that porting forces.
- **Not porting** luminosity work items 0001/0004/0005 (the epics/stories that
  parent the source ADRs) — only the two feeding spikes (lum 0002, 0003).
- **Not superseding ADR-0001** (only ADR-0027 on the filesystem axis; only
  ADR-0016 + ADR-0017 on the config axis).
- **Not editing the pre-existing local ADRs beyond two frontmatter fields.** Per
  immutability and the bilateral convention, the only changes to ADR-0016/0017/0027
  are flipping `status` to `superseded` and adding `superseded_by` (Phase 3, after
  the supersets are accepted). No body content — including the in-body `**Status**`
  block — is touched.
- **Not enacting** the Phase 3 supersession (status flip + `superseded_by`) until
  the relevant superset is accepted — that human acceptance gate
  (`accepted → superseded` legality) cannot be crossed autonomously.

## Implementation Approach

### Settled decisions (from clarification + research leanings)

| Decision | Resolution |
|---|---|
| ADR-0001 fate | **Supersede ADR-0027 only**; link ADR-0001 via `relates_to`. |
| Spike status | Import as **`done`** with a "ported from luminosity" note. |
| Toolchain count (ADR-0048 / ADR-0055) | **Edit to four** (add TS/React); rework ADR-0055's eval reasoning so it argues Inspect on its own merits, not "no fourth toolchain". |
| CLI naming (ADR-0053/0054) | Command is **`accelerator`**, dispatched git-style to on-demand **`accelerator-<sub>`** binaries; the existing `accelerator-visualiser` folds in as **`accelerator visualiser …`**. |
| `producer` on ported ADRs | **Omit** (match the live local ADR corpus, which omits it). |
| `parent` on ported ADRs | **Omit** (luminosity parents 0004/0005 not ported); record luminosity origin in `## References` as a full URL. |
| Spike `parent`/`blocks` | **Omit** (all targets are un-ported luminosity work items). |
| Spike-fed ADR provenance | ADR-0053/0054/0055: `relates_to` the spike (0158/0159) as **primary** provenance + References note; luminosity original full-URL secondary. ADR-0046: `relates_to` spike 0158 as a contributing input only (primary = Accelerator's existing distribution pipeline). |
| Luminosity origin in frontmatter | **Do not** add a typed `source:`/linkage key pointing at a GitHub URL — a URL is not a `doc-type:id` typed reference (ADR-0034) and `source` is not an `adr` linkage key, so it would fail validation as `BAD-LINKAGE-SHAPE` (not `DANGLING-REF`) or be silently ignored. Cite the URL in prose `## References` only. |
| Class-B import gate | Import all 11 as `proposed` now. **Load-bearing assumption** — see Risks below. |

### ID-assignment and reference-rewrite map (research §D)

| lum ADR | → local | rewritten `relates_to` (local refs) | Provenance / supersession |
|---|---|---|---|
| 0001 | **0045** | — | |
| 0002 | **0046** | adr:ADR-0045 | `relates_to` spike work-item:0158 |
| 0003 | **0047** | adr:ADR-0045, adr:ADR-0046 | **supersedes** local 0016, 0017 |
| 0004 | **0048** | adr:ADR-0045, adr:ADR-0046 | |
| 0005 | **0049** | adr:ADR-0048, 0046, 0047, 0045 | |
| 0006 | **0050** | adr:ADR-0045, 0048, 0049 | |
| 0007 | **0051** | adr:ADR-0045 | |
| 0008 | **0052** | adr:ADR-0051, 0045 | **supersedes** local 0027; `relates_to` 0001 |
| 0009 | **0053** | adr:ADR-0045, 0046, 0048 | `relates_to` spike work-item:0158 |
| 0010 | **0054** | adr:ADR-0045, 0046, 0053 | `relates_to` spike work-item:0158 |
| 0011 | **0055** | adr:ADR-0048, 0050, 0051, 0052 | `relates_to` spike work-item:0159 |

> Embedded **Accelerator** references (ADR-0016/0017/0020/0021 in lum 0003's
> prose) are already-correct local numbers — leave them. Only luminosity
> self-references shift +44.

### TDD framing for Markdown artifacts

These artifacts have no unit tests; their "test" is the corpus validator plus a
small battery of `grep`-based assertions. For each phase, **define the expected
end-state checks first, then author until they pass**:

1. Run `bash scripts/validate-corpus-frontmatter.sh meta/` (must stay `exit 0`).
2. Run the file-existence / status / cross-reference `grep` assertions listed in
   each phase's Success Criteria.
3. Spot-review the cross-reference graph against the §D map (manual — the
   validator confirms refs *resolve* but not that they point at the *intended*
   target).

### Phase independence

Each phase is a standalone, reviewable, validator-green unit. Phases are
**sequenced** (Phase 2's spike `relates_to` resolves only after Phase 1 lands;
Phase 3's `status` flip is legal only after Phase 2's supersets are accepted) but
each merges on its own leaving `meta/` green. Phase 3 touches only the `status`
field of the three superseded ADRs and is gated on human acceptance.

### Risks and Assumptions

- **Load-bearing: the Rust-CLI direction is being pursued.** Six of the eleven
  ADRs (ADR-0045/0046/0048/0053/0054/0055 — the Class-B set) record a Rust-CLI
  architecture Accelerator has *researched* (0136) but **not yet adopted**. The
  work item's Assumptions back importing them now, but this is the single
  assumption the Class-B half rests on. **If the direction is not settled**, those
  ADRs describe an unadopted architecture — the inverse of the "reclaim our own
  decisions" framing. **Mitigation**: import as `proposed` (this plan's scope);
  hold the acceptance of the Class-B ADRs (Phase 3 lifecycle) until 0136 lands, or
  `reject` them if it does not. The two supersets (ADR-0047 config, ADR-0052
  filesystem) are Class-A (record existing reality) and are *not* exposed to this
  risk; their acceptance is independent of the Rust-CLI question.
- **CLI command name is provisional.** ADR-0053/0054 fix the name as
  `accelerator`; 0136 still lists it as open. The ADRs flag it as provisional
  pending 0136 (see their specs) so acceptance does not freeze an unsettled name.
- **Corpus may move between planning and authoring.** The §D map hard-codes
  0045/0158; a concurrent ADR/work-item landing would invalidate it. Phase 1's
  pre-flight allocator check (and re-running it before Phase 2) guards this.

---

## Phase 1: Port the two feeding spikes

### Overview

Bring the two `done` spikes into `meta/work/` as local 0158/0159 so the
spike-fed ADRs in Phase 2 have resolvable provenance targets.

### Changes Required:

#### 1. Allocate IDs

Allocate via the script (do not hand-pick):

```bash
bash skills/work/scripts/work-item-next-number.sh --count 2   # → 0158, 0159
```

#### 2. Port lum 0002 → `meta/work/0158-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md`

**Source**: `../luminosity/meta/work/0002-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md`

- Frontmatter: unified base (`type: work-item`, `id: "0158"`,
  `kind: spike`, `status: done`, `schema_version: 1`); `author`/`date`/
  `last_updated`/`last_updated_by` → porting context (Toby Clemson,
  2026-06-27T12:23:42+00:00).
- **Omit** `parent` and `blocks` (luminosity targets 0001/0005/0007/0008 not
  ported) — omit-when-empty per ADR-0040. **Omit** `external_id`/`relates_to`
  (absent upstream).
- Preserve `## Spike Outcome` and `## Recommendation` verbatim, including
  date-sensitive facts — this spike (lum 0002) carries the
  "sigstore-verification archived May 2026" fact — as historical record.
- Add to `## References` (creating it if absent): a "Ported from luminosity"
  note and the full-URL origin
  `https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0002-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md`.
- Rewrite any luminosity self-paths/names to full URLs or first person; reframe
  product references to Accelerator.

#### 3. Port lum 0003 → `meta/work/0159-skill-evaluation-framework-selection.md`

**Source**: `../luminosity/meta/work/0003-skill-evaluation-framework-selection.md`

- Same treatment as 0158. Preserve `## Spike Outcome` / `## Recommendation`
  including this spike's (lum 0003) date-sensitive fact — promptfoo
  "acquired by OpenAI (9 Mar 2026)".
- Full-URL origin
  `https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0003-skill-evaluation-framework-selection.md`
  in `## References`.

### Success Criteria:

#### Automated Verification:

- [ ] **Pre-flight — IDs unmoved**: `bash skills/work/scripts/work-item-next-number.sh` still returns `0158` before authoring. If the corpus moved, re-allocate and regenerate the §D map rather than proceeding with hard-coded numbers.
- [ ] Both files exist: `ls meta/work/0158-*.md meta/work/0159-*.md | wc -l` → `2`
- [ ] Both `done` (sum-aware): `grep -l '^status: done' meta/work/0158-*.md meta/work/0159-*.md | wc -l` → `2`
- [ ] No un-ported parent/blocks leak: `! grep -E '^(parent|blocks):' meta/work/0158-*.md meta/work/0159-*.md`
- [ ] Luminosity origin cited as full URL: `grep -l 'github.com/atomicinnovation/luminosity' meta/work/0158-*.md meta/work/0159-*.md | wc -l` → `2`
- [ ] Date-sensitive anchor survived in 0158 (lum 0002): `grep -l 'sigstore\|archived May 2026' meta/work/0158-*.md | wc -l` → `1`.
- [ ] Date-sensitive anchor survived in 0159 (lum 0003): `grep -l 'OpenAI' meta/work/0159-*.md | wc -l` → `1`. (Confirm the exact upstream wording of each anchor before relying on these tokens.)
- [ ] No residual luminosity product-name/command in the ported spikes (token-level): `grep -roEn 'Luminosity|\.luminosity/|\bluminosity\b' meta/work/0158-*.md meta/work/0159-*.md` — every hit confirmed part of the origin URL.
- [ ] Coarse content-survival guard against silent truncation: each ported spike's line count is within tolerance of its `../luminosity` source (the spikes are ~373 / ~316 lines — a large shortfall signals lost research).
- [ ] Corpus validator green: `bash scripts/validate-corpus-frontmatter.sh meta/` (exit 0)
- [ ] `mise run check` passes

#### Manual Verification:

- [ ] `## Spike Outcome` / `## Recommendation` content matches the luminosity
      originals (diff against the `../luminosity` source — no research lost),
      with date-sensitive facts intact. **Boundary**: these two sections are
      preserved as verbatim historical record (only luminosity self-paths/names
      rewritten); the earlier narrative/options sections additionally get
      first-person/Accelerator perspective inversion.
- [ ] Prose reads in the first person / Accelerator voice; no dangling
      references to luminosity work items by bare number, and no residual
      `luminosity`/`.luminosity/` command names (grep the ported spike files).

---

## Phase 2: Port the eleven ADRs as `proposed`

### Overview

Author `ADR-0045`…`ADR-0055` from the luminosity sources, in **ascending order**
(every `relates_to` points at a lower-numbered ADR, so each file's targets
already exist), applying renumbering, perspective inversion, the §D
cross-reference rewrites, minimal content edits, and the two superset
declarations. All import as `proposed`; **no existing local ADR is edited in this
phase.**

### Changes Required (common to all 11):

**Frontmatter** (match the live local ADR key shape; ADR-0033/0034/0040):
- `type: adr`; `id: "ADR-00NN"` (quoted); `title: "ADR-00NN: <title>"`
  (the `ADR-NNNN:`-prefixed form — the corpus majority; note the three most
  recent ADRs 0042/0043/0044 drop the prefix, so this is a deliberate choice of
  the dominant convention); `schema_version: 1`; `status: proposed`.
- `author`/`date`/`last_updated`/`last_updated_by` → porting context.
- **Omit** `producer` and `parent` — this follows the **live local ADR corpus**
  (which omits `producer`) and the validator's required-field set, in preference
  to ADR-0040's literal base-field list which names `producer`. Deliberate
  divergence, recorded here.
- `relates_to` rewritten per the §D map. Spike-fed ADRs add the spike
  work-item ref.
- **No `revision`/`repository`** (ADRs are not code-state-anchored).

**Body** (ADR-0030 sections already present in sources — edit, don't
restructure): H1 → **three-line status block** (`**Date**` / `**Status**:
Proposed` / `**Author**`, matching ADR-0030 and the live ADRs — not a one-line
block) → `## Context` → `## Decision Drivers` → `## Considered Options` →
`## Decision` → `## Consequences` (Positive/Negative/Neutral) → `## References`.

**Universal edits** (apply to every ADR):
- "Luminosity" → "Accelerator"/first person; `.luminosity/` → `.accelerator/`;
  drop/first-person external `../accelerator` &
  `github.com/atomicinnovation/accelerator` references; every luminosity origin →
  full URL in `## References`.
- **In-prose ordinal/number cross-references** — "the 5th ADR in this set",
  "the 5th and 6th ADRs", "decision 9", "decision 10/11" — rewrite to the
  assigned **local** ADR number, not just the frontmatter `relates_to`. A
  reference that reads correctly as a luminosity ordinal is wrong locally.
  Confirmed present (per source read) in **ADR-0045** ("decision 9"), **ADR-0046**
  ("decision 10" ×2), **ADR-0048** ("the 5th/6th ADR…", "in this set"), and
  **ADR-0049/0053/0054/0055**. Grep every ported ADR for `the [0-9]+(st|nd|rd|th)
  ADR` and `decision [0-9]+` and reclassify each hit.
- **Embedded luminosity work-item references — both path and bare-prose forms.**
  Path-shaped (`meta/work/000N-…md`) *and* bare-prose ("work item 0002", "epic
  0001", "story 000N", "slice 000N" — lum 0001 alone has several) references to
  luminosity work items (research §F) — drop, repoint to the ported spikes
  (0158/0159) where applicable, or render as a full luminosity GitHub URL. No
  bare luminosity work-item reference (path or prose) may remain.

### Per-ADR edit specifications:

#### ADR-0045 ← lum 0001 — Skills-vs-CLI Division of Labour *(Heavy)*
Reframe the cautionary framing: lum 0001 casts Accelerator's bash body as "the
model to leave behind". In first person this becomes Accelerator's *own* history
("our deterministic logic grew into a large body of bash scripts"). Frontmatter
`relates_to` is empty, **but the prose is not ref-free**: it carries the ordinal
"decision 9" (→ ADR-0053) and bare work-item references ("work item 0002" →
spike 0158, "work item 0009", "epic 0001") that must be rewritten per the
universal edits.

#### ADR-0046 ← lum 0002 — Zero-Setup Static-Binary Distribution *(Medium)*
Names/paths. "Accelerator's distribution pipeline ported cleanly" → first
person. `relates_to: ["adr:ADR-0045", "work-item:0158"]`. **Provenance wording**:
the distribution decision's primary basis is Accelerator's *existing* binary
distribution pipeline (the visualiser), with spike 0158 a contributing input —
word the References note accordingly (don't overstate the Rust-CLI spike as the
sole provenance) and cite luminosity ADR-0002 as a full-URL secondary reference.
ADR-0053/0054/0055 (genuinely spike-derived) keep the "spike primary" wording.

#### ADR-0047 ← lum 0003 — Multi-Level Userspace Configuration Model *(Heavy; SUPERSET)*
Near-circular: lum 0003 explicitly adopts Accelerator's ADR-0016/0017/0020/0021.
**Keep those four local refs as-is.** Author as the superset of {lum 0003, local
ADR-0016, local ADR-0017} — the merged `## Decision` must carry forward
ADR-0016's specific decisions (config-file scheme, injection points, nesting,
scope axes) and ADR-0017's extension points, not merely append a supersedes
note. Declare `supersedes: ["adr:ADR-0016", "adr:ADR-0017"]` in frontmatter.
State in `## Decision`/`## Consequences` that this ADR supersedes ADR-0016 and
ADR-0017. This ADR declares `supersedes:`; the reciprocal enactment on the old
ADRs (`status: superseded` + `superseded_by: "adr:ADR-0047"`, no body edits) is
deferred to Phase 3 after this ADR is accepted. Note ADR-0016's "forthcoming
superseder" is this ADR. `relates_to: ["adr:ADR-0045", "adr:ADR-0046"]`.

#### ADR-0048 ← lum 0004 — Toolchain Split *(Heavy)*
**"Three" → "four" toolchains** (add the TypeScript/React visualiser frontend).
Reframe Rust-as-product as forward-looking for Accelerator.
`relates_to: ["adr:ADR-0045", "adr:ADR-0046"]`.

#### ADR-0049 ← lum 0005 — Bash 3.2 Compatibility Floor *(Medium)*
Fix the false-for-Accelerator claim cluster — both "today the only `.sh` file is
the linter itself" **and** the adjacent "`lint-bashisms.sh` is the sole operative
enforcement" framing — Accelerator has ~226 `.sh` files and a full
ShellCheck/shfmt pipeline (see related research 0136). Keep the macOS 3.2
rationale. `relates_to: ["adr:ADR-0048", "adr:ADR-0046", "adr:ADR-0047", "adr:ADR-0045"]`.

#### ADR-0050 ← lum 0006 — mise + invoke Task Runner *(Light–medium)*
Matches Accelerator reality; tool-list edits only.
`relates_to: ["adr:ADR-0045", "adr:ADR-0048", "adr:ADR-0049"]`.

#### ADR-0051 ← lum 0007 — Skills as the Product *(Medium)*
Reframe "inherited from Accelerator" → first person. Keep the minimum-Claude-Code
-version fact. `relates_to: ["adr:ADR-0045"]`.

#### ADR-0052 ← lum 0008 — Filesystem as Message Bus *(Heavy; SUPERSET)*
**Drop the `content/` marketing half entirely** (articles/social/ads/imagery/
video; "run Luminosity alongside Accelerator" coexistence reasoning). Keep the
`meta/`-as-message-bus half, and have `## Context` note that the luminosity
original additionally addressed a `content/` deliverables tree that is out of
scope for a plugin and was intentionally not ported (so the scope-narrowing is
traceable against the cited source URL). Author as superset of {lum 0008
meta-half, local ADR-0027} — carry ADR-0027's persist-structured-outputs
decision into the merged `## Decision`. Declare `supersedes: ["adr:ADR-0027"]`;
**link, not supersede,** ADR-0001 via `relates_to`. This ADR declares
`supersedes: ["adr:ADR-0027"]`; the reciprocal enactment on ADR-0027
(`status: superseded` + `superseded_by: "adr:ADR-0052"`, no body edits) is
deferred to Phase 3. `relates_to: ["adr:ADR-0051", "adr:ADR-0045", "adr:ADR-0001"]`.

#### ADR-0053 ← lum 0009 — Thin CLI over Hexagonal Core *(Medium)*
Crate/command names → `accelerator` / `accelerator-<sub>`. Tooling
(cargo-deny/pup). Note inline (Decision or References) that the `accelerator`
command name aligns with the 0136 migration direction and is **provisional
pending that work** — these ADRs are `proposed`, and 0136 still lists the CLI
name as open; flag it rather than presenting it as settled.
`relates_to: ["adr:ADR-0045", "adr:ADR-0046", "adr:ADR-0048", "work-item:0158"]`;
References: spike 0158 primary + lum ADR-0009 full URL.

#### ADR-0054 ← lum 0010 — Git-Style Modular CLI of On-Demand Static Binaries *(Medium)*
Command `accelerator`, git-style dispatch to on-demand `accelerator-<sub>`
binaries; **fold the existing `accelerator-visualiser` in as
`accelerator visualiser …`** (binary `accelerator-visualiser`). Replace all
`luminosity` command/crate names. `relates_to: ["adr:ADR-0045", "adr:ADR-0046", "adr:ADR-0053", "work-item:0158"]`;
References: spike 0158 primary + lum ADR-0010 full URL.

#### ADR-0055 ← lum 0011 — Inspect as the Skill-Evaluation Harness *(Medium)*
**Rework the "no fourth language toolchain" argument** — Accelerator already has
four toolchains, so argue Inspect on its own merits (Python-native, fits the
existing build-system toolchain) rather than toolchain-count minimisation.
`configure`-skill target stays. `relates_to: ["adr:ADR-0048", "adr:ADR-0050", "adr:ADR-0051", "adr:ADR-0052", "work-item:0159"]`;
References: spike 0159 primary + lum ADR-0011 full URL.

### Success Criteria:

#### Automated Verification:

- [ ] All 11 exist (and the glob expands to 11 — the gate for the negative checks below): `ls meta/decisions/ADR-004[5-9]-*.md meta/decisions/ADR-005[0-5]-*.md | wc -l` → `11`
- [ ] All `proposed` (sum-aware): `grep -l '^status: proposed' meta/decisions/ADR-004[5-9]-*.md meta/decisions/ADR-005[0-5]-*.md | wc -l` → `11`
- [ ] No `producer:` / `parent:` keys (only meaningful given the glob-expands-to-11 gate above): `! grep -E '^(producer|parent):' meta/decisions/ADR-004[5-9]-*.md meta/decisions/ADR-005[0-5]-*.md`
- [ ] Each cites luminosity origin: `grep -L 'github.com/atomicinnovation/luminosity' meta/decisions/ADR-004[5-9]-*.md meta/decisions/ADR-005[0-5]-*.md` → empty
- [ ] No leftover product-name reference **outside the origin URL** (token-level, not line-level — a line carrying both the origin URL and a stray `Luminosity` must still be caught): `grep -roEn 'Luminosity|\.luminosity/' meta/decisions/ADR-004[5-9]-*.md meta/decisions/ADR-005[0-5]-*.md` then confirm every hit is part of the `github.com/atomicinnovation/luminosity` origin URL (the `-o` token match avoids the whole-line `grep -v` false-negative).
- [ ] No bare luminosity work-item reference (path **or** prose): `grep -rinE 'meta/work/000[0-9]|(work[ -]?item|epic|story|slice) 000[0-9]' meta/decisions/ADR-004[5-9]-*.md meta/decisions/ADR-005[0-5]-*.md` — every hit must be either a rewritten local ref or inside the origin URL.
- [ ] **§D `relates_to` map exact-match** (the highest-risk wrong-but-resolvable class) — runnable per ADR: `grep '^relates_to:' meta/decisions/ADR-00NN-*.md | grep -oE 'adr:ADR-[0-9]{4}' | sort` must equal the sorted `adr:` set from the §D table for that ADR. Note: compare only `adr:` tokens (lines also carry research/plan/work-item refs). **ADR-0052's full `adr:` set is ADR-0051 + ADR-0045 + ADR-0001** (ADR-0001 is a deliberate `relates_to`, listed in the supersession column of §D, not the `relates_to` column — include it). ADR-0047's prose config refs (0016/0017/0020/0021) are *not* in `relates_to` and must not appear there. Walk all 11.
- [ ] Supersets declare the **exact** `supersedes` set (presence *and* no extras): `grep 'supersedes:' meta/decisions/ADR-0047-*.md | grep -oE 'ADR-[0-9]{4}' | sort` equals `ADR-0016 ADR-0017`; ADR-0052's equals `ADR-0027` (guards against an accidental extra such as ADR-0001 in ADR-0052's `supersedes`).
- [ ] Suspect in-prose ordinals flagged for review: `grep -rinE 'the [0-9]+(st|nd|rd|th) ADR|decision [0-9]+' meta/decisions/ADR-004[5-9]-*.md meta/decisions/ADR-005[0-5]-*.md` — every hit confirmed to name the correct **local** number.
- [ ] **No pre-existing ADR edited in Phase 2**: `jj diff --stat` shows no changes to `meta/decisions/ADR-0016-*.md`, `ADR-0017-*.md`, or `ADR-0027-*.md` (the `status` flip is Phase 3, after acceptance).
- [ ] Corpus validator green (referential integrity over all new refs): `bash scripts/validate-corpus-frontmatter.sh meta/` (exit 0)
- [ ] `mise run check` passes

#### Manual Verification:

- [ ] Cross-reference graph matches the §D map; no luminosity self-ref left
      un-shifted and no Accelerator ref (0016/0017/0020/0021) wrongly shifted.
      (Backed by the §D exact-match automated assertion above.)
- [ ] No in-prose ordinal cross-reference survives in luminosity terms — search
      ADR-0049/0053/0054/0055 (and any other) for "the Nth ADR" / "decision N"
      phrasings and confirm each names the correct **local** number.
- [ ] Perspective inversion complete — no sentence treats Accelerator as an
      external/precedent repo; ADR-0045's cautionary framing reads as
      Accelerator's own history.
- [ ] ADR-0048 says four toolchains; ADR-0055's reasoning no longer depends on
      "no fourth toolchain" — `grep -in 'fourth toolchain\|three languages\|no fourth' meta/decisions/ADR-0055-*.md` returns no count-minimisation phrasing.
- [ ] ADR-0052 contains no `content/` marketing material; supersedes ADR-0027
      only and `relates_to` ADR-0001.
- [ ] ADR-0054 describes `accelerator visualiser …` as the folded-in visualiser.

---

## Phase 3: Enact supersession on the superseded ADRs (gated on human acceptance)

### Overview

Enact the two supersession edges by, on each pre-existing ADR, flipping `status`
to `superseded` **and** adding `superseded_by` pointing at its superset — the
bilateral pattern matching ADR-0036/0043 and `create-adr --supersedes`. This
**cannot be done autonomously**: ADR-0031 permits `accepted → superseded` only
once the superseding ADR is itself `accepted`, so this phase runs after a human
accepts ADR-0047 and ADR-0052 via `/review-adr`.

Only those **two frontmatter fields** change on each old ADR; **no body content**
is touched (the in-body `**Status**` block is left as-authored, matching ADR-0036,
which keeps `**Status**: Accepted` in its body while its frontmatter says
`superseded`). The sanctioned `create-adr --supersedes` mechanism cannot be
reused as-is (the supersets already exist from Phase 2 rather than being minted
now), so Phase 3 reproduces that mechanism's exact frontmatter changes by hand —
the same owner-sanctioned end-state.

If a superset is **rejected** rather than accepted (a legal `proposed → rejected`
transition), its enactment is simply never performed — the old ADR stays
`accepted` with no `superseded_by`, and the corpus remains coherent. No
contingency edits.

### Changes Required:

#### 1. Acceptance gate (human, via `/review-adr`)
- `/review-adr` ADR-0047 → `accepted`; ADR-0052 → `accepted`. (Pre-flight:
  confirm local ADR-0016/0017/0027 are currently `accepted` so the
  `accepted → superseded` transition is legal.)

#### 2. Enact supersession (two frontmatter fields per old ADR)
- Local **ADR-0016** and **ADR-0017** → `status: superseded` +
  `superseded_by: "adr:ADR-0047"`.
- Local **ADR-0027** → `status: superseded` + `superseded_by: "adr:ADR-0052"`.
- **Do not** touch the in-body `**Status**` block or any other content — `status`
  and the added `superseded_by` are the only fields changed.

### Success Criteria:

#### Automated Verification:

- [ ] Pre-flight (before enactment): `grep '^status: accepted'` matches all of ADR-0016/0017/0027 (transition legality).
- [ ] Supersets accepted: `grep '^status: accepted' meta/decisions/ADR-0047-*.md meta/decisions/ADR-0052-*.md` (both match).
- [ ] Status flipped: `grep -l '^status: superseded' meta/decisions/ADR-0016-*.md meta/decisions/ADR-0017-*.md meta/decisions/ADR-0027-*.md | wc -l` → `3`.
- [ ] **`superseded_by` target correct** (the validator does NOT check this key — assert by hand): `grep '^superseded_by:' meta/decisions/ADR-0016-*.md meta/decisions/ADR-0017-*.md` each → `"adr:ADR-0047"`; `grep '^superseded_by:' meta/decisions/ADR-0027-*.md` → `"adr:ADR-0052"`.
- [ ] **Reverse-link invariant** (catches a half-applied/stranded flip): every ADR with `status: superseded` among the three is named in its superset's `supersedes:` AND its own `superseded_by` names that same superset — i.e. ADR-0016/0017 ↔ ADR-0047, ADR-0027 ↔ ADR-0052 round-trip.
- [ ] **Only `status` + `superseded_by` changed** on the three old ADRs: `jj diff meta/decisions/ADR-0016-*.md meta/decisions/ADR-0017-*.md meta/decisions/ADR-0027-*.md` shows only the `status:` line changed and the `superseded_by:` line added — no body edits.
- [ ] Reciprocal `supersedes:` targets still correct: ADR-0047 names `adr:ADR-0016` + `adr:ADR-0017`, ADR-0052 names `adr:ADR-0027`.
- [ ] Corpus validator green: `bash scripts/validate-corpus-frontmatter.sh meta/` (exit 0)
- [ ] `mise run check` passes

#### Manual Verification:

- [ ] ADR-0047/0052 `## Decision` genuinely subsumes the decisions of the ADRs
      they supersede (ADR-0016/0017 config axes; ADR-0027 persist-outputs), not
      merely a supersedes note.
- [ ] ADR-0016's body "forthcoming superseder" note is now satisfied by ADR-0047
      (machine-linked via the added `superseded_by`); the body sentence itself
      stays as-authored (body is immutable) — this stale-but-now-linked phrasing
      is a tolerated, acknowledged consequence of body immutability.
- [ ] The intentional frontmatter/in-body `**Status**` divergence on the three
      flipped ADRs (frontmatter `superseded`, body block unchanged) matches the
      live ADR-0036 convention — frontmatter is authoritative.
- [ ] ADR-0001 remains `accepted`/active and is *not* superseded.

---

## Testing Strategy

### Primary gate (every phase)
`bash scripts/validate-corpus-frontmatter.sh meta/` — whole-corpus schema +
referential-integrity. Baseline is green; it must stay green. It resolves every
`parent`/`supersedes`/`relates_to` ref (so a dangling `supersedes:` target
fails). **Coverage caveats the assertion battery must cover instead**: the
validator does *not* check `superseded_by` at all (it is not an `adr` linkage
key — so a typo'd `superseded_by` target passes green; Phase 3 asserts it by
hand), does *not* check supersede symmetry or status-transition legality (Phase 3
adds an explicit reverse-link invariant for this), and confirms a ref *resolves*
but not that it points at the *intended* §D target (the §D exact-match assertion
covers this).

### Secondary gate
`mise run check` (format/lint/types across components) — guards the repo's
standing CI mirror; the ported Markdown should not perturb it.

### Assertion battery
The `grep` assertions in each phase's Automated Verification act as the
test-first specification: write them, watch them fail against an empty/partial
port, author until green.

### Manual review
Perspective inversion and the merged-superset `## Decision` quality require human
judgement. The cross-reference graph is now largely mechanised (the §D exact-match
`relates_to` assertion and the ordinal/path greps in Phase 2), with a manual
spot-review as backstop — the validator confirms refs *resolve*, not that they
point where the §D map intends.

## Migration Notes

- Allocate IDs only via `adr-next-number.sh` / `work-item-next-number.sh`
  (ADR-0029) — never hand-pick; re-run before authoring in case the corpus moved.
- Supersession is **bilateral**, matching the live ADR-0036/0043 pattern and
  `create-adr --supersedes`: the new superset declares `supersedes:`, and the old
  ADR gets `status: superseded` + `superseded_by:` (two frontmatter fields; no
  body edits), enacted in Phase 3 after the supersets are accepted. The
  `accepted → superseded` transition is legal only post-acceptance (ADR-0031),
  which gates Phase 3. Because the validator does not check `superseded_by`,
  Phase 3 asserts the target and a reverse-link invariant by hand.
- VCS recovery (jj) is the rollback path for any mis-applied edit; no
  dry-run/preview scaffolding needed.

## References

- Work item: `meta/work/0157-port-luminosity-adrs-and-feeding-spikes.md`
- Research: `meta/research/codebase/2026-06-27-0157-porting-luminosity-adrs-and-feeding-spikes.md`
- Related research: `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`
- Source ADRs: https://github.com/atomicinnovation/luminosity/tree/main/meta/decisions
- Source spikes:
  https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0002-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md ,
  https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0003-skill-evaluation-framework-selection.md
- Conventions: `meta/decisions/ADR-0029` (sequential IDs), `ADR-0030` (template),
  `ADR-0031` (immutability), `ADR-0033` (base frontmatter), `ADR-0034` (typed
  linkage), `ADR-0040` (omit-when-empty)
- Validator: `scripts/validate-corpus-frontmatter.sh`
- Overlap targets: `meta/decisions/ADR-0001`, `ADR-0016`, `ADR-0017`, `ADR-0027`
