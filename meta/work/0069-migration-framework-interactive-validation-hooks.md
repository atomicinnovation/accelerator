---
id: "0069"
title: "Extend Migration Framework with Interactive Validation Hooks"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
kind: story
status: done
priority: medium
parent: "work-item:0057"
tags: [migration, framework, accelerator-plugin]
type: work-item
schema_version: 1
last_updated: "2026-05-17T17:16:35+00:00"
last_updated_by: Toby Clemson
blocked_by: ["work-item:0092", "work-item:0037", "work-item:0062", "work-item:0038", "adr:ADR-0037", "adr:ADR-0038"]
blocks: ["work-item:0070", "work-item:0038", "adr:ADR-0038"]
relates_to: ["adr:ADR-0037", "adr:ADR-0031", "work-item:0057", "work-item:0068", "work-item:0023", "work-item:0037", "work-item:0031", "adr:ADR-0038", "adr:ADR-0023"]
external_id: PP-91
---

# 0069: Extend Migration Framework with Interactive Validation Hooks

**Kind**: Story
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Implement the opt-in interactive contract defined by ADR-0037 (supplement to ADR-0023) in the Accelerator meta-directory migration runner (`skills/config/migrate/scripts/run-migrations.sh`), so migration authors can declare a hook that surfaces selected transformations to the user. The runner evaluates whatever trigger predicate the migration declares (operating over migration-declared fields, including a confidence-valued field) and routes matching transformations to the prompt; the meaning of the predicate's fields is the migration's concern, parameterised per ADR-0038 for the unified-schema migration. Migrations that do not declare the hook take the existing mechanical-default path unchanged. The first consumer is work item 0070 (the unified-schema corpus migration) via ADR-0038's parameterisation.

## Context

The migration framework today is purely mechanical per ADR-0023: no prompts, no dry-run, VCS-as-rollback. Spike 0068 measured the unified-schema migration's body-section inference at 11.3% wrong-rate (and ~5.3% under the spike's "cheap-fix counterfactual" — the residual error rate if the three highest-leverage parser fixes are applied), both above the 5% deterministic threshold the corpus is willing to accept.

ADR-0037 (produced by work item 0092) supplements ADR-0023 with a broad, opt-in interactive contract any migration may adopt without re-amending ADR-0023. ADR-0038 (produced by work item 0062) parameterises that contract for the unified-schema migration. This story implements ADR-0037's contract in the runner so ADR-0038's parameterisation — and any future migration that opts in — can run against it. The three-layer chain is: ADR-0037 (framework contract) → per-migration ADRs (e.g. ADR-0038) → this story (runner implementation).

ADR-0023 itself is not edited (corpus accepted-ADR immutability per ADR-0031); the supplement-form precedent set by ADR-0033 and ADR-0035 is the model.

## Requirements

