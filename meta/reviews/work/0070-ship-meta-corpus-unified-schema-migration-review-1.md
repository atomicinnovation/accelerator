---
type: work-item-review
id: "0070-ship-meta-corpus-unified-schema-migration-review-1"
title: "Work Item Review: Ship `meta/` Corpus Unified-Schema Migration"
date: "2026-06-06T21:07:30+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0070"
parent: "work-item:0057"
work_item_id: "0070"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 4
tags: [migration, frontmatter, schema, dogfood]
last_updated: "2026-06-06T23:20:24+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Ship `meta/` Corpus Unified-Schema Migration

**Verdict:** REVISE

This is a mature, exceptionally thorough draft: every standard section is present and densely populated, ownership boundaries against sibling migrations are sharply drawn, and implementation anchors give concrete file:line starting points. The REVISE verdict is driven not by missing content but by seven major findings clustering around three themes — an underspecified inference-band vocabulary that is both unclear and unverifiable, an XL scope spanning five workstreams across two component boundaries, and runtime/cross-repo ordering couplings that live in the body prose but never reach the Dependencies section. None are critical; the work item is structurally sound and the changes are sharpening rather than rebuilding.

### Cross-Cutting Themes

- **Inference-band vocabulary is undefined and unverifiable** (flagged by: clarity, testability) — The mechanical-vs-interactive routing turns on classifying inferences into "resolved" vs "ambiguous" bands, but the band set and classification rule are never defined (clarity), and no oracle exists to verify a reference was banded correctly (testability). This is the pivot of the interactive-migration decision, so the ambiguity matters.
- **XL scope across multiple workstreams and component boundaries** (flagged by: scope, dependency, completeness) — The story self-declares "XL — five distinct workstreams," crosses the shell-migration / Rust-visualiser-server boundary, and bundles a separable notes backfill. Multiple lenses independently flagged the notes backfill and the Rust removal as candidates for extraction.
- **Runtime and cross-repo ordering couplings missing from Dependencies** (flagged by: dependency, clarity) — The migration's hard reliance on migrations 0005/0006 having *run* (not just existing), and the fallback-removal's assumption that every userspace repo has already run `/accelerator:migrate`, are asserted in body prose but absent from Dependencies. Clarity separately flagged the "0065→0070 transition" referent as requiring the reader to reconstruct this same lifecycle.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Clarity**: Base schema field named both `type` and `kind` — contradictory referents for the same discriminator
  **Location**: Requirements
  The frontmatter uses `kind: story` and Requirements name the `type:`→`kind:` rename (owned by 0005), yet the same Requirements list base fields as "(`type`, identity, …)" and notes get `type: note`. A reader cannot tell whether the unified discriminator this migration writes is `type` or `kind`, nor whether notes and work-items differ by design.

- 🟡 **Dependency**: Runtime ordering dependency on migrations 0005 and 0006 having run is not captured in Dependencies
  **Location**: Dependencies
  The story assumes migrations 0005 and 0006 have already run against the corpus before 0007 executes — a hard runtime ordering constraint between migration scripts, distinct from the story-level `Blocked by` links (which only guarantee the migrations *exist*). If the runner doesn't enforce ordering, 0007 against an un-migrated repo would mis-transform or refuse.

- 🟡 **Dependency**: Cross-repo per-userspace coupling for removing transitional fallbacks is asserted but not captured
  **Location**: Requirements
  Removing the visualiser-server `work-item:` fallback and the 0065 dual-key path is justified by "every userspace repo will have run `/accelerator:migrate` at least once" — a cross-repo condition this story cannot itself verify. If any repo upgrades before migrating, the visualiser graph breaks with no grace.

- 🟡 **Scope**: Story self-declares XL across five distinct workstreams
  **Location**: Technical Notes
  Sized "XL — five distinct workstreams" (linkage parser, interactive-migration authoring, awk rewrite, Rust removal, notes backfill) plus a dogfood pass. Several have independent boundaries (notes backfill, Rust removal could each ship/revert alone), making the item hard to plan, review, and partially complete.

- 🟡 **Scope**: Story crosses the shell-migration / Rust-visualiser service boundary
  **Location**: Requirements
  Beyond the shell/awk migration, the story removes transitional Rust code across three files (`frontmatter.rs`, `cluster_key.rs`, `indexer.rs`) in a different component. The increment can't be owned end-to-end within one component's review/test cycle.

- 🟡 **Testability**: Headline criterion uses unbounded "every artifact" and undefined "cleanly"
  **Location**: Acceptance Criteria
  "applies cleanly … leaving every artifact conforming" has no oracle distinguishing a clean apply from one with DIVERGE/REFUSE/MALFORMED diagnostics. Needs a checkable assertion (exit 0, zero refusals, validates against a named schema validator, re-run is a no-op).

