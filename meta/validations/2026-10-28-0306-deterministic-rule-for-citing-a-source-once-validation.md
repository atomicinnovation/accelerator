---
type: "plan-validation"
id: "2026-10-28-0306-deterministic-rule-for-citing-a-source-once-validation"
title: "Validation Report: Deterministic Rule for Citing a Source Once Implementation Plan"
date: "2026-10-29T08:32:03+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-10-28-0306-deterministic-rule-for-citing-a-source-once"
tags: ["research", "deep-research", "composer", "synthesise", "tiers"]
last_updated: "2026-10-29T08:32:03+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Deterministic Rule for Citing a Source Once Implementation Plan

All three phases are built as planned, and `mise run --jobs 3` exits 0.
The result is `pass`. The 36 recorded Phase 3 sessions still pass every
criterion, both through their checker and on direct reading. Three
low-severity issues remain (see Potential Issues), none of which breaks
a promise in the plan's Desired End State.

### Implementation Status

✓ Phase 1: The citing-once rule file - Fully implemented
✓ Phase 2: Composer and `synthesise` apply the rule - Fully implemented
✓ Phase 3: Behavioural acceptance - Fully run, every criterion held

Every plan criterion is ticked except Phase 2's manual docs-preview item,
which the plan's "As built" notes say nobody has read in a preview yet.

### Automated Verification Results

✓ Full local CI mirror passes: `mise run --jobs 3` exited 0 in 2486 s,
  with 65 tasks finished and no formatter drift. `test:unit:frontend`
  passed 2615 of 2615 tests.
✗ The first `mise run --jobs 3` failed only in `test:unit:frontend`. It
  passed 2615 of 2615 tests, but vitest counted one worker
  `Timeout calling "onTaskUpdate"` error. This is the load flake the plan
  records for Phase 2, and the branch changes no frontend file. The
  failure stopped the run early, so the second run is the one that
  covers the Rust, server and docs lanes.
✓ Structure and discovery tests pass:
  `uv run pytest tests/unit/tasks/test_research_structure.py tests/unit/tasks/test_skill_frontmatter_validation.py tests/unit/tasks/test_skill_frontmatter_population.py`
  (245 passed).
✓ No byte-for-byte wording remains:
  `rg -n "byte-for-byte" agents/composer.md skills/research/research-topic/SKILL.md skills/research/outputters/citing-once/SKILL.md`
  prints nothing.
✓ The docs lane passes within the full run (`docs:generate:check`,
  `docs:audit:check`). The built `research/index.html` carries
  `#citing-a-source-once` with both merged-tier examples, and its
  `citing-once` links resolve to `reference/skills/internal/citing-once/`.
  The generated skills index lists `citing-once` under internal skills.
✓ Every Phase 3 finding and dossier validates
  (`accelerator corpus frontmatter validate`). The re-run checker covers
  all 36.
- The red-before-green ordering and the guard mutations were not
  re-run. They rest on the plan's "As built" notes.

### Code Review Findings

#### Matches Plan:

- `skills/research/outputters/citing-once/SKILL.md` is the plan's rule file
  word for word: nine part headings named as in the work item, the URL
  table, the 19-key table in `RECOMMENDED_KEYS` order and the worked
  example.
- The worked example holds by hand. All three URLs normalise to
  `blog.example/notifier`, the `tier-3 (retracted)` citation is
  representative, and the block takes `doi` and `openalex_id` from the
  tier-1 citation, `published` from the tier-3 one and `accessed` from the
  tier-2 one, in canonical order.
- `conduct`'s one composer spawn (`research-topic/SKILL.md:440`) injects
  the rule path after the outputter path. The other spawn carrying the
  outputter path (`:413`) is the depth-1 researcher, which the plan
  deliberately leaves unmerged.
- `agents/composer.md` names the rule in its description, task
  instructions, read-first step, step 3 and read guard, and holds no path.
  It still grants only `Read` and `Write`.
