---
date: "2026-05-26T16:58:27+01:00"
author: Toby Clemson
revision: "745a54caf27eb8e5344023cc826231aacda82356"
repository: accelerator
topic: "Codebase research for ADR work item 0062 (Interactive Validation for Corpus Migration)"
tags: [research, codebase, adr-0062, migration, interactive-validation, typed-linkage, vocabulary, frontmatter]
status: complete
last_updated: "2026-05-26T00:00:00+00:00"
last_updated_by: Toby Clemson
type: codebase-research
id: "2026-05-26-0062-adr-interactive-validation-for-corpus-migration"
title: "Research: Codebase grounding for ADR-0062 (Interactive Validation for Corpus Migration)"
schema_version: 1
relates_to: ["adr:ADR-0023", "adr:ADR-0031", "adr:ADR-0037", "adr:ADR-0033", "adr:ADR-0035", "codebase-research:2026-05-26-0092-adr-optional-interactive-contract-for-migration-framework", "codebase-research:2026-04-26-remaining-ticket-references-post-migration"]
derived_from: ["codebase-research:2026-05-24-0068-related-documents-inference-accuracy", "codebase-research:2026-05-26-0092-adr-optional-interactive-contract-for-migration-framework"]
---

# Research: Codebase grounding for ADR-0062 (Interactive Validation for Corpus Migration)

**Date**: 2026-05-26T16:58:27+01:00
**Author**: Toby Clemson
**Git Commit**: 745a54caf27eb8e5344023cc826231aacda82356
**Branch**: ticket-management (jj workspace)
**Repository**: accelerator

## Research Question

Provide the codebase-grounded inputs the drafter of ADR-0062 ("Interactive Validation for Corpus Migration") needs in order to produce the ADR — covering the spike verdict it consumes, the framework contract it adopts (ADR-0037), the upstream ADRs it must conform to or supplement (ADR-0023 / 0030 / 0033 / 0034), the surrounding work items (0057, 0068, 0069, 0070, 0092), the current state of the migration framework in code, and the corpus precedents for the vocabulary gaps the ADR must resolve.

The refined work item lives at `meta/work/0062-adr-corpus-migration-strategy.md` (workspace). The older draft variant — read for input comparison — lives at the parent repo's `meta/work/0062-adr-corpus-migration-strategy.md` and reflects the pre-2026-05-26 split, before ADR-task 0092 was extracted.

## Summary

The decision space the ADR has to navigate is now well-bounded:

- **Strategy is committed.** Spike 0068's 11.3% wrong-rate (vs the pre-committed ≤5% threshold) plus the cheap-fix counterfactual (still ~5.3%) make interactive hooks binding. The verdict is "not a borderline result".
- **Framework contract is settled.** ADR-0037 was accepted 2026-05-26 — it supplements ADR-0023, defines the trigger predicate / display elements / accept-edit-skip / resumability primitives, and explicitly admits both uniform and hybrid application. ADR-0037 mandates nothing about band design and nothing about persistence path/format — both are per-migration calls 0062 owns.
- **Three open decisions are exclusively 0062's.** Confidence-band design (two-band vs three-band), hybrid-vs-uniform shape, parser-fix ownership, and the two vocab-gap resolutions are all left to this ADR by ADR-0037 and by neither 0069 nor 0070 (which are draft-status downstream consumers).
- **One load-bearing premise in the work item is incorrect against ADR-0034.** The work item asserts that the vocab-canonical type for plan→work-item is `target`. ADR-0034:92 establishes it as `parent`. The "`Source:`-prose" vocab-gap discussion needs to be reframed around the choice between `parent`, `derived_from`, `source`, or a documented exception — not the work item's stated `{target, source, derived_from}` set.
- **The framework code has no interactive scaffolding today.** ADR-0037 is paper-only; the runner captures stdout/stderr to a tempfile with no tty pass-through (`run-migrations.sh:184`), so 0069 is greenfield.
- **The "broader workstream" gap is already being navigated in prose.** A literal `Broader workstream:` References label is in active use (e.g. research 2026-05-21:607); structurally, authors use `parent:` pointing at an epic ticket.

## Detailed Findings

### 1. The spike verdict (ADR's binding input)

Source: `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`

