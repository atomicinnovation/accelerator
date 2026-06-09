---
id: "ADR-0038"
date: "2026-05-26T17:09:45+01:00"
author: Toby Clemson
status: accepted
tags: [migration, interactive-validation, typed-linkage, frontmatter, accelerator-plugin]
type: adr
title: "ADR-0038: Interactive validation parameters for unified-schema linkage migration"
schema_version: 1
last_updated: "2026-05-26T17:09:45+01:00"
last_updated_by: Toby Clemson
relates_to: ["adr:ADR-0023", "adr:ADR-0033", "adr:ADR-0034", "adr:ADR-0037", "codebase-research:2026-05-24-0068-related-documents-inference-accuracy", "codebase-research:2026-05-26-0062-adr-interactive-validation-for-corpus-migration", "work-item:0057", "work-item:0062", "work-item:0068", "work-item:0069", "work-item:0070"]
---

# ADR-0038: Interactive validation parameters for unified-schema linkage migration

**Date**: 2026-05-26
**Status**: Accepted
**Author**: Toby Clemson

## Context

The unified-schema migration (work item 0057) rewrites two transformation classes on every artifact in `meta/`:

1. **Deterministic transformations**: field renames and shape normalisation against the unified base schema (ADR-0033) and typed-linkage vocabulary (ADR-0034).
2. **Inferential transformations**: best-effort parsing of free-form body prose (`## References`, `## Dependencies`, `## Related Research`, `## Historical Context`, `## Source References`) into typed-linkage frontmatter keys.

The second class cannot meet the ≤5% wrong-rate threshold the corpus is willing to accept for mechanical application. Spike 0068 measured 11.3% wrong on a stratified sample of 150 inferences (84.0% correct, 4.7% uncertain). A cheap-fix counterfactual — patching the three highest-yield parser failure modes (`template-path`, `prose-keyword-false-match`, `sibling-as-deriv`) — reduces the wrong-rate to ~5.3%, still above threshold. The verdict is binding: this migration cannot apply inferential transformations mechanically.

ADR-0037 establishes the framework-level optional interactive contract — trigger predicate, runner-surfaced display elements, accept/edit/skip controls, resumability persistence artefact. The contract is parameterisable per migration. This ADR sets those parameters for the unified-schema migration's inferential transformations, decides the confidence model the parameters are expressed in, decides whether the contract applies uniformly or hybridly, decides whether the parser incorporates the spike's targeted accuracy fixes before applying inferences, and resolves two vocabulary gaps the spike surfaced.

The spike's calibration data shows the parser's `high` and `medium` confidence bands are indistinguishable on the spike's data (88% vs 90% correct on n=50 per band — the spike characterises the gap as "calibration smell, largely cosmetic"), while `low` is materially worse (74% correct). This is the input to the band-design decision below.

## Decision Drivers

- The migration must apply inferential transformations within the deterministic-acceptable wrong-rate threshold, or apply them through a contract that lets the user correct or skip wrong inferences.
- The user-interaction count should be minimised where the parser's confidence is justified — applying the hook uniformly across all ~1,231 inferences would pay a large UX cost for no accuracy gain on the high-confidence majority.
- The confidence model the predicate operates on must reflect what the parser can actually discriminate, not invent distinctions the calibration data does not support.
- Edit semantics must let the user correct both the inferred linkage key (e.g. `parent` vs `derived_from`) and the inferred target identifier — both are bug-prone in different ways.
- Resumability persistence must align with the framework's existing audit-trail conventions (`.accelerator/state/migrations-{applied,skipped}`) so the migration runner has one consistent state-file shape to reason about.
- Vocabulary gaps surfaced by the spike must resolve to a canonical interpretation the parser can encode, so the migration produces v1-vocabulary-conformant frontmatter.

## Considered Options

### Band design

1. **Two-band (resolved / ambiguous)** — Collapse the spike's high/medium/low into a binary: confident enough to apply mechanically (resolved), or not (ambiguous). Matches the calibration finding that high and medium are not separable. Drops the option of using `high` as a strict-bypass for any future tightening.
2. **Three-band with sharpened high gate** — Retain three bands; invest parser work to make `high` accurate enough to bypass interactive review where `medium` would not. Pays parser-complexity cost upfront for a future bypass option this migration does not exploit.