- 🟡 **Testability**: Inference-band classification criterion has no verification oracle
  **Location**: Acceptance Criteria
  The criterion requires resolved-band applied mechanically / ambiguous-band routed to the hook, but provides no way to confirm a reference was banded correctly — no expected mapping, no threshold. The criterion could pass even if the parser systematically mis-bands.

#### Minor

- 🔵 **Clarity**: Band-classification vocabulary (`resolved` / `ambiguous`) used before definition
  **Location**: Requirements
  The band taxonomy is treated as defined but never stated; the Summary uses the looser "confidently resolvable" for what may be the same concept. Link to where ADR-0038 / spike 0068 defines the rule.

- 🔵 **Clarity**: "this story" / "the 0065→0070 transition" referents require tracking long cross-references
  **Location**: Requirements
  Timing claims central to removal-safety ("same release that closes out 0070", "this transition") are stated as prose assertions rather than one explicit precondition. State the gating precondition once and reference it.

- 🔵 **Clarity**: `work_item_id:` referenced as retained foreign key, removed alias, and renamed identity without local disambiguation
  **Location**: Technical Notes
  The same token denotes 'keep' (foreign ref on plans), 'remove' (review-template alias), and 'rename' (own identity → `id:`) across nearby sentences. Qualify the surface each obligation applies to.

- 🔵 **Completeness**: Story kind lacks an identified user or beneficiary
  **Location**: Summary / Context
  Typed `kind: story`, but neither Summary nor Context names who benefits — framing is purely the corpus rewrite and closing 0057. Add the consumer (visualiser-graph epic, userspace repos) and their need, or reclassify.

- 🔵 **Completeness**: `schema_version` frontmatter field absent on this work item
  **Location**: Frontmatter: schema_version
  The file omits the `schema_version` it mandates and keeps legacy `work_item_id:`. Drafting Notes document this as deliberate ("converting it is precisely this migration's job"), so this is a knowingly-deferred omission, not an oversight.

- 🔵 **Dependency**: Pending supplementary ADR for the `pr:` reference vocabulary is an unnamed upstream dependency
  **Location**: Requirements
  The `pr:`-tolerance carve-out depends on a pending supplementary ADR not listed in Dependencies/References. Add it as a Related dependency so the eventual reconciliation is tracked.

- 🔵 **Dependency**: Hand-off from 0067 (create-note) for notes backfill understated
  **Location**: Requirements
  The notes baseline schema this story emits must match `create-note`'s (0067) `note` schema, but the contract is treated as settled while Drafting Notes flag the notes decisions as "this story's calls." Make the schema-conformance coupling explicit.

- 🔵 **Scope**: Notes backfill is a self-contained sub-deliverable bundled into the corpus migration
  **Location**: Requirements
  The `meta/notes/` backfill has its own inference logic and hand-off owner (0067) and addresses hand-written notes rather than the structured-artifact corpus. Confirm it must ship inside the same migration run, or extract it.

- 🔵 **Testability**: Notes-backfill author resolution lacks a verifiable expected outcome
  **Location**: Acceptance Criteria
  "resolved `author`" collapses a two-branch behaviour (VCS history vs conservative fallback) into one word. Split into two checks and specify the fallback value.

- 🔵 **Testability**: "sensible defaults where missing" is not measurable
  **Location**: Requirements
  No criterion enumerates the per-field defaults, so "sensible" can't be verified. Add a table of default values per base field when absent.

- 🔵 **Testability**: 0065 dual-key fallback removal spans multiple sites but criterion checks only removal in aggregate
  **Location**: Acceptance Criteria
  Removals are located across three named sites (`frontmatter.rs`, `cluster_key.rs:parent_or_legacy_id`, `indexer.rs` filename fallback); partial removal could pass. Enumerate the sites or add a grep-anchored "no dual-key read path remains" check.

#### Suggestions

- 🔵 **Testability**: "exercised" for parser fixes lacks a defined exercising input
  **Location**: Acceptance Criteria
  "encoded and exercised" could be claimed by coverage alone. For each fix, state the negative/positive case (e.g. "code-block" prose produces no `blocks` linkage; a literal `ADR-NNNN.md` is not emitted as a link).

### Strengths

