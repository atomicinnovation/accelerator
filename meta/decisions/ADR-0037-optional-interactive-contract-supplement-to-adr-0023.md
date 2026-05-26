---
adr_id: ADR-0037
date: "2026-05-26T15:07:14+01:00"
author: Toby Clemson
status: accepted
tags: [migration, framework, accelerator-plugin, interactive-hooks]
---

# ADR-0037: Optional interactive contract for the migration framework — supplement to ADR-0023

**Date**: 2026-05-26
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0023 (Meta-Directory Migration Framework) established a mechanical-by-default migration framework. Two clauses in particular foreclose user prompting at apply time:

- §Considered Options → Dry-run model rejects a `--dry-run` flag and chooses "clean-tree pre-flight + preview … one-line preview per pending migration before applying" — the "report-then-act" idiom.
- §Decision → Driver steps 3 and 4 print the preview and then apply with no intervening confirmation.

The mechanical-default contract is the right default for migrations whose transformations are deterministic. A migration class is now in scope, however, for which the framework cannot produce correct transformations from source content alone: migrations that infer typed values from prose. Spike 0068 measured the first concrete instance (the unified-schema migration's body-section linkage inference) and demonstrated that even with plausible parser improvements the residual wrong-rate exceeds the threshold the corpus is willing to accept. Further failure patterns reflect genuine ambiguity in the source rather than parser bugs, and are unrecoverable by mechanical work alone. The verdict — that this migration requires per-transformation user validation — generalises: any migration that infers structured values from prose is likely to surface a similar long tail of low-confidence transformations that no amount of parser work eliminates.

ADR-0023 is `accepted` and therefore immutable under ADR-0031. This ADR is structured as a supplement to ADR-0023 — adopting the supplement-form precedent set by ADR-0033 (supplements ADR-0028) and the explicit `-supplement-to-adr-NNNN` filename convention introduced by ADR-0035 (supplements ADR-0026) — rather than as an in-place edit (forbidden by ADR-0031) or a full supersession (which would mark ADR-0023's still-load-bearing mechanical-default contract as `superseded`).

This ADR defines the framework-level contract any migration may opt into. A migration's specific parameterisation (which confidence values fire the prompt; which transformation fields are surfaced; what an edit mutates; whether the hook applies uniformly or hybridly) is the migration's concern; for the unified-schema migration that parameterisation lives in work item 0062's sibling ADR-task. The runner-side implementation of the contract — process model, persistence format, per-prompt rendering — lives in 0069. The three-layer chain is: framework contract (this ADR) → migration application (per-migration ADRs) → runner implementation (0069).

## Decision Drivers

- The framework must accommodate migrations whose transformations cannot be inferred reliably enough to apply mechanically, without re-amending ADR-0023 for each such migration.
- The mechanical-default posture of ADR-0023 must survive: migrations that do not opt in run identically to today.
- The contract must be specifiable at an abstraction level that does not commit to runner architecture (callback ABI, process model, persistence format) — those are an implementation concern.
- The contract must admit both corrective edits (where the inferred value is wrong) and accept-degraded shapes (where the inferred value is too specific but a looser value is valid), and both uniform and hybrid application by a migration.
- The contract must be implementable in the existing migration runner (`skills/config/migrate/scripts/run-migrations.sh`); modifying or restructuring the runner is acceptable where the contract requires it.

## Considered Options

1. **Defer interactive-requiring migrations until inference improves enough to apply mechanically** — Hold any migration that cannot meet the deterministic-acceptable threshold in a pre-apply state; invest in inference improvements; re-measure; ship only once the threshold is met. Leaves the migration framework mechanical-only and ADR-0023 untouched. Treats interactivity as an admission of failure rather than a legitimate contract surface, and accepts indefinite deferral for migrations whose residual error rate reflects genuine ambiguity in the source content rather than fixable inference bugs.
2. **Apply mechanically anyway and remediate via VCS revert** — Run such migrations on ADR-0023's mechanical path; accept the known wrong-rate; treat post-migration as a cleanup phase using VCS revert plus targeted hand-edits to fix the wrong transformations identified by human review of the migrated corpus. Leaves the migration framework mechanical-only and ADR-0023 untouched. Trades upfront interactive cost for post-hoc remediation cost; confidence information available at inference time is not carried into the cleanup phase, so reviewers must rediscover which transformations were low-confidence by re-examining the migrated corpus.
3. **One-off interactive script outside the migration framework** — Build a bespoke prompt-driven tool per interactive-requiring migration: its own clean-tree pre-flight, its own state ledger, its own user-prompt format, run via a one-off script rather than via the standard `accelerator:migrate` skill. The standard migration framework remains mechanical-only and ADR-0023 unchanged; interactive-requiring migrations ship as a separate, parallel apply-time mechanism users invoke explicitly. Concentrates the interactive logic in one place specifically tuned to the migration, but creates a second apply-time entry point users and future contributors must learn alongside the standard one, and each subsequent interactive-requiring migration repeats the cost.
4. **Per-migration ad-hoc interaction inside the migration script** — Keep ADR-0023's mechanical-only framework as the framework contract; loosen ADR-0023's "captured stdout, no stdin" runner contract so individual migration scripts can prompt directly if they choose. Each migration that needs interactivity invents its own trigger logic, display format, accept/edit/skip semantics, and resumability mechanism inline. The framework runner gains no new responsibilities beyond letting migrations talk to the user; migrations that don't need to prompt continue to behave exactly as today. Distributes the interactive concern to the migrations themselves, on the theory that each interactive migration is sufficiently different that a shared contract would constrain more than it helps; the cost is paid in inconsistent user experience across migrations and no shared resumability infrastructure.
5. **Opt-in framework-level interactive contract** — Extend the migration framework with a permanent, declarable interactive contract. Migrations opt in by declaring a trigger predicate (over fields including a confidence-valued one) and a resumability persistence artefact; the framework runner guarantees the prompt's display elements, the accept/edit/skip control semantics, and the resume behaviour across invocations. Migrations that do not declare the hook run identically to today on the mechanical path. ADR-0023's mechanical-default posture is preserved as the framework's default; the interactive path is an additive capability migrations adopt as needed. Each interactive-requiring migration consumes the same framework primitives without re-amending ADR-0023, presenting a consistent user experience and sharing resumability infrastructure.

## Decision

We choose option 5 ("Opt-in framework-level interactive contract"). The remainder of this section specifies the framework primitives a migration declares to opt in, and the guarantees the framework runner provides in return. Migrations that do not declare these primitives are unaffected and continue to run on ADR-0023's mechanical-default path.

### 1. Trigger predicate (framework primitive)

A migration that opts in declares a **trigger predicate**: a boolean function over a named field set the migration also declares. The named field set must admit at least one confidence-valued field (e.g. an enumerated band, a numeric score, or a categorical label). The predicate's evaluation surfaces the candidate transformation for user interaction; predicates that evaluate `false` for a given transformation produce mechanical-path behaviour for that transformation.

The contract admits both:

- A predicate that names a single confidence value (e.g. `band == 'low'`); and
- A predicate that combines multiple declared fields (e.g. `band == 'low' OR target_resolves_on_disk == false`).

The framework imposes no preference between **uniform application** (the predicate fires across all transformations the migration produces) and **hybrid application** (the predicate fires on a subset, others apply mechanically). Migrations choose.

The framework imposes no preference on band design (two-band, three-band, scalar, categorical). Bands are a migration concern; the contract does not assume any particular band shape is inherently meaningful.

### 2. Runner-surfaced display elements (framework primitive)

When the trigger predicate fires for a transformation, the runner surfaces a prompt containing at least the following three framework-mandatory display elements:

1. **The proposed transformation** — the artifact mutation that would apply if accepted (e.g. the frontmatter field name and the inferred value).
2. **The source location** — the artifact path and the structural anchor within it (line number, section heading, or other unambiguous locator).
3. **The trigger predicate's evaluated value** — the band, predicate name, or evaluated field set that fired the prompt, sufficient for the user to understand why the prompt appeared.

The migration may parameterise additional display elements (e.g. surrounding-prose context, target-resolves-on-disk status, alternative candidates considered). Per-prompt rendering — formatting, sequencing, batching — is the runner's concern (see 0069).

### 3. Resumability mechanism (framework primitive)

A migration that opts in declares a **resumability persistence artefact**: a file the migration writes during the apply phase, recording the per-transformation outcome and any user-supplied values. Two framework terms used throughout the rest of this ADR and downstream:

- **Session log** — the persistence artefact itself: the file (path and format the migration controls) the runner guarantees is written incrementally as the user interacts with each transformation.
- **Resume state** — the in-memory reconstruction of progress that a subsequent invocation produces by reading the session log on re-entry, sufficient to skip already-decided transformations and resume from the first undecided one.

The framework guarantees:

- The session log is written incrementally (per-transformation, not at the end of the migration), so a session interrupted mid-migration loses no decisions.
- A subsequent invocation of the same migration reads the session log and reconstructs resume state before prompting the user; transformations already in the session log are not re-prompted.
- A migration whose session log records every pending transformation as decided is treated as complete and the migration ID is appended to `.accelerator/state/migrations-applied` per ADR-0023's existing ledger contract.

The choice of persistence mechanism — a JSON checkpoint file rewritten on each decision, an append-only line-delimited log (matching the format style of the existing `.accelerator/state/migrations-{applied,skipped}` ledgers), or another scheme — is the implementer's. The framework requires only that the chosen mechanism satisfies the three guarantees above.

The mechanical-path migration safety net is unchanged: VCS revert remains the rollback path, per ADR-0023.

### 4. Accept / edit / skip controls (framework primitive)

The framework guarantees three user-control verbs. Each is defined by (i) its effect on the artifact and (ii) its effect on the session log (per §3):

- **Accept** — apply the proposed transformation verbatim. **Artifact effect**: the migration performs the proposed mutation. **Session-log effect**: the resume-state record marks the transformation `accepted`, sufficient to skip it on re-entry.
- **Edit** — apply a user-modified form of the proposed transformation. **Artifact effect**: the migration performs the mutation using a user-supplied value in place of the inferred one. The contract admits "accept-degraded" (relaxing the inferred value to a looser-but-valid one — e.g. a more general linkage type) as a special case of edit, not as a separate control. **Session-log effect**: the resume-state record marks the transformation `edited` and stores the user-supplied value alongside, sufficient to replay the edited form on re-entry.
- **Skip** — leave the artifact untouched for this transformation. **Artifact effect**: no mutation. **Session-log effect**: the resume-state record marks the transformation `skipped`, sufficient to skip it on re-entry. Skip is per-transformation; the existing migration-level `--skip` mechanism (ADR-0023) is unchanged.

The contract does not specify what makes a value "valid" — that is the migration's responsibility, declared alongside the trigger predicate.

### 5. Recursive supplement clause

Once this ADR is `accepted` it too becomes immutable, and a further extension to the optional interactive contract (a new control verb beyond accept/edit/skip, a new framework-level display element, a new resumability guarantee) must be recorded in a new supplementary ADR (supplementing either ADR-0023 or this one — both records remain authoritative for the contract elements they define).

## Consequences

### Positive

- Migrations whose transformations legitimately need user judgement have a documented contract to adopt, rather than each re-litigating the ADR-0023 amendment or inventing bespoke prompt mechanisms.
- The mechanical-default path is unchanged: every migration shipped to date and every future migration that does not opt in continue to run identically to today.
- The contract is general: it pre-commits to no specific confidence band design, predicate shape, display field set, or persistence mechanism, so it accommodates any migration-specific shape of low-confidence transformation without further framework changes.
- The three-layer chain (framework contract here → per-migration application ADRs → runner implementation in 0069) gives each downstream consumer a stable foundation: migration ADRs parameterise this contract without inventing it; the runner implements it without re-deciding the user-facing semantics.
- The session-log / resume-state primitive aligns conceptually with the existing `.accelerator/state/migrations-{applied,skipped}` ledger, so the addition is a natural extension of the framework's existing audit trail rather than a new concept.

### Negative

- The contract delegates the choice of persistence mechanism, hybrid-vs-uniform application, and concrete band design to downstream ADRs. The deferred decisions are unambiguous (each has a named owner) but a reader looking for end-to-end specificity must follow the chain.
- An opt-in interactive path widens the framework's behavioural surface: the runner now has two paths to test, two failure modes to document, and two user-facing experiences to maintain.

### Neutral

- The contract describes only what a migration declares and what the runner guarantees. It does not constrain how the runner invokes the hook (in-process callback, subprocess, separate channel) — that is 0069's call. Reasonable runner-level implementations include extending the existing single-Bash-driver shape, and this ADR neither requires nor forbids restructuring.
- The framework's existing safety contract (clean-tree pre-flight, VCS-revert rollback, idempotent migrations) applies to both paths unchanged. An interactive-path migration must still be idempotent on re-run for the resumability guarantee to hold.
- The contract is neutral on prompt sequencing (per-transformation, batched, or otherwise) — that is a runner concern.

## References

- `meta/decisions/ADR-0023-meta-directory-migration-framework.md` — foundation record this supplement extends
- `meta/decisions/ADR-0031-skill-level-adr-immutability.md` — immutability rule that motivates the supplement-vs-edit choice
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` — prior precedent for the supplement pattern (supplements ADR-0028)
- `meta/decisions/ADR-0035-brand-layer-indirection-supplement-to-adr-0026.md` — prior precedent for the supplement pattern (supplements ADR-0026)
- `meta/decisions/ADR-0030-adr-template.md` — template authority
- `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md` — parent epic framing the extension as a deliberate departure from ADR-0023's mechanical contract
- `meta/work/0062-adr-corpus-migration-strategy.md` — sibling ADR-task; parameterises this contract for the unified-schema migration's linkage validation
- `meta/work/0068-spike-related-documents-inference-accuracy.md` — spike whose verdict motivates the contract; status done
- `meta/work/0069-migration-framework-interactive-validation-hooks.md` — runner-implementation story; implements this contract
- `meta/work/0070-ship-meta-corpus-unified-schema-migration.md` — downstream consumer (via 0062 and 0069)
- `meta/work/0092-adr-optional-interactive-contract-for-migration-framework.md` — work item this ADR satisfies
- `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md` — spike findings and verdict
- `meta/research/codebase/2026-05-26-0092-adr-optional-interactive-contract-for-migration-framework.md` — ADR drafting research
- `skills/config/migrate/SKILL.md` — current user-facing migration framework contract
- `skills/config/migrate/scripts/run-migrations.sh` — current migration runner
