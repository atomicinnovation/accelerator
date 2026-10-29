---
type: "pr-description"
id: "173"
title: "[0306] Cite each research source exactly once"
date: "2026-10-29T10:12:44+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0306"
parent: "work-item:0306"
relates_to: ["work-item:0348", "work-item:0281", "work-item:0349", "work-item:0305", "work-item:0311", "work-item:0280", "work-item:0283", "work-item:0284"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/173"
pr_number: 173
tags: ["research", "deep-research", "composer", "synthesise", "tiers"]
revision: "7a8120748839923d15392131e5fb438eed728fcb"
repository: "accelerator"
last_updated: "2026-10-29T10:12:44+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0306] Cite each research source exactly once

## Summary

A finding the composer writes, and the dossier `synthesise` writes, now
cite each source once, at its lowest tier, listing the source's other
tiers in an `also` suffix: `tier-3 (also tier-2, tier-1)`. Until now a
composer had no rule for a source cited at several tiers, and in one
depth-3 `conduct` run ten composers resolved those conflicts three
different ways. The rule lives in one injected contract,
`skills/research/outputters/citing-once/SKILL.md`, and replaces 0348's
byte-for-byte carry-forward of metadata blocks.

Stacked on #171 (0348): DOI identity reads the `doi` metadata key that
0348 adds, so #171 merges first.

## Changes

### The citing-once contract

- `citing-once/SKILL.md` is a non-invocable skill
  (`user-invocable: false`, `disable-model-invocation: true`) with one
  heading per part: Same source, Carrying a DOI, Merge rule, Retraction
  suffixes, Citation order, Representative citation, Metadata merge,
  Full ties, Block order.
- Identity: two citations are one source when they share a DOI (from the
  `doi` key or a `doi.org`/`dx.doi.org` URL, compared
  case-insensitively) or a normalised URL (scheme, fragment, leading
  `www.` and trailing slash dropped, host lowercased, path and query case
  kept). Matches chain, and normalisation never rewrites a cited URL.
- Tier text: the lowest base tier, the others next-lowest first in
  `also`. `retracted` and `withdrawn` marks survive the merge, ordered
  before `also` and joined by `; `:
  `tier-3 (retracted; withdrawn; also tier-1)`.
- Representative citation: first by tier, then venue (named venue before
  bare domain, case-folded then byte-wise, no venue last), then URL
  (`https://`, `http://`, no scheme). It supplies the entry's title,
  venue and URL as recorded.
- Metadata: the merged block carries every key any citation carries, each
  from its first carrier in citation order, with full ties broken
  case-folded then byte-wise. Every block a carrier writes lists the 19
  recommended keys in table order, then other keys byte-wise, and keeps
  each value as recorded.

### Carriers

- `conduct` injects the rule path into its one composer spawn, after the
  outputter path. `agents/composer.md` admits the rule to its read-first
  step and read guard, and still grants only `Read` and `Write`.
- `synthesise` reads the rule with the Read tool before writing
  `synthesis.md`, and merges duplicates across findings and within one
  finding.
- Both carriers name the rule's parts and restate none of them. Neither
  keeps the byte-for-byte, never-reorder or stays-one-line instruction.
- The finding outputter documents the `also` suffix, keeps the
  researcher's copy-unchanged rule for blocks, and hands a composer's
  blocks to **Metadata merge**, **Full ties** and **Block order**.
- `configure`'s composer-override warning says an override must read the
  rule and cite by it.
- `research.md` gains "Citing a source once", linking the generated
  `reference/skills/internal/citing-once` page rather than copying the
  rule.

### Tests

- `test_research_structure.py` pins the rule's nine headings, 17 decision
  phrases, both merged-tier examples, and its key table against
  `RECOMMENDED_KEYS`. It also checks that **Retraction suffixes** names
  every mark a source profile prescribes.
- The carrier tests check the rule path's injection and read order, that
  no carrier restates a decision phrase, and that no contract still
  carries a block byte-for-byte.

### Housekeeping

- `pulldown-cmark` `=0.13.4` → `=0.13.5` for RUSTSEC-2026-0414, with the
  third-party notices regenerated. `recipe.json` now records a list of
  parser versions per digest recipe, and the recipe guard accepts a
  version appended to the current recipe when every golden digest still
  holds. `DigestRecipe::CURRENT` stays 1.
- The same `Cargo.lock` change re-resolves `tempfile`'s `getrandom` from
  0.3.4 to 0.4.3. Both versions were already locked, and `deny:check`
  passes.

## Context

- Work item: `meta/work/0306-deterministic-rule-for-citing-a-source-once.md`
- Plan: `meta/plans/2026-10-28-0306-deterministic-rule-for-citing-a-source-once.md`
- Research: `meta/research/codebase/2026-10-28-0306-deterministic-rule-for-citing-a-source-once.md`
- Plan review: `meta/reviews/plans/2026-10-28-0306-deterministic-rule-for-citing-a-source-once-review-1.md`
- Validation: `meta/validations/2026-10-28-0306-deterministic-rule-for-citing-a-source-once-validation.md`
  (result `pass`)

## Testing

- [x] `mise run check` exits 0 on `8eaf86c2` (re-run while writing this
      description)
- [x] Structure and frontmatter discovery tests pass, 246 of 246:
      `uv run pytest tests/unit/tasks/test_research_structure.py tests/unit/tasks/test_skill_frontmatter_validation.py tests/unit/tasks/test_skill_frontmatter_population.py`
      (re-run while writing this description)
- [x] `cargo test -p work-adapters --test golden_digests` passes, 5 of 5,
      so every golden digest holds under 0.13.5 (re-run while writing this
      description)
- [x] Full default `mise run --jobs 3` exits 0, 65 tasks, per the
      validation report. It predates the commit that switches `synthesise`
      to the Read tool, and was not re-run here.
- [x] Behavioural acceptance: 36 headless sessions (27 composer, 9
      `synthesise`) against one-off fixtures, every criterion holding in
      every run. That includes the determinism checks: nine composers over
      three note orders, and six dossiers over two finding orders.
- [ ] The 9 `synthesise` sessions ran before `synthesise` was told to read
      the rule with the Read tool; 8 of them read it with `Bash` `cat`.
      They have not been re-run since that wording change.
- [ ] "Citing a source once" not yet viewed in a docs-site preview. The
      built HTML's text and links are verified.

## Notes for Reviewers

- The rule is prose an LLM follows, and no code parses or deduplicates a
  Sources entry. The structure tests fix the rule's shape and wiring.
  Only the Phase 3 sessions evidence behaviour, and their fixtures live
  outside the repository.
- A depth-1 finding is written by a researcher, not a composer, so its
  duplicates stay unmerged until `synthesise` merges them.
- ⚠️ Only `retracted`, `withdrawn` and `also` survive a merge. A suffix a
  profile prescribes later would be dropped silently, and the
  prescribed-mark test catches it only when the mark is one lowercase
  word.
- The recipe guard's `extends` check applies to every history entry, so
  a parser version appended to an earlier recipe passes. Only the latest
  entry is checked against the locked version, so this is harmless today,
  but it loosens the rewritten-entry guard.
- Existing findings and dossiers stay valid. `synthesise` re-merges their
  citations on its next run, and no shipped code parses tier text yet;
  0281 and 0284 will consume the `also` suffix.
- About 2,200 of the 2,711 added lines are `meta/`: plan, research, plan
  review, validation and the refined work item.
