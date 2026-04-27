# Skill Benchmark: create-work-item

**Model**: claude-sonnet-4-6  
**Date**: 2026-04-27T22:05:54Z  
**Evals**: 35 evals, 1 run each

## Summary

| Metric | With Skill | Old Skill | Delta |
|--------|-----------|-----------|-------|
| Pass Rate | 100% (n=35) | 81% (n=18) | +0.19 |

## Per-Eval Results

### with_skill
- ✓ **bare-invocation** (1): 3/3 (100%)
- ✓ **epic-type-requirements** (10): 3/3 (100%)
- ✓ **drafting-notes-populated** (11): 3/3 (100%)
- ✓ **author-resolution** (12): 3/3 (100%)
- ✓ **template-driven-types** (13): 2/2 (100%)
- ✓ **references-populated** (14): 3/3 (100%)
- ✓ **enrich-existing-path-like** (15): 3/3 (100%)
- ✓ **enrich-existing-numeric-single-match** (16): 3/3 (100%)
- ✓ **enrich-existing-numeric-multi-match** (17): 3/3 (100%)
- ✓ **enrich-existing-missing-path-fallback** (18): 3/3 (100%)
- ✓ **enrich-existing-unparseable-frontmatter** (19): 3/3 (100%)
- ✓ **vague-topic** (2): 2/2 (100%)
- ✓ **enrich-existing-numeric-no-match-fallback** (20): 3/3 (100%)
- ✓ **enrich-existing-gap-analysis** (21): 4/4 (100%)
- ✓ **enrich-existing-gap-mixed-section** (21a): 3/3 (100%)
- ✓ **enrich-existing-gap-instructional-prose** (21b): 3/3 (100%)
- ✓ **enrich-existing-rich-confirms** (22): 3/3 (100%)
- ✓ **enrich-existing-self-not-duplicate** (23): 2/2 (100%)
- ✓ **enrich-existing-augmentation-proposal** (24): 3/3 (100%)
- ✓ **enrich-existing-preserves-identity** (25): 4/4 (100%)
- ✓ **enrich-existing-no-next-number-call** (26): 1/1 (100%)
- ✓ **enrich-existing-h1-real-number** (27): 2/2 (100%)
- ✓ **enrich-existing-overwrites-path** (28): 2/2 (100%)
- ✓ **enrich-existing-status-preserved** (29): 2/2 (100%)
- ✓ **business-context-first** (3): 2/2 (100%)
- ✓ **enrich-existing-status-changed** (30): 3/3 (100%)
- ✓ **enrich-existing-oblique-status-mention** (30a): 2/2 (100%)
- ✓ **enrich-existing-identity-swap-check-performed** (31): 2/2 (100%)
- ✓ **topic-string-flow-still-calls-next-number** (32): 3/3 (100%)
- ✓ **model-proposes-type** (4): 2/2 (100%)
- ✓ **challenge-untestable-ac** (5): 2/2 (100%)
- ✓ **bug-type-requirements** (6): 3/3 (100%)
- ✓ **draft-xxxx-placeholder** (7): 2/2 (100%)
- ✓ **number-not-consumed** (8): 2/2 (100%)
- ✓ **near-duplicate-surfaced** (9): 3/3 (100%)

### old_skill
- ✓ **bare-invocation** (1): 3/3 (100%)
- ✓ **epic-type-requirements** (10): 3/3 (100%)
- ✓ **drafting-notes-populated** (11): 3/3 (100%)
- ✓ **author-resolution** (12): 3/3 (100%)
- ✓ **template-driven-types** (13): 2/2 (100%)
- ✓ **references-populated** (14): 3/3 (100%)
- ✗ **enrich-existing-path-like** (15): 0/3 (0%)
- ✓ **vague-topic** (2): 2/2 (100%)
- ✗ **enrich-existing-gap-analysis** (21): 0/4 (0%)
- ✗ **enrich-existing-augmentation-proposal** (24): 0/3 (0%)
- ~ **enrich-existing-preserves-identity** (25): 2/4 (50%)
- ✓ **business-context-first** (3): 2/2 (100%)
- ✓ **model-proposes-type** (4): 2/2 (100%)
- ✓ **challenge-untestable-ac** (5): 2/2 (100%)
- ✓ **bug-type-requirements** (6): 3/3 (100%)
- ✓ **draft-xxxx-placeholder** (7): 2/2 (100%)
- ✓ **number-not-consumed** (8): 2/2 (100%)
- ✓ **near-duplicate-surfaced** (9): 3/3 (100%)

## Analyst Notes

- with_skill achieves 100% pass rate (35/35 evals) with zero failures.
- old_skill runs were limited to 18 representative evals; the remaining 17 enrich-existing evals were omitted since they all test a mode that does not exist in the old skill — including them would trivially inflate the delta without adding insight.
- old_skill failures are all on enrich-existing mode evals: eval 15 (path-like discriminator), eval 21 (gap analysis), eval 24 (augmentation proposal) all score 0% because the old skill treats path-like input as a topic string.
- eval 25 (enrich-existing-preserves-identity) scores 50% in old_skill: author and status fields coincidentally match (both resolved from git config / set as default draft), but work_item_id and date are fresh values — showing identity is not preserved.
- All 14 pre-existing evals (1–14) pass 100% in both configs — no regressions introduced by the enrich-existing extension.
- Evals 1–14 assertions are identical for both configs: the enrich-existing changes are purely additive (new Step 0 discriminator path) and do not alter existing topic-string flow behavior.