- **Sample**: 150 stratified candidates (50 per band: high / medium / low) from 1,231 parser-produced linkages across 267 qualifying files (file:29, 42, 74). Low-band exhaustion at 50/79 = 63% means low-band-frequent patterns are structurally common.
- **Result**: 84.0% correct / **11.3% wrong** / 4.7% uncertain (file:31, 85). Per-band: high 88%, medium 90%, low 74% (file:82-85).
- **Rubric**: pre-committed ≤5% wrong AND ≤15% uncertain → deterministic+report; otherwise → interactive hooks (file:120). Verdict robust: "wrong-rate sits at over twice the threshold" (file:133).
- **Cheap-fix counterfactual** (file:109-112): three patterns marked cheap-fix:
  - `template-path` (7) — literal placeholders like `ADR-NNNN.md` quoted in prose; fix is a blocklist of placeholder regexes.
  - `prose-keyword-false-match` (1) — `\bblocks?\b` matched "block" in "code-block"; fix is tighten the regex.
  - `sibling-as-deriv` (1) — `Sibling:` not in hint vocab; fell through to `derived_from` default; fix is add `\bsibling\b` → `relates_to` hint.
  - Arithmetic: 17 wrong − 9 reclassified = 8/150 ≈ **5.3%** — still over threshold. The work item's "~5.3%" figure matches the detailed counterfactual exactly. **Note**: the spike's Summary section (file:35) cites a different counterfactual (~6.7% from fixing only two patterns); minor internal inconsistency, but the 5.3% from the detailed catalogue is the binding number.
- **Calibration smell** (file:87): high 88% and medium 90% are statistically indistinguishable; "the parser's confidence scoring isn't separating the two cleanly." Position taken at file:154: production parser should "either collapse to a two-band model … or invest in sharper high-band criteria"; both presented as acceptable.
- **Vocabulary gaps surfaced**:
  - `Source:`-prose-on-plans recorded at file:167 as warranting a "small ADR within epic 0057" — both choices (vocab-canonical vs author intent) have migration consequences. The spike does not pre-decide.
  - "Broader workstream" gap at file:153: "no clean type for 'this work item is the broader workstream for that plan' (closest fit: `relates_to`, but loses information)." No recommendation.
- **Parser-fix ownership** (file:168): explicitly an **open question** — "worth implementing in the production parser. The interactive-hooks branch reduces the cost of leaving them unfixed, but they're cheap enough to fix anyway." Left for downstream decision.
- **Failure-pattern catalogue** (file:93-105): full table (11 patterns) follows; cheap-fix triad above accounts for 9/17 wrongs. The remaining 8 are non-trivial (`source-note-vs-relates` 4, `plan-target-ambiguous` 3, `plan-source-vs-target` 2, plus structural one-offs).

### 2. The framework contract (ADR-0037)

Source: `meta/decisions/ADR-0037-optional-interactive-contract-supplement-to-adr-0023.md` (accepted 2026-05-26)

Primitives the contract establishes that 0062 inherits verbatim:

- **Trigger predicate** (ADR-0037:48-59): "a boolean function over a named field set the migration also declares … must admit at least one confidence-valued field (e.g. an enumerated band, a numeric score, or a categorical label)". `true` → interactive path; `false` → mechanical path.
- **Runner-surfaced display elements** (§2, lines 61-69) — three mandatory slots: (i) the proposed transformation, (ii) the source location (artifact path + structural anchor), (iii) the trigger predicate's evaluated value. The migration may add more.
- **Accept / edit / skip** (§4, lines 88-96): each verb defined as artifact-mutation + session-log-mark pair. **`accept-degraded` is "a special case of edit, not a separate control"** (line 93) — relevant terminology to reuse if 0062's edit semantics differ for borderline cases. Skip is per-transformation, distinct from ADR-0023's migration-level `--skip`.
- **Resumability** (§3, lines 71-86): migration declares its "resumability persistence artefact" (file path + format). Framework guarantees: (i) incremental write (per-transformation, not at end), (ii) re-entry skips transformations already in the session log, (iii) on completion the migration ID is appended to `.accelerator/state/migrations-applied` per ADR-0023.
- **Three-layer chain** explicit at ADR-0037:26: "framework contract (this ADR) → migration application (per-migration ADRs) → runner implementation (0069)". 0062 is the middle layer.

What ADR-0037 explicitly leaves to per-migration ADRs (line 26, line 59, line 84):