- ✅ Ownership boundaries against sibling migrations are sharply drawn — the `type:`→`kind:` rewrite (0005), the `work-item:`/`researcher:` renames (0006), and `specs/`/`global/` are all explicitly disowned, preventing duplication.
- ✅ Every standard section is present and substantively populated; no empty, placeholder, or sparse sections.
- ✅ The Open Questions section is explicitly resolved, with each prior question's resolution traced into Requirements and Acceptance Criteria — no dangling unknowns.
- ✅ All ten upstream producer stories (0060–0069) are enumerated with their ADRs/migrations and confirmed done; the downstream visualiser-graph epic is captured as a Blocks entry.
- ✅ Implementation anchors give concrete file:line referents (e.g. `frontmatter.rs:334-341`, migration `0006` as precedent), disambiguating otherwise-vague phrases and giving an implementer real starting points.
- ✅ Acceptance Criteria align tightly with Requirements, and several are mechanically checkable (named Rust test removal, `no git_commit/branch remains`, idempotency with a defined detection key).
- ✅ Cleanup obligations other stories conditioned on "0070 having run" are deliberately consolidated here, which is coherent because they become safe only once this migration runs.

### Recommended Changes

1. **Disambiguate the `type` vs `kind` discriminator** (addresses: clarity "Base schema field named both `type` and `kind`")
   State once, with a link to ADR-0033's identity/discriminator definition, which field name is canonical in the unified schema — and if work-items use `kind` while other artifact types use `type` by design, say so explicitly and explain the split. Audit every base-field mention in Requirements and Acceptance Criteria for consistency.

2. **Define the inference-band vocabulary and give it a verification oracle** (addresses: clarity "Band-classification vocabulary", testability "Inference-band classification criterion has no oracle")
   On first use, define the band set (resolved / ambiguous) and the classification rule, or cite where ADR-0038 / spike 0068 defines it (the 11.3% wrong-rate / ≤5% threshold suggests a citable rule). Then add an acceptance criterion with a concrete expectation — a fixture set of references with expected bands, or a spot-check assertion against the measured wrong-rate — and name the artefact (session log) that records which references went to the hook.

3. **Surface runtime and cross-repo ordering couplings in Dependencies** (addresses: dependency "Runtime ordering dependency", dependency "Cross-repo per-userspace coupling", clarity "0065→0070 transition referent")
   Add to Dependencies: (a) the migration-level prerequisite that 0007 requires migrations 0005 and 0006 to have been *applied* first, noting how the runner's ordered ledger enforces it; (b) the cross-consumer condition that the fallback-removal release depends on userspace repos having run `/accelerator:migrate`, naming any migrate-on-upgrade mechanism that guarantees it. State the removal-gating precondition once and reference it from each removal obligation.

4. **Resolve the XL scope question explicitly** (addresses: scope "XL across five workstreams", scope "crosses service boundary", scope "notes backfill bundled")
   Decide and document one of: (a) split the Rust visualiser-server removal and/or the notes backfill into sibling stories sequenced after the corpus migration, leaving 0070 focused on the core rewrite + dogfood; or (b) justify in Technical Notes why the workstreams are indivisible (e.g. the dogfood must exercise all of them together, and "same release" requires "same story"), making the XL sizing a deliberate, defended call.

5. **Tighten the headline conformance criterion into a checkable assertion** (addresses: testability "unbounded every artifact / undefined cleanly")
   Reframe to name the pass condition: runner exits 0 with zero REFUSE/MALFORMED diagnostics, every file validates against the named unified-schema validator, and a re-run emits no further changes.

6. **Add per-field default and notes-author criteria** (addresses: testability "sensible defaults not measurable", testability "notes-backfill author resolution", dependency "0067 hand-off understated")
   Add a table of default values per base field when absent; split the notes `author` criterion into VCS-history and conservative-fallback branches with the fallback value specified; and note that the notes baseline schema must match `create-note`'s (0067) emitted `note` shape.

7. **Make the multi-site and parser-fix removals individually verifiable** (addresses: testability "0065 dual-key removal aggregate", testability "exercised lacks input")
   Enumerate the three 0065 dual-key sites (or add a grep-anchored "no dual-key read path remains" check), and give each parser fix an input/expected-output pair.

8. **Minor polish** (addresses: clarity "`work_item_id:` three roles", completeness "story beneficiary", dependency "pending `pr:` ADR")
   Qualify each `work_item_id:` obligation by the surface it applies to; add a beneficiary sentence to Context; list the pending `pr:`-vocabulary ADR as a Related dependency. (Note: the `schema_version` omission on this file is intentional per Drafting Notes — no change needed.)

## Per-Lens Results

### Clarity

