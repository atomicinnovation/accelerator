---
date: "2026-05-26T14:58:00+01:00"
author: Toby Clemson
git_commit: 0f16c47c1f7cadb49bb69b320d94c7b36565c4d9
branch: HEAD
repository: accelerator
topic: "ADR drafting context for 0092 — optional interactive contract amending ADR-0023's mechanical-by-default migration framework"
tags: [research, codebase, adr, migration, framework, interactive-hooks, adr-0023, adr-0030, adr-0031, accelerator-plugin]
status: complete
last_updated: 2026-05-26
last_updated_by: Toby Clemson
---

# Research: ADR drafting context for 0092 — optional interactive contract amending ADR-0023's mechanical-by-default migration framework

**Date**: 2026-05-26 14:58:00 BST
**Author**: Toby Clemson
**Git Commit**: 0f16c47c1f7cadb49bb69b320d94c7b36565c4d9
**Branch**: HEAD (unbookmarked jj working copy)
**Repository**: accelerator

## Research Question

For work item 0092 (`meta/work/0092-adr-optional-interactive-contract-for-migration-framework.md`), surface everything an ADR draft needs to land cleanly:

1. The exact ADR-0023 clauses the new ADR must engage with, quoted verbatim.
2. The ADR-0030 template shape and the corpus's amendment / supersession conventions (ADR-0031 immutability, ADR-0035 supplement-style precedent).
3. The framework-level abstraction boundary — what 0092 owns vs what is reserved for 0062 (migration application) and 0069 (runner implementation).
4. The spike 0068 verdict that motivates the contract, and what it does / does not constrain.
5. The current migration runner's shape, so the abstract contract is consistent with the implementation it will be applied to.
6. Any prior review of 0092 whose findings the ADR must honour.

## Summary

**Top-line shape for the ADR**

- This is a **supplement-style ADR**, not a supersession. The work item explicitly invokes the accepted-ADR immutability convention (ADR-0031); the established precedent is ADR-0035 (supplement to ADR-0026). ADR-0023 itself is NOT mutated — the amendment lives in the new ADR's text. Adopt ADR-0035's scaffolding (title pattern `ADR-NNNN: [topic] — supplement to ADR-0023`, Context paragraph naming the supplement relationship and invoking ADR-0031, Considered Options rejecting both in-place edit and full supersession).
- **Next ADR id available: ADR-0037** (current corpus is contiguous through ADR-0036; no rejected/superseded gaps).
- **Important wrinkle on immutability**: ADR-0031 (verbatim) forbids any content edit on non-`proposed` ADRs and contemplates only full supersession as the change path — it does NOT explicitly sanction the supplement pattern. The supplement pattern is a convention extension de-facto established by ADR-0033 and ADR-0035 (both supplement accepted ADRs without flipping them). The 0092 review (pass 2) endorses this pattern explicitly. Make the supplement framing explicit in the new ADR's Context, citing ADR-0031 as the constraint and ADR-0035 / ADR-0033 as the precedents.

**Clauses of ADR-0023 the new ADR must quote and amend**

The phrasings "no prompts" / "mechanical-by-default" do NOT appear verbatim in ADR-0023. The load-bearing clauses to engage are:

1. ADR-0023 §Considered Options → Dry-run model option 1 — `--dry-run` flag rejected on maintenance-surface grounds.
2. ADR-0023 §Considered Options → Dry-run model option 2 (chosen) — clean-tree pre-flight + one-line preview + immediate apply, "report-then-act" idiom.
3. ADR-0023 §Decision → Driver step 1 — clean-tree check is the sole user-facing guard.
4. ADR-0023 §Considered Options → Rollback support — "VCS revert only".
5. ADR-0023 §Consequences → Negative — "The framework has no rollback support".
6. ADR-0023 §Decision Drivers bullet 4 — "destructive actions are guarded, not prevented".

(Verbatim text for each is in §"ADR-0023 clauses to amend" below.)

**The four contract primitives 0092 must specify (and what they must NOT specify)**

| Primitive | 0092 owns (abstract framework shape) | Reserved for 0062 / 0069 |
|---|---|---|
| **Trigger predicate** | Shape must admit BOTH a confidence-band-valued input AND a named boolean predicate over inferred-transformation fields. (Per 0062 AC L54.) | Concrete band design (two- vs three-band), which fields are inspected — 0062's. |
| **Runner-surfaced display elements** | The runner is the surface; the framework guarantees a fixed set of display slots a migration parameterises. | Per-migration field lists (inferred linkage type, source line, etc.) — 0062's. |
| **Accept / edit / skip controls** | Each defined by (i) abstract artifact-mutation surface (ii) session-log / resume-state side-effect. Must admit accept-as-inferred, edit-to-correct, AND accept-degraded (loose-but-valid) shapes implied by spike 0068's `loose-but-valid` and `vocab-ambiguity` patterns. | Concrete mutation targets (which frontmatter fields) — 0062's. Per-prompt rendering / one-at-a-time sequencing — 0069's. |
| **Resumability mechanism** | Migration declares a persistence artefact; framework supports resume from it. | Choice of mechanism (checkpoint file vs transactional VCS commits per accepted transformation) — explicitly left to the implementer per 0069 Tech Notes L69. Specific path/format for the unified-schema migration — 0062's. |

