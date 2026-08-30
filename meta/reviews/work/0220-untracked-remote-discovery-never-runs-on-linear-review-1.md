---
type: work-item-review
id: "0220-untracked-remote-discovery-never-runs-on-linear-review-1"
title: "Work Item Review: Untracked-Remote Discovery Never Runs on Linear"
date: "2026-08-30T14:50:49+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
parent: "work-item:0146"
target: "work-item:0220"
work_item_id: "0220"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-08-30T16:50:38+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Untracked-Remote Discovery Never Runs on Linear

**Verdict:** REVISE

This is a strong, tightly-argued bug work item — complete in structure, well-scoped to a single defect with clean edges against parent epic 0146, and highly testable with code-anchored referents and Given/When/Then criteria. Two major findings hold it back from approval: an internal-consistency tension between the Context (Linear search is "already bounded") and the Fix/AC requirement to hard-fail when the key is unset, and a real ordering coupling to the sibling config-model rename that the Dependencies section records as "none". Both are documentation-consistency fixes, not scope or design problems.

### Cross-Cutting Themes

- **The `default_project_code` / key / team concept is under-specified across sections** (flagged by: clarity, dependency, testability) — the same value is called `work.default_project_code`, `scope.project`, a "key", and a "team key", carries a future rename to `work.key`, and must resolve to a team UUID. Clarity flags the overloaded naming and the unnamed field in Assumptions; dependency flags the missing coupling to the sibling that renames it; testability flags "correctly bounded" restating intent instead of naming the resolved-UUID observable. Sharpening this one concept resolves findings in three lenses.
- **Self-bounding semantics need stating up front** (flagged by: clarity, testability) — the Context frames Linear's search as inherently safe (credentialed-team fallback), which reads as licensing key-optional discovery, yet the Fix and AC3 mandate a config error when the key is unset. The reconciliation ("credentialed team is access control, not scope authority") lives only in Assumptions, after the apparent contradiction.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Clarity**: Context "already bounded" contradicts Fix/AC "require the key"
  **Location**: Context / Requirements (Fix shape) / Acceptance Criteria
  The Context argues Linear discovery is inherently safe because the client falls back to the credentialed team and "was already bounded", implying discovery can run with no configured key — yet Fix shape #2 and AC3 require a config error when the key is unset. The reconciliation (credentialed team is access control, not scope authority) appears only later, in Assumptions.

- 🟡 **Dependency**: Sibling config-model rename coupling absent from Dependencies
  **Location**: Dependencies
  Requirements and Drafting Notes describe a sibling config-model child under 0146 that renames `work.default_project_code` → `work.key`, the exact field this fix reads, yet Dependencies records "Blocked by: none. Blocks: none." Whichever item ships second must adapt, but the ordering coupling is invisible to anyone scheduling the two.

#### Minor

- 🔵 **Clarity**: "project" / "key" / "team" used near-interchangeably for one value
  **Location**: Requirements (Fix shape) / Technical Notes
  The config field is `work.default_project_code`, the runtime field `scope.project`, but on Linear the value is a team key resolving to a team UUID. The reader must hold three names for one concept to follow the Fix shape — the very conflation the bug arises from.

- 🔵 **Dependency**: `relates_to` links (0194, 0204) uncharacterised
  **Location**: Dependencies
  Frontmatter and References list `relates_to: 0194, 0204`, but neither Dependencies nor the body states whether either is an upstream prerequisite, a downstream consumer, or merely adjacent — so a related item that is actually an ordering constraint would be missed at planning time.

- 🔵 **Testability**: "never enumerates the wider workspace" lacks a defined observable
  **Location**: Acceptance Criteria
  AC2's negative ("never enumerates the wider workspace") has no stated observable. A tester could confirm the keyed team's issues appear yet be unable to prove the search did not touch the wider workspace.