**Summary**: The work item is densely written but largely internally coherent, with a clear single intent (ship the meta/ corpus migration that closes epic 0057) and well-attributed actors via ADR/story references. The dominant clarity defect is a genuine contradiction in the base-field vocabulary: the schema field is named both `type` and `kind` across different sections, which a reader cannot reconcile without guessing. Secondary issues include an undefined band-classification vocabulary (`resolved`/`ambiguous`) and a few referents ("this story", "the transition") that depend on the reader tracking long cross-references.

**Strengths**:
- Actors and ownership are almost always explicitly named — each rename or cleanup obligation is attributed to a specific migration number, story, or ADR, so responsibility is rarely ambiguous.
- The Open Questions section explicitly states all three prior questions are resolved and points to where each resolution now lives.
- Implementation anchors give concrete file:line referents (e.g. `frontmatter.rs:334-341`) that disambiguate otherwise-vague phrases.

**Findings**:
- 🟡 **major** (high) — Base schema field named both `type` and `kind` — contradictory referents for the same discriminator (Requirements). The frontmatter uses `kind: story` and Requirements state the `type:`→`kind:` rename is owned by 0005, implying `kind` is canonical; yet the same Requirements list base fields as "(`type`, identity, …)" and notes get `type: note`. A reader cannot tell whether the unified base field is `type` or `kind`, nor whether notes and work-items use different discriminator names by design. The discriminator is the central pivot of the migration; guessing could produce a corpus that fails the unified-schema goal.
- 🔵 **minor** (medium) — Band-classification vocabulary (`resolved` / `ambiguous`) used before definition (Requirements). The migration logic hinges on classifying inferences into bands, but the work item never states the band set or classification rule, and the Summary uses the looser "confidently resolvable" for what may be the same concept.
- 🔵 **minor** (medium) — "this story" / "the 0065→0070 transition" referents require tracking long cross-references (Requirements). Timing claims central to removal-safety are stated as prose assertions rather than a single explicit precondition.
- 🔵 **minor** (medium) — `work_item_id:` referenced as both retained foreign key and removed transitional alias without local disambiguation (Technical Notes). The same token denotes 'keep', 'remove', and 'rename' across nearby sentences, inviting conflation.

### Completeness

**Summary**: An exceptionally complete story: all expected sections are present and densely populated, and it demonstrably resolves its prior open questions and folds them into requirements and criteria. Frontmatter is well-formed for kind story. The only notable gaps are a kind-appropriate concern — the story expresses no user/beneficiary need despite being typed as a story — and a minor frontmatter omission (schema_version absent, though intentionally explained).

**Strengths**:
- Every standard section is present and substantively populated — no empty, placeholder, or sparse sections.
- Open Questions explicitly resolved with each resolution traced into Requirements/Acceptance Criteria.
- Dependencies and Assumptions fully populated with rationale.
- Technical Notes and Implementation Anchors give concrete starting points.
- Frontmatter integrity is strong (kind, status, priority, parent, dates all present and well-formed).

**Findings**:
- 🔵 **minor** (medium) — Story kind lacks an identified user or system whose need is met (Summary / Context). The framing is purely about the corpus rewrite and closing epic 0057, not who benefits. Add a beneficiary (downstream visualiser-graph epic, userspace repos) or reclassify.
- 🔵 **minor** (high) — schema_version frontmatter field absent on this work item (Frontmatter: schema_version). The file mandates `schema_version` yet omits it and keeps legacy `work_item_id:`. The Drafting Notes explain this is deliberate ("converting it is precisely this migration's job"), so it is a knowingly-deferred omission, not an oversight.

### Dependency

**Summary**: Unusually well-dependency-mapped: every upstream producer story (0060-0069) is named and marked done, the downstream visualiser-graph epic is captured as a Blocks entry, and the coupled visualiser-server removal is explicitly anchored. The principal gaps are runtime ordering couplings stated in the body but absent from Dependencies — the hard reliance on migrations 0005 and 0006 having already run, and the cross-repo per-userspace coupling whereby removing fallbacks assumes every userspace repo has run `/accelerator:migrate`.

**Strengths**:
- All ten upstream producer stories (0060-0069) explicitly enumerated in 'Blocked by' with ADRs/migrations and confirmed done.
- The downstream consumer (visualiser-graph epic) captured as a Blocks entry.
- Coupled cross-component removals tied back to the stories that introduced them (0064, 0065).
- Ownership boundaries with migrations 0005 and 0006 spelled out, preventing duplicate rewrites.