The contract must also explicitly admit **both hybrid and uniform application shapes** without picking one (per 0062 AC L57).

**Spike 0068 verdict (load-bearing)**

- Measured 84.0% correct / 11.3% wrong / 4.7% uncertain on a stratified sample of 150 across 1,231 candidate linkages in 381 files.
- Pre-committed rubric (wrong ≤ 5% AND uncertain ≤ 15%) FAILS on wrong-rate. Verdict: **interactive hooks**.
- Cheap-fix counterfactual (resolve `template-path`, `prose-keyword-false-match`, `sibling-as-deriv`) reduces wrong to 5.3% — still over threshold. Verdict robust to plausible parser improvements.
- High/medium bands statistically indistinguishable (88% / 90%). The trigger-predicate primitive 0092 specifies must NOT assume bands are inherently meaningful — 0062 may collapse them.
- The spike does NOT specify hook shape (trigger, display, accept/edit/skip, resumability) — those primitives are 0092's contribution. The spike is binding only on "interactive hooks are needed".
- Spike Open Question hints at "per-artifact interaction count" — weak suggestion of per-inference rather than batch-summary granularity; treat as suggestive, not constraint.

**Discrepancy to be aware of**: the spike's Summary cites a ~6.7% cheap-fix residual, while its Detailed Findings table-derives 5.3%. The work item's "~5.3%" matches the rigorous calculation. The ADR should either avoid the contested figure or cite the 5.3% derivation.

**Current runner shape (constrains how abstractly 0092 can phrase the contract)**

The runner is a single Bash driver (`skills/config/migrate/scripts/run-migrations.sh`) that:

1. Pre-flights clean tree (lines 41-70), state files at `.accelerator/state/migrations-{applied,skipped}` (newline-delimited IDs).
2. Computes pending set, prints non-interactive preview banner.
3. Invokes each migration as `bash <file>` child with `PROJECT_ROOT`, `CLAUDE_PLUGIN_ROOT`, `ACCELERATOR_MIGRATION_MODE=1` exported. stdout/stderr captured to tempfile.
4. Reads exit code + a single stdout sentinel (`MIGRATION_RESULT: no_op_pending`).
5. No stdin read. No TTY test. No callback slot. **Migrations cannot prompt today** because their stdout is captured.

Natural seam for the interactive hook is **between preview and apply** (run-migrations.sh:175 → 177), with optional per-migration seam at the apply loop head (line 181-184). 0092 should describe the contract abstractly enough that either seam is admissible — runner-level seam choice belongs to 0069. The seam observation matters for the ADR only as evidence the contract is implementable on the existing runner.