### Application shape

1. **Uniform application** — Trigger predicate fires on every inferential transformation regardless of band. Produces a complete session log of every inference, accepts UX cost of ~1,231 prompts.
2. **Hybrid application** — Trigger predicate fires only when the band is `ambiguous`; `resolved` transformations apply mechanically. Matches the spike's implicit recommendation; produces a partial session log (only ambiguous decisions recorded).

### Parser accuracy fixes from the spike

The spike identified three targeted accuracy fixes (`template-path` blocklist for literal placeholders like `ADR-NNNN.md`; `\bblocks?\b` regex tightened to avoid matching "block" in "code-block"; `\bsibling\b` hint mapping to `relates_to` instead of falling through to `derived_from`). Each is cheap to implement and lowers the wrong-rate from 11.3% to ~5.3%.

1. **Incorporate the fixes in the production parser** — The parser encodes the three fixes before the band classifier runs; fewer inferences classify as `ambiguous`, the user sees fewer prompts.
2. **Omit the fixes** — Run the parser without the fixes; interactive hooks catch the resulting wrong inferences as `ambiguous`-band prompts.

### `"Source:"` prose on plans (vocab-canonical interpretation)

1. **`parent`** — ADR-0034's type-pair table establishes plan→work-item as `parent`. The migration interprets `- Source: meta/work/NNNN-...md` on a plan as the canonical hierarchical relationship.
2. **`derived_from`** — Reads the prose literally as "this plan was derived from that work item"; uses the generative-source key.
3. **`source`** — Widens ADR-0034's `source` semantic (today: external / non-meta origin) to cover plan→work-item.
4. **Documented exception** — Keep the prose as-is in the body; do not migrate it to a typed linkage.

### Broader-workstream linkage

1. **New vocab type `workstream`** — Extend ADR-0034's vocabulary mid-migration.
2. **Reuse `relates_to`** — Loss-of-information mapping per the spike's note.
3. **Documented limitation** — Cite the existing `kind: epic` + `parent:` convention as the supported expression of a workstream; declare multi-ticket groupings without an epic ticket as unsupported.

## Decision

### Band design: two-band

We adopt a **two-band** confidence model: `resolved` (the parser is confident the inferred linkage is correct) and `ambiguous` (the parser is not). The spike's calibration data shows `high` (88%) and `medium` (90%) are indistinguishable on the spike's measurements and therefore not load-bearing; preserving them as distinct bands would encode a distinction the parser cannot justify. The two-band model is what the calibration data supports, and what the trigger predicate operates over.

### Application shape: hybrid

The trigger predicate is `band == 'ambiguous'`. Inferences classified `resolved` apply mechanically; inferences classified `ambiguous` invoke the framework-level interactive hook. This matches the spike's implicit recommendation, minimises user-interaction count to the subset where the parser cannot justify acting alone, and still produces the per-decision session log for the consequential subset.

The alternative — uniform application across all ~1,231 inferences — was rejected on UX grounds: the high-confidence majority gains no accuracy from being routed through user review, and the resulting prompt count would dominate the migration's runtime cost without changing its outcome.

### Trigger predicate parameterisation

Per ADR-0037's framework primitive, the migration declares:

- **Predicate**: `band == 'ambiguous'` — a single confidence-valued field, evaluated per transformation.
- **Field set**: `{band, inferred_key, inferred_target, artifact_path, source_anchor}` — `band` is the confidence-valued field; the rest carry forward as the predicate's evaluation context for display.

### Display elements (beyond the framework-mandatory three)

In addition to ADR-0037's three mandatory slots (proposed transformation; source location; predicate-evaluated value), the runner surfaces:

- The literal prose line that produced the inference (the substring from the body section the parser matched).
- The section heading the prose appeared under (e.g. `## References`).
- Alternative linkage keys the parser considered, when more than one candidate scored close to the chosen one.
- The on-disk-resolution status of the inferred target (resolves / does not resolve).