**Findings**:
- 🟡 **major** (high) — Runtime ordering dependency on migrations 0005 and 0006 having run is not captured in Dependencies. The story assumes 0005/0006 have already run against the corpus before 0007 executes — a hard runtime ordering constraint distinct from the story-level `Blocked by` links (which only guarantee the migrations exist). If the runner doesn't enforce ordering, 0007 against an un-migrated repo would mis-transform or refuse.
- 🟡 **major** (medium) — Cross-repo per-userspace coupling for removing transitional fallbacks is asserted but not captured as a coordination dependency. Removal is justified by "every userspace repo will have run `/accelerator:migrate` at least once" — an external condition this story cannot verify. If any repo upgrades before migrating, the visualiser graph breaks with no grace.
- 🔵 **minor** (medium) — Pending supplementary ADR for the `pr:` reference vocabulary is an unnamed upstream dependency (Requirements). The `pr:`-tolerance carve-out depends on a pending ADR not listed in Dependencies/References, so the eventual reconciliation is invisible.
- 🔵 **minor** (medium) — Hand-off from 0067 (create-note) for notes backfill is captured as a blocker but the active coupling is understated (Requirements). The notes baseline schema this story emits must match `create-note`'s emitted `note` schema, but the contract is treated as settled while Drafting Notes flag the notes decisions as "this story's calls."

### Scope

**Summary**: The story has a strong unifying purpose: migrate the existing `meta/` corpus to the unified frontmatter schema. However, it self-declares as "XL" spanning "five distinct workstreams" — including a net-new linkage parser, interactive-migration authoring, an awk rewrite, a Rust visualiser-server removal across three files, and a notes backfill, plus a dogfood pass. Several of these are separable units with their own boundaries, and the story crosses the shell-migration / Rust-visualiser service boundary.

**Strengths**:
- Summary, Requirements, and Acceptance Criteria are tightly aligned around a single theme with a clear in/out-of-scope boundary (specs/ and global/ excluded).
- Scope boundaries against sibling migrations are sharply drawn (0005, 0006, specs/, global/ all disowned).
- As the named closing integration of epic 0057, the story has genuine standalone deliverable value.
- Cleanup obligations other stories conditioned on "0070 having run" are coherently consolidated here.

**Findings**:
- 🟡 **major** (high) — Story self-declares XL across five distinct workstreams (Technical Notes). Several workstreams (notes backfill, Rust removal) could each be delivered and rolled back independently. An XL five-workstream story is hard to plan, review, and verify; partial completion leaves it neither shippable nor cleanly revertable.
- 🟡 **major** (high) — Story crosses the shell-migration / Rust-visualiser service boundary (Requirements). The story removes transitional Rust code across three named files in a different component than the migration framework; the increment can't be owned end-to-end within one component's review/test cycle.
- 🔵 **minor** (medium) — Notes backfill is a self-contained sub-deliverable bundled into the corpus migration (Requirements). It has its own inference logic and hand-off owner (0067) and addresses hand-written notes rather than the structured-artifact corpus; it could be delivered independently.

### Testability

**Summary**: An unusually thorough Acceptance Criteria section that maps closely to the Requirements, with most criteria referencing concrete, observable outcomes. The main gaps are unbounded "every artifact" / "cleanly" language in the headline criterion, criteria whose pass condition depends on subjective inference-band classification without a defined oracle, and a missing criterion for the notes-backfill author resolution behaviour.

**Strengths**:
- Most criteria specify mechanically checkable outcomes (`producer:` rename, no `git_commit`/`branch`, `schema_version: 1`, named Rust test removal).
- The idempotency criterion is well-formed and verifiable with a defined detection key.
- The three spike-mandated parser fixes are enumerated explicitly.
- Acceptance Criteria align tightly with Requirements.

**Findings**:
- 🟡 **major** (high) — Headline criterion uses unbounded "every artifact" and undefined "cleanly" (Acceptance Criteria). No procedure distinguishes a clean apply from one with DIVERGE/REFUSE/MALFORMED diagnostics. Reframe to name the pass condition (exit 0, zero refusals, validates against a named validator, re-run no-op).
- 🟡 **major** (medium) — Inference-band classification criterion has no verification oracle (Acceptance Criteria). No way to verify a reference was banded correctly — no expected mapping, no threshold. The criterion could pass even if the parser systematically mis-bands.
- 🔵 **minor** (medium) — Notes-backfill author resolution lacks a verifiable expected outcome (Acceptance Criteria). "resolved `author`" collapses a two-branch behaviour into one word; split into VCS-history and fallback checks with the fallback value specified.
- 🔵 **minor** (medium) — "sensible defaults where missing" is not measurable (Requirements). No criterion enumerates the per-field defaults. Add a table of default values per base field when absent.
- 🔵 **minor** (medium) — 0065 dual-key fallback removal spans multiple sites but criterion checks only removal in aggregate (Acceptance Criteria). Removals are located across three named sites; partial removal could pass. Enumerate the sites or add a grep-anchored check.
- 🔵 **suggestion** (low) — "exercised" for parser fixes lacks a defined exercising input (Acceptance Criteria). For each fix, state the negative/positive case it must satisfy, turning each into an input/expected-output pair.

