---
type: codebase-research
id: "2026-06-06-0067-create-note-skill"
title: "Research: Implementing the create-note Skill (0067)"
date: "2026-06-06T09:33:34+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0067"
parent: "work-item:0067"
relates_to: ["codebase-research:2026-05-30-0065-update-artifact-templates-to-unified-schema", "codebase-research:2026-06-02-0093-extend-templates-with-typed-linkage-slots"]
topic: "Implementing the create-note skill and templates/note.md"
tags: [research, codebase, skills, notes, templates, frontmatter, typed-linkage]
revision: "70e86d6dd86759a59677bba83c922c83b08300b5"
repository: "ticket-management"
last_updated: "2026-06-06T09:33:34+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Implementing the create-note Skill (0067)

**Date**: 2026-06-06T09:33:34+00:00
**Author**: Toby Clemson
**Git Commit**: 70e86d6dd86759a59677bba83c922c83b08300b5
**Branch**: ticket-management (jj workspace)
**Repository**: ticket-management

## Research Question

What does it take to implement story 0067 — a new `create-note` skill at
`skills/notes/create-note/SKILL.md` plus the `templates/note.md` template it
consumes — in conformance with the plugin's prevailing skill conventions, the
unified base frontmatter schema (ADR-0033), the typed-linkage vocabulary
(ADR-0034), and the omit-when-empty emission rule (ADR-0040)? Where do the
relevant precedents live, what shape must the artifacts take, and what open
decisions remain?

## Summary

Everything the story needs already exists as precedent; nothing in the
implementation is novel, only newly-assembled. The work is two new files:

1. **`templates/note.md`** — a new template that does not yet exist. It must
   follow the shared frontmatter "spine" used by every shipped template, with
   `type: note`, the 11 unified base fields, the note-specific `topic` extra,
   the `revision`/`repository` provenance bundle, and the omit-when-empty
   typed-linkage slots shown present-but-empty as the authoring surface.
2. **`skills/notes/create-note/SKILL.md`** — a new skill in a new `skills/notes/`
   category (the category does not exist yet). It mirrors `create-work-item`
   structurally, but is materially simpler: no sequential allocator, no
   approval-gated number reservation, and a date-prefixed-slug filename instead.

The two open questions the story carried (file-naming and linkage) are resolved
in the story itself and confirmed by the corpus + ADRs. The two open questions
*this research* set out to resolve — where the verbatim "omit-when-empty" rule
lives, and what the 0067 review demanded — are now answered: the rule is
**ADR-0040** (accepted), and the review is **APPROVE** with a clear linkage
design. The one genuinely-unresolved decision is the **note status vocabulary**,
which no ADR defines and the review did not address (see Open Questions).

The deepest implementation subtlety is the **author/provenance shape choice**:
notes are the first artifact type that is simultaneously *author-authored*
(like work-item/adr, which use a bare `author:` and carry no provenance) yet
*provenance-bearing* (carrying `revision`/`repository` like research/plan, which
use a VCS-derived quoted `author:`). The note template must pick a coherent
combination; this research recommends following the research-template style
(`author: "{author from VCS}"` + provenance bundle) since the provenance bundle
implies the artifact is anchored to a code state.

## Detailed Findings

### 1. Skill-convention precedent: `create-work-item`

`create-work-item` is the canonical precedent the story names, at
`skills/work/create-work-item/SKILL.md` (674 lines). Key structural elements to
mirror:

**Frontmatter** (`SKILL.md:1-9`):
```yaml
---
name: create-work-item
description: Interactively create a well-formed work item. Use when capturing a
  feature, bug, task, spike, or epic as a structured work item in meta/work/.
argument-hint: "[topic or existing work item path/number]"
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/*)
---
```
For `create-note`, the second `allowed-tools` glob (the work-item scripts dir) is
**not needed** — notes have no allocator and no per-skill scripts. The note skill
needs only `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)` (for
`config-read-path.sh` and `artifact-derive-metadata.sh`). The `description` must
carry the note-capture intent keywords ("note" + "capture"/"jot") per AC9.

**Body structure** — a header block of `!`-prefixed config injections, a
preamble, then numbered steps. The dynamic injections at the top
(`SKILL.md:13-23`) load project context, agent names, and resolve the output
directory via `config-read-path.sh`. The template is read at load time via
`config-read-template.sh work-item` (`SKILL.md:31`).

