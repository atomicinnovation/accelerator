---
id: "ADR-0040"
date: "2026-06-04T17:22:01+01:00"
author: Toby Clemson
status: accepted
tags: [frontmatter, schema, artifacts, emission, linkage]
type: adr
title: "ADR-0040: Omit-When-Empty Frontmatter Emission — supplement to ADR-0033"
schema_version: 1
last_updated: "2026-06-04T17:22:01+01:00"
last_updated_by: Toby Clemson
relates_to: ["adr:ADR-0033", "adr:ADR-0034", "adr:ADR-0031", "adr:ADR-0035", "adr:ADR-0037", "adr:ADR-0030", "work-item:0093", "work-item:0070"]
---

# ADR-0040: Omit-When-Empty Frontmatter Emission — supplement to ADR-0033

**Date**: 2026-06-04
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0033 (Unified Base Frontmatter Schema) mandates that a fixed set of
**base fields** is present on every artifact (`type`, `id`, `title`,
`date`, `author`, `producer`, `status`, `tags`, `last_updated`,
`last_updated_by`, `schema_version`), with `tags` explicitly allowed to
be present-but-empty. ADR-0033 is silent, however, on the *optional
non-base* fields — foreign references, lifecycle markers, and (since
ADR-0034) the typed-linkage vocabulary. The template convention to date
has been to carry those optional keys present-but-empty (`""` / `[]`) as
the authoring surface, and producers have copied that scaffold into
generated artifacts. On real artifacts this produces verbose, noisy
frontmatter: a freshly drafted work item carries a stack of empty
`parent: ""`, `blocks: []`, `blocked_by: []`, `derived_from: []`,
`relates_to: []`, `source: ""`, `external_id: ""` placeholders that
encode no information.

ADR-0034 (Typed-Linkage Vocabulary) adds the linkage keys but governs
their *value shape and semantics*, not whether a key must be present
when it has no value. Crucially, every consumer already reads an empty optional key and
an absent optional key identically as "no value" — the visualiser's
typed-ref parser returns `None` on empty, `read_ref_keys` treats absent
keys as absent edges, and `cluster_key` tolerates both the typed
(`"work-item:NNNN"`) and bare-id (`"NNNN"`) shapes. The corpus is
therefore *already* tolerant of omission, and is *already* inconsistent:
some files carry empty optionals (`external_id: ""`), others omit the
same keys entirely (`parent: ""` on one work item, the key absent on
another). What is missing is a deliberate, uniform rule.

ADR-0033 is `accepted` and therefore immutable under ADR-0031. This ADR
is structured as a **supplement** to ADR-0033 — following the
supplement-form precedent set by ADR-0033 itself (supplements ADR-0028)
and the explicit `-supplement-to-adr-NNNN` filename convention introduced
by ADR-0035 (supplements ADR-0026) and reused by ADR-0037 (supplements
ADR-0023) — rather than as an in-place edit (forbidden by ADR-0031) or a
full supersession (which would mark ADR-0033's still-load-bearing
base-field-presence contract as `superseded`). It supplements ADR-0033's
field-presence contract; it does not alter ADR-0034's value-shape
contract.

## Decision Drivers

- Real artifacts are cluttered with empty optional placeholders that
  encode no information, making frontmatter harder to read and review.
- Consumers already read empty and absent optional keys identically as
  "no value", so omission changes no consumer behaviour.
- The corpus is already inconsistent — some files carry empty optionals,
  some omit them — so a single uniform rule is needed to settle the drift
  in one direction.
- The rule must not erode ADR-0033's base-field-presence guarantee or
  the always-present signal carried by always-valued per-type extras.

## Considered Options

1. **Keep present-but-empty everywhere (status quo).** Continue copying
   the template's empty `""` / `[]` placeholders into generated
   artifacts for every optional non-base key. Requires no change.
   Rejected: it is the source of the verbose, noisy frontmatter, and it
   is already contradicted by the existing omissions in the corpus, so
   "status quo" is not even uniformly true today.
2. **Omit-when-empty for optional non-base keys (chosen).** Producers
   emit an optional non-base key only when it resolves to a non-empty
   value, and omit it entirely otherwise. Base fields (incl. `tags`) and
   always-valued per-type extras remain present. Templates retain the
   documented empty slots as the authoring surface.
3. **Omit *all* non-mandatory keys, including always-valued extras.**
   Drop any key not in ADR-0033's base set whenever a producer could in
   principle leave it out. Rejected: it loses the useful always-present
   signal carried by always-valued extras (`kind`, `priority`, `topic`,
   `verdict`, the design-inventory provenance set, …) and conflicts with
   ADR-0033's rule that `tags` stays present even when empty.

## Decision

We choose option 2 ("Omit-when-empty for optional non-base keys").