---

## Re-Review (Pass 2) — 2026-06-06T22:29:24+00:00

**Verdict:** REVISE

All seven major findings from pass 1 are resolved or downgraded. The verdict
remains REVISE only because two new (or escalated) major findings cross the
2-major threshold — both are narrow, sharpening fixes rather than structural
gaps, and the trajectory is strongly positive.

### Previously Identified Issues

- 🟡 **Clarity**: Base schema field named both `type` and `kind` — **Resolved**. The `type:` (ADR-0033 discriminator) vs `kind:` (work-item subtype) distinction is now explicitly stated and was cited as a strength.
- 🟡 **Dependency**: Runtime ordering dependency on migrations 0005/0006 — **Resolved**. Now a dedicated Dependencies bullet separating "migrations exist" from "migrations applied", with the runner's ordered ledger named as the enforcement mechanism; cited as a strength.
- 🟡 **Dependency**: Cross-repo per-userspace coupling — **Resolved** (capture). The coupling is now explicit in Dependencies. Residual: a new minor notes the precondition still has no verification mechanism (see below).
- 🟡 **Scope**: Story self-declares XL across five workstreams — **Resolved**. The indivisibility justification (shared dogfood gate, file-write overlap) was accepted as a sound coherence argument; downgraded to a low-confidence "consciously accept the size" suggestion.
- 🟡 **Scope**: Story crosses the shell-migration / Rust-visualiser boundary — **Partially resolved**. Downgraded to minor; the reviewer accepts the same-release ordering rationale but flags it for conscious acceptance and asks whether "same release" strictly requires "same story".
- 🟡 **Testability**: Headline criterion unbounded "every artifact" / "cleanly" — **Resolved**. AC1 now gives a runnable procedure (exit 0, zero REFUSE/MALFORMED, validator pass, re-run no-op); cited as a strength.
- 🟡 **Testability**: Inference-band classification had no oracle — **Partially resolved**. The fixture-set half is resolved and cited as a strength; the dogfood ≤5% wrong-rate spot-check still lacks a defined sampling/adjudication procedure (escalated as a new major, below).
- 🔵 **Clarity**: Band vocabulary used before definition — **Resolved** (defined inline; cited as a strength).
- 🔵 **Clarity**: "0065→0070 transition" referent — **Partially resolved**. The gating precondition is now in Dependencies, but a new minor flags the "in the same release that closes out 0070" phrasing as circular (this story *is* 0070).
- 🔵 **Clarity**: `work_item_id:` three roles — **Still present (escalated)**. Now flagged major: the token spans four distinct referents and the added qualifier covers only the alias-removal bullet, not every mention (see below).
- 🔵 **Completeness**: Story lacks a beneficiary — **Resolved** (beneficiaries named in Summary; cited as a strength).
- 🔵 **Completeness**: `schema_version` absent on this file — **Resolved/accepted** (not re-flagged; intentional deferral stands).
- 🔵 **Dependency**: Pending `pr:` ADR unnamed — **Partially resolved**. Now captured as a Pending bullet; residual minor: it carries no tracking reference/number.
- 🔵 **Dependency**: 0067 notes-schema hand-off understated — **Resolved** (schema-conformance coupling now explicit; cited as a strength).
- 🔵 **Scope**: Notes backfill bundled — **Resolved** (absorbed into the accepted indivisibility justification).
- 🔵 **Testability**: Notes-author resolution not verifiable — **Resolved** (AC14 now verifies both branches; cited as a strength).
- 🔵 **Testability**: "Sensible defaults" not measurable — **Partially resolved**. Defaults are now enumerated; residual minor: the "no ad-hoc default" clause is a negative absolute that is hard to confirm.
- 🔵 **Testability**: 0065 dual-key removal per-site — **Resolved** (AC now names all three sites with a grep oracle; cited as a strength).
- 🔵 **Testability**: Parser fixes "exercised" — **Resolved** (each fix now paired with an input/expected-output fixture; cited as a strength).

### New Issues Introduced

#### Major
- 🟡 **Testability**: Wrong-rate spot-check lacks a defined sampling and adjudication procedure (Acceptance Criteria) — AC8's "spot-checked … ≤5%" gives no sample size, stratification, or "wrong" adjudication rule; spike 0068 used a stratified n=150. Two verifiers could reach different verdicts on the same corpus.
- 🟡 **Clarity**: `work_item_id:` overloaded across four distinct referents (Requirements) — own-identity field, retained foreign reference, removed review-template alias, and 0065 read-path key. The added qualifier covers only the alias bullet; the reviewer asks for distinguishing phrasing on every mention.