**Elicitation flow** — staged across steps, model-led not form-filling. The
no-argument "Initial Setup" response (`SKILL.md:135-145`) greets the user and
asks an opening question, then waits. For a note this can be much shorter than
create-work-item's multi-step research-and-propose flow: the story only requires
eliciting **topic, body content, and optional tags** (AC4), plus optionally a
related artifact for linkage (AC6).

**Deterministic path derivation** — create-work-item uses a sequential allocator
`work-item-next-number.sh` (`SKILL.md:406-417`), allocated *only at write time*
(placeholder `XXXX` until then). **The story explicitly forbids a notes
allocator** — notes use `meta/notes/YYYY-MM-DD-<topic-slug>.md` instead. So
create-note skips the allocator entirely and derives the filename from the
creation date (from `artifact-derive-metadata.sh`) + a kebab slug of the topic.

**Template consumption / field substitution** (`SKILL.md:443-456`) — the skill
reads the template and substitutes each frontmatter field with a resolved value;
the substitution list is the model worth copying. The metadata helper
(`scripts/artifact-derive-metadata.sh`) supplies `date:`/`last_updated:` (and,
for notes, `revision:`/`repository:`).

**Linkage handling** (`SKILL.md:458-484`) — the omit-by-default rule: "the
template shows each as `""`/`[]`, but write a key into the artifact **only** when
it has a value, and omit it entirely otherwise." This is the exact behaviour
create-note must apply to its linkage slots.

### 2. Template precedent: the shipped 0065/0093 templates

`templates/note.md` does **not** exist; it must be created. All shipped templates
(`templates/work-item.md`, `codebase-research.md`, `plan.md`, `adr.md`,
`rca.md`, `validation.md`, `design-gap.md`) share a strict frontmatter spine:

1. `type:` — bareword discriminator, comment `# artifact-type discriminator`
2. `id:` — **always a quoted string**
3. `title:` — quoted
4. `date:` — quoted ISO timestamp
5. `author:` — bare `Author Name` for author-authored types (work-item, adr);
   quoted `"{author from VCS}"` for VCS-derived types (research, plan)
6. `producer:` — bareword skill name (`create-note`)
7. `status:` — default value with an inline pipe-delimited valid-status comment
   (`# a | b | c`)
8. type-specific scalars (e.g. `topic:` for research)
9. foreign references (e.g. `work_item_id: ""`) — placed *above* the linkage header
10. typed-linkage block, opened by the verbatim header comment:
    `# typed-linkage slots — omit-when-empty in artifacts (drop any left empty)`,
    each slot a `""` (single ref) or `[]` (list) with a per-slot value-shape comment
11. `tags:` — `[]` or a hint list
12. provenance bundle `revision` / `repository` (research/plan only; absent on
    work-item/adr)
13. `last_updated:` / `last_updated_by:` matching the `author` quoting style
14. `schema_version: 1` — **bare integer, always last**

The closest single exemplar for `note.md` is **`templates/codebase-research.md`**
because it is the only shipped template that combines a `topic` field *and* the
`revision`/`repository` provenance bundle — exactly the note extra-field set.
Its frontmatter (`codebase-research.md:1-20`):
```yaml
type: codebase-research                      # artifact-type discriminator
id: "{filename-stem}"                        # filename without .md
title: "Research: {User's Question/Topic}"
date: "{ISO timestamp from artifact-derive-metadata.sh}"
author: "{author from VCS}"
producer: research-codebase
status: complete                             # complete
work_item_id: ""                             # foreign reference; omitted when no linked work item
# typed-linkage slots — omit-when-empty in artifacts (drop any left empty)
parent: ""                                   # typed-linkage ref: "work-item:NNNN" or ""
relates_to: []                               # typed-linkage list: ["codebase-research:NNNN", ...] or []
topic: "{User's Question/Topic}"
tags: [research, codebase, relevant-component-names]
revision: "{commit hash from artifact-derive-metadata.sh}"
repository: "{repo name from artifact-derive-metadata.sh}"
last_updated: "{ISO timestamp}"
last_updated_by: "{Researcher name}"
schema_version: 1
```
For `note.md`: `type: note`, `id: "{filename-stem}"`, `producer: create-note`,
`topic: "{Note topic}"`, the provenance bundle, and the linkage slots
**restricted to `parent` and `relates_to`** (the only keys a note legitimately
writes — see §4). `relates_to`'s per-slot comment should reference the note's
own type: `["note:<id>", ...]` or, more usefully, `["work-item:NNNN", ...]`.

**Note the `id` shape**: research/plan templates derive `id` from the filename
stem (`"{filename-stem}"`), which for a note is the full
`2026-06-06-<topic-slug>` string. This is consistent with ADR-0033's "slug/path
-derived value otherwise" rule.