- `synthesise` reads the rule path before `write synthesis.md`, merges
  in-finding duplicates and keeps the slot-line instruction.
- The finding outputter documents both `also` examples, scopes the
  copy-unchanged rules to a researcher and hands a composer's blocks to
  **Metadata merge**, **Full ties** and **Block order**.
- The configure warning and the docs page change as planned; the new
  `## Citing a source once` section links the generated contract page and
  holds no copy of the rule.
- The structure tests match the plan, including the replaced byte-for-byte
  test, the deleted one-line test and the rule path in the injection test's
  parameters.
- No contract in `skills/research`, `agents` or the research docs still
  carries tiers verbatim, carries a block byte-for-byte or keeps a citation
  one-line.

#### Deviations from Plan:

All but one are recorded in the plan's "As built" notes.

- The configure warning reads "the injected finding outputter and the
  citing-once rule" (`configure/SKILL.md:281-283`). The planned wording
  lacked the article its own test asserts.
- `test_every_carrier_takes_every_tier_from_its_inputs` zips with
  `strict=True` for ruff B905.
- A separate commit upgrades `pulldown-cmark` from `=0.13.4` to `=0.13.5`
  for RUSTSEC-2026-0414, regenerates the third-party notices and lets
  `golden_digests.rs` record several parser versions per digest recipe.
  `DigestRecipe::CURRENT` stays 1.
- Not recorded in the plan: the same commit's `Cargo.lock` re-resolves
  `tempfile`'s `getrandom` dependency from 0.3.4 to 0.4.3. Both versions
  were already in the lock and `deny:check` passes, so this is benign.
- Phase 2 landed as three commits rather than one, each green alone.

#### Potential Issues:

- `synthesise` read the rule with `Bash` `cat` in 8 of 9 Phase 3 sessions.
  `cat` is not in `research-topic`'s `allowed-tools`, so on a machine whose
  permissions don't already allow it the user gets a permission prompt
  mid-`synthesise`. The research guard does not apply, because it confines
  subagents only. Fixed after validation: `synthesise` now reads the rule
  with the Read tool, and
  `test_synthesise_reads_the_citing_once_rule_with_the_read_tool` failed
  first.
- The recipe guard's `extends` check applies to every history entry, not
  only the latest. A parser version appended to an earlier recipe after a
  later one exists passes as an extension. That is harmless, because only
  the latest entry is checked against the locked version, but it loosens
  the "earlier recipe's history entry was rewritten" guard.
- The rule keeps only `retracted`, `withdrawn` and `also` in merged tier
  text. Any other suffix a profile later prescribes would be silently
  dropped on merge. `test_the_citing_once_rule_merges_every_prescribed_mark`
  catches a new mark only when it is one lowercase word.

### Manual Testing Required:

1. Docs site (the plan's one unticked item):
  - [ ] Run the `docs-site` preview and read `## Citing a source once` on
        the research page. The built HTML's text and links are verified
        above, but the page has not been viewed rendered.

2. Behavioural acceptance (already done, re-checked here):
  - [x] Re-ran the Phase 3 checker over all 36 recorded outputs and logs
        from the implementing session's scratchpad. The fixture pre-check
        passed and all 36 sessions passed every criterion.
  - [x] Read the C5, S2 and S1 outputs directly rather than through the
        checker's oracle. All nine C5 findings carry the same seven entries,
        each with the work item's expected tier text, and eugene-khyst and
        `brandur.org/notifier` with the expected venue and URL. All six S2
        dossiers cite `brandur.org/notifier` once as
        `tier-3 (also tier-2, tier-1)`, with the findings' filename order
        swapped between the two sets. S1's 11 entries match their criteria.

### Recommendations:

- Read the docs section in a preview, then tick Phase 2's last manual item.
- Raise a separate work item for the vitest worker
  `Timeout calling "onTaskUpdate"` error. It fails `test:unit:frontend`
  under load with every test passing, independent of this branch, which
  touches no frontend file.
- Optionally confine the recipe guard's version extension to the latest
  entry.