A producer emits an optional non-base frontmatter key **only** when it
resolves to a non-empty value; otherwise the key is **omitted entirely**
from the generated artifact. Base fields (incl. `tags: []`) and
always-valued per-type extras are always present. Templates retain every
documented optional slot present-but-empty (`""` / `[]`) as the authoring
surface — producers resolve that scaffold the same way they already
resolve `status: ""  # draft | ready | …` into a bare `status: ready`:
comments are dropped and empty optional keys are simply not written.

The **canonical shape** for a new typed-linkage write is the typed form
(`"work-item:NNNN"`, `"plan:NNNN"`, `"adr:ADR-NNNN"`, …); the bare-id
form (`"NNNN"`) is a tolerated legacy shape that consumers continue to
accept but that new writes should not produce.

### Emission classification (the omit-when-empty boundary)

**Always emitted (present on every artifact):**

- **Base fields (ADR-0033 mandate):** `type`, `id`, `title`, `date`,
  `author`, `producer`, `status`, `tags`, `last_updated`,
  `last_updated_by`, `schema_version`. `tags: []` stays when empty.
- **Provenance (code-state-anchored types):** `revision`, `repository`.
  (design-inventory already omits these when the source is not a code
  repo — a pre-existing, separate carve-out, left as-is.)
- **Always-valued per-type extras:** `kind`, `priority` (work-item);
  `topic` (research); `result`, `verdict`, `lenses`, `review_number`,
  `review_pass` (reviews/validation); `pr_number` (pr-description,
  pr-review); `source`, `source_kind`, `source_location`, `crawler`,
  `sequence`, `screenshots_incomplete` (design-inventory);
  `current_inventory`, `target_inventory` (design-gap); the transitional
  `work_item_id` alias on work-item-review (mirrors `target`, always
  valued).

**Emitted only when non-empty (omit otherwise):**

- **Typed-linkage keys (ADR-0034):** `parent`, `blocks`, `blocked_by`,
  `derived_from`, `relates_to`, `source`, `supersedes`, `superseded_by`,
  `target`.
- **Foreign references:** `work_item_id`, `external_id`.
- **Optional extras / lifecycle markers:** `decision_makers` (adr),
  `reviewer` (plan), `pr_url` and `merge_commit` (pr-description).

### Recursive supplement clause

Once this ADR is `accepted` it too becomes immutable under ADR-0031, and
a future extension of the omit-when-empty boundary — a new optional-field
class, a reclassification of an always-emitted field as omit-when-empty
(or vice versa), or any change to the Emission classification above —
must be recorded in a further supplementary ADR (supplementing either
ADR-0033 or this one — both records remain authoritative for the contract
elements they define), never as an in-place edit to this record.

## Consequences

### Positive

- Generated artifacts are cleaner and less noisy: a fresh draft carries
  only the keys that actually have values, not a stack of empty
  placeholders.
- One uniform rule applies across every producer, settling the existing
  corpus inconsistency in a single direction.
- The rule matches the corpus's existing tolerance — consumers already
  read empty and absent keys identically — so no consumer or runtime
  change is required to adopt it.

### Negative

- **Reader-facing rule:** an absent optional key MUST be read as "no
  value", never as an error or as missing/oversight data. There is no
  in-artifact signal distinguishing a deliberate omission from an
  accidental one, so consumers, readers, and downstream migrations
  (notably 0070) rely on this rule. It is stated here explicitly so they
  can depend on it.
- The convention is enforced only at the *guidance* level: producer
  SKILL.md instructions name each omit-when-empty field with a fill/omit
  note, and a contract test checks for that guidance. No test inspects a
  *generated artifact* to confirm an empty optional key was actually
  omitted, so a producer that emits `external_id: ""` would violate the
  convention with no test signal. This is a deliberate scope boundary; a
  fixture-based producer-output test can be scoped as follow-up if drift
  appears.

### Neutral

- Template comments document the omit rule; producer SKILL.md guidance
  says fill/omit. The templates keep their empty slots as the authoring
  surface, so the authoring experience is unchanged.
- No corpus migration is performed by this ADR. Existing artifacts under
  `meta/` retain their current shape; 0070 owns the corpus migration and
  its inferred-link writes follow this rule.

## References

- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` — foundation record this supplement extends (base-field presence)
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md` — governs linkage value-shape and semantics, not slot presence
- `meta/decisions/ADR-0031-skill-level-adr-immutability.md` — immutability rule that motivates the supplement-vs-edit choice
- `meta/decisions/ADR-0035-brand-layer-indirection-supplement-to-adr-0026.md` — prior precedent for the supplement filename convention
- `meta/decisions/ADR-0037-optional-interactive-contract-supplement-to-adr-0023.md` — prior precedent for the supplement pattern
- `meta/decisions/ADR-0030-adr-template.md` — template authority
- `meta/work/0093-extend-templates-with-typed-linkage-slots.md` — work item this ADR supports
- `meta/work/0070-ship-meta-corpus-unified-schema-migration.md` — downstream consumer; its inferred-link writes follow this rule
