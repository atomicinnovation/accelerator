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
revision: "f23215c6108ba9c19c9374d8180f5cae3a029d8b"
repository: "accelerator"
last_updated: "2026-10-29T16:46:44+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0306] Cite each research source exactly once

## Summary

A finding the composer writes, and the dossier `synthesise` writes, now
cite each source once, at its weakest tier, with the source's other tiers in
an `also` suffix: `tier-3 (also tier-2, tier-1)`. Before this, a composer had
no rule for a source cited at several tiers, and in one depth-3 `conduct` run
ten composers resolved the conflicts in three different ways. The rule lives
in one injected file, `skills/research/outputters/citing-once/SKILL.md`, and
replaces 0348's byte-for-byte carry-forward of metadata blocks. A committed,
on-demand eval checks that composers and `synthesise` follow it.

This PR is stacked on #171 (0348). DOI identity reads the `doi` metadata key
that 0348 adds, so #171 merges first.

## Changes

### The citing-once rule

- **Form**: a non-invocable skill (`user-invocable: false`,
  `disable-model-invocation: true`) with one heading per part. The parts are
  Same source, Carrying a DOI, Merge rule, Retraction suffixes, Citation
  order, Representative citation, Metadata merge, Full ties and Block order.
- **Identity**: two citations are one source when they share a DOI or a
  normalised URL.
  - A DOI comes from the `doi` key, or from a `doi.org` or `dx.doi.org` URL's
    path, percent-decoded. DOIs compare case-insensitively.
  - Normalising a URL drops its scheme, fragment, leading `www.` and trailing
    slash, and lowercases the host. The path and query keep their case.
  - Matches chain. Normalisation never rewrites a cited URL.
  - Identity is judged only over the citations the carrier reads.
- **Tier text**: the weakest base tier comes first, then the others in an
  `also` list, next-weakest first. `retracted` and `withdrawn` marks survive a
  merge and come before `also`, joined by `; `:
  `tier-3 (retracted; withdrawn; also tier-1)`.
- **Representative citation**: the first citation ordered by tier, then
  venue, then URL. A named venue sorts before a bare domain, venues compare
  case-folded then byte-wise, and a citation with no venue sorts last. URLs
  sort `https://`, then `http://`, then no scheme. The entry takes this
  citation's title, venue and URL as recorded.
- **Metadata**: the merged block carries every key any citation carries,
  each taken from its first carrier in citation order. Full ties are broken
  case-folded, then byte-wise. Every block lists the 19 recommended keys in
  table order, then any other key byte-wise, and keeps each value as
  recorded.
- **Closures**: the rule itself states the only changes allowed to tier text
  (**Merge rule**, **Retraction suffixes**) and to a metadata block
  (**Metadata merge**, **Full ties**, **Block order**).

### Carriers

- **`conduct`** injects the rule's path into its one composer spawn, after
  the outputter path. `agents/composer.md` adds the rule to its read-first
  step and to its read guard, and still grants only `Read` and `Write`.
- **`synthesise`** reads the rule with the Read tool before writing
  `synthesis.md`. It cites only sources that appear in the findings, and
  merges duplicates both across findings and within one finding.
- **Both carriers** say every tier comes from their inputs, that they never
  add a source, and that the citing-once rule makes the only changes to tier
  text and metadata blocks. Neither names the rule's parts or restates it,
  and neither keeps the byte-for-byte, never-reorder or stays-one-line
  instruction.
- **The finding outputter** gives only the `also` suffix's form and examples,
  keeps the researcher's copy-unchanged rule for blocks, and sends a
  composer's blocks to the citing-once rule.
- **The synthesis template** has the dossier's Findings prose take a
  source's tier and venue from its Sources entry.
- **`configure`'s** composer-override warning says an override must read the
  rule.
- **`research.md`** gains a "Citing a source once" section, which links the
  generated `reference/skills/internal/citing-once` page. It defines the
  weakest tier and gives the reasons for choosing it, and names the three
  ways one source can still be cited twice.
- **`CHANGELOG.md`** announces citing once and drops 0348's byte-for-byte
  wording.

### Behavioural eval

- **`mise run eval:citing-once`** runs the 36 acceptance sessions as headless
  `claude -p` runs and judges them. `test_mise.py` keeps it out of `check`,
  `default` and `test`, because it needs the `claude` CLI, network access and
  model usage.
- **Fixtures**: C1 to C5, S1 and S2, as seven JSON files under
  `skills/research/outputters/citing-once/evals/`.
- **Code**: `tasks/eval/citing_once/` holds a reading of the rule (the
  oracle), project seeding, the session runner, log checks and the checker.
- **Options**: `--family` narrows a run, `--jobs` sets how many sessions run
  at once, and `--check-only --out <dir>` re-judges a finished run without
  spending anything.

### Tests

- **`test_research_structure.py`** pins:
  - the rule's 9 headings and 28 decision phrases, its merged-tier examples
    and its key table against `RECOMMENDED_KEYS`;
  - both closures, built from part groups that must be headings of the rule;
  - that no carrier, and not the finding outputter, names a part or restates
    a decision phrase;
  - that every mark a profile prescribes, of any shape, is one the rule
    merges.