- 🔵 **Testability**: "search is correctly bounded" uses a subjective qualifier
  **Location**: Acceptance Criteria
  AC4 ends with "the search is correctly bounded", where "correctly" has no defined threshold — it restates intent rather than a checkable outcome.

#### Suggestions

- 🔵 **Scope**: Cross-tracker observability fix folded into a Linear-specific bug
  **Location**: Acceptance Criteria
  AC6 (skipped runs must be distinguishable from empty searches) is a tracker-agnostic observability improvement governing Jira too, while the rest of the item is Linear-specific. The Drafting Notes justify folding it in; no change required if the team accepts that.

- 🔵 **Clarity**: Assumptions cites "the field's doc comment" without naming the field
  **Location**: Assumptions
  The first Assumption references a doc comment without naming `work.default_project_code`, forcing the reader to infer the referent among several fields in play.

- 🔵 **Dependency**: Catalogue key→UUID mapping is an unnamed data prerequisite
  **Location**: Requirements
  The fix depends on `catalogue.json` carrying the team key→UUID mapping. If absent or stale, resolution silently yields no team and reproduces zero-pulls for a third reason — worth naming as a runtime precondition.

- 🔵 **Testability**: First criterion relies on an unstated precondition
  **Location**: Acceptance Criteria
  AC1 depends on an untracked remote issue existing (stated only in Reproduction). Read standalone it could pass vacuously against a team with no untracked issues.

### Strengths

- ✅ Complete bug specification: reproduction gives config keys, precondition, exact command (`accelerator work sync --preview`), and distinct Expected/Actual outcomes, plus a dated environment observation.
- ✅ Frontmatter fully populated with recognised values (`kind: bug`, `status: draft`, `priority: medium`) and parent/relates_to/external_id linkage.
- ✅ Well-scoped: the two "coupled changes" are demonstrated interdependent (the Compounding trap shows the gate fix fails silently without the key→UUID resolution), and the broader config redesign is explicitly deferred to 0146.
- ✅ Acceptance criteria in Given/When/Then form with named trackers and observable outcomes, including an explicit regression-test criterion with a falsifiability requirement (must fail against the current gate).
- ✅ The Compounding trap section pre-empts the plausible misreading that passing the raw key would work, and explains exactly why it fails.
- ✅ Actors and triggers named actively ("the gate reads", "the JQL builder refuses", "the client falls back") rather than hidden behind passive constructions.

### Recommended Changes

1. **State the credentialed-team-is-access-control point up front in Context** (addresses: Context "already bounded" contradicts Fix/AC; Self-bounding semantics theme)
   Add a sentence in Context clarifying the credentialed-team fallback is access control, not the intended scope authority, so the later "require the key" rule reads as consistent rather than contradictory. This pulls the Assumptions reconciliation forward to where the tension arises.

2. **Record the config-model rename coupling in Dependencies** (addresses: Sibling config-model rename coupling absent)
   Replace "Blocked by: none. Blocks: none." with a note naming the config-model sibling under 0146, stating 0220 deliberately targets the current `work.default_project_code` field and the rename item must reconcile this consumer. Characterise 0194 and 0204 while there (e.g. "related, no blocking relationship").

3. **Add a one-line glossary for the key concept** (addresses: overloaded project/key/team naming; Assumptions unnamed field; key concept theme)
   Early in Requirements or Technical Notes, define once: "key" denotes the value in `work.default_project_code` / `scope.project` — a team key on Linear, a project key on Jira. Name the field explicitly in the first Assumption too.

4. **Anchor the two vague acceptance criteria to concrete observables** (addresses: "never enumerates" lacks observable; "correctly bounded" subjective; first criterion vacuous precondition)
   AC2: assert the emitted Linear filter carries `{team:{id:{eq:<UUID>}}}` for the resolved team UUID. AC4: assert the resolved UUID (not the raw key `PP`) appears in the filter. AC1: fold "at least one team issue with no local work item" into the Given so it cannot pass vacuously.

