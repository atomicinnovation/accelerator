---
adr_id: ADR-0033
date: "2026-05-19T08:17:35+00:00"
author: Toby Clemson
status: accepted
tags: [frontmatter, schema, artifacts, meta-directory, migration]
---

# ADR-0033: Unified base frontmatter schema for meta/ artifacts

**Date**: 2026-05-19
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0027 established that every skill producing structured output
writes to `meta/`. ADR-0028 followed with a minimal common
frontmatter schema — `date`, `type`, `skill`, `status` — extended
per-artifact-type.

Since 0028, the corpus has grown to twelve artifact types (work
items, plans, plan validations, plan reviews, work-item reviews, pr
reviews, pr descriptions, ADRs, codebase research, issue research,
design inventories, design gaps) plus an under-served `note`
category. Three of these types (pr-reviews, pr-descriptions, issue
research) are not yet present in this repository's `meta/` but are
in regular use in consumer repositories of this plugin; their
schemas are defined here ahead of first local use.

The per-skill extensions evolved independently and accumulated three
classes of inconsistency:

- **Field-name conflicts** — `work_item_id` vs `work-item` for the
  same concept; `author` vs `researcher` for the same role; the
  work-item `type:` field overloaded with its semantic kind
  (story/epic/...) rather than artifact-type discrimination.
- **Shape variance** — `adr_id` unquoted while `work_item_id` is
  quoted; `date` quoted/unquoted; `last_updated` sometimes a calendar
  date, sometimes a full ISO timestamp.
- **Missing fields** — no `schema_version` (every future migration
  must sniff field presence to detect old shapes); no uniform
  identity field; no provenance for artifacts anchored to a specific
  code state (plans, codebase research, issue research,
  design-inventory, pr-description).

The visualiser tool currently presents the corpus as a linear
pipeline in its user-facing rendering. The corpus is in reality a
graph — many plans derived from one research doc, ADRs superseding
each other, design gaps referencing inventory pairs. Moving the
visualiser to a graph rendering depends on typed structured linkages
in frontmatter, which in turn depend on a stable base schema.

Three work items — 0021 (artifact persistence lifecycle), 0022
(artifact metadata and lifecycle), 0023 (ADR system design) —
produced the prior ADRs in this area (0027, 0028, and the 0029–0032
series). This ADR supplements them by widening the base and
resolving the ambiguities the prior ADRs intentionally deferred.

## Decision Drivers

- Uniform machine-parseability across every artifact type, including
  identity and provenance — not just type and status
- Deterministic detection of old-shape documents during future
  schema migrations, without field-presence sniffing
- Elimination of field-name and value-shape inconsistencies that
  currently force consumers to special-case per artifact type
- VCS-neutral provenance for code-state-anchored artifacts (no
  `git_commit`, no `branch`)
- A single source of truth that producer skills, templates, and
  migrations can reference without ambiguity
- Forward path to typed cross-linkages (decided separately in a
  sibling ADR) and ultimately a graph-rendered corpus

## Considered Options

1. **Keep ADR-0028 as-is, extend per-skill** — Each new field
   negotiated story-by-story. Continues the drift the inconsistencies
   above already demonstrate.
2. **Expand the base, define provenance, fix per-type extras in one
   ADR** — Single document covers base fields, provenance bundle,
   `schema_version` contract, identity-shape contract, and per-type
   extras for all twelve types plus notes. Higher upfront definition
   cost; one source of truth.
3. **Two-document split — base/provenance in ADR, per-type extras in
   a machine-readable schema file** — Cleaner separation between
   decision and reference data, but creates a synchronisation surface
   between two artifacts.
4. **Single global `schema_version`** — One integer for the whole
   corpus. Simpler to reason about, but couples migration cadence
   across artifact types — bumping work-item's shape would force a
   no-op bump on plans.

## Decision

We will adopt option 2 — a single ADR that defines the unified base
schema, the provenance bundle, the `schema_version` contract, the
identity-value shape contract, and the per-artifact-type extras for
every artifact type the Accelerator plugin produces.

This ADR **supplements** ADR-0028 — the prior decision that every
meta/ artifact carries common machine-parseable frontmatter stands.
The base set defined here widens that minimum, overrides specific
shape decisions where the corpus is inconsistent, and adds
provenance and `schema_version` that ADR-0028 did not address. Work
items 0021, 0022, 0023 remain valid; this ADR is the supplementing
decision.

### Base schema (every artifact)

Every artifact carries the following frontmatter fields:

