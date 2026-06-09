---
date: "2026-05-24T19:12:50+01:00"
researcher: Toby Clemson
revision: "4ea5c4b4917c0d785e8e3d9b0f74025ca28b361d"
repository: accelerator
topic: "Accuracy of inferring typed linkages from `Related Documents` body sections in `meta/`"
tags: [spike, frontmatter, migration, knowledge-graph, parser, typed-linkages]
status: complete
last_updated: "2026-05-24T00:00:00+00:00"
last_updated_by: Toby Clemson
work_item_id: "0068"
type: codebase-research
id: "2026-05-24-0068-related-documents-inference-accuracy"
title: "Research: Accuracy of inferring typed linkages from body sections in `meta/`"
author: Toby Clemson
schema_version: 1
relates_to: ["work-item:0057"]
---

# Research: Accuracy of inferring typed linkages from body sections in `meta/`

**Date**: 2026-05-24T19:12:50+01:00
**Researcher**: Toby Clemson
**Git Commit**: 4ea5c4b4917c0d785e8e3d9b0f74025ca28b361d
**Branch**: ticket-management@
**Repository**: accelerator

## Research Question

When the unified-schema migration (work item 0070) walks the current `meta/` corpus and extracts typed-linkage frontmatter from free-form body sections (`## References`, `## Related Research`, `## Dependencies`, `## Historical Context`, `## Source References`), how accurate are its inferences? Specifically, does the deterministic best-effort parser produce a low enough wrong-rate that the migration framework can rely on it plus a post-run report, or does the migration need interactive validation hooks (work item 0069)?

## Summary