### 3. Unified base schema (ADR-0033) — what a note must carry

`meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` (accepted) defines
the 11 mandatory base fields (lines 113-125): `type`, `id`, `title`, `date`,
`author`, `producer`, `status`, `tags`, `last_updated`, `last_updated_by`,
`schema_version`.

- `note` is an enumerated valid `type` value (line 115).
- Per-type extras for notes (line 219, verbatim): **"- **note**: topic,
  provenance bundle."** This is the authoritative statement that notes carry
  `topic` + `revision`/`repository`, even though the provenance-bundle prose at
  lines 129-131 doesn't name notes in its narrative list.
- `id` is **always quoted** (line 116, 143-146).
- `schema_version` is a **bare integer**, initial value `1` (lines 125, 170-177).
- Status vocabularies are explicitly **out of scope** (lines 121, 232-236) — the
  ADR fixes field *presence/shape*, not value sets. **No note status vocabulary
  is defined anywhere.** (See Open Questions.)

### 4. Typed-linkage vocabulary (ADR-0034) — note linkage semantics

`meta/decisions/ADR-0034-typed-linkage-vocabulary.md` (accepted). The full key
set (lines 46-56): `parent` (single, hierarchical owner), `supersedes`/
`superseded_by`, `blocks`/`blocked_by`, `target` (single), `derived_from`
(list, fan-in generative source), `relates_to` (list, loose), `source` (single,
external/non-meta extraction origin).

**Ref shape** (lines 67-74): the canonical form is `doc-type:id`, e.g.
`"work-item:0057"`, **as a single quoted YAML string** (`"plan:0042"`, never
`plan:"0042"`); lists are YAML arrays of such strings. A project-root-relative
path (e.g. `"meta/notes/2026-01-15-pipeline-incident.md"`) is the alternate form
when the target lacks a stable id. Since notes now get a real `id`, references
*to* a note should prefer `"note:<id>"`.

**Directionality — the critical rule for notes** (lines 89-104): in every
note-related edge, the note is the **target/origin**, never the owner of
`source`/`derived_from`. A work item extracted from a note carries
`source: "note:<id>"` / `derived_from: ["note:<id>"]` pointing *back* at the
note (lines 100, 104, 56). The note does **not** write these forward. Therefore:

- **`relates_to`** — what a note writes by default when the user names a related
  artifact.
- **`parent`** — written *only* when the user confirms the related artifact
  *owns* the note (hierarchical ownership); the skill must not infer ownership.
- **`source` / `derived_from`** — a note **never** writes these (AC7 makes
  non-emission a pass/fail criterion).

This is exactly the design the 0067 review converged on (§7).

### 5. Omit-when-empty emission (ADR-0040) — the real authority

The story cites "omit-when-empty ... per ADR-0034", but ADR-0034 only establishes
that linkage keys are *optional/absence-tolerant*. The **verbatim omit-when-empty
rule lives in `meta/decisions/ADR-0040-omit-when-empty-frontmatter-emission-supplement-to-adr-0033.md`**
(accepted), which the research surfaced as the load-bearing authority.

Core rule (lines 91-98): "A producer emits an optional non-base frontmatter key
**only** when it resolves to a non-empty value; otherwise the key is **omitted
entirely** ... Templates retain every documented optional slot present-but-empty
(`""` / `[]`) as the authoring surface."

Emission classification (lines 107-131):
- **Always emitted** (even when empty): the 11 base fields incl. `tags: []`;
  provenance `revision`/`repository` for code-state-anchored types;
  always-valued per-type extras — **`topic` is named here as always-valued**
  (line 117 lists `topic` for research; the same applies to notes).
- **Emitted only when non-empty** (omit otherwise): typed-linkage keys
  (`parent`, `relates_to`, `source`, `derived_from`, etc.), foreign references
  (`work_item_id`, `external_id`), optional lifecycle markers.

**Producer-vs-template split** (lines 75-79, 175-177): the template keeps the
empty `parent: ""` / `relates_to: []` slots as the authoring surface; the
**producer skill** drops them when empty. So `templates/note.md` *shows* the
slots; `create-note` *omits* them at write time unless populated. This precisely
mirrors how the skill resolves `status: ""  # draft | ready | …` into a bare
`status:` value (comment dropped, empty optionals not written).

This means for the note's `topic`: because it is an always-valued extra, the
template's `topic:` is **always** written (populated from the elicited topic),
never omitted — consistent with the elicitation requirement (AC4).