- Implement the **trigger predicate** plumbing of ADR-0037 §1 in the runner: migrations declare a boolean predicate over a named field set including a confidence-valued field; the runner evaluates the predicate per candidate transformation and routes `true` outcomes to the prompt path and `false` outcomes to the mechanical path. Support both uniform application (every transformation evaluated) and hybrid application (a subset evaluated, others mechanical).
- Implement the **runner-surfaced display** of ADR-0037 §2: when the predicate fires, the runner surfaces the three framework-mandatory display elements — (a) the proposed transformation (mutation target and inferred value), (b) the source location (artifact path and structural anchor — line number, section heading, or other unambiguous locator), (c) the trigger predicate's evaluated value (band, predicate name, or evaluated field set) — plus any additional display fields the migration declared.
- Implement the **accept / edit / skip controls** of ADR-0037 §4 with the artifact-and-session-log effects it specifies: accept applies the proposed mutation verbatim and writes a record marking the transformation `accepted`; edit applies the user-supplied value — including the *accept-degraded* sub-case (relaxing the inferred value to a looser-but-valid one in place of the inferred one, per ADR-0037 §4) — and writes a record marking the transformation `edited` with the user-supplied value; skip applies no mutation and writes a record marking the transformation `skipped`. The migration owns value-validity; the runner forwards the migration's validation outcome (valid → apply; invalid → re-prompt with the migration's error message).
- Implement the **resumability mechanism** of ADR-0037 §3 with all three framework guarantees: (i) the session log (the on-disk persistence artefact whose path and format the migration controls) is written incrementally per transformation; (ii) a subsequent invocation reads the session log and reconstructs the in-memory resume state before prompting, skipping any transformation whose outcome is already recorded; (iii) a migration whose session log records every pending transformation as decided is treated as complete and the migration ID is appended to `.accelerator/state/migrations-applied` per ADR-0023's ledger contract. Use "session log" for the on-disk artefact and "resume state" for the in-memory reconstruction throughout the implementation and docs.
- Define and implement a **transformation ordering invariant**: within a single run, the runner processes transformations in the order the migration emits them. This makes the run's prompt sequence deterministic and lets the partial-run AC name a definite "first un-prompted transformation".
- Define and implement a **transformation key schema** for session-log records: each record carries a deterministic key — the migration-supplied transformation ID, or the artifact path plus structural anchor when no ID is supplied. Resume correctness depends on key matching (not on emission order being preserved across runs), so re-entry can match recorded outcomes to pending transformations even if the source order shifts between runs.
- Define and implement the **source-drift behaviour** ADR-0037 leaves unspecified: when a recorded transformation key matches a pending transformation but the proposed-value-at-decision-time differs from the value the migration now proposes (i.e. the source content has changed between runs), the runner re-prompts the user with the new proposed value and discards the old record for that transformation. The new outcome replaces the old. Flag this as a runner-level decision (not a framework primitive) in the documentation so a future ADR can promote it if needed.
- Preserve the existing mechanical path verbatim: migrations that do not declare the hook take the same runner code path as today, including the existing clean-tree pre-flight, one-line preview, and `.accelerator/state/migrations-{applied,skipped}` ledger handling.
- Update the migration framework's user-facing documentation (`skills/config/migrate/SKILL.md` and any helper docs) to describe (a) how a migration declares the hook (predicate, field set, session-log path/format, optional declared display fields), (b) the runner's guarantees from ADR-0037 §§1–4, (c) the runner-level source-drift behaviour above, (d) the transformation ordering invariant, and (e) a worked example using a synthetic fixture migration. Link to ADR-0037 (contract) and ADR-0038 (first consumer's parameterisation).

## Acceptance Criteria

- [ ] **Mechanical path unchanged**: running the framework against every migration present at HEAD under `skills/config/migrate/scripts/` that does not declare a hook produces byte-identical migrated artefacts and the same exit code as the pre-change runner, verified by a snapshot test that diffs both runs against the same input fixture set (the per-migration golden inputs under each migration's existing test directory).
- [ ] **Trigger predicate routes correctly**: given a fixture migration that declares a predicate, transformations for which the predicate returns `true` reach the prompt; transformations for which it returns `false` are applied mechanically. Tests cover predicate=true-only, predicate=false-only, and mixed transformation sets; both uniform-application and hybrid-application predicate shapes are exercised.
- [ ] **Display elements are present**: when the prompt fires, the runner's captured stdout contains, for each prompted transformation, (a) the proposed transformation's mutation target and value, (b) the source location per ADR-0037 §2 item 2 (the fixture migration emits `path:line` anchors and the test asserts the `path:line` string is present verbatim), and (c) the trigger predicate's evaluated value. Tests assert each element by string-match against a fixture migration's known outputs.
- [ ] **Migration-declared display extras are surfaced**: a fixture migration declares at least one extra display field (e.g. `surrounding_prose`); when the prompt fires, the captured stdout contains the declared field's value verbatim. Verified by string-match.
- [ ] **Accept control**: invoking accept on a prompted transformation (a) writes the inferred value to the artifact (verified by reading the artifact post-run), and (b) appends a session-log record whose schema consists of exactly `{transformation_key, outcome: "accepted", proposed_value, timestamp}` — no additional fields, no missing fields (verified by reading the session log).
- [ ] **Edit control**: invoking edit on a prompted transformation (a) writes the user-supplied value to the artifact (verified by reading the artifact post-run), and (b) appends a session-log record whose schema consists of exactly `{transformation_key, outcome: "edited", user_value, proposed_value, timestamp}` (verified by reading the session log). A fixture exercises the accept-degraded sub-case (user supplies a looser-but-valid value) and asserts the user value is recorded.
- [ ] **Skip control**: invoking skip on a prompted transformation (a) leaves the artifact unmodified at that transformation site (verified by diff), and (b) appends a session-log record whose schema consists of exactly `{transformation_key, outcome: "skipped", proposed_value, timestamp}`.
- [ ] **Edit validation re-prompt**: a fixture migration declares validation that rejects a specific user-supplied value (e.g. empty string); when the user edits and submits that value, the runner displays the migration's error message and re-prompts. Test covers one valid edit and one rejected edit followed by a corrected edit.
- [ ] **Resumability — incremental write**: after each accept/edit/skip decision, the session log on disk contains a record for that decision before the runner prompts for the next transformation (verified by reading the log between simulated decisions). Crash durability is verified by sending SIGKILL to the runner after the first decision is written and before the second prompt is displayed; the earlier decision's record is retained.
- [ ] **Resumability — partial run resume**: a fixture run that records N decisions, is interrupted, and is re-invoked emits zero prompts for those N transformations and resumes prompting at transformation N+1, identified via the recorded `transformation_key`.
- [ ] **Resumability — full-run idempotency**: re-running a fully-decided migration emits zero prompts, makes no further artefact changes (verified by `git diff --exit-code` against the post-first-run state), and appends the migration ID to `.accelerator/state/migrations-applied` exactly once (verified by ledger contents post-second-run).
- [ ] **Source-drift re-prompt**: a fixture in which the source content changes between runs such that the proposed-value-at-decision-time no longer matches the new proposed value triggers a re-prompt; the user's new outcome replaces the old record (verified by session-log inspection and final-artefact state).
- [ ] **Documentation**: `skills/config/migrate/SKILL.md` describes hook declaration, the three runner guarantees from ADR-0037 §§1–4, the source-drift behaviour, the transformation ordering invariant, and the transformation key schema. Docs include a fixture-migration worked example with a sample prompt transcript and a session-log excerpt; the example is exercised by a CI test that runs the fixture migration and asserts the transcript and log excerpt match the doc verbatim, so the example cannot drift from the implementation. Docs link to ADR-0037 and ADR-0038 by ID.

## Open Questions

- None at story level. ADR-0037 settles framework-level primitives; ADR-0038 settles the first consumer's parameterisation; this story's source-drift and transformation-ordering decisions are runner-level and called out as such in the docs. Any genuinely new gap surfaced during implementation routes back to ADR-0037 as a supplementary-ADR candidate per ADR-0037 §5.

## Dependencies

- Blocked by: 0092 (work item that produced ADR-0037 — the framework contract this story implements; status done), 0062 (work item that produced ADR-0038 — the first consumer's parameterisation; status done). The blocking relationship is on the published ADRs (ADR-0037, ADR-0038), both accepted.
- Blocks: 0070 (corpus migration shipping — consumes this runner extension with ADR-0038's parameterisation).
- Related: 0057 (parent epic), 0068 (spike — motivating verdict; status done), 0023 (mechanical-contract ADR — supplemented by ADR-0037, not edited per ADR-0031's immutability rule; also authoritative for the `.accelerator/state/migrations-{applied,skipped}` ledger schema this story continues to write).

## Assumptions

- The hook is opt-in per migration; the framework's default path stays mechanical.
- VCS revert remains the migration safety net (per ADR-0023, preserved by ADR-0037). The session log is not a rollback mechanism — it is a resumability artefact.
- ADR-0037 and ADR-0038 are stable and authoritative; this story implements them rather than re-deciding their primitives. Any new framework primitive needed surfaces as a supplementary ADR per ADR-0037 §5, not as a story-level decision.
- The existing migration runner (`skills/config/migrate/scripts/run-migrations.sh`) can be modified or restructured where this story's contract requires it, per ADR-0037's Decision Drivers.
- Fixtures: the existing migrations under `skills/config/migrate/scripts/` provide the corpus for the mechanical-path snapshot test. Hook-declaring fixtures are authored as part of this story (a small synthetic migration plus, for the accept-degraded and source-drift cases, scenario-specific variants).

## Technical Notes

- ADR-0037 §3 leaves the session-log persistence mechanism to the implementer (JSON checkpoint, append-only line-delimited log matching the `.accelerator/state/migrations-{applied,skipped}` style, or another scheme), so long as the three §3 guarantees hold. Pick one and document it in `SKILL.md`; the choice is reversible by a follow-up.
- The hook is general-purpose at the framework level (any future migration may opt in) per ADR-0037's broad supplement scope; the unified-schema migration (via ADR-0038 and 0070) is simply the first consumer.
- The runner is confidence-agnostic. Confidence bands, thresholds, and the predicate's shape are migration concerns parameterised in per-migration ADRs (e.g. ADR-0038); the runner only evaluates whatever predicate the migration declares.
- Source-drift and transformation-ordering are runner-level choices flagged as not-yet-promoted to ADR-0037. If a second interactive-requiring migration disagrees with either, that is the signal to promote to a supplementary ADR per ADR-0037 §5.

## Drafting Notes

- Reframed during pass-1 review (2026-05-30): originally marked conditional on 0062/0068 outcomes, but both ADRs are now done and resolved in favour of interactive hooks. Requirement 2 was originally "define the hook's contract"; ownership of contract definition sits with ADR-0037 (work item 0092), so this story now implements rather than defines.
- Sharpened during pass-2 review (2026-05-30): ADR-task vs ADR identifier confusion resolved (canonical contracts are ADR-0037 and ADR-0038; work items 0092 and 0062 are their producing tasks). Critical AC behaviours (display elements, accept/edit/skip session-log effects, source-drift) inlined from ADR-0037 §§1–4 rather than referenced opaquely. Source-drift and transformation-ordering — both silent in ADR-0037 — promoted to explicit story-level decisions.
- Polished during pass-3 review (2026-05-30): ADR-0037 §2.2 reference corrected to §2 item 2; accept-degraded glossed on first use; ordering invariant split from transformation-key schema; "confidence-agnostic" wording removed from Summary in favour of an in-place introduction of the predicate's field set (kept fuller in Technical Notes); session-log schemas tightened from "includes" to "consists of exactly"; structural-anchor form constrained to `path:line` in the Display AC; interruption mechanism specified as SIGKILL; `git diff --exit-code` named for idempotency; documentation worked example tied to a CI test against drift; ADR-0023's ledger-schema role surfaced in Dependencies.
- Story sizing: the deliverable spans seven multi-part requirements and thirteen acceptance criteria. Decomposition was considered (e.g. splitting documentation into a sibling story) and rejected — the parts collectively describe one apply-time interactive flow and partial implementation would not deliver standalone value. Size is inherent to the contract surface ADR-0037 defines.

## References

- ADR-0037 (`meta/decisions/ADR-0037-optional-interactive-contract-supplement-to-adr-0023.md`) — framework contract this story implements
- ADR-0038 (`meta/decisions/ADR-0038-interactive-validation-parameters-for-unified-schema-linkage-migration.md`) — first consumer's parameterisation
- ADR-0023 (`meta/decisions/ADR-0023-meta-directory-migration-framework.md`) — mechanical-default contract supplemented by ADR-0037
- ADR-0031 (`meta/decisions/ADR-0031-skill-level-adr-immutability.md`) — immutability rule; explains why ADR-0023 is not edited
- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md` (parent epic)
- Related work items: 0062 (ADR-0038 task), 0068 (spike), 0070 (downstream consumer), 0092 (ADR-0037 task)
- `skills/config/migrate/SKILL.md` — current user-facing framework contract
- `skills/config/migrate/scripts/run-migrations.sh` — current migration runner