#### Minor
- 🔵 **Clarity**: "in the same release that closes out 0070" is circular (this story is 0070); rephrase to "within this story, in the same release as the corpus migration".
- 🔵 **Clarity**: The three body-section bullets (parse / classify-bands / disambiguate-prose) overlap; add a one-line framing naming the ordered pipeline stages.
- 🔵 **Clarity**: Migration number is "verify, don't hard-code" yet later sections use "0007" concretely without re-flagging it as provisional.
- 🔵 **Clarity**: AC8 "spot-checked" uses passive voice leaving the verifying actor/method implicit.
- 🔵 **Dependency**: Cross-repo migrate precondition has no named verification/gate before the fallback-removal release.
- 🔵 **Dependency**: Pending `pr:` ADR is named but carries no tracking work-item/ADR reference.
- 🔵 **Testability**: AC1's "DIVERGE … investigated and explicitly accepted" rests on unbounded human judgement with no recorded artefact.
- 🔵 **Testability**: AC7's "no ad-hoc or undocumented default" is a negative absolute; reframe as a bounded positive sample check.
- 🔵 **Testability**: AC15's idempotent re-run does not name the observable interactive-skip signal (zero new prompts, zero new session-log records).
- 🔵 **Testability**: AC7's "only the canonical bidirectional side is written" has no check confirming reciprocal keys (`blocked_by` / `superseded_by`) are absent.

#### Suggestions
- 🔵 **Completeness**: Clarify whether the VCS-history→`Unknown` author rule applies corpus-wide or only to notes.
- 🔵 **Completeness**: Per-artifact extras are deferred wholesale to ADR-0033/0057 without an inline pointer to the extras table.
- 🔵 **Dependency**: The downstream visualiser-graph epic in Blocks is unnumbered.
- 🔵 **Scope**: XL size noted for conscious planning acceptance (no decomposition recommended).

### Assessment

The revision is a clear success: every one of the seven pass-1 majors is resolved or downgraded, and several became cited strengths. The verdict stays REVISE purely on the 2-major threshold, but both remaining majors are narrow and quick to close — (1) pin AC8's wrong-rate verification to spike 0068's stratified sampling procedure, and (2) apply distinguishing `work_item_id:` phrasing on every mention rather than only the alias bullet. Addressing those two (plus, optionally, the cheap clarity/testability minors) would clear the path to APPROVE. The work item is close to implementation-ready.

---

## Re-Review (Pass 3) — 2026-06-06T22:45:30+00:00

**Verdict:** REVISE

Focused verification of the two lenses that carried the pass-2 majors (clarity,
testability). Both targeted majors are **confirmed closed and are now cited
strengths**. The deeper pass surfaced three further majors (one fixed in-pass);
the other three lenses (completeness, dependency, scope) carried no majors in
pass 2 and were untouched by the pass-2/pass-3 edits, so they contribute no
majors to the verdict.

### Previously Identified Issues

- 🟡 **Testability**: Wrong-rate spot-check lacked a sampling/adjudication procedure — **Resolved**. AC8 now reproduces spike 0068's procedure (stratified sample ≥150 resolved-band linkages across the five header types, correct/wrong vs source prose, ≤5%); cited as a strength.
- 🟡 **Clarity**: `work_item_id:` overloaded across four referents — **Resolved**. The own-identity / foreign / review-template-alias qualifiers are now applied on every mention; cited as strength #1.

### New Issues Introduced

#### Major
- 🟡 **Clarity**: Body-section header set differed between Requirements (three headers, incl. `## Related Documents`), Acceptance Criteria, and Implementation anchors (five de-facto headers; `## Related Documents` never appears) — a pre-existing contradiction sharpened by the pass-2 AC8 edit. **Fixed in this pass**: the Requirements parsing bullet now enumerates the authoritative five headers.
- 🟡 **Testability**: AC1's DIVERGE "explicitly accepted" branch has no defined verification artefact — no stated record, owner, or threshold, so an accepted DIVERGE can't be distinguished from one that slipped through. (Escalated from a pass-2 minor.)
- 🟡 **Testability**: Ambiguous-band hook routing has no pass/fail success condition — AC7/AC8 require ambiguous references to be logged but never state the expected post-session terminal state (e.g. every routed reference reaches APPLIED_CONFIRM; a known-ambiguous fixture produces the expected linkage). The ambiguous band is the larger risk per spike 0068's 11.3%, yet only the resolved band has a measured gate.