| Concern | 0062 must decide |
|---|---|
| Which confidence values fire the prompt | Yes — trigger predicate |
| Which transformation fields are surfaced | Yes — display elements beyond the 3 mandatory |
| What an edit mutates | Yes — frontmatter field(s) the edit may touch |
| Uniform vs hybrid application | Yes — line 57: "The framework imposes no preference" |
| Concrete band design | Yes — line 59: "no preference on band design (two-band, three-band, scalar, categorical)" |
| Persistence mechanism | Yes — line 84: "path and format the migration controls" |
| Validity rules for edited values | Yes — line 96 |
| Per-prompt rendering, formatting, sequencing | No — deferred to 0069 runner |

**Idempotency** (line 120, Neutral): "An interactive-path migration must still be idempotent on re-run for the resumability guarantee to hold."

Vocabulary primitives 0062 should reuse verbatim: *trigger predicate*, *uniform application*, *hybrid application*, *session log*, *resume state*, *accept / edit / skip*, *accept-degraded*, *resumability persistence artefact*, *confidence-valued field*.

### 3. The upstream ADRs

#### ADR-0023 (meta-directory migration framework, 2026-04-25)

- **Mechanical contract** (lines 76-77, 82-85, 114-116): no prompts, no `--dry-run` flag, VCS revert as the only rollback, clean-tree pre-flight (uncommitted changes in `meta/`/`.claude/accelerator*.md` abort the run; `ACCELERATOR_MIGRATE_FORCE=1` bypasses).
- **Atomic state writes** (lines 118-120): success appends ID to state file (temp-then-rename); failure aborts with no partial write.
- **MUST NOTs**: no runtime beyond POSIX shell (line 47); migrations rewrite plugin-level expectations not user intent (lines 123-124); no rollback machinery inside the framework (lines 165-167).
- ADR-0023 itself does not reference ADR-0037 — the supplement linkage lives on ADR-0037's side only.

#### ADR-0030 (ADR template, 2026-03-18)

- **Required frontmatter**: `adr_id`, `date`, `author`, `status`, `tags`. Optional: `supersedes`, `superseded_by`.
- **Required body sections**, in order: Context · Decision Drivers · Considered Options · Decision · Consequences (Positive / Negative / Neutral subsections) · References. Confirmed exactly as the work item's AC enumerates.
- **In-body status block** required because rendered markdown hides YAML (lines 69-71).
- **Status lifecycle delegated to ADR-0031** — ADR-0030 itself does not enumerate states.
- **No `amends` / `amended_by` / `derived_from` fields** in the template. The supplement vocabulary is structurally incomplete — see vocab-gap discussion in §5.

#### ADR-0033 (unified base frontmatter schema, 2026-05-19, supplements ADR-0028)

- Base fields (every artifact): `type`, `id`, `title`, `date`, `author`, `producer`, `status`, `tags`, `last_updated`, `last_updated_by`, `schema_version`.
- Code-state-anchored types additionally carry `revision`, `repository` (no `branch`).
- **Typed cross-linkage keys explicitly deferred** (lines 177-180, 219-221): linkage vocabulary is decided by sibling ADR-0034, not enumerated here. Identity-value contract (lines 142-148) only commits to "linkage values follow the unified `id` shape (quoted string)".
- Per-artifact-type extras enumerated at lines 182-201; the ADR is "the single source of truth. No separate machine-readable schema file is produced" (lines 206-208).

#### ADR-0034 (typed linkage vocabulary, 2026-05-20) — **CRITICAL: vocab table differs from work item premise**

The full v1 vocabulary is **9 keys**, not the work item's implied `{target, source, derived_from}` decision set:

| Key | Cardinality | Semantic |
|-----|---|---|
| `parent` | single | Hierarchical owner; corpus-wide |
| `supersedes` | list | Replaces referenced |
| `superseded_by` | single | Inverse of `supersedes`; derivable |
| `blocks` | list | Prerequisite for referenced |
| `blocked_by` | list | Inverse of `blocks`; derivable |
| `target` | single | What this artifact is *about*; open-domain (reviews/validations) |
| `derived_from` | list | Generative source(s); fan-in key |
| `relates_to` | list | Loose linkage; flat (no qualifier in v1) |
| `source` | single | External / non-meta origin (e.g. note file for extracted work item) |