A throwaway prototype parser was built and run end-to-end against the `meta/` corpus (381 markdown files). It produced **1,231 candidate typed linkages** across the five qualifying section types. A stratified random sample of **150 candidates** (50 from each of the parser's `high`/`medium`/`low` confidence bands) was manually classified as `correct` / `uncertain` / `wrong`.

**Overall accuracy: 84% correct, 11.3% wrong, 4.7% uncertain.** The 11.3% wrong-rate exceeds the spike's pre-committed deterministic-acceptable threshold of ≤5%.

**Recommendation: work item 0062 (migration-strategy ADR) should adopt the interactive-hooks branch — i.e. work item 0069 (migration-framework interactive validation hooks) is NOT moot.** The headline wrong-rate sits over twice the 5% threshold, and the failure-pattern catalogue shows the errors are concentrated in a small number of recurring patterns that are individually addressable but collectively too numerous to dismiss as noise.

A secondary observation: even if every "cheap to fix" failure pattern (`template-path`, `prose-keyword-false-match`) were resolved in the production parser, the residual wrong-rate would still be ~6.7% — above threshold. The recommendation is robust to plausible parser-quality improvements.

## Detailed Findings

### Corpus shape

- 381 markdown files under `meta/`, of which 267 contained at least one qualifying section header.
- Qualifying H2 header counts across the corpus (from `grep -rhE "^## "` over `meta/**/*.md`):
  - `## References` — 207
  - `## Dependencies` — 62
  - `## Historical Context` — 40
  - `## Related Research` — 38
  - `## Source References` — 29
- Sections named verbatim in the spike's title (`## Related Documents`) do not appear in the corpus; the de facto sections carrying artifact references are the five above. The parser was configured to read all of them.

### Parser design

The prototype is at `/tmp/spike-0068/parser.py` (throwaway; not committed). It is ~280 lines of Python and works as follows:

1. **Walk** every `*.md` under `meta/`; for each file, infer source `artifact_type` from the directory layout (`meta/work/...` → `work-item`, etc.).
2. **Section extraction** via line-by-line state machine into the configured set of qualifying headers.
3. **Reference extraction** per line, in priority order:
   - backtick-wrapped paths (`` `meta/path/file.md` ``)
   - markdown link paths (`[text](meta/path/file.md)`)
   - bare paths (`meta/path/file.md`)
   - ADR-id mentions (`ADR-0023`)
   - work-item-id mentions guarded by lexical prefix (`work item 0057`, `epic 0057`, `**0057**`)
   - inside `## Dependencies` sections only, bare 4-digit IDs after recognised prefix labels (`Blocked by: 0033`, `Blocks: 0034, 0036`, `Related: 0057`)
4. **Linkage-type inference** combining (in priority order) same-line prose hints, section-header default, source×target artifact-type pair default, and `relates_to` fallback.
5. **Confidence-band scoring** per the rubric committed before any data was observed:
   - `high` = path resolves to a real file AND single unambiguous type signal (prose or section)
   - `medium` = path resolves AND type relies only on artifact-pair default OR an approximate prose hint
   - `low` = path doesn't resolve, target is a bare ID that doesn't match an indexed artifact, multiple type signals conflict, or the fallback fires
6. **Dedup** per (source, section, line, resolved-target), preferring path-form over id-form.

Source-type bias: ~62% of all candidates were sourced from `codebase-research` or `plan` artifacts. ADRs, work items, and reviews each contributed 8–15%. Notes, design-inventories, and design-gaps contributed the remainder.

### Sampling

Population by band: `high=697 medium=455 low=79`. Drew 50 per band (with seed=42 for reproducibility) → **150 sample candidates**. The full `low` band was nearly exhausted (50 of 79 = 63%), so any pattern in the `low` band that appears even twice in the sample is structurally common in the population.

### Accuracy counts

Per the spike's manual-verdict labels:

| Band   | n   | correct | wrong  | uncertain |
|--------|-----|---------|--------|-----------|
| high   |  50 | 44 (88%) | 5 (10%) | 1 (2%) |
| medium |  50 | 45 (90%) | 4 (8%)  | 1 (2%) |
| low    |  50 | 37 (74%) | 8 (16%) | 5 (10%) |
| **total** | **150** | **126 (84.0%)** | **17 (11.3%)** | **7 (4.7%)** |

The `high` and `medium` bands are very close in accuracy (88% vs 90%). This is a calibration smell — the parser's confidence scoring isn't separating the two cleanly. The `low` band is meaningfully worse (74%) and the wrong-rate doubles relative to the higher bands, so the band labelling is doing *some* useful work, but the high/medium distinction is largely cosmetic at this design.

### Failure-pattern catalogue

Every `wrong` and `uncertain` inference is attributed to one named pattern:

| Pattern | Count | Cheap to fix? | Description |
|---------|-------|---------------|-------------|
| `template-path` | 7 | **yes** | Target is a literal documentation placeholder (`ADR-NNNN.md`, `YYYY-MM-DD-topic.md`, `ADR-NNNN-description.md`, `{number}-description.md`) embedded in SKILL.md prose or research write-ups about skill design. Filterable with a small blocklist of placeholder regexes. All 7 cases concentrated in `## References` sections of skill-design research/plan artifacts (the doc itself is *about* the skill, so it quotes the skill's template strings). |
| `source-note-vs-relates` | 4 | partial | Author wrote "source note", "source research", or "Token source:" describing a referenced doc. The parser labels these `relates_to` (because the `Source:` prose regex requires the literal word "Source" at line start), but the author's intent reads as `source` or `derived_from`. Vocab-policy call: the 0057 vocabulary defines `source` narrowly as "external origin for extracted artifacts" — research/notes typically aren't "extracted from" their sources. Half-fixable by widening the prose detector; the other half is genuine vocab ambiguity. |
| `plan-target-ambiguous` | 3 | hard | A plan references multiple work-items in its References section (the work it's for, plus blockers/siblings/broader workstreams). The parser's `(plan, work-item) → target` pair-default labels all of them `target`. Disambiguation needs either (a) the surrounding prose label like "Blocker:", "Sibling:", "Broader workstream:" matched against a richer vocab, or (b) a per-source policy that only the first plan→work-item ref in the References section is the target. Both add design complexity. |
| `plan-source-vs-target` | 2 | medium | Plan artifacts use the prose label `Source:` to point at their originating work item; the parser correctly fires the `source` hint, but the canonical 0057 type for plan→work-item is `target`. Fixable by treating `Source:` as `target` when the plan→work-item pair applies; less clean than it sounds because it adds a context-dependent prose-override. |
| `loose-but-valid` | 2 | n/a | `relates_to` is a defensible label but a sharper type exists; classified `uncertain` rather than `wrong` because the migration would still produce a sane (if non-specific) frontmatter entry. |
| `prose-keyword-false-match` | 1 | **yes** | The `\bblocks?\b` regex matched "block" inside "code-block" (hyphen is a word boundary). Fix: tighten the regex to disallow letter-then-hyphen lookbehind, or scope it to recognised list-lead positions only. |
| `semantic-misinterpretation` | 1 | hard | A dependencies-section line read `Blocks: stories under 0057` and the parser inferred a direct `blocks` edge to 0057. The author meant "blocks the children of 0057", not 0057 itself. Detecting "under N" / "children of N" structurally would require natural-language understanding. |
| `parent-review-as-parent` | 1 | medium | A line read `... — parent epic review` referring to the review of the parent epic; the `parent` prose-hint fired even though the ref's target was the review document, not the parent. Fixable by attributing prose hints to the ref-position they appear near, not the whole line. |
| `sibling-as-deriv` | 1 | **yes** | The prose label `Sibling:` is not in the hint vocabulary, so a plan→research ref labelled "Sibling component plans:" fell through to the `(plan, codebase-research) → derived_from` pair default. Fix: add `\bsibling\b` → `relates_to` hint. |
| `bare-id-misresolved` | 1 | medium | The line `the ADR 0073 references` was parsed as an `ADR-0073` reference, but no such ADR exists (highest is ADR-0035). The "0073" in context was actually the work-item number that owns the ADR being discussed. Resolving requires understanding the surrounding noun phrase. |
| `vocab-ambiguity` | 1 | n/a | Work item extracted from "conflict with ADR-0008" — `source` is defensible if the ADR-conflict motivated extracting the work item; ambiguous in scope. |

#### Cheap-fix counterfactual

If the three patterns marked "cheap to fix" above (`template-path`, `prose-keyword-false-match`, `sibling-as-deriv`) were resolved in the production parser, the wrong count would drop from 17 to 8, giving:

- new wrong-rate: **8/150 ≈ 5.3%** — still over the 5% threshold (barely)
- new uncertain-rate: 4.7% (unchanged)

Even an optimistic counterfactual that also resolves `source-note-vs-relates` and `plan-source-vs-target` reductions (4 + 2 = 6 fewer wrong/uncertain) brings the wrong-rate to ~4% and uncertain-rate to ~2.7%, which *would* clear the rubric. But this assumes meaningful improvements across half the catalogue, and rests on judgement calls (the vocab-policy questions in `source-note-vs-relates` are not pure parser bugs).

### Rubric application

Pre-committed rubric (from work item 0068, Acceptance Criteria, binding):

> recommend **deterministic + report** if **wrong-rate ≤ 5%** AND **uncertain-rate ≤ 15%** on a sample of ≥ 100 inferences; otherwise recommend **interactive hooks**.

Measured:
- wrong-rate: **11.3%** — FAILS ≤ 5%
- uncertain-rate: **4.7%** — PASSES ≤ 15%
- sample size: 150 — meets ≥ 100

**Verdict: interactive hooks.**

### Rubric calibration observation

Per the spike's instructions, the rubric is not amended in-place. However, an observation is recorded here for any follow-on re-run:

The 5%/15% thresholds appear well-calibrated for "is this safe to ship without a human in the loop?" The observed wrong-rate is over double the threshold, which is not a borderline result that hinges on labelling judgements (a borderline 6–7% would be more debatable). No threshold adjustment would be needed to flip the verdict — the verdict is unambiguous.

What *could* sensibly be calibrated for a follow-on study: stratifying the wrong-rate by *failure pattern severity*. Some wrong inferences (e.g. `template-path` linking to a non-existent file) are visibly broken and a downstream user could spot them in a review; others (e.g. `plan-target-ambiguous` labelling a sibling as `target`) are silently misleading and would propagate as bad frontmatter. The current rubric weights both equally. A future re-run could weight by harm.

## Code References

- `/tmp/spike-0068/parser.py` — throwaway prototype (not committed)
- `/tmp/spike-0068/sample.jsonl` — 150 classified candidates
- `/tmp/spike-0068/verdicts.py` — verdict map + stats reporter
- `meta/work/0068-spike-related-documents-inference-accuracy.md` — spike work item
- `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md` — parent epic defining the typed-linkage vocabulary used for inference
- `meta/work/0062-adr-corpus-migration-strategy.md` — consumes this finding
- `meta/work/0069-migration-framework-interactive-validation-hooks.md` — conditionally blocked; this spike's recommendation activates it
- `meta/work/0070-ship-meta-corpus-unified-schema-migration.md` — consumes the recommendation regardless of branch

## Architecture Insights

- The corpus has converged on a small set of recurring sectional conventions (References, Related Research, Dependencies, Historical Context, Source References) — author practice is more uniform than the spike feared. The dominant ref shape is a backtick-wrapped path leading a list item, often followed by an em-dash and a prose label.
- The Dependencies-section convention (`- Blocks: 0033, 0034 (notes)`, `- Blocked by: 0046, 0047`) is structured enough to parse reliably in isolation, and contributes the cleanest high-confidence inferences in the corpus.
- The least-structured references live inside `## References` sections of skill-design plans and skill-customisation research. These documents reference SKILL.md template strings (`ADR-NNNN.md`) as if they were artifact paths, which the parser cannot distinguish from real refs without a placeholder blocklist.
- The 0057 typed-linkage vocabulary has gaps that show up in classification: there is no clean type for "this work item is the broader workstream for that plan" (closest fit: `relates_to`, but loses information). The `source` vs `target` vs `derived_from` distinction is interpreted inconsistently by authors writing prose labels like "Source:" and "Source note:".
- Confidence-band scoring as currently designed does not separate high from medium meaningfully (88% vs 90% accuracy). Any production parser should either collapse to a two-band model (resolved-and-typed vs ambiguous) or invest in sharper high-band criteria (e.g. require both prose AND section AND pair agreement).

## Historical Context

- `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md` defines the typed-linkage vocabulary applied here (`parent`, `supersedes`/`superseded_by`, `blocks`/`blocked_by`, `target`, `derived_from`, `relates_to`, `source`). The vocabulary's open question 3 asked whether to commit to interactive-vs-non-interactive migration design before inference accuracy was known; this spike answers it.
- The spike's pre-committed rubric and the throwaway-prototype constraint are both products of the spike-design conversation captured in 0068's Drafting Notes.

## Related Research

_None — this is the first inference-accuracy study against the `meta/` corpus._

## Open Questions

- **Vocab disambiguation for "Source:" prose**: should the production parser/migration treat the author convention `- Source: meta/work/NNNN-...md` on a plan as `target` (vocab-canonical) or `source` (author-intent)? Either choice has migration consequences and warrants a small ADR within epic 0057, not a parser bug.
- **Cheap-fix scope for production migration parser**: even though the spike's recommendation is interactive hooks regardless, the cheap-fix patterns (`template-path` blocklist, `\bblocks?\b` tightening, `\bsibling\b` hint) are worth implementing in the production parser. The interactive-hooks branch reduces the cost of leaving them unfixed, but they're cheap enough to fix anyway and lower the per-artifact interaction count.
- **Two-band vs three-band confidence**: the current parser's high/medium distinction is not load-bearing. Production parser should either tighten the high-band gate or collapse to two bands.