| Field             | Shape                                 | Notes |
|-------------------|---------------------------------------|-------|
| `type`            | kebab-case string                     | Artifact-type discriminator — one of `work-item`, `plan`, `plan-review`, `plan-validation`, `work-item-review`, `pr-review`, `pr-description`, `adr`, `codebase-research`, `issue-research`, `design-inventory`, `design-gap`, `note` |
| `id`              | quoted YAML string                    | The artifact's **own** identity — unified key across all artifact types. Value is the natural ID where it has one (e.g. `"0042"`, `"ADR-0033"`), or a slug/path-derived value otherwise. Always quoted. A reference to a **foreign** artifact, where keyed by artifact type rather than by relationship, uses `<snake_case_type>_id` instead — see §Identity-value shape contract. |
| `title`           | string                                | Kept in sync with body H1 where applicable |
| `date`            | quoted ISO UTC timestamp              | Creation timestamp, `"YYYY-MM-DDTHH:MM:SS+00:00"` |
| `author`          | string                                | Human identity of the artifact's creator. Replaces `researcher`. |
| `producer`        | string                                | Identifier of the skill (or other automated agent) that produced the artifact, where applicable. Replaces ADR-0028's `skill` field. Distinct from `author` — see §ADR-0028 override below. |
| `status`          | string                                | Artifact-specific vocabulary; value set is per artifact type, not unified across the corpus |
| `tags`            | YAML array of strings, possibly empty | |
| `last_updated`    | quoted ISO UTC timestamp              | Refreshed only by skills that touch the artifact (manual edits will not auto-update) |
| `last_updated_by` | string                                | |
| `schema_version`  | integer                               | Per-artifact-type version — see §Schema versioning |

### Provenance bundle (code-state-anchored artifacts)

Artifacts whose content is meaningful only relative to a specific
code state — **plans, codebase research, issue research / RCA,
design-inventory, pr-description** — additionally carry:

| Field        | Shape  | Notes |
|--------------|--------|-------|
| `revision`   | string | VCS-neutral commit identifier of the code state the artifact describes. Replaces `git_commit`. Producers write the change ID under jujutsu and the commit SHA under git. |
| `repository` | string | Repository name or path (whichever the producer canonically uses) |

`branch` is removed from the schema — `revision` alone identifies
the snapshot, and branches are mutable references that go stale.

### Identity-value shape contract

The base `id` field is always a quoted YAML string, regardless of
artifact type or value shape (numeric-looking, prefixed, slug). This
generalises the contract already enforced for `work_item_id` per
`skills/config/configure/SKILL.md` to every artifact's identity.
Consumers can locate the identity at a single, predictable key
without per-type parsing.

Two distinct identity-key roles follow from this:

- **Own identity** — the artifact's own ID is always keyed `id`.
  This is the single, type-independent key consumers read to
  identify the document in front of them.
- **Foreign reference** — a frontmatter key that references another
  artifact by its ID, where the key is named after the referenced
  artifact's *type* rather than a relationship, is keyed by that
  type's snake_case name suffixed with `_id` (e.g. a pr-description
  referencing its work-item uses `work_item_id`; a reference to an
  ADR uses `adr_id`). Foreign-reference values follow the same
  quoted-string contract as `id`.

Relationship-named cross-linkage keys (`parent`, `supersedes`,
`target`, `derived_from`, etc.) are a separate vocabulary decided in
the sibling typed-linkage ADR; they are named by relationship, not
by `<type>_id`, and are out of scope for this contract.

### Schema versioning

`schema_version` is a per-artifact-type integer. Each artifact type
owns its own version counter; bumping the work-item schema does not
force a version bump on plans. Future migrations detect old-shape
documents deterministically by reading `schema_version` rather than
sniffing field presence.

The initial value at this ADR's acceptance is `1` for every artifact
type. Migrations that change a type's shape MUST bump that type's
`schema_version` and MUST set the new value on every artifact of
that type they touch.

### ADR-0028 override

ADR-0028's base set was `date`, `type`, `skill`, `status`. This ADR
keeps `date`, `type`, `status` and renames `skill` to `producer` in
the mandatory base. `author` is reserved for the human identity of
the artifact's creator (subsuming `researcher` where it was used);
`producer` carries the skill or other automated agent identifier.
Keeping the two distinct preserves machine-parseability of "what
produced this artifact" — a question consumers ask independently of
"who is responsible for it".

### Per-artifact-type extras

In addition to the base fields, each artifact type carries the
type-private extras listed below. **Typed cross-linkage keys**
(linkages between artifacts — e.g. parent/child, supersedes,
review-of, derived-from) are decided in a sibling ADR and are NOT
listed here. The two decisions move independently.

