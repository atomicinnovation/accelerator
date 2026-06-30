---
type: pr-description
id: "12"
title: "Update skills docs (0176)"
date: "2026-06-30T07:57:19+00:00"
author: Phil Helm
producer: describe-pr
status: complete
work_item_id: "0176"
parent: "work-item:0176"
relates_to: ["plan:2026-06-29-0176-skill-reference-index-and-subsections"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/12"
pr_number: 12
tags: [docs, skills, documentation]
revision: "04a4886a18d67f152ffd16f6a9ff7b18257fbb67"
repository: "barcelona"
last_updated: "2026-06-30T07:57:19+00:00"
last_updated_by: Phil Helm
schema_version: 1
---

# Update skills docs (0176)

> 📖 **Preview the rendered docs:**
> <https://github.com/atomicinnovation/accelerator/tree/docs/0176-update-skill-docs>
> — browse `docs/skills/index.md` and the family/concept pages as GitHub
> renders them (subsections, anchors, and tables).

## Summary

Completes the per-skill-family reference layer (work item 0176) on top of the
0175 `docs/skills/` split. Every one of the **46** user-invokable skills now has
a uniform, deep-linkable reference subsection, a master **All Skills** index
links them all, and a TDD drift-guard test keeps the docs honest against
`SKILL.md` frontmatter.

Base branch is `docs/0175-slim-readme-split-docs-tree`, so this diff is the 0176
work alone.

## Changes

Delivered as three independently CI-green increments:

- **Phase 1 — close documentation gaps.** Documented the three
  previously-undocumented skills in each page's existing format: `conduct-spike`
  (planning), `refine-work-item` and `stress-test-work-item` (work-items).
- **Phase 2 — templated per-skill subsections.** Gave all 46 skills a uniform
  `### <name>` subsection (*What it does / How to use it / Advice & guidelines*)
  with a stable GitHub anchor:
  - Six family pages converted their tables/prose to H3 subsections
    (`work-items`, `vcs-and-pr`, `issue-trackers`, `adrs`, `design-convergence`,
    `planning`). `issue-trackers` keeps its compact Jira/Linear parity tables
    **alongside** the 16 subsections, preserving the side-by-side view.
  - Four concept pages kept their narrative and gained subsections for the
    skills they home (`development-loop`, `configuration`, `migrations`,
    `visualiser`).
  - `init` is re-homed to `configuration.md`; its `internals.md` mention is now
    a cross-reference, not a second documented home.
  - Each "What it does" reproduces the first sentence of the skill's `SKILL.md`
    `description` verbatim; `review-pr` / `review-plan` / `review-work-item`
    cross-link the Review System.
- **Phase 3 — master index + drift-guard test (TDD).** Added
  `docs/skills/index.md` ("All Skills"), grouped by the nine navigational
  families with a deep link per skill, and linked it first under the README
  **Skills** section. Added `scripts/test-skills-index.sh`, written red-first.

### Drift-guard test

`scripts/test-skills-index.sh` derives the user-invokable set from `SKILL.md`
frontmatter and asserts five invariants:

1. every invokable skill is referenced in the index via its
   `/accelerator:<name>` invocation;
2. no internal (`user-invocable: false`) skill is;
3. the invokable set is **exactly 46** (liveness gate);
4. every deep link `<page>.md#<name>` resolves to a real `### <name>` heading on
   its target page;
5. each index gloss and home-page "What it does" reproduce the first sentence of
   the `SKILL.md` description verbatim (whitespace-normalised).

A negative self-test mutates a temp index (drop one invokable, inject one
internal) and asserts the checker reports FAIL, proving the assertions are not
vacuous. The suite is auto-discovered by `test:integration:config`; the suite
floor in `tasks/test/integration.py` is bumped 19 → 21.

## Context

- Work item: `meta/work/0176-per-skill-family-reference-docs.md`
- Plan: `meta/plans/2026-06-29-0176-skill-reference-index-and-subsections.md`
- Plan review: `meta/reviews/plans/2026-06-29-0176-skill-reference-index-and-subsections-review-1.md`
- Sibling (base branch): 0175 — slim the README and split into a `docs/` tree.

## Testing

- [x] `bash scripts/test-skills-index.sh` — 257 assertions pass (fails red
      before `index.md` existed, green after).
- [x] `mise run test:integration:config` — full suite green; the new suite is
      auto-discovered and the 21-suite floor holds.
- [x] `mise run scripts:check` — shellcheck, shfmt, bashisms (bash 3.2 floor),
      and the exec-bit invariant all pass on the new entrypoint.
- [x] `mise run build-system:check` — ruff + pyrefly clean for the
      `integration.py` floor bump.
- [x] Load-bearing grep contracts preserved (`work.integration` in
      `work-items.md`; design tokens in `configuration.md` / `internals.md`).
- [ ] GitHub rendering spot-check (use the preview link above) — subsections,
      anchors, and the issue-tracker parity tables.

## Notes for Reviewers

- **Two adaptations from the plan**, both worth a look:
  1. *First-sentence derivation.* The plan's "truncate at first `. `" rule
     breaks on `show-jira-issue` / `show-linear-issue` ("e.g. PROJ-123") and on
     `description: >` block scalars / `configure`'s quoted scalar. The test's
     `compute_first` is robust to all three (protects `e.g.`/`i.e.`, strips the
     `>` indicator and surrounding quotes), and the docs match it.
  2. *Tab-split bug.* `IFS=$'\t' read` collapses the empty `user-invocable`
     field (tab is IFS-whitespace), which would silently swallow the
     description and make the description-match assertions pass vacuously. Fixed
     with parameter-expansion splitting in the test.
- `review-system.md` is intentionally unchanged (it lists lenses, not skills);
  the review skills cross-reference it.
- Hand-authored index + drift test is deliberate; full frontmatter-driven
  generation is deferred to 0177.