On invocation of the `edit` control, the runner additionally surfaces the full ADR-0034 v1 linkage-key set as the menu the user selects from (not just the parser's close-scoring candidates), so a correction can reach any v1 key regardless of whether the parser considered it.

### Edit mutation targets

The `edit` control admits two mutation forms:

1. **Linkage-key correction** — replace the inferred key with a different vocab key from ADR-0034's v1 set.
2. **Target-identifier correction** — replace the inferred target with a different artifact reference (in either `doc-type:id` or path form per ADR-0034).

Both forms may be combined in a single edit. The migration validates the edited shape against ADR-0034's vocabulary and ADR-0033's identity-value contract before applying; an invalid edit is re-prompted, not silently accepted. ADR-0037's `accept-degraded` shorthand applies when the edit relaxes the inferred key to a looser-but-valid one (e.g. `parent` → `relates_to`).

### Resumability persistence artefact

The migration writes a session log at:

```
.accelerator/state/migrations-<migration-id>-session.jsonl
```

where `<migration-id>` is the migration's own identifier (the `NNNN-<slug>` form of its file in `skills/config/migrate/migrations/`), assigned at the time the migration script is added.

Format: line-delimited JSON, one record per inferential transformation, appended atomically (temp-then-rename per ADR-0023's atomic-state-write convention) on each user decision. Each record carries `{artifact_path, source_anchor, inferred_key, inferred_target, band, decision, edited_key?, edited_target?}` where `decision ∈ {accepted, edited, skipped}` and the edited fields are present only on `edited` decisions.

The line-delimited shape mirrors the existing `.accelerator/state/migrations-{applied,skipped}` ledger style. On re-entry, the migration reads the session log and skips any transformation whose `(artifact_path, source_anchor)` key is already recorded; the first un-recorded transformation is the resume point. When every pending inferential transformation has a record, the migration's ID is appended to `.accelerator/state/migrations-applied` per ADR-0023.

### Parser accuracy fixes: incorporate

The parser incorporates the three fixes before the band classifier runs. They do not change the interactive-hooks verdict — the residual ~5.3% wrong-rate is still above threshold — but they reduce the prompt count by ~6% (~9 per 150 inferences) and improve the quality of what survives to the `ambiguous` band: the fixes eliminate failure modes the spike has already characterised as parser bugs (literal-placeholder false matches, substring false matches, missing hint vocabulary), so what remains in `ambiguous` is genuine source ambiguity rather than mechanical parser error.

Omitting the fixes was rejected: it would route known-fixable parser bugs through user prompts, paying interaction cost to remediate failures the parser could correctly handle alone. Interactive hooks are the correct mechanism for genuine ambiguity, not for parser bugs whose fixes are characterised in the spike write-up.

### `"Source:"` prose on plans: interpret as `parent`

When `- Source: meta/work/NNNN-...md` (or the `work-item:NNNN` form) appears in a plan's References section, the migration interprets it as a `parent` linkage. ADR-0034's type-pair table establishes plan→work-item as `parent`; the migration normalises the corpus's informal prose conventions onto the v1 vocabulary. The `"Source:"` prose label and the `parent:` key describe the same relationship — the plan is owned by the work item that specified it — and the migration writes the canonical form.

`derived_from` was rejected: it reads the prose more literally but conflicts with ADR-0034's published type-pair table, which assigns `derived_from` to plan→codebase-research / plan→issue-research, not plan→work-item. Adopting `derived_from` here would produce frontmatter that contradicts ADR-0034's table. `source` was rejected: ADR-0034 scopes `source` to external / non-meta origins (e.g. notes extracted into work items); using it for plan→work-item would widen the semantic mid-migration. Documented exception was rejected: leaving the linkage in prose forfeits the migration's goal of typed cross-linkage.

The decision applies only to plan→work-item targets. Other `"Source:"` targets on plans (research, notes, external URLs) map to their respective canonical keys per ADR-0034's type-pair table — for plan→codebase-research / plan→issue-research, the canonical key is `derived_from`; for non-meta targets, `source`.

### Broader-workstream linkage: documented limitation

A linkage whose semantic is "this artifact belongs to a multi-ticket workstream" without a single artifact representing that workstream is unsupported. The supported expression is to declare an epic ticket (`kind: epic`) and use `parent:` to point at it — the convention already in active use across the corpus (`0045-work-management-integration`, `0036-sidebar-redesign`, `0057-unified-artifact-frontmatter-and-typed-cross-linking`).

The unsupported input shape is a body-prose label like `Broader workstream: <free-text description>` that does not name a single artifact target. The migration leaves such prose untouched in the body. We do not introduce a `workstream` linkage type: adding vocabulary mid-migration would require a downstream amendment of ADR-0034 and gain nothing the existing epic + `parent:` pattern cannot already express. We do not reuse `relates_to` as a catch-all because the spike documented the information loss that mapping entails; promoting it to canonical would normalise that loss. No follow-up ticket is created — the existing convention is sufficient.

## Consequences

### Positive

- The trigger predicate is grounded in calibration data: it fires exactly where the parser's accuracy degrades and nowhere else.
- Hybrid application bounds the user-interaction count to the subset of inferences the classifier routes for review (an upper bound estimated at ~16% of inferences, the sum of the spike's measured wrong-rate (11.3%) and uncertain-rate (4.7%); the parser's `ambiguous` band is what the trigger predicate actually fires on, not a directly measured 16% category), rather than the full inference set.
- The session-log format matches the framework's existing state-file conventions, so the migration runner has one persistence pattern rather than two.
- Edit semantics cover the two distinct failure modes the spike characterised: wrong key (e.g. `parent` inferred where `derived_from` was correct) and wrong target (e.g. resolves-on-disk false). Both are correctable without skipping.
- Incorporating the spike's accuracy fixes reduces the migration's prompt count before the user ever sees the hook, and ensures the `ambiguous` band carries genuine source ambiguity rather than known parser bugs.
- Interpreting `"Source:"` prose as `parent` produces ADR-0034-conformant frontmatter without expanding the v1 vocabulary.
- The broader-workstream documented limitation rests on a corpus convention already in active use; no migration code is needed for it.

### Negative

- The two-band model forfeits the option of a `high`-band strict-bypass without re-opening this ADR. If the parser is sharpened later such that a top tier is reliably distinguishable, the migration's predicate must be re-parameterised.
- Hybrid application produces an incomplete session log — only ambiguous decisions are recorded — so a post-migration audit cannot replay the resolved-band inferences from the log alone. The migrated frontmatter is the only record for that subset.
- Edit mutation that touches the linkage key requires the user to know ADR-0034's vocabulary; the runner surfaces the key set, but the burden of selecting the right key is the user's. Mis-selection produces an ADR-0034-conformant but semantically wrong linkage that the migration cannot detect.
- The `"Source:"`-as-`parent` decision binds the migration to one interpretation; in cases where the author's intent was specifically "derived from" (rather than "parented by"), the migration writes the canonical key over the literal intent. The information loss is small but real.
- The broader-workstream documented limitation leaves a class of authorial intent unexpressed in typed linkages; authors who want first-class workstream linkage must create an epic ticket as a side effect of their main work.

### Neutral

- The session-log path includes the migration's own ID (`migrations-<migration-id>-session.jsonl`); any other interactive-path migration must follow the same shape to keep the state-file directory predictable, but no automation enforces this.
- The validation step on edited values catches ADR-0034 / ADR-0033 conformance violations but not semantic correctness; the runner's responsibility ends at "the edit produces a well-formed linkage", not "the edit produces the right linkage".

## References

- `meta/decisions/ADR-0023-meta-directory-migration-framework.md` — mechanical-default migration framework
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` — base schema the migration writes against
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md` — v1 linkage vocabulary and type-pair table
- `meta/decisions/ADR-0037-optional-interactive-contract-supplement-to-adr-0023.md` — framework-level interactive contract this ADR parameterises
- `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md` — spike findings, calibration data, failure-pattern catalogue
- `meta/research/codebase/2026-05-26-0062-adr-interactive-validation-for-corpus-migration.md` — ADR drafting research
- `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md` — parent epic
- `meta/work/0062-adr-corpus-migration-strategy.md` — work item this ADR satisfies
- `meta/work/0068-spike-related-documents-inference-accuracy.md` — spike whose verdict this ADR consumes
- `meta/work/0069-migration-framework-interactive-validation-hooks.md` — runner-implementation downstream
- `meta/work/0070-ship-meta-corpus-unified-schema-migration.md` — corpus-migration shipping downstream