- **Plan → work-item canonical type is `parent`** (ADR-0034:92): "plan `parent` work-item — plan owned by work item". `target` is for reviews/validations, *not* plan-owner relationships. **This contradicts the 0062 work item's stated premise** that "the vocab-canonical type for plan→work-item per ADR-0034 is `target`". The drafter must address this discrepancy — either by re-reading ADR-0034 and re-framing the `Source:`-prose decision around `{parent, derived_from, source, documented-exception}`, or by flagging the work-item premise for correction. This is the single largest correction the ADR drafter needs to make to the work item before drafting.
- `source` IS already in v1 vocabulary — but defined as **external / non-meta origin** (ADR-0034:54), e.g. the notes file from which a work item was extracted. Using `source` to express plan→work-item linkage would either widen the type's semantic or violate it.
- **No `amends` / `amended_by`** in vocabulary. ADR-0037's "supplements ADR-0023" relationship cannot be expressed in typed linkage today — it lives in prose only.
- **Extensibility**: type-pair table is "illustrative and extensible by future ADRs" (ADR-0034:87). Vocabulary additions are treated as ADR-amendments via downstream ADRs (the immutability rule shapes how — sufficiency-not-exclusivity at line 65: only one side establishes the edge so the immutable side need not be updated).
- **No `workstream` artifact type or linkage key.** Granularity is artifact-to-artifact only.

### 4. Surrounding work items

| ID | Status | Bearing on 0062 |
|----|---|---|
| 0057 | in-progress | Parent epic. **OQ3** (line 124) — "deterministic-vs-interactive migration design" — is the open question 0062's acceptance resolves. |
| 0068 | done | Spike whose verdict 0062 cites; research write-up confirmed at `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`. |
| 0069 | draft | Framework-extension implementation. Does NOT own parser fixes. Hybrid-vs-uniform deferred (its OQ at line 54: "single-purpose linkage-inference vs general-purpose any low-confidence step"). Stale wording (line 22, 73) still says "conditional" — 0068's verdict activated it. |
| 0070 | draft | Corpus-migration shipping. Body-section parser "shared with 0068's spike prototype where practical, or rebuilt with the spike's findings encoded" (line 85). **Parser-fix ownership unstated.** Renames already migrated by 0005 / 0006 are explicitly excluded (lines 33-43). |
| 0092 | ready | Produced ADR-0037 (already accepted 2026-05-26); ticket status not yet flipped to done — a stale-status finding for downstream cleanup. Drafting Notes (line 77) confirm 0092 carries the framework contract and explicitly leaves *confidence-band design, hybrid-vs-uniform, parser-fix ownership, vocab-gap resolutions* to 0062. |

**Parser-fix ownership is ambiguous today**: 0069 disclaims it (framework plumbing only), 0070's text implies it falls to the production parser build but doesn't say so explicitly, 0092 disclaims it. 0062 must assign — to 0070's parser build, to a new parser-quality story, or to "out of scope" (with rationale).

### 5. The framework code today

Root: `skills/config/migrate/`. Layout:

```
skills/config/migrate/
├── SKILL.md
├── migrations/
│   ├── 0001-rename-tickets-to-work.sh
│   ├── 0002-rename-work-items-with-project-prefix.sh
│   ├── 0003-relocate-accelerator-state.sh
│   ├── 0004-restructure-meta-research-into-subject-subcategories.sh
│   ├── 0005-rename-work-item-type-to-kind.sh
│   └── 0006-canonicalise-work-item-id-and-author.sh
└── scripts/
    ├── run-migrations.sh   (driver, 221 lines)
    ├── test-migrate.sh
    └── test-fixtures/
```

Shared helpers at `scripts/` (workspace root level): `atomic-common.sh`, `config-common.sh`, `log-common.sh`.

**Mechanical contract enforced structurally**, not asserted. `run-migrations.sh:184` captures stdout+stderr to a tempfile with no stdin/tty pass-through — any `read` from a migration would block. Grep for `read -p`, `read -e`, `/dev/tty`, `interactive`, `prompt`, `confirm`, `band`, `trigger_predicate`, `session_log`, `resumab`, `checkpoint` under `skills/config/migrate/` returns no functional matches. SKILL.md:47 documents the no-dry-run posture.