### 6. Path config & filename convention

**The `notes` path key is already wired** — no new config needed:
- `scripts/config-defaults.sh:37` → `"paths.notes"`, default at line 57 →
  `"meta/notes"`.
- `skills/config/paths/SKILL.md:40` documents it; `skills/config/configure/SKILL.md:401`
  tabulates it; `.accelerator/config.md` can override via a `paths: { notes: ... }`
  block.
- Consumption pattern (copy from `research-codebase/SKILL.md:24`):
  `!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh notes``, permitted by the
  `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)` allowed-tools glob.

**Filename convention** — the existing corpus (14 notes in `meta/notes/`) all
follow `YYYY-MM-DD-<topic-slug>.md`, e.g.
`2026-05-26-toast-correlation-should-use-document-id.md`. The
`research-codebase` skill's filename rule (`SKILL.md:114-121`) is the direct
precedent for the "without work item" form `YYYY-MM-DD-description.md`. The date
comes from `artifact-derive-metadata.sh`'s `Current Date/Time` (date portion);
the slug is a meaningful kebab summary of the topic (not raw input).

**Existing corpus is free-form**: 13 of 14 notes have *no* frontmatter (they open
directly with `# H1`); one outlier
(`2026-04-17-security-lens-owasp-ai-top-10.md`) carries an ad-hoc
`date/author/tags/status` block, not the unified schema. **No existing note uses
a `topic` field.** This confirms create-note introduces the frontmatter
convention for new notes; the *migration* of these hand-written notes is story
0070's problem (Technical Notes / Dependencies), not this story's.

### 7. The 0067 work-item review — APPROVE, with a settled linkage design

`meta/reviews/work/0067-create-note-skill-review-1.md` ran three passes ending in
`verdict: APPROVE`, `review_pass: 3`; the story was transitioned `draft → ready`.
The review's findings directly shaped the story's current linkage wording and
confirm the implementation target:

- **Linkage** (lines 41-47, 103-107, 214-215): `relates_to` by default; `parent`
  only on **confirmed user ownership** (the skill must not infer it); a note
  **never writes `source`/`derived_from`** — other artifacts write those back to
  the note as extraction origin. A negative AC asserting `source`/`derived_from`
  non-emission was added (AC7).
- **Filename/path** (lines 99, 193): validated as a *strength* — deterministic,
  date-sourced, slug-derived, under `meta/notes`.
- **`producer` policy** (lines 67-69, 220): `producer` records the *originating*
  skill by policy (so the note's `producer` is `create-note`).
- **Testability fixes**: AC6 enumerated to concrete checks (allowed-tools,
  conversational flow, deterministic path); AC7 split into a keyword check (AC9)
  + a routing clause with a dispatch threshold (AC10); a dedicated elicitation
  AC with an explicit fail condition was added.
- **Note status vocabulary**: *not addressed by the review* — remains open.

## Code References

- `skills/work/create-work-item/SKILL.md:1-9` — skill frontmatter / allowed-tools precedent
- `skills/work/create-work-item/SKILL.md:135-145` — "Initial Setup" elicitation greeting
- `skills/work/create-work-item/SKILL.md:443-456` — frontmatter field substitution list
- `skills/work/create-work-item/SKILL.md:458-484` — omit-by-default linkage handling
- `templates/codebase-research.md:1-20` — closest template exemplar (topic + provenance + linkage)
- `templates/work-item.md:1-24` — author-authored frontmatter spine + linkage header comment
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md:113-125` — the 11 base fields
- `meta/decisions/ADR-0033-...:219` — note per-type extras (`topic`, provenance bundle)
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md:46-56` — linkage key table
- `meta/decisions/ADR-0034-...:67-74` — `doc-type:id` ref shape & quoting
- `meta/decisions/ADR-0034-...:89-104` — extraction directionality (note is target, not owner)
- `meta/decisions/ADR-0040-omit-when-empty-frontmatter-emission-supplement-to-adr-0033.md:91-131` — omit-when-empty rule + emission classification
- `scripts/config-defaults.sh:37,57` — `notes` → `meta/notes` default
- `scripts/config-read-path.sh` — single-key path resolver
- `scripts/artifact-derive-metadata.sh:22-25` — date / revision / repository helper
- `skills/research/research-codebase/SKILL.md:114-121` — date-slug filename precedent
- `meta/reviews/work/0067-create-note-skill-review-1.md:214-215,238-242` — settled linkage design + APPROVE

## Architecture Insights

- **Template = authoring surface; producer = emission policy.** ADR-0040 cleanly
  separates the two: templates keep empty `""`/`[]` slots so the schema is
  self-documenting, while skills resolve them (drop comments, omit empty
  optionals) at write time. This is why `templates/note.md` shows `parent`/
  `relates_to` even though most notes will emit neither.
- **Notes are the "extraction origin" end of the linkage graph.** The whole
  point of a note in ADR-0034's model is to be a thing *other* artifacts get
  lifted from. That asymmetry — notes never own `source`/`derived_from` — is the
  single most important semantic constraint and is enforced as a negative AC.
- **No allocator, no approval gate.** Unlike numbered artifacts (work-item, ADR),
  notes are date-slug-named, so create-note is structurally simpler than
  create-work-item: no `next-number.sh`, no "placeholder until write-time
  approval" dance, no concurrent-write number-collision guard (though a
  path-existence check before writing is still prudent, as create-work-item does).