- **`test_citing_once_rule.py`** (35 tests) runs the oracle over the rule
  file's own URL table and worked example, both parsed from the rule rather
  than copied.
- **`test_citing_once_eval.py`** (53 tests) checks that the fixtures'
  criteria share no source and that the oracle predicts every one. It also
  checks that the checker fails on a changed `accessed` value, a duplicated
  entry and an added source.

### Housekeeping

- **`pulldown-cmark`** moves from `=0.13.4` to `=0.13.5` for
  RUSTSEC-2026-0414, with the third-party notices regenerated.
  - `recipe.json` records a `pulldown_cmark_versions` list per digest recipe,
    and refuses anything but a list.
  - Only the latest recipe may gain a version. Each recipe failure has its
    own message, and the remediation names `recipe.json`.
  - The pin comment, `DigestRecipe::CURRENT`'s doc and the guard's message
    all state when a release may join the current recipe: its upstream diff
    touches only disabled extensions or non-parse code, and every golden
    digest still holds. `DigestRecipe::CURRENT` stays 1.
- **`Cargo.lock`**: the same change re-resolves `tempfile`'s `getrandom`
  from 0.3.4 to 0.4.3. Both versions were already locked, and `deny:check`
  passes.
- **Work items**: 0281 and 0284 now require tier-text readers to accept
  findings from before and after 0306 and to tolerate duplicate entries.
  0284's parser in `cli/` owns the grammar.

## Context

- Work item: `meta/work/0306-deterministic-rule-for-citing-a-source-once.md`
- Plan: `meta/plans/2026-10-28-0306-deterministic-rule-for-citing-a-source-once.md`
- Research: `meta/research/codebase/2026-10-28-0306-deterministic-rule-for-citing-a-source-once.md`
- Plan review: `meta/reviews/plans/2026-10-28-0306-deterministic-rule-for-citing-a-source-once-review-1.md`
- Validation: `meta/validations/2026-10-28-0306-deterministic-rule-for-citing-a-source-once-validation.md`
  (result `pass`)
- PR review: `meta/reviews/prs/173-review-1.md`, and the replies on this PR

## Testing

- [x] The full default `mise run` exits 0 on `a351c35c`, after this
      round's last commit. It ran in 345 s, and its formatters changed no
      file.
- [x] The structure, frontmatter, citing-once and mise tests pass, 503 of
      503 (re-run while writing this description):
      `uv run pytest tests/unit/tasks/test_research_structure.py tests/unit/tasks/test_skill_frontmatter_validation.py tests/unit/tasks/test_skill_frontmatter_population.py tests/unit/tasks/test_citing_once_rule.py tests/unit/tasks/test_citing_once_eval.py tests/unit/tasks/test_mise.py`
- [x] `cargo test -p work-adapters --test golden_digests` passes, 6 of 6,
      so every golden digest holds under 0.13.5 (re-run while writing this
      description).
- [x] Behavioural acceptance, re-run against the post-review wording through
      `mise run eval:citing-once`: 36 sessions (27 composer, 9 `synthesise`),
      with every criterion holding in every session.
  - The runs include the determinism checks: nine composers over three note
    orders, and six dossiers over two finding orders.
  - C2 gained a criterion for the percent-decoded DOI.
  - All 9 `synthesise` sessions read the rule with `Read`; the first run
    had seen `cat` in 8 of 9.
  - Every session loaded only this repository's plugin.
  - The run took 8 minutes at six sessions at once.
- [ ] "Citing a source once" has not been viewed in a docs-site preview.
      `docs:check`, part of the default run, builds the site and validates
      its links.

## Notes for Reviewers

- **Rule as prose**: the rule is prose that an LLM follows, and no shipped
  code parses or deduplicates a Sources entry. The oracle in `tasks/eval/` is
  a second reading of the rule that predicts outputs; it ships no behaviour.
  Run `mise run eval:citing-once` after changing the rule, a carrier or the
  model. The 36 sessions cost about $16 on `claude-opus-5-5`.
- **Duplicates on disk**: one source can still be cited twice in three
  places.
  - A depth-1 finding, because a researcher, not a composer, writes it.
  - A finding written before this rule.
  - A dossier, when a composer kept a URL no other finding cites and dropped
    the one that linked them.

  `synthesise` merges the first two. The third is an accepted gap: a merged
  entry has one URL slot. The gap is recorded in the rule, the docs and 0281.
- **Closed mark list**: only `retracted`, `withdrawn` and `also` survive a
  merge. A mark a profile adds later, of any shape, fails
  `test_the_citing_once_rule_merges_every_prescribed_mark` until the rule
  names it.
- **Existing sets**: existing findings and dossiers stay valid, and
  `synthesise` re-merges their citations on its next run. No shipped code
  parses tier text yet; 0281 and 0284 will be the first to.
- **Diff size**: about 2,260 of the 7,099 added lines are in `meta/`. The
  eval adds about 1,550 lines of JSON fixtures, 1,250 lines in `tasks/eval/`
  and 610 lines of tests.