**Review pass on 0092 (already APPROVE'd; ADR drafter should honour the polish items)**

The pass-2 review at `meta/reviews/work/0092-...-review-1.md` flagged (and the work item resolved or deferred to drafting):

- Define "session log" and "resume state" relative to the resumability artefact — currently undefined in the work item.
- Identifier formatting: use `ADR-NNNN` consistently (not bare numerals).
- AC 3 trigger predicate: specify a REQUIRED minimum shape, not just "e.g."
- AC 4 display elements: NAME concrete display elements with a minimum count, not just "enumerates".
- AC 7 template-conformance: inline the ADR-0030 required body sections and frontmatter fields explicitly.
- ADR-0023 mutation: forbidden. Amendment lives in the new ADR; readers find it via the supersession-chain cross-reference.
- Transitive consumer **0070** is not in 0092 Dependencies but the ADR should know it.

The reviewer explicitly endorses the three-layer split (0092 framework / 0062 application / 0069 implementation): "Three-layer chain holds." Preserve the boundary.

## Detailed Findings

### ADR-0023 clauses to amend (verbatim)

ADR-0023 file: `meta/decisions/ADR-0023-meta-directory-migration-framework.md`. Frontmatter: `adr_id: ADR-0023`, `status: accepted`, dated 2026-04-25.

**A. Dry-run model (§Considered Options, lines 70-79):**

> **Dry-run model:**
> 1. **`--dry-run` flag (the original research proposal)** — rejected: requires migration scripts to branch on a flag; doubles the code path; increases the maintenance surface
> 2. **Clean-tree pre-flight + preview** — chosen: the driver verifies no uncommitted changes in `meta/` or `.claude/accelerator*.md` before mutating; it prints a one-line preview per pending migration before applying; this provides the same user protection as dry-run without an extra flag, and matches the `init` skill's report-then-act idiom
> 3. **Always-destructive (no guard)** — rejected: too risky for files users may have uncommitted changes in

**B. Rollback support (§Considered Options, lines 81-85):**

> **Rollback support:**
> 1. **Per-migration undo scripts** — rejected: maintenance multiplier; VCS revert is always available and is the right tool for "undo migration"
> 2. **VCS revert only** — chosen: the clean-tree pre-flight ensures there is a clean VCS state to revert to; no additional rollback mechanism needed

**C. Driver step 1 (§Decision, lines 113-116):**

> 1. Verify clean working tree (uncommitted changes in `meta/` or `.claude/accelerator*.md` abort the run). `ACCELERATOR_MIGRATE_FORCE=1` bypasses this check for advanced users.

**D. Driver step 3 (§Decision, line 118):**

> 3. Print one-line preview per pending migration.

(Step 4 — applying — follows immediately with no confirmation between the two.)

**E. Decision Drivers (lines 45-46):**

> - The safety model must be compatible with the plugin's "destructive actions are guarded, not prevented" philosophy (see `init` skill precedent)

**F. Negative consequence (lines 166-167):**

> - The framework has no rollback support; users who need to undo a migration must use VCS revert; this is intentional but requires VCS to be present

The "no prompts" framing is the user's paraphrase. The closest ADR-0023 verbatim is the dry-run-flag rejection (A.1) and the preview-without-confirmation design (A.2 + D). Quote those exactly when amending.

### ADR-0030 template (the new ADR must conform)

ADR-0030 file: `meta/decisions/ADR-0030-adr-template.md`. Template emitted by `create-adr`: `templates/adr.md`.

**Required body sections in order:**

1. `## Context`
2. `## Decision Drivers`
3. `## Considered Options`
4. `## Decision`
5. `## Consequences`
   - `### Positive`
   - `### Negative`
   - `### Neutral`
6. `## References`

Above the body: H1 (`# ADR-NNNN: Title as Short Noun Phrase`), then an in-body status block (`**Date**:`, `**Status**:`, `**Author**:`).

**Required frontmatter (all 5 keys mandatory):**

```yaml
---
adr_id: ADR-NNNN
date: "YYYY-MM-DDTHH:MM:SS+ZZ:ZZ"
author: Author Name
status: proposed
tags: [tag1, tag2]
---
```

Optional: `supersedes: ADR-NNNN` (only if replacing another ADR). No `amends` / `amended_by` / `derived_from` fields exist in the ADR frontmatter contract — ADR-0030 names only the supersession pair.

**Conventions:**

- One decision per ADR. One to two pages max. Active voice ("We will…", "We chose…").
- Title is a short noun phrase; em-dash separates supplement suffix (per ADR-0035 precedent: `ADR-NNNN: [topic] — supplement to ADR-YYYY`).
- Considered Options: numbered, bold name + em-dash + prose. Per-option Pros/Cons forbidden (Consequences carries balanced trade-offs).
- Consequences: ALL three subsections (Positive / Negative / Neutral) should have content. Empty = signal to revisit. "Fairy Tale" pattern (only positives) explicitly forbidden by `create-adr` SKILL.
- References: backticked project-relative paths with em-dash descriptions naming the relational role (e.g. "foundation record this supplement extends").

### ADR-0031 immutability convention (verbatim Decision)

`meta/decisions/ADR-0031-skill-level-adr-immutability.md`, accepted.

> Immutability is enforced at the skill level. Only `proposed` permits content edits. Transitions out of `proposed` and out of `accepted` are performed atomically by the ADR skills: the skill writes the new `status` together with the reason field associated with that transition (`rejected_reason` for rejection, `superseded_by` for supersession, `deprecated_reason` for deprecation). Once written, no further edits are permitted on non-`proposed` ADRs.

Transition table (verbatim):

| From       | To                  | Via                          |
|------------|---------------------|------------------------------|
| proposed   | accepted, rejected  | `review-adr`                 |
| accepted   | superseded          | `create-adr --supersedes`    |
| accepted   | deprecated          | `review-adr --deprecate`     |

Status vocabulary: `proposed`, `accepted`, `rejected`, `superseded`, `deprecated`.

**Critical gap**: ADR-0031 contemplates ONLY full supersession as the path to change an accepted ADR. The "supplement-style" pattern (new ADR cross-references and refines without flipping status) is not contemplated, prohibited, or named. The corpus practice (ADR-0033 supplementing ADR-0028; ADR-0035 supplementing ADR-0026) is a convention extension de-facto established by precedent and endorsed by the 0092 review. The new ADR should make this framing explicit in its Considered Options (as ADR-0035 does — see below) and cite ADR-0031 as the constraint that rules out in-place edit.

### ADR-0035 supplement-style scaffold (to mirror)

`meta/decisions/ADR-0035-brand-layer-indirection-supplement-to-adr-0026.md`, accepted, dated 2026-05-23.

**Filename pattern**: `ADR-NNNN-[topic-slug]-supplement-to-adr-NNNN.md`.

**H1**: `# ADR-NNNN: Topic — supplement to ADR-YYYY` (em-dash).

**Frontmatter**: standard 5 keys only — no special supplements field.

**Context opening pattern (verbatim from ADR-0035, adaptable):**

> ADR-0026 (CSS Design-Token Application Conventions) defined [scope enumerated]. ADR-0026 is `accepted` and therefore immutable under ADR-0031.
>
> [What new question(s) the supplement answers.]
>
> A supplement to ADR-0026 records both answers without violating the immutability of the accepted record.

**Considered Options scaffold (verbatim from ADR-0035, adaptable):**

> 1. **Edit ADR-YYYY in place** — Add […]. Rejected: violates ADR-0031's skill-level immutability rule for `accepted` ADRs.
> 2. **Supersede ADR-YYYY with a new ADR** — Republish the full set of conventions plus the additions. Rejected: ADR-YYYY is still load-bearing; superseding would mark it `superseded` and bury a record that future readers still need.
> 3. **New supplementary ADR cross-referenced from ADR-YYYY's neighbours** — Add the new rules in a new ADR explicitly framed as a supplement; existing comments referencing the rule point at the new ADR rather than at ADR-YYYY. Precedent: ADR-0033 ("Unified base frontmatter schema") uses this pattern to supplement ADR-0028 without editing it. Accepted.

**References convention (verbatim role labels from ADR-0035):**

- ADR-0026 — "foundation record this supplement extends"
- ADR-0031 — "immutability rule that motivates the supplement-vs-edit choice"
- ADR-0033 — "prior precedent for the supplement pattern (supplements ADR-0028)"

The new ADR's References should list ADR-0023 first ("foundation record this supplement extends"), then ADR-0031 ("immutability rule that motivates the supplement-vs-edit choice"), then ADR-0033 and/or ADR-0035 ("prior precedent for the supplement pattern").

**Recursive supplement clause to consider including** (ADR-0035 §Decision §1 closing):

> Once this ADR is `accepted` it too becomes immutable, and a further [extension] must be recorded in a new supplementary ADR (supplementing either ADR-YYYY or this one — both records remain authoritative for the [extensions] they list).

Apply to the migration framework contract: future framework extensions (e.g. a new accept/edit/skip variant, a new resumability mechanism) would extend either ADR-0023 or this new ADR via further supplements.

### Existing ADR corpus inventory

36 ADRs on disk (ADR-0001 → ADR-0036, contiguous). 34 accepted, 2 proposed (ADR-0024 Kanban column set, ADR-0025 work-item cross-ref aggregation).

**Next available id: ADR-0037.**

Amendment / supersession relationships:

| Relationship | Frontmatter declared? | Older ADR mutated? |
|---|---|---|
| ADR-0036 supersedes ADR-0026 (partial, typography only) | Yes — `supersedes: ["adr:ADR-0026"]` / `superseded_by: "adr:ADR-0036"` | YES — frontmatter `superseded_by` added; body status note added at ~line 17. Note: this is a corpus exception to ADR-0031's "no further edits" rule, justified by the partial-supersession framing. |
| ADR-0035 supplements ADR-0026 | No — relationship encoded only in title and Context prose | NO |
| ADR-0033 supplements ADR-0028 | No — body-prose only | NO |
| ADR-0016 partial supersession (informal) | No — body-only note | Body-only |

The cleanest precedent for "amend without supersede" is ADR-0035 → ADR-0026 (no mutation, no frontmatter linkage; relationship lives in title + Context). The work item 0092 explicitly aligns to this pattern: "ADR-0023 itself is not mutated — the amendment is recorded in the new ADR's text, and readers discover it via the supersession / amendment cross-reference declared by the new ADR."

### Spike 0068 verdict

`meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`, dated 2026-05-24.

**Verdict (verbatim Rubric application):** "Verdict: interactive hooks."

**Headline figures** (matching work item):

| Band   | n   | correct  | wrong   | uncertain |
|--------|-----|----------|---------|-----------|
| high   | 50  | 44 (88%) | 5 (10%) | 1 (2%)    |
| medium | 50  | 45 (90%) | 4 (8%)  | 1 (2%)    |
| low    | 50  | 37 (74%) | 8 (16%) | 5 (10%)   |
| total  | 150 | 126 (84.0%) | 17 (11.3%) | 7 (4.7%) |

Population: 1,231 candidate linkages, 381 files (267 with qualifying sections). Sample seed=42.

**Rubric (verbatim from work item 0068 AC, applied in the spike):** "recommend deterministic + report if wrong-rate ≤ 5% AND uncertain-rate ≤ 15% on a sample of ≥ 100 inferences; otherwise recommend interactive hooks."

**Cheap-fix counterfactual (verbatim Detailed Findings):**

> If the three patterns marked "cheap to fix" above (`template-path`, `prose-keyword-false-match`, `sibling-as-deriv`) were resolved in the production parser, the wrong count would drop from 17 to 8, giving:
> - new wrong-rate: **8/150 ≈ 5.3%** — still over the 5% threshold (barely)
> - new uncertain-rate: 4.7% (unchanged)

(Spike's Summary cites a different ~6.7% figure. Detailed Findings = 5.3% is the rigorous derivation; work item also cites 5.3%; ADR should cite 5.3% or avoid the contested figure.)

**Full failure-pattern catalogue** (11 patterns):

| Pattern | Count | Cheap to fix? |
|---|---|---|
| `template-path` | 7 | yes |
| `source-note-vs-relates` | 4 | partial |
| `plan-target-ambiguous` | 3 | hard |
| `plan-source-vs-target` | 2 | medium |
| `loose-but-valid` | 2 | n/a |
| `prose-keyword-false-match` | 1 | yes |
| `semantic-misinterpretation` | 1 | hard |
| `parent-review-as-parent` | 1 | medium |
| `sibling-as-deriv` | 1 | yes |
| `bare-id-misresolved` | 1 | medium |
| `vocab-ambiguity` | 1 | n/a |

**Calibration smell**: high (88%) and medium (90%) bands statistically indistinguishable. The spike recommends collapsing to two bands or tightening the high-band gate. Implication for 0092: the trigger-predicate primitive must NOT assume bands are inherently meaningful — admit a band-valued shape, but do not pre-commit to a particular band count.

**What the spike does NOT specify** (the gap 0092 fills): hook trigger predicate, displayed content, accept/edit/skip semantics, session-log/resumability shape. The spike's only direct hint at hook granularity is the Open Question phrase "per-artifact interaction count" — weakly suggestive of per-inference rather than batch.

**What 0092's contract should accommodate** (inferred from the failure-pattern axes):

- Path resolution status (template-path, bare-id-misresolved) → display must include "does the target resolve on disk?"
- Vocab-policy disputes vs parser bugs (source-note-vs-relates, plan-source-vs-target) → user controls must include "accept-degraded to a looser type" alongside accept-as-inferred and edit-to-correct.
- Prose attribution at ref-position (parent-review-as-parent, plan-target-ambiguous) → display must include the surrounding sentence / section context, not just the inferred edge.
- Natural-language understanding (semantic-misinterpretation) → unrecoverable by parser improvement; permanent demand for an interactive surface.

### Three-layer chain and abstraction boundary

The 0062 review of pass 3 split what was originally one ADR-task into 0092 (framework) + 0062 (application). The boundary statements are the cleanest constraint on what 0092 owns.

**From 0062 §Context (verbatim, line 39 of 0062):**

> Sibling ADR-task 0092 owns the framework-level optional interactive contract — the amendment to ADR-0023 and the framework-level shapes of trigger predicate, runner-surfaced display elements, accept/edit/skip semantics, and resumability mechanism. This ADR adopts that contract and parameterises it for the unified-schema migration's linkage validation, and resolves the linkage-vocabulary gaps the spike surfaced.

**From 0062 §Tech Notes (verbatim, line 84):**

> Sibling ADR 0092 carries the broad ADR-0023 amendment and the framework-level contract primitives; this ADR adopts and parameterises that contract for the unified-schema migration. The framework primitives' shapes (trigger predicate, display elements, control semantics, resumability) are 0092's; this ADR fills in the migration-specific values.

**From 0062 §AC (verbatim, line 54 — trigger-predicate shape constraint):**

> Trigger criterion: names exactly one confidence band, or a named predicate expressed as a boolean function over the inferred linkage's fields, that fires the prompt.

**Application shape** (0062 AC L57): "The framework contract (0092) admits both [hybrid and uniform application]; this migration must pick."

**From 0069 §Dependencies (verbatim, line 57):**

> Blocked by: 0092 (framework-level optional interactive contract — defines the contract this story implements), 0062 (linkage-application ADR — decides whether and how this migration uses the contract), 0068 (spike — informs whether interactive hooks are needed).

**From 0069 §Tech Notes (verbatim, line 69):**

> Resumability via a checkpoint file (e.g. `meta/.migrate-state.json`) is one option; transactional VCS commits per accepted transformation is another. **The implementer decides.**

So 0092 must define the resumability primitive abstractly (a migration declares a persistence artefact; framework supports resume from it) without picking checkpoint vs transactional. 0069's open questions on concurrency, hook genericity (single-purpose vs general-purpose), and per-prompt rendering are runner mechanics — 0092 should NOT specify them.

**From 0057 §Tech Notes (verbatim, line 145):**

> The migration framework (`skills/config/migrate/`) today is purely mechanical (no prompts, no dry-run, VCS-as-rollback). Extending it with optional interactive validation hooks is a deliberate departure from the contract documented in ADR-0023.

This is the "deliberate departure" framing the new ADR should capture in its Context.

### Current migration runner (constraints from implementation)

Files:

- `skills/config/migrate/SKILL.md` — user-facing contract.
- `skills/config/migrate/scripts/run-migrations.sh` — the driver.
- `skills/config/migrate/migrations/0001-…0006-…sh` — six migrations shipped to date, all mechanical.
- `.accelerator/state/migrations-{applied,skipped}` — persistence ledger (newline-delimited migration IDs).
- `hooks/migrate-discoverability.sh` — SessionStart discoverability hook.

**Runner ↔ migration interface (run-migrations.sh:184):**

```
PROJECT_ROOT=… CLAUDE_PLUGIN_ROOT=… ACCELERATOR_MIGRATION_MODE=1 bash <migration.sh> >tempfile 2>&1
```

- Input: env vars only (no argv).
- Output: exit code (0 success / non-zero failure) plus optional stdout sentinel `MIGRATION_RESULT: no_op_pending`.
- stdout/stderr captured to tempfile, NOT streamed — migrations cannot prompt today.
- Required migration shape: shebang on line 1, `# DESCRIPTION: <text>` on line 2, idempotent, must use `atomic-common.sh` writes, must not honour `DRY_RUN`.

**Loop shape (numbered like ADR-0023's lifecycle):**

1. Pre-flight clean tree (lines 41-70). Aborts on dirty `meta/`, `.claude/accelerator*.md`, `.accelerator/`. `ACCELERATOR_MIGRATE_FORCE=1` bypasses.
2. Read state (lines 72-85). Slurp both ledger files into arrays. Missing files = empty sets.
3. Discover (lines 87-93). `find` migrations dir for `[0-9]{4}-*.sh`, NUL-sorted.
4. Reconcile (lines 95-131). Warn on unknown IDs (preserved verbatim for downgrade safety); warn on cross-state collision (applied wins).
5. Compute pending (lines 133-148). Pending iff in neither array.
6. Preview-or-exit (lines 150-175). Banner: one `<id> — <description>` line per pending plus per-migration `--skip <id>` hint, then a paragraph reiterating the destructive-write/clean-tree contract. **No prompt, no pause, no stdin read.**
7. Apply (lines 177-208). Linear for-loop. On success, `atomic_append_unique` the ID. On failure, dump captured output to stderr and exit 1 immediately (no rollback).
8. Summary (lines 210-220).

**Interactive seams visible in the current code:**

- **Between preview and apply (lines 175 → 177)** — the natural batch-confirm seam. Has access to fully-computed `pending_files` and `skipped_ids` arrays.
- **Per-migration at apply loop head (line 181-184)** — per-migration confirm seam.
- **Argv / env-var extension** — existing patterns (`--skip`, `--unskip`, `ACCELERATOR_MIGRATE_FORCE`, `ACCELERATOR_MIGRATIONS_DIR`, `ACCELERATOR_MIGRATION_MODE`) are the precedent for new toggles.

What is NOT a current extension point: stdin reads, TTY tests, callback slots, migrations-prompting-via-stdout (captured to tempfile).

The new ADR should describe the contract abstractly enough that EITHER batch-confirm or per-migration-confirm is admissible — the runner-level seam choice is 0069's. The seam observation matters for the ADR only as evidence the contract is implementable on the existing runner without restructuring it.

### Prior review of 0092 (pass-2 APPROVE; drafter must honour the polish items)

`meta/reviews/work/0092-adr-optional-interactive-contract-for-migration-framework-review-1.md`, frontmatter `verdict: APPROVE`, `review_pass: 2`, dated 2026-05-26.

**Reframings from pass 1 → pass 2 the ADR drafter must observe:**

1. AC 7 now inlines ADR-0030 required body sections AND frontmatter fields (`adr_id`, `date`, `author`, `status`, `tags`). The ADR must conform exactly.
2. ADR-0023 follow-on text edit reframed per accepted-ADR immutability — **the older ADR is NOT mutated**; the amendment lives in the new ADR's text; readers find it via the supersession / amendment cross-reference declared by the new ADR.

**Pass-2 residual polish items the reviewer explicitly defers to ADR drafting:**

- Define "session log" and "resume state" relative to the resumability persistence artefact (currently undefined).
- Disambiguate "this ADR" (the deliverable) vs "the ADR" (the work item).
- Use consistent identifier formatting (`ADR-NNNN`, not bare `NNNN`).
- Define the artefact set behind "frontmatter (or other artifact) mutation".
- Tighten "broad … rather than narrow" — restated point.
- AC 3 trigger predicate: specify a required minimum shape, not just "e.g." — the ADR should name the minimum trigger-predicate shape concretely.
- AC 4 display elements: NAME the display elements concretely (minimum-count, not just "enumerates").

**Transitive consumer flagged but not in 0092 Dependencies:**

- 0070 (`meta/work/0070-ship-meta-corpus-unified-schema-migration.md`) consumes the contract via 0062's parameterisation and 0069's implementation.

**Reviewer endorsement of the three-layer split (verbatim, multiple places):**

> Single coherent purpose; three-layer split with 0062 / 0069 holds end-to-end.

> Tightly-scoped single-concern ADR task. Split from 0062 cleanly separates framework primitives here from migration-specific values there. Three-layer chain holds.

Preserve the boundary: framework primitives only; no migration-specific values; no runner implementation details.

## Code References

- `skills/config/migrate/SKILL.md` — user-facing contract today (lifecycle steps 1-7).
- `skills/config/migrate/scripts/run-migrations.sh:41-70` — clean-tree pre-flight (ADR-0023 Driver step 1).
- `skills/config/migrate/scripts/run-migrations.sh:150-175` — preview banner (no prompt; the seam an interactive contract must extend).
- `skills/config/migrate/scripts/run-migrations.sh:177-208` — apply loop. Migration invocation at line 184 with env vars `PROJECT_ROOT`, `CLAUDE_PLUGIN_ROOT`, `ACCELERATOR_MIGRATION_MODE=1`.
- `skills/config/migrate/scripts/run-migrations.sh:192` — stdout sentinel `MIGRATION_RESULT: no_op_pending`.
- `skills/config/migrate/scripts/run-migrations.sh:205` — `atomic_append_unique "$STATE_FILE" "$id"` (the only ledger mutation).
- `.accelerator/state/migrations-applied` — applied ledger (5 entries today, including 0001-0006 minus 0002 which is skipped).
- `.accelerator/state/migrations-skipped` — skipped ledger (0002-rename-work-items-with-project-prefix).
- `skills/config/migrate/scripts/test-migrate.sh` and `scripts/test-fixtures/000{1..6}/` — test harness and per-migration fixture corpora (precedent for any new interactive-mode test).
- `hooks/migrate-discoverability.sh` — SessionStart discoverability hook (advertises pending migrations).

## Architecture Insights

**Existing patterns the new ADR aligns with:**

- **Mechanical-default with env-var bypass** (`ACCELERATOR_MIGRATE_FORCE=1`). The optional interactive contract is the natural generalisation — opt-in switches from "off by default" to "declared by the migration", but the mechanical-default posture is preserved for non-declaring migrations.
- **State ledger as the resumability substrate**. `.accelerator/state/migrations-{applied,skipped}` already function as an audit trail. The contract's "session log / resume state" primitive can extend this idea conceptually without specifying the format (which is 0069's call).
- **Idempotent migrations + atomic writes** (per SKILL.md:43-48). Any accept/edit/skip mutation must remain idempotent on re-run for resumability to work — this is an implicit constraint the contract should name even if it does not specify the mechanism.
- **Init skill's "report-then-act" idiom** (the precedent ADR-0023 §Decision Drivers cites). The interactive contract refines this: preview becomes per-transformation; the act phase is gated by per-transformation accept/edit/skip rather than batch immediate-apply.
- **Supplement-pattern precedent**. ADR-0035 establishes the title format, Context scaffold, and Considered Options rejection-of-edit/rejection-of-supersession structure. Mirror it.

**Architecture-level boundary the ADR must respect:**

- The runner is a single Bash driver. The interactive hook contract must be specifiable without committing to runner architecture (e.g. callback ABI, sub-process protocol). 0069 picks. 0092 specifies the contract at the abstraction level of "what does a migration declare" and "what does the runner guarantee", not "how does the runner invoke the hook".

**One genuine open architectural question 0092 should resolve abstractly:**

- Is the trigger predicate evaluated by the framework runner (which then calls into the migration to render the prompt content), or evaluated inside the migration (which then signals the framework to surface a prompt)? The work item AC L40 says "a boolean function over named transformation fields" — implying framework-evaluated. The 0062 AC L54 admits either "a confidence band" (likely migration-emitted) or "a boolean predicate over the inferred linkage's fields" (likely framework-evaluated). 0092 should specify the contract precisely enough that either is admissible OR pick one — but the choice belongs to 0092, not 0062 or 0069. Recommendation: define the trigger predicate as a declarable boolean function over a named field set the migration also declares — that admits both a "band == 'low'" predicate and a "named-field predicate over arbitrary inferred fields" predicate.

## Historical Context

- `meta/decisions/ADR-0023-meta-directory-migration-framework.md` — the mechanical-by-default framework being supplemented. Accepted 2026-04-25.
- `meta/decisions/ADR-0030-adr-template.md` — template authority. Required body sections and frontmatter fields.
- `meta/decisions/ADR-0031-skill-level-adr-immutability.md` — immutability convention. Forbids edits on non-`proposed` ADRs; explicitly contemplates only supersession. The supplement pattern is a de-facto convention extension.
- `meta/decisions/ADR-0035-brand-layer-indirection-supplement-to-adr-0026.md` — supplement-style precedent (closest structural template).
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` — earlier supplement-style precedent (supplements ADR-0028).
- `meta/decisions/ADR-0036-typography-font-size-consumption-rule.md` — sole corpus example of a full supersession; partially supersedes ADR-0026 and DID mutate the older ADR's frontmatter and body. 0092 should NOT follow this pattern (work item explicitly forbids ADR-0023 mutation).
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md` — provides `relates_to`, `derived_from`, `parent`, etc. as corpus-wide linkage keys. Not part of ADR-0030 frontmatter contract, but available if the new ADR wants `relates_to` to declare the ADR-0023 amendment relationship at the frontmatter level (note: no current ADR uses this for ADR-to-ADR linkage; the precedent is body-prose linkage only).
- `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md` — parent epic. Frames the interactive extension as "a deliberate departure from the contract documented in ADR-0023".
- `meta/work/0062-adr-corpus-migration-strategy.md` — sibling ADR-task. Owns the migration-specific application of the contract 0092 defines.
- `meta/work/0068-spike-related-documents-inference-accuracy.md` — spike whose verdict motivates the amendment. Status: done.
- `meta/work/0069-migration-framework-interactive-validation-hooks.md` — runner implementation story. Owns concrete persistence mechanism, concurrency guards, hook genericity, per-prompt rendering.
- `meta/work/0070-ship-meta-corpus-unified-schema-migration.md` — transitive downstream consumer (the migration that adopts 0062's parameterisation via 0069's implementation).
- `meta/reviews/work/0092-adr-optional-interactive-contract-for-migration-framework-review-1.md` — pass-2 APPROVE review. Drafter must honour deferred polish items.
- `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md` — spike findings and verdict.

## Related Research

- `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md` — spike findings (this research's binding upstream).
- `meta/research/codebase/2026-04-25-rename-tickets-to-work-items.md` — ADR-0023's design exploration (dry-run alternatives, pinned-path semantics).
- `meta/research/codebase/2026-03-28-initialise-skill-requirements.md` — `init` skill's report-then-act idiom (the precedent ADR-0023 cites).

## Open Questions

The following are not gaps in the ADR draft — they are decision points the drafter must resolve while writing, surfaced here for explicit handling:

1. **Trigger-predicate framework-evaluated vs migration-evaluated**: pick a position (recommendation: declarable boolean function over a named field set; admits both shapes) and state it. The 0062 AC implies either is admissible; 0092 should pin this down so 0062 isn't forced to invent the contract retroactively.

2. **Display element minimum set**: per the 0092 review, the ADR must name concrete display elements with a minimum count, not just "enumerates". Candidates implied by spike 0068's failure-pattern axes:
   - The proposed transformation (what mutation would occur)
   - The source location (file + line / section heading)
   - The trigger predicate's evaluated value (band, predicate name, or both)
   - Optionally: target-resolves-on-disk? (relevant for the linkage case; arguably framework-general if the framework knows "this transformation touches files")

   The ADR should pick a minimum set. Recommend the first three as framework-level mandatory; the fourth as migration-parameterised.

3. **Accept-degraded as a fourth control vs a flavour of accept**: spike 0068's `loose-but-valid` and `vocab-ambiguity` patterns argue for an "accept with degraded specificity" option distinct from accept-as-inferred. Decide whether to specify three controls (accept / edit / skip — current AC) or four (accept / accept-degraded / edit / skip). Recommendation: keep three, define edit as the route to accept-degraded (edit the inferred type to a looser value, then accept the edited form). Calling this out explicitly in the ADR is worth doing because the work item's AC names only three.

4. **Session log vs resume state — definitions**: the review flagged these as undefined. Recommend defining "session log" as the persistence artefact the migration declares for resumability (path + format the migration controls; framework guarantees re-entry from it), and "resume state" as the in-memory reconstruction of progress that an invocation produces by reading the session log. The two are coupled but conceptually distinct — the ADR should name them.

5. **Hybrid vs uniform application — explicit non-decision**: the framework contract must admit both. Recommend stating this as a Decision sub-clause: "Migrations may apply the trigger predicate uniformly to all transformations OR hybridly (deterministic for some, interactive for others). The framework imposes no preference." This forecloses 0062 from being forced to re-establish the framework's neutrality on this axis.

6. **How to record the ADR-0023 amendment relationship in frontmatter**: the corpus has no `amends` field. Options:
   - Use no frontmatter field; relationship lives in title + Context (ADR-0035 precedent).
   - Use ADR-0034's `relates_to` field (no current precedent for ADR-to-ADR usage, but defensible).
   - Add a body-prose cross-reference and rely on the supersession-chain convention the work item invokes (note: ADR-0023 has no `superseded_by` and won't gain one; the "supersession chain" wording in the work item is loose — it appears to mean "readers find the amendment via the new ADR's References section pointing back at ADR-0023", not a formal chain).

   Recommendation: follow ADR-0035 precedent exactly — title carries the relationship; Context names it; References labels ADR-0023 as "foundation record this supplement extends". No frontmatter field.

7. **Whether to include the recursive-supplement clause**: ADR-0035 includes one ("once this ADR is `accepted` it too becomes immutable, and a further extension must be recorded in a new supplementary ADR"). For a framework contract that may grow more primitives over time, this is worth including.