- **The author/provenance combination is novel.** Notes are the first
  author-authored *and* provenance-bearing type. The recommendation is to follow
  the research-template convention (`author: "{author from VCS}"` + provenance
  bundle) because carrying `revision`/`repository` signals a code-state anchor;
  using the bare-author work-item style alongside a provenance bundle would be
  internally inconsistent. (This is a design call to confirm — see Open Questions.)
- **The story's ADR citation is slightly off but harmless.** The story attributes
  omit-when-empty to ADR-0034; the actual authority is ADR-0040. The behaviour
  required is identical, so no story change is needed, but the implementation
  should cite ADR-0040 where it documents the fill/omit rule (ADR-0040 lines
  164-171 require producer SKILL.md guidance to name each omit-when-empty field).

## Historical Context

- `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md` —
  parent epic (in-progress); defines the sibling split where 0067 owns
  `templates/note.md`.
- `meta/work/0060-...` (done) → produced ADR-0033; `meta/work/0061-...` (done) →
  produced ADR-0034 — the two hard blockers, both satisfied.
- `meta/work/0065-...` (done) — updated all artifact templates to the unified
  schema; the precedent `templates/note.md` must mirror.
- `meta/work/0070-...` (draft) — corpus migration; owns the decision of how to
  treat the 14 existing hand-written notes (this story does not migrate them).
- `meta/plans/2026-06-02-0093-extend-templates-with-typed-linkage-slots.md` +
  its research/validation/review cluster — how typed-linkage slots were added to
  templates; the direct model for the note template's linkage block.
- `meta/research/codebase/2026-05-30-0065-update-artifact-templates-to-unified-schema.md`
  — maps the full 0057 sibling dependency graph, including 0067's ownership of
  `templates/note.md`.
- `meta/decisions/ADR-0038-interactive-validation-parameters-for-unified-schema-linkage-migration.md`
  — relevant to 0070's migration, not directly to this story.

## Related Research

- `meta/research/codebase/2026-05-30-0065-update-artifact-templates-to-unified-schema.md`
- `meta/research/codebase/2026-06-02-0093-extend-templates-with-typed-linkage-slots.md`

## Open Questions

1. **Note status vocabulary (genuinely unresolved).** ADR-0033 mandates a
   `status` field but defers value sets per-type; no ADR defines note statuses,
   and the 0067 review did not address it. Notes are short-form captures with no
   real lifecycle — candidate vocabularies: a single `status: captured` (mirrors
   research's single-value `complete`), or a minimal `draft | active | archived`.
   The simplest defensible choice is a **single `captured` value** (or `draft`)
   with an inline `# captured` comment, matching the research/issue/validation
   precedent of single-value status enums. This needs a decision before the
   template's `status:` line can be finalized.
2. **Author/provenance shape (needs confirmation).** Recommended: research-style
   (`author: "{author from VCS}"` + `revision`/`repository` provenance), since
   the provenance bundle implies a code-state anchor. Alternative: work-item-style
   bare `author:` — but that pairs oddly with a provenance bundle. Confirm before
   writing the template's `author:`/`last_updated_by:` quoting.
3. **`relates_to` per-slot comment target.** Should the template's `relates_to`
   comment show `["note:<id>", ...]` (own-type, matching the research template's
   convention) or `["work-item:NNNN", ...]` (the more realistic note→work-item
   link)? Minor, but worth a consistent choice; own-type matches precedent.
4. **Path-existence guard.** create-work-item aborts if the target path already
   exists. Two notes created the same day with the same topic slug would collide.
   Worth deciding whether create-note guards (abort / disambiguate slug) or
   silently overwrites. Recommend a guard consistent with create-work-item.