5. **Optionally record the catalogue mapping as a runtime prerequisite** (addresses: catalogue key→UUID data prerequisite)
   Note in Dependencies or Assumptions that the catalogue must carry the team key→UUID mapping, and confirm behaviour when the key is present in config but absent from the catalogue.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: An unusually clear, tightly-argued bug work item: the defect, its two independent causes, and the fix shape are all expressed with named actors and code-anchored referents, and the Acceptance Criteria use explicit given/when/then form. The one genuine clarity risk is an internal-consistency tension between the Context's claim that the Linear search is "already bounded" and the Fix/AC requirement to hard-fail when the key is unset — reconciled only late, in Assumptions. Overloaded "project"/"key" terminology adds minor friction but is largely explained in place.

**Strengths**:
- Acceptance Criteria use explicit given/when/then form with named trackers and observable outcomes, leaving little room for interpretation.
- The "Compounding trap" section pre-empts a plausible misreading (that passing the raw key would work) and explains exactly why it fails.
- Actors and triggers are consistently named — "the gate reads", "the JQL builder refuses", "the client falls back" — rather than hidden behind passive constructions.

**Findings**:
- 🟡 **major** (confidence: medium) — _Context / Requirements (Fix shape) / Acceptance Criteria_. The Context argues the Linear search is inherently safe because it "falls back to the credentialed team" and "was already bounded; the gate suppressed one that was never at risk of flooding" — implying discovery can safely run even with no configured key. But Fix shape #2 and AC3 require the run to raise a config error when the key is unset. A reader reaching the Fix section can reasonably ask: if the client already bounds itself to the credentialed team, why is the key mandatory? The reconciliation exists only in Assumptions ("the configured key is the scope authority; the credentialed team provides … access control, not the scope"), which appears after the apparent contradiction. Impact: an implementer could read Context as licensing key-optional discovery and Fix/AC as mandating key-required discovery. Suggestion: state up front in Context that the credentialed-team fallback is access control rather than the intended scope authority.
- 🔵 **minor** (confidence: medium) — _Requirements (Fix shape) / Technical Notes_. The terms "project", "key", and "team" are used near-interchangeably around one value: the config field is `work.default_project_code`, the runtime field is `scope.project`, yet on Linear this holds a team key (e.g. `PP`) that must resolve to a team UUID. The reader must hold three names for one concept to follow the Fix shape and Compounding trap. Impact: raises the chance an implementer conflates a Jira project scope with a Linear team scope — precisely the confusion the bug arises from. Suggestion: add a one-line glossary note early.
- 🔵 **suggestion** (confidence: medium) — _Assumptions_. The first Assumption cites "the field's doc comment" without naming which field, forcing the reader to infer it is `work.default_project_code`. Suggestion: name the field explicitly.

### Completeness

**Summary**: Exemplary from a completeness standpoint: every expected section is present and substantively populated, and the frontmatter is complete with a recognised kind, status, and priority. As a bug it carries the full reproduction unit — configuration input, action, expected outcome, and actual outcome — plus environment observation, seven specific acceptance criteria, and populated Context, Dependencies, and Assumptions. No completeness gaps rise to a reportable finding.