**Migration-authoring shape**: `migrations/NNNN-<slug>.sh` with shebang + literal `# DESCRIPTION:` line (driver parses at lines 165-166). Migration receives `PROJECT_ROOT`, `CLAUDE_PLUGIN_ROOT`, `ACCELERATOR_MIGRATION_MODE=1`. Must be idempotent. May emit `MIGRATION_RESULT: no_op_pending` to defer.

**Linkage parsing precedent**: none. `config_extract_body` (`scripts/config-common.sh:90-100`) strips frontmatter; nothing splits sections. 0070's body-section parser is greenfield.

**Frontmatter manipulation utilities**: `config_extract_frontmatter` / `config_extract_body` (config-common.sh:73-100); inline-array parser (`config_parse_array`, lines 108-121); private scalar/array readers in `config-read-value.sh` and `config-read-review.sh`. **No dedicated "set field" or "merge frontmatter" helper** — migrations 0005/0006 reconstruct frontmatter inline with awk. Interactive edit mutations will either need a new helper (introduced by 0069) or repeat that pattern.

**VCS-rollback assumption** holds end-to-end. The only deviation is migration 0004's `jj op-id` breadcrumb (lines 15-23) — a hint printed to stderr, not a framework primitive. 0004 also `log_die`s if no VCS is detected (`ACCELERATOR_MIGRATE_FORCE_NO_VCS=1` bypass), reinforcing VCS-as-safety-net.

### 6. Vocabulary-gap precedents in the corpus

#### (a) The `"Source:"`-prose pattern on plans

- **Plans using `- Source:`** — only 4 files (workspace):
  - `meta/plans/2026-05-08-0046-work-management-system-configuration.md:1264` → `meta/work/0046-...md`
  - `meta/plans/2026-05-20-0063-rename-work-item-type-to-kind.md:1300` → `meta/work/0063-...md`
  - `meta/plans/2026-05-21-0064-canonicalise-work-item-id-and-author-fields.md:2052` → `meta/work/0064-...md`
  - `meta/plans/2026-04-08-ticket-management-phase-1-foundation.md:686` → `path/to/source-document.md` (template placeholder — would correctly be excluded by the `template-path` cheap-fix from spike 0068).