- **work-item**: `kind` (renamed from `type:` to free the slot for
  the artifact-type discriminator — values
  `story | epic | task | bug | spike`), `priority`, `external_id`
  (cross-system pointer per epic 0045 conventions).
- **plan**: provenance bundle, `reviewer` once reviewed.
- **plan-validation**: `result`, baseline fields.
- **plan-review / work-item-review / pr-review**: `reviewer`,
  `verdict`, `lenses`, `review_number`, `review_pass` where
  applicable.
- **pr-description**: `pr_url`, `pr_number`, provenance bundle,
  `merge_commit` once merged.
- **adr**: `decision_makers`.
- **codebase-research / issue-research**: provenance bundle,
  `topic`.
- **design-inventory**: existing schema is already rich; align
  field-name conventions to the base schema only.
- **design-gap**: keep `current_inventory` / `target_inventory` as
  type-specific keys — they add semantic value beyond the generic
  linkage vocabulary defined in the sibling ADR.
- **note**: `topic`, provenance bundle.

### Source of truth

This ADR is the single source of truth. No separate machine-readable
schema file is produced. Future migrations, templates, and consumer
skills reference this ADR by number. If at some future point a
machine-readable schema becomes load-bearing (e.g., for a validation
tool that fails CI on schema drift), introducing one is a follow-up
decision; the cost of the sync surface is not justified today.

### Out of scope

- **Vocabulary unification** — `status` values, review verdict
  enums, and similar value-set unification are out of scope. The
  base schema fixes field presence and shape, not value
  vocabularies. Each artifact type continues to own its `status` and
  verdict value sets.
- **Typed cross-linkage vocabulary** — `parent`, `supersedes`,
  `blocks`, `derived_from`, etc. are decided in a sibling ADR so the
  two decisions can move independently.
- **Producer skill updates and corpus migration** — Tracked under
  the parent epic (0057) as separate stories that consume this
  ADR's decisions.

## Consequences

### Positive

- Every artifact carries identity, provenance (where applicable),
  and a version counter — consumers can locate, attribute, and
  migrate artifacts without per-type knowledge.
- Future migrations detect old shapes deterministically via
  `schema_version`; field-presence sniffing is no longer the only
  signal available.
- Field-name and shape conflicts (`work_item_id` vs `work-item`,
  quoted vs unquoted identities) are resolved in one document
  instead of being negotiated per-skill.
- VCS-neutral `revision` decouples the schema from git as the only
  supported VCS.
- The visualiser's path to graph rendering becomes unblocked at the
  schema layer.

### Negative

- A meaningful migration burden — every existing artifact in `meta/`
  needs frontmatter rewrites for field names, shapes, and the new
  baseline fields. Tracked under 0057.
- The work-item `type:` → `kind:` rename is disruptive — templates,
  resolver scripts, agent prompts, and helpers all change in one
  coordinated story.
- Producer skills that emit frontmatter inline rather than via
  templates now have a longer mandatory field set to keep in sync.
- The single-ADR choice means this document is longer than the
  Accelerator ADR norm; per-type extras inflate its surface area.

### Neutral

- `last_updated` is refreshed only by skills, not by manual edits —
  a deliberate trade-off accepted under the parent epic.
- Per-artifact-type `schema_version` decouples migration cadences
  but multiplies the version counters to track.
- ADR-0028's `skill` field is renamed to `producer` and remains in
  the mandatory base; `author` is reserved for human creators
  (subsuming the prior `researcher` field).

## References

- `meta/work/0060-adr-unified-base-frontmatter-schema.md` — Source
  task for this ADR
- `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md` —
  Parent epic
- `meta/decisions/ADR-0027-persist-structured-skill-outputs-to-meta.md` —
  Establishes that structured outputs persist to `meta/`
- `meta/decisions/ADR-0028-common-frontmatter-schema-for-meta-artifacts.md` —
  Defines the prior minimal base schema this ADR supplements and
  partially overrides
- `meta/work/0021-artifact-persistence-lifecycle.md` — Supplemented
- `meta/work/0022-artifact-metadata-and-lifecycle.md` — Supplemented
- `meta/work/0023-adr-system-design.md` — Supplemented
- `meta/work/0061-adr-typed-linkage-vocabulary.md` — Sibling ADR
  task defining the typed cross-linkage vocabulary (decided
  independently)
- `meta/work/0045-work-management-integration.md` — Defines
  `external_id` conventions referenced under work-item extras
- `skills/config/configure/SKILL.md` — Identity-value quoting
  contract that this ADR generalises