**Strengths**:
- The bug's reproduction is complete as a single logical unit: numbered setup/input, the concrete action (`accelerator work sync --preview`), an explicit Expected outcome, and an explicit Actual outcome, plus a dated environment observation.
- Frontmatter is fully populated with recognised values — `kind: bug`, `status: draft`, `priority: medium`, plus parent/relates_to linkage and external_id.
- The Summary is a single, unambiguous statement of the defect, and the Context explains the underlying forces (Jira-derived gate semantics vs. Linear's self-bounding search).
- Acceptance Criteria are numerous and specific, covering the Linear happy path, boundedness, the config-error path, key→UUID resolution, Jira non-regression, observability of skipped discovery, and a regression test.
- Optional sections (Dependencies, Assumptions) are populated rather than left blank, and Drafting Notes justifies the absence of an Open Questions section (the two former questions were resolved and moved to the parent).

**Findings**:
_None._

### Dependency

**Summary**: The work item captures its parent epic (0146) and deliberately decouples itself from the config-model rename by pinning to the current `work.default_project_code` field, which is good dependency hygiene. However, the Dependencies section is marked entirely "none" despite the body describing a sibling work item that renames the very field this fix relies on — an ordering/coupling relationship that is not recorded. The `relates_to` links (0194, 0204) are also left uncharacterised, and the internal catalogue key→UUID resolution is a real data prerequisite worth surfacing.

**Strengths**:
- Explicitly decouples from the config-model rename by pinning to the current `work.default_project_code` field, avoiding a hard blocker on the sibling item.
- Parent epic (0146) is captured in both frontmatter and References, and the Drafting Notes explain the reparenting from 0171.
- The "Compounding trap" section makes the internal key→UUID resolution dependency explicit, preventing a silent second-order failure.
- Acceptance Criteria pins Jira's behaviour as unchanged, protecting the other tracker's contract from regression.

**Findings**:
- 🟡 **major** (confidence: medium) — _Dependencies_. The Requirements and Drafting Notes describe a sibling configuration-model child under 0146 that renames `work.default_project_code` → `work.key` and introduces layered `<tracker>.<entity>_key` ownership — the exact field this fix reads. Yet the Dependencies section records "Blocked by: none. Blocks: none.", so this bidirectional ordering coupling is invisible to anyone scheduling the two items. Impact: if the rename lands first, 0220's implementation goes stale mid-flight; if 0220 lands first, the rename must account for this new consumer — neither team sees the coupling. Suggestion: add an ordering note in Dependencies naming the config-model sibling under 0146.
- 🔵 **minor** (confidence: medium) — _Dependencies_. The frontmatter and References list `relates_to: 0194, 0204`, but the Dependencies section and body never state the nature or direction of these relationships. Impact: a related item that is actually an ordering constraint or waiting consumer would be missed at planning time. Suggestion: briefly characterise 0194 and 0204.
- 🔵 **suggestion** (confidence: low) — _Requirements_. The fix depends on `catalogue.json` containing a team key→UUID mapping for the configured key; the AC assumes "a populated catalogue" but Dependencies does not name catalogue population/staleness as a precondition. Impact: if the catalogue lacks the team key or is stale, resolution silently yields no team and reproduces the zero-pulls symptom for a third reason. Suggestion: note the catalogue mapping as an explicit runtime prerequisite and confirm behaviour when the key is in config but absent from the catalogue.

### Scope

**Summary**: A well-scoped, coherent bug fix: a single defect (untracked-remote discovery suppressed on Linear) with clearly delineated boundaries. The two "coupled changes" in the Fix shape are genuinely interdependent rather than independently deliverable — the Compounding trap section explicitly justifies why the gate change and the key→UUID resolution must ship together. The broader scope/config redesign is deliberately and explicitly deferred to the parent epic 0146, giving this item clean edges as "its first, shippable child".

**Strengths**:
- The Summary, Requirements, and Acceptance Criteria all describe the same scope — one tracker-aware discovery defect — with no drift between sections.
- The two changes in the Fix shape are demonstrated to be coupled, not bundled: the Compounding trap shows the gate fix fails silently without the key→UUID resolution.
- Adjacent scope (the config field rename, per-tracker pull scope, filter schema) is explicitly excluded and attributed to parent epic 0146.
- Kind selection (bug vs task) is deliberately reasoned in Drafting Notes, and the item is sized appropriately for a bug.

**Findings**:
- 🔵 **suggestion** (confidence: medium) — _Acceptance Criteria_. AC6 ("Given any run where discovery does not execute, then the report states it was skipped and why") is a tracker-agnostic observability improvement that also governs Jira runs, whereas the rest of the item is a Linear-specific discovery defect. It is a candidate for a separately deliverable concern, though the Drafting Notes explicitly justify folding it in on the grounds that the silent skip is what kept the defect invisible. Impact: low. Suggestion: no change required if the team accepts the justification; if a narrower slice is preferred, extract the generic skip-reporting behaviour as a sibling under 0146.

### Testability

**Summary**: Strongly testable: the reproduction fully specifies configuration, action, expected, and actual outcomes, and six of seven acceptance criteria are framed as Given/When/Then with observable results, including a dedicated regression-test criterion and an observability criterion that closes the silent-skip gap. The main gaps are a couple of criteria that assert negatives or use the word "correctly" without naming the concrete observable a verifier would inspect. No criterion is tautological or unbounded in a way that would prevent a definitive pass/fail.

**Strengths**:
- The Reproduction block gives a complete, executable trigger (config keys, precondition of an unpulled remote issue, and the exact command) with distinct Expected and Actual outcomes.
- Acceptance Criteria are expressed as observable behaviours (Given/When/Then) rather than implementation instructions.
- The Jira-unchanged criterion pins down the negative-space behaviour so a verifier can confirm no regression on the sibling tracker.
- A regression-test criterion is explicit and includes the falsifiability requirement that it must fail against the current gate.
- The observability criterion turns the Summary's core complaint into a verifiable report-output check.

**Findings**:
- 🔵 **minor** (confidence: medium) — _Acceptance Criteria_. AC2 asserts discovery is "bounded to the keyed team and never enumerates the wider workspace", but a negative like "never enumerates" has no stated observable a verifier can inspect. Impact: a tester could confirm the keyed team's issues appear yet be unable to prove the search did not touch the wider workspace. Suggestion: anchor it to a concrete observable, e.g. the emitted Linear search filter carries `{team:{id:{eq:…}}}` for the resolved team UUID.
- 🔵 **minor** (confidence: medium) — _Acceptance Criteria_. The key→UUID criterion (AC4) ends with "the search is correctly bounded", where "correctly" is subjective with no defined threshold. Impact: two verifiers could disagree on whether bounding is "correct". Suggestion: replace with the concrete observable — the resolved team UUID (not the raw key `PP`) appears in the search filter, and untracked issues from that team are returned.
- 🔵 **suggestion** (confidence: low) — _Acceptance Criteria_. AC1 ("untracked remote issues … are discovered and reported") depends on at least one untracked remote issue existing, stated only in the Reproduction block. Impact: read standalone, the criterion could pass vacuously against a team with no untracked issues. Suggestion: fold the precondition into the Given.

## Re-Review (Pass 2) — 2026-08-30

**Verdict:** COMMENT

The five recommended changes were applied to the work item. Both pass-1 majors are resolved, along with all four minors and three of the four suggestions; the scope suggestion (AC6) was accepted as folded-in and left unchanged. Re-review across clarity, dependency, scope, and testability surfaces one new major and a handful of minors/suggestions — most as a direct, foreseeable consequence of the edits (the Dependencies note added a third failure mode and named a sibling descriptively). The item is now acceptable; the new major is a one-line acceptance-criterion gap worth closing.

### Previously Identified Issues

- 🟡 **Clarity** (major): Context "already bounded" contradicts Fix/AC "require the key" — **Resolved**. The Context now states the credentialed-team fallback is access control, not the scope authority, before the Fix's require-the-key rule; clarity re-review flags no consistency tension.
- 🟡 **Dependency** (major): Sibling config-model rename coupling absent from Dependencies — **Resolved**. The ordering coupling is now documented with the ship-order reconciliation rule and praised as unusually well-mapped.
- 🔵 **Clarity** (minor): "project"/"key"/"team" used near-interchangeably — **Resolved**. The Fix shape now defines "key" once; the glossary is called out as a strength.
- 🔵 **Dependency** (minor): `relates_to` links (0194, 0204) uncharacterised — **Resolved**. Both are now characterised as thematically adjacent, no blocking relationship.
- 🔵 **Testability** (minor): "never enumerates the wider workspace" lacks an observable — **Resolved**. AC2 now asserts the `{team:{id:{eq:…}}}` filter for the resolved UUID.
- 🔵 **Testability** (minor): "search is correctly bounded" subjective — **Resolved**. AC4 now asserts the resolved UUID (not the raw key) appears in the filter.
- 🔵 **Clarity** (suggestion): Assumptions cites "the field's doc comment" without naming the field — **Resolved**. Named as `work.default_project_code`.
- 🔵 **Dependency** (suggestion): Catalogue key→UUID mapping is an unnamed prerequisite — **Resolved**. Now a runtime-prerequisite line in Dependencies, including the stale/absent failure mode.
- 🔵 **Testability** (suggestion): AC1 relies on an unstated precondition — **Resolved**. Precondition folded into AC1's Given.
- 🔵 **Scope** (suggestion): Cross-tracker observability (AC6) folded into a Linear-specific bug — **Still present** (accepted). Re-review reaffirms it is defensible as folded-in; no split required.

### New Issues Introduced

- 🟡 **Testability** (major): Catalogue-miss failure mode has no acceptance criterion. The Dependencies edit added a third silent-zero-pulls case (key present in config but absent/stale in `catalogue.json`, which the fix "must surface as an error"), but AC3 covers only the *unset* key and AC4 only a *resolvable* key. A verifier could pass all criteria while the catalogue-miss case still returns zero pulls silently. Suggested AC: "Given a configured Linear key that resolves to no team in the catalogue (absent or stale), when a sync runs, then the run reports a resolution error naming the unresolved key, rather than returning zero pulls silently."
- 🔵 **Testability + Clarity** (minor, both lenses): Reproduction omits the sync direction the bug depends on. Step 3 runs `accelerator work sync --preview`, but the gate is direction-sensitive (`!matches!(request.direction, SyncDirection::PushOnly)`) and the ACs speak of a *bidirectional* sync; `--preview` is a mode, not a direction. State the direction explicitly.
- 🔵 **Dependency** (minor) / **Clarity** (suggestion): The sibling config-model child is named descriptively three times but never given a work-item ID. Cite its ID (and add to `relates_to`), or state it is not yet created so the coupling is a tracked follow-up.
- 🔵 **Dependency** (suggestion, low): Linear credential/API access is implied by the reproduction but not captured as a coupling. State whether tests run against mocked Linear responses, or add credential/API access as a test-environment prerequisite.
- 🔵 **Testability** (suggestion, low): AC5's "discovery behaviour is unchanged" is broad; drop "unchanged" and keep only the enumerated checkable outcomes (project required, unbounded search refused).
- 🔵 **Clarity** (suggestion, low): JQL used without expansion on first use — safe to leave if Jira fluency is assumed.

### Assessment

The work item is ready for implementation as-is (COMMENT). Pass 2 has one major and no criticals — below the REVISE threshold. The single major is a genuine one-line gap: the catalogue-miss case is now documented as a requirement in Dependencies but is not pinned by an acceptance criterion, so closing it (add the unresolvable-key AC) is recommended before planning. The sync-direction and sibling-ID minors are cheap, high-value clarifications; the remaining suggestions are optional polish.

### Verdict Update — 2026-08-30

**Verdict:** APPROVE (reviewer override of the pass-2 COMMENT)

All pass-2 findings were subsequently addressed in the work item: the catalogue-miss acceptance criterion was added, the reproduction states the bidirectional direction, the sibling config-model item is flagged as living in parent epic 0146, the Linear credential coupling is captured as a mocked-test prerequisite, AC5 was reworded to enumerated outcomes, and JQL is expanded on first use. With no outstanding findings, the reviewer approves the work item for planning.

---
*Re-review generated by /accelerator:review-work-item*