#### Minor
- 🔵 **Clarity**: The migration state-file path is framed as both an open "reconcile at implementation time" task (Technical Notes) and already-resolved (Implementation anchors); state the resolution once.
- 🔵 **Clarity**: "this migration" (awk artifact) vs "this story" (full deliverable incl. Rust removal + dogfood) are used near-interchangeably; add a one-line scope definition.
- 🔵 **Testability**: AC2 asserts an upstream precondition (0005/0006 already ran) rather than an outcome 0070 produces; reframe as a negative behavioural assertion (migration leaves pre-set fields byte-identical) or move to Dependencies.
- 🔵 **Testability**: AC5 doesn't name the code-state-anchored artifact set inline (it lives only in Requirements); inline it or cross-reference.
- 🔵 **Testability**: AC9 (omit-when-empty) states no concrete check; add a corpus grep for empty optional/typed-linkage values excepting `tags: []`.

### Assessment

The two majors targeted this round are definitively closed and became strengths,
confirming the revision did what it set out to do. The verdict stays REVISE only
because deeper inspection surfaced new AC-precision gaps — none structural. After
the in-pass header fix, two testability majors remain: define the DIVERGE
acceptance artefact, and add a success condition for the ambiguous-band hook
routing. Both are localised Acceptance-Criteria additions. The work item's
structure, scope, dependencies, and core requirements are sound and stable across
three passes; remaining work is verification-precision polish, and the review is
in clear diminishing-returns territory.

---

## Re-Review (Pass 4) — 2026-06-06T22:55:08+00:00

**Verdict:** COMMENT

Focused confirmation that the two pass-3 testability majors are closed (plus the
in-pass-3 clarity header fix). No major findings remain on any lens, so the
verdict moves from REVISE to **COMMENT**: the work item is acceptable for
implementation, with only minor verification-precision polish outstanding.

### Previously Identified Issues

- 🟡 **Clarity**: Body-section header set inconsistency — **Resolved** (pass 3): the Requirements parsing bullet now enumerates the authoritative five de-facto headers, consistent with the Acceptance Criteria and Implementation anchors.
- 🟡 **Testability**: DIVERGE "explicitly accepted" had no verification artefact — **Resolved**. AC1 now requires each remaining DIVERGE be recorded in the dogfood gap-fix log with a rationale and zero un-annotated DIVERGE lines; cited as a strength.
- 🟡 **Testability**: Ambiguous-band hook routing had no pass/fail success condition — **Resolved**. A new criterion requires every session-log reference to reach `APPLIED_CONFIRM` and a known-ambiguous fixture to produce the expected linkage after a scripted hook decision; cited as a strength.

### New Issues Introduced

#### Minor
- 🔵 **Testability**: AC2 still asserts an upstream precondition (0005/0006 ran) rather than a post-condition 0070 produces; reframe as "after 0070 runs, these fields are present and unchanged on every applicable file".
- 🔵 **Testability**: AC1's "validates against the unified-schema validator" does not name the validator command or its pass condition (exit 0 / zero error-level diagnostics).
- 🔵 **Testability**: Base-field default seeding leaves two boundary cases undocumented — the default when a source artifact lacks `date` entirely, and `last_updated_by` when `author` itself fell back to `Unknown`.
- 🔵 **Testability**: No coverage check that every reference-bearing line in the five header sections was either emitted as resolved linkage or routed to the hook (a silently-dropped reference would pass AC8/AC9); consider a reconciliation check with zero unclassified-and-dropped lines.

(Also still open from pass 3, not re-surfaced this pass: clarity's state-file-path framing and the "this migration" vs "this story" scope note; dependency's cross-repo verification gate, untracked `pr:` ADR, and unnumbered visualiser-graph epic. All minor/suggestion.)

### Assessment

The review has converged. Every major finding across all four passes is now
resolved — the seven structural/clarity/dependency/scope/testability majors from
pass 1, and the clarity-header and two testability majors from pass 3. The work
item is **acceptable for implementation (COMMENT)**: its structure, scope,
dependencies, and core requirements have been stable and sound throughout, and
what remains is a handful of minor Acceptance-Criteria precision refinements that
an implementer can reasonably resolve during planning. No further review pass is
recommended — the artifact is in clear diminishing-returns territory.

---

## Verdict Decision — 2026-06-06T23:20:24+00:00

**Verdict: APPROVE** (reviewer decision, Toby Clemson)

The reviewer accepts the work item for implementation. All majors across four
passes are resolved; the residual minor Acceptance-Criteria precision items are
accepted as planning-time refinements and do not block approval. The work item
status is transitioned to `ready` alongside this decision.

---
*Review generated by /accelerator:review-work-item*