- **Across all of `meta/`**, `^- Source:` appears ~66 times in ~64 files. Dominant locus is `meta/work/*` References sections (the spike's headline-source convention for work items, not plans).
- **Variant phrasings**: `Sources:` (plural, used as sub-heading) in several research docs; `Based on:` at `meta/plans/2026-03-18-adr-skills.md:12`; `Source work item:` at `meta/research/codebase/2026-05-21-0064-...md:602`. Not found: `Source(s):`, `From:`, `Derived from:`, `Origin:`.
- **Targets are heterogeneous**: research docs (~25 work items pointing to `meta/research/design-gaps/*`), notes (`meta/work/0052-...md:232`), skill files (`meta/work/0059-...md:202`), HTML prototypes (`meta/work/0073-...md:246`), external URLs (`meta/research/design-inventories/*/inventory.md:418, 549`), plan documents (`meta/work/0030-...md:174`).
- **ADRs do not use the `- Source:` convention** — `^- Source:` returns no matches under `meta/decisions/`.

Implication for the ADR drafter: the `Source:` prose is doing **multiple jobs** — naming the work item a plan was generated from (plan→work-item), naming the research a work item was extracted from (work-item→research), naming an external origin (work-item→note / →URL). The single decision the ADR makes about plan→work-item interpretation does not cover all of these; the ADR should be explicit about scope (i.e. "decision applies to the plan→work-item case; other source-targets follow §X").

The `target`-vs-`source`-vs-`derived_from`-vs-`documented-exception` framing in the work item should — given the ADR-0034 vocabulary table actually establishes plan→work-item as `parent` — be re-cast around the choice between **`parent`** (vocab-canonical), **`derived_from`** (author-intent reading: "plan was derived from the work item"), **`source`** (would require widening the v1 semantic from external/non-meta), or a documented exception.

#### (b) The "broader workstream" gap

- **No `workstream:` frontmatter field exists anywhere** in `meta/`.
- **Prose convention already in active use**: a literal `Broader workstream:` References label:
  - `meta/research/codebase/2026-05-21-0064-canonicalise-work-item-id-and-author-fields.md:607` → `meta/work/0070-...md`.
  - `meta/plans/2026-05-21-0064-canonicalise-work-item-id-and-author-fields.md:2061` → same target with annotation "(housekeeping target in Phase 5)".
  - Same research doc uses `### Broader workstream` as a section heading at :383 and :525.
- **Structural convention**: `parent:` pointing at an epic ticket. Three artifacts declare `kind: epic` (`^kind:\s*epic`): `0045-work-management-integration.md`, `0036-sidebar-redesign.md`, `0057-unified-artifact-frontmatter-and-typed-cross-linking.md` — the de facto workstream proxies. Epics are referenced as `parent:` by their child work items.
- The 0062 review at `meta/reviews/work/0062-adr-corpus-migration-strategy-review-1.md:42, 158, 222, 313` and the 0061 review at :240 ("visualiser-graph epic named as the headline downstream consumer but absent from `Blocks`") both flag the gap: an epic-as-workstream is sometimes the right target but isn't always the literal `parent`.

Implication for the ADR drafter: the cleanest "documented limitation" framing is that **a multi-ticket workstream that doesn't have an epic ticket** is unsupported; readers who want to link to a workstream should create an epic and link via `parent:` (already a vocab-canonical pattern). If the ADR records a new vocab term, the candidates the corpus suggests are `workstream` or extending `relates_to` with a qualifier (the latter explicitly deferred by ADR-0034:54 — "flat (no qualifier in v1)"). Documented-limitation appears cheaper than adding a new vocab key, and consistent with ADR-0034's existing extensibility posture.

## Code References

- `skills/config/migrate/scripts/run-migrations.sh:17-39` — `--skip`/`--unskip` flag handling
- `skills/config/migrate/scripts/run-migrations.sh:41-70` — clean-tree pre-flight (`ACCELERATOR_MIGRATE_FORCE=1` bypass)
- `skills/config/migrate/scripts/run-migrations.sh:87-93` — migration discovery (`ACCELERATOR_MIGRATIONS_DIR` override)
- `skills/config/migrate/scripts/run-migrations.sh:177-208` — apply loop (no stdin pass-through at line 184)
- `skills/config/migrate/SKILL.md:39-48` — migration-authoring contract
- `skills/config/migrate/SKILL.md:47` — "NOT honour any DRY_RUN env var — this framework has no dry-run mode"
- `skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh:15-23` — `jj op-id` breadcrumb (opportunistic, not framework-level)
- `scripts/config-common.sh:73-85` — `config_extract_frontmatter`
- `scripts/config-common.sh:90-100` — `config_extract_body`
- `scripts/atomic-common.sh:16-32` — `atomic_write` (canonical write helper)
- `meta/decisions/ADR-0023-meta-directory-migration-framework.md:76-77, 82-85, 114-116` — mechanical contract clauses ADR-0037 supplements
- `meta/decisions/ADR-0030-adr-template.md:54-58` — template required sections and frontmatter
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md:114-125, 177-180` — base fields; typed-linkage deferral
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md:46-56, 65, 87, 89-104` — v1 vocabulary, sufficiency rule, extensibility, type-pair table
- `meta/decisions/ADR-0037-optional-interactive-contract-supplement-to-adr-0023.md:26, 48-59, 61-69, 71-86, 88-96, 100, 120` — framework primitives + recursive-supplement clause + idempotency obligation
- `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md:31, 82-87, 93-105, 109-112, 120, 133, 153-154, 167-168` — verdict, calibration, failure catalogue, cheap-fix arithmetic, ownership-open-question

## Architecture Insights

- **Three-layer chain crystallised**. The 2026-05-26 split of 0062 / 0092 produces a clean separation: ADR-0037 is the framework contract; per-migration ADRs (0062 is the first) parameterise it; 0069 implements the runner. Each layer's responsibilities are now non-overlapping.
- **Supplement-pattern as the amendment vehicle**. ADR-0037 supplements ADR-0023 rather than superseding it; ADR-0033 supplements ADR-0028; ADR-0035 supplements ADR-0026. ADR-0031 is the immutability rule that motivates this pattern. 0062 inherits the convention — the ADR will conform to ADR-0030's template while citing ADR-0037 / ADR-0023 / ADR-0034 / ADR-0033 in References without modifying them.
- **The typed-linkage vocabulary cannot yet express "amends"**. ADR-0037's relationship to ADR-0023 is recorded in body prose only — no `amends:` linkage exists in v1. This is a deferred vocab gap that does not block 0062 but should be flagged.
- **`accept-degraded` as terminology**. ADR-0037's framing of "edit" as "user accepts a corrected value" rather than "user supplies an arbitrary value" — and the explicit `accept-degraded` shorthand — gives 0062 a clean way to express "the parser was close but not quite; user nudges to a vocab-valid value".
- **`parent:` as workstream proxy is already idiomatic**. The cleanest move on the "broader workstream" gap is to declare the existing pattern (epic + `parent:`) sufficient and document the limitation, rather than expand vocabulary mid-migration.

## Historical Context

- `meta/decisions/ADR-0023-meta-directory-migration-framework.md` — the mechanical-default contract that 0037 supplements.
- `meta/decisions/ADR-0031-skill-level-adr-immutability.md` — referenced by ADR-0037:126 as the immutability rule motivating the supplement form.
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` — supplement-pattern precedent (line 102-107).
- `meta/decisions/ADR-0035-brand-layer-indirection-supplement-to-adr-0026.md` — second supplement-pattern precedent.
- `meta/research/codebase/2026-05-26-0092-adr-optional-interactive-contract-for-migration-framework.md` (39.8K) — the prior research write-up for ADR-0037; provides the framework-contract design background.
- `meta/research/codebase/2026-04-26-remaining-ticket-references-post-migration.md` (21.6K) — earlier corpus-migration retrospective; predates the unified-schema work but useful as context on multi-pass migrations.

## Related Research

- `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md` — the spike whose verdict 0062 consumes (primary input).
- `meta/research/codebase/2026-05-26-0092-adr-optional-interactive-contract-for-migration-framework.md` — research backing the framework contract 0062 adopts.

## Open Questions

These remain genuinely open for the ADR drafter — not gaps in this research:

1. **Hybrid vs uniform** — ADR-0037 admits both. Hybrid (interactive only on low-confidence) is the spike's implicit recommendation and minimises user friction on the ~84% of high-confidence inferences; uniform is more conservative and offers a richer session log. The choice is the drafter's call.
2. **Two-band vs three-band** — spike calibration shows high/medium statistically indistinct; collapsing to two bands is the cleaner story but loses the option of using "high" as the strict-bypass shape. Sharpening the high-band gate retains three bands at the cost of parser complexity.
3. **Parser-fix ownership** — the three cheap-fixes from spike 0068 lower the per-artifact interaction count but do not change the verdict. Plausible placements: (a) 0070's parser build (cheapest), (b) a new parser-quality work item, (c) out-of-scope ("interactive hooks subsume them").
4. **`Source:`-prose vocab-canonical type** — the work item's framing needs correction (plan→work-item is `parent` not `target` per ADR-0034:92); the real choice is between `parent` (vocab-canonical), `derived_from` (author intent), or a documented exception. The drafter should pick and justify.
5. **"Broader workstream" treatment** — documented limitation citing existing `parent:`-to-epic pattern is the lowest-cost resolution; new vocab term (`workstream` or qualified `relates_to`) is more expensive and out-of-step with ADR-0034's flat-vocabulary v1.

## Anomalies & Inconsistencies Flagged

For the drafter's awareness — these need addressing somewhere in the ADR drafting flow but not necessarily in this ADR:

1. **Work item premise error** (load-bearing): the work item asserts the vocab-canonical plan→work-item type is `target`. ADR-0034:92 establishes it as `parent`. Drafter should correct framing.
2. **Spike write-up internal inconsistency**: file:35 (Summary) cites ~6.7% counterfactual from fixing two cheap-fixes; file:109-112 (detailed catalogue) gives 5.3% from fixing three. The work item's "~5.3%" matches the detailed catalogue. Non-blocking but worth a side-note.
3. **0069 status text stale**: 0069 still says "conditional" (lines 22, 73) — 0068's verdict activated it; 0069 should be re-statused at some point. Not 0062's concern but adjacent.
4. **0092 status stale**: 0092 is `ready` in frontmatter even though ADR-0037 has been accepted (2026-05-26). Should flip to `done`.
5. **`amends` not in vocabulary**: ADR-0037's "supplements ADR-0023" is in body prose only — typed-linkage v1 has no field for it. Future vocab gap, not 0062's to fix.
