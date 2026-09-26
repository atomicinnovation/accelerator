---
type: "codebase-research"
id: "2026-09-26-0283-recursive-finding-deepening"
title: "Codebase seams for 0283 recursive finding deepening"
date: "2026-09-26T01:04:58+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0283"
parent: "work-item:0283"
relates_to: ["codebase-research:2026-09-20-0282-tunable-depth-and-breadth", "codebase-research:2026-09-23-0280-academic-source-profiles"]
topic: "Codebase seams for 0283 recursive finding deepening"
tags: ["research", "codebase", "research-topic", "topic-research", "write-guard", "frontmatter-validation", "config", "agents"]
revision: "de7d0f16e42b4b61a960c5604e60bfcd03487f22"
repository: "accelerator"
last_updated: "2026-09-26T01:04:58+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Codebase seams for 0283 recursive finding deepening

**Date**: 2026-09-26T01:04:58+00:00
**Author**: Toby Clemson
**Git Commit**: de7d0f16e42b4b61a960c5604e60bfcd03487f22
**Branch**: detached working copy on the 0280 research branch (PR #134)
**Repository**: accelerator

## Research Question

For work item 0283 (`meta/work/0283-recursive-finding-deepening.md`): where
does each requirement land in the code as merged through 0280, are the work
item's Technical Notes references still accurate, and what existing
constraints does the design collide with?

## Summary

Every seam 0283 names exists, and the design fits the hexagonal split already
in place. Deterministic derivation goes in `corpus` (`round.rs`), filesystem
reading goes in `corpus-adapters`, confinement goes in `research`, and
orchestration stays in `SKILL.md` prose. The code has no notion of depth,
level, lineage or composition today. Depth exists only as a catalogue default
(`research.topic.depth = "1"`) and as `SKILL.md` prose that clamps it and
prints the dormant notice.

Five collisions with existing validators and conventions are not addressed in
the work item. Each would fail an acceptance criterion if planned as written:

1. **`follow_ups: []` is rejected.** `check_empty_placeholders`
   (`cli/corpus/src/frontmatter_validation/mod.rs:388-400`) flags any `[]`
   value except on `tags`. A pruned or leaf node's note would fail validation,
   `outstanding` would treat it as missing, and the node would be re-spawned
   forever.
2. **Level-note ids collide corpus-wide.** `duplicate_check` keys on
   `type:id`. The id is the `id:` field, falling back to the filename stem
   (`cli/corpus-adapters/src/frontmatter_validation.rs:67-93`). A note whose id
   is its lineage (`1`, `2-1`) collides with every other pair's note, so
   whole-corpus `frontmatter validate` fails with `DUPLICATE-ID`.
3. **Adding `depth` to the finding row makes it required.** Row extras are
   required unless they are listed in the global `OPTIONAL_EXTRAS`
   (`schema.rs:305-312`). A legacy finding would then fail `MISSING-EXTRA`,
   `read_finding` would mark it invalid, and `outstanding` would re-plan its
   pair, contradicting "a legacy finding without `depth` reads as `depth: 1`".
4. **Misplaced flags are ignored, not rejected.** The convention in
   `skills/research/research-topic/SKILL.md:78-80` is that a misplaced flag is
   "ignored with a one-line note". The work item says `--concurrency` on
   `outline` is "rejected as a misplaced flag".
5. **A test helper errors on a subdirectory.** `highest_round` in
   `cli/corpus-cli/tests/frontmatter_goldens.rs:449-468` reads every entry of
   `findings/` with `fs::read_to_string`, so a `.levels/` directory in a
   fixture makes it fail.

Three of the Technical Notes references need correcting:

- The guard's agent identification is `Researchers::identify` in
  `cli/research/src/confinement.rs:74`, not `cli/research-cli/src/guard.rs:74`.
- The `public-api.txt` snapshots that change are `corpus` and `research`, not
  `config`.
- The researcher's tool list includes `Bash`, pinned by
  `research_agent_contract.rs`.

## Detailed Findings

### `outstanding`: planner and adapter

The domain type `Round::plan(&RoundInputs)` (`cli/corpus/src/topic_research/round.rs:304-309`)
drives `Planner`. `Planner::new` (`:325-351`) builds an `answered` list from
retained findings. It also computes `highest` as the maximum `index_of` over
every finding name, retained or invalid, and every marker name
(`:337-344`).

`plan_item` (`:377-415`) classifies each eligible (question, profile) as
answered or outstanding. It allocates one shared index per focus area, on
the question's first sighting only (`:379`, `:398-413`). It emits
`findings/{index:02}-{slug}-{profile}.md` (`:407-410`).

`index_for` (`:434-459`) picks the index in this order:

1. The lowest index among retained findings for the question.
2. Otherwise, the lowest index among quarantine markers whose frontmatter
   `question` matches.
3. Otherwise, `next_index`.

`normalised` (`:292-294`) is private. It collapses whitespace and does not
fold case.

The output types are fixed today:

- **`Round`** is `{ items, pairs, skipped, warnings }` (`:213-218`). A `Pair`
  is `{ question, profile, path }` (`:232-236`). It has no
  "awaiting composition" category and no per-node list.
- **`RoundInputs`** (`:201-209`) has no depth and no level-note data.
- **`Finding`** (`:150-180`) carries only `name` and an optional
  `(question, profile)`. It carries no `round` or `depth`.

The adapter is `read_round_inputs` / `read_findings`
(`cli/corpus-adapters/src/topic_research.rs:70-112`). It lists `findings/`
one level deep and sorts the names. `.<x>.invalid` names become markers, and
non-dot `.md` names go to `read_finding` (`:119-140`). That function calls
`validate_path` and requires string `question` and `source_profile` values.
Everything else, including a `03-a-web.levels` directory, is ignored
silently. `DirReader::list` (`cli/corpus/src/scan.rs:9-27`, `RealFs` in
`fs.rs:20-58`) returns files and directories unfiltered, so the adapter can
list `<stem>.levels/` through the existing port.

The CLI side:

- `TopicResearchAction::Outstanding { slug, profiles_dir }` is defined at
  `cli/corpus-cli/src/cli.rs:51-70`. It has no `--depth` flag. For numeric
  flags, the precedent is `AdrAction::NextNumber.count` (`:76-83`), a raw
  `String` validated by hand so that bad input exits 1 rather than clap's 2.
- Dispatch is at `cli/corpus-cli/src/main.rs:136-154`. JSON rendering is at
  `cli/corpus-cli/src/topic_research.rs:20-74`, which makes each path
  absolute (`:51`). The contract is "fields are only ever added".
  `conduct` ignores fields it does not name (`SKILL.md:232-236`), so new
  categories are forward-compatible.
- Black-box tests are in `cli/corpus-cli/tests/topic_research_outstanding.rs`.
  They include `every_allocated_path_is_a_finding_path_the_guard_admits`
  (`:312-341`), which ties the planner to the guard predicate.

For 0283, missing-node derivation, the level barrier, the cap trim and
dedupe, and the composition-only category all belong in `round.rs` as pure
functions over new `RoundInputs` data. `read_findings` gains a per-stem
`.levels/` reader. `index_of` already parses `03-a-web.levels` as `3`, but
the adapter never passes directory names to the planner, so index retention
needs a new input to `highest` and `index_for`, keyed on the `question` of
the `1.md` or `.1.md.invalid` note.

### Write guard

The guard's data flow for a write:

```text
hooks.json PreToolUse (Bash | Write|Edit|MultiEdit|NotebookEdit)
  -> bin/accelerator research guard --fail-safe --non-blocking
  -> research-cli guard::run            (guard.rs:50-83)
       Project::researchers             (guard.rs:170-184, catalogue::agent_name)
       confined_researcher              (research/src/confinement.rs:74-113)
       locate(path, cwd, topics)        (write_target.rs:18-80)
       decide(&action, &CorpusFindings) (confinement.rs:212-228)
         CorpusFindings::contains -> corpus::topic_research::is_finding_path
```

- **Path predicate.** `is_finding_path`
  (`cli/corpus/src/topic_research/finding_path.rs:5-15`) accepts exactly
  `[set, "findings", name.md]` with no leading dots. It does not require the
  `<nn>-` index (`s/findings/x.md` passes). A `.levels/` path has four
  segments and is refused. The 0283 rules therefore need two new shapes:
  - a researcher level-note shape, which checks the index and the lineage;
  - a composer finding shape, which requires the `<nn>-` stem. Whether the
    researcher's finding rule should also require the index is a decision
    for the plan.
- **Agent identity.** `Researchers::identify` (`confinement.rs:74-83`)
  recognises `accelerator:researcher` and the configured
  `agents.researcher` only. There is no other agent-type rule anywhere in
  the codebase.
- **Researcher-specific wiring.** Four pieces are hard-wired to the
  researcher:
  - `FindingsScope` is the single write-scope seam (`confinement.rs:141-144`),
    and `CorpusFindings` is passed to it unconditionally (`guard.rs:91`).
  - The `Command` arm always applies the rule that only
    `accelerator research fetch ` may run.
  - The refusal text hard-codes the scope `<set>/findings/<name>.md`
    (`confinement.rs:440-442`).
  - The `Researcher` enum names the role.

  Adding a composer role means varying the identity, the scope and the
  refusal text by role.
- **Write-target resolution.** `locate` rejects `.`/`..` components and
  symlinks below the canonical topics directory. It lets a first write
  through when directories are missing, because `NotFound` ends the walk. A
  first write into a new `<stem>.levels/` directory is therefore already
  admitted structurally.
- **Tool grants.** The tests include `the_researcher_agent_grants_a_bounded_tool_set`
  (`cli/corpus-adapters/tests/research_agent_contract.rs:36-57`), which pins
  `agents/researcher.md` to `WebSearch, WebFetch, Write, Read, Bash`. A
  composer equivalent would pin `Read, Write`. Because the composer has no
  `Bash` grant, the `Command` arm never fires for it.
- **Existing tests to extend:**
  - table-style predicate tests in `finding_path.rs:21-61`;
  - domain tests in `cli/research/tests/confinement.rs`, including
    `every_write_refusal_names_its_cause` (`:325-361`), whose prefix contains
    `<set>/findings/<name>.md`;
  - end-to-end tests in `cli/research-cli/tests/guard.rs`, with a `Project`
    builder and `assert_blocks(substring)`.

  `every_call_not_from_a_researcher_passes` (`:333`) uses
  `accelerator:reviewer`, so a composer rule does not contradict it.

### `conduct`, researcher, outputter and templates

`skills/research/research-topic/SKILL.md` is the only file in the skill. The
`conduct` steps (`:218-312`):

1. **Plan.** Runs `outstanding` (`:226-237`).
2. **Clear.** Quarantines any existing pair path (`:239-242`).
3. **Depth.** Resolves depth and prints the dormant notice (`:244-246`, notice
   text at `:82-86`).
4. **Spawn.** Spawns every pair at once with the Task tool (`:248-268`), with
   no batching. The researcher's `subagent_type` is resolved inline with
   `` !`accelerator config agent researcher --fail-safe` `` (`:249`).
5. **Check the outcome.** Validates each write and quarantines invalid ones
   (`:270-279`).
6. **Tick.** Re-runs `outstanding` and ticks the outline from disk
   (`:281-284`).
7. **Update the manifest.** Writes `round_count` / `finding_count`
   (`:286-296`), using the disk-count rule at `:436-442`.
8. **Summarise.** Emits a reason-to-next-step table (`:298-312`).

Knob resolution works as follows:

- **Reading.** Values are read by `!` preprocessor at `:59-60`. Precedence is
  flag > personal > team > built-in default, and the flag step is applied in
  prose because the preprocessor cannot see invocation arguments.
- **Clamping.** Clamping and the warning
  `Warning: research.topic.<knob> must be a positive integer, got '{value}' — clamping to 1`
  live only in prose (`:66-80`). No Rust code clamps.
- **Empty values.** An empty value stops the verb. That is why
  `research.topic.concurrency` needs a catalogue default of `24`: without
  one, `config get` prints nothing.

The other pieces:

- **`agents/researcher.md`.** Its tools are `WebSearch, WebFetch, Write,
  Read, Bash`. It returns "a short summary — two or three sentences" and
  stamps no `depth`.
- **Outputters.** Only `skills/research/outputters/finding-outputter/SKILL.md`
  exists. Its sections are Sink, Injected Values, Shape and Body, and it is
  injected by absolute path (`SKILL.md:254`). A level-note outputter would
  mirror it and add `level`, `depth` and `follow_ups` as injected values.
- **Finding template.** `templates/topic-research-finding.md` carries `round`,
  `question` and `source_profile`, with body sources formatted as
  `- [Title](url) — tier-N — {domain/venue}`. It has no `depth`.
- **Profiles.** Three live at `skills/research/profiles/{web,openalex,arxiv}-profile/SKILL.md`.
  They are injected by path, which gives profile confinement per node for
  free.

### Config catalogue and agent key

- **Catalogue entries.** The research knobs are
  `RESEARCH_KEYS` (`cli/config/src/catalogue.rs:183-186`) and the agents are
  `AGENT_KEYS` (`:214-225`). `config agent composer` already resolves to
  `accelerator:composer` without a catalogue entry (`agent_name`, `:287-296`).
  Without the entry, though, `composer` is absent from the "Agent Names"
  block, and a user override of it is warned as unknown.
- **Fixture churn** from adding `research.topic.concurrency` and `composer`:

  | Fixture | Test | Change |
  |---|---|---|
  | `catalogue.rs:307-318` | `the_catalogue_holds_sixty_five_keys_across_seven_groups` | 65 → 67, rename |
  | `cli/launcher/tests/fixtures/dump/dump.golden` | `dump_matches_the_committed_golden` | +2 rows |
  | `cli/launcher/tests/fixtures/agents/agents.golden` | `agents_matches_the_committed_golden` | +1 line |
  | `cli/launcher/tests/fixtures/baseline/agents.golden` | `agents_against_the_baseline_matches_its_golden` | +1 line |
  | `cli/config-adapters/tests/parity.rs:81-87` | `the_research_knobs_resolve_personal_over_team` | optional pin |
  | `cli/config/tests/fixtures/public-api.txt` | — | no change (entries, not items) |

- **Documentation.** `skills/config/configure/SKILL.md` needs these edits:
  - the research table (`:324-327`);
  - "both are positive integers" (`:320-322`);
  - the depth caveat (`:336-338`), which reads "has no behavioural effect
    yet" and contains no literal "0283";
  - the agents table (`:135-146`);
  - the hook-confinement note (`:166-171`).

  Two stale "nine agents" references already miss `researcher`: in
  `docs-site/src/content/docs/reference/agents.md` and in
  `cli/launcher/src/config_command/core/agents.rs:1`.

### Frontmatter schema and walkers

- **Two row definitions.** Each `(type, kind)` row is defined twice, and
  tests keep the copies in step:
  - `SCHEMA: [SchemaRow; 18]` (`cli/corpus/src/frontmatter_validation/schema.rs:19`,
    topic-research rows at `:189-239`);
  - `templates-schema.tsv:15-19`.

  Each TSV row names a template that must exist. The template list is
  cross-checked against the Schema Reference tables of work items 0065, 0066
  and 0067. The topic-research rows are listed in
  `meta/work/0065-update-artifact-templates-to-unified-schema.md:83-87`.
- **Adding `level-note`** therefore touches:
  - the `SCHEMA` length (18 → 19) and the tests `eighteen_rows_are_present`
    and `topic_research_kinds_each_resolve_to_a_distinct_row`;
  - a TSV row, plus a new `templates/topic-research-level-note.md`;
  - the 0065 table;
  - `cli/corpus/tests/fixtures/public-api.txt:128`.

  `config template topic-research --kind level-note` resolves generically
  already. Making the template user-overridable also touches `TEMPLATE_KEYS`,
  `dump.golden` and the visualiser's `template-tier.ts`.
- **Kind and type resolution.** The row is chosen from the file's own `type:`
  and `kind:` fields, never from its path. There is no default row for
  `topic-research`, so a missing row gives `UNKNOWN-KIND`.
- **Value scanning.** The validator scans raw lines, not YAML. Its rules:
  - flow lists only;
  - every element double-quoted;
  - bare integers pass, so `level: 2` and `depth: 3` are fine.
- **Template-to-row contract.** `the_finding_template_matches_the_finding_schema_row`
  (`research_agent_contract.rs:328-353`) asserts that the template fields
  equal the base fields plus the row extras. `depth` must therefore appear in
  both the row and the template, or in neither.
- **The walk.** `RealFs::walk_markdown` (`cli/corpus-adapters/src/fs.rs:73-113`)
  recurses with no dot-skip and keeps `*.md` files (case-sensitive), so notes
  are validated. `.x.md.invalid` markers are skipped because of their
  extension.
- **Migrations.**
  - m0008 (`cli/migrate/src/migrations/m0008.rs:90-150`) re-renders and
    validates every `.md` recursively. An invalid or unregistered note
    aborts it.
  - m0007 skips topic-research rewrites but gates on full validation
    (`m0007/mod.rs:189-196`, `:332-335`).
- **Visualiser.** `cli/visualiser/server/src/file_driver.rs:258-321` indexes
  only `<set>/manifest.md`, and `indexer.rs:2012-2060` asserts this. The
  recursive watcher (`watcher.rs:62-73`) rescans on `.md` events inside
  `.levels/`, which adds churn but no new entries.

## Code References

- `cli/corpus/src/topic_research/round.rs:201-218` — `RoundInputs` and `Round`; new level-note inputs and output categories go here
- `cli/corpus/src/topic_research/round.rs:292-302` — `normalised` (private) and `index_of`
- `cli/corpus/src/topic_research/round.rs:325-351` — `Planner::new`, the `highest` reservation
- `cli/corpus/src/topic_research/round.rs:377-459` — `plan_item` and `index_for`
- `cli/corpus-adapters/src/topic_research.rs:92-140` — `read_findings` and `read_finding`, flat listing plus validation
- `cli/corpus-cli/src/cli.rs:51-83` — the `Outstanding` clap definition and the hand-validated numeric flag precedent
- `cli/corpus-cli/src/topic_research.rs:20-74` — JSON rendering
- `cli/corpus-cli/tests/topic_research_outstanding.rs:312-341` — planner and guard coupling test
- `cli/corpus/src/topic_research/finding_path.rs:5-19` — `is_finding_path`
- `cli/research/src/confinement.rs:74-113,141-144,212-228,417-492` — identity, `FindingsScope`, `decide`, refusal text
- `cli/research-cli/src/guard.rs:85-103,170-196` — judging, researcher resolution, topics directory
- `cli/research-cli/src/write_target.rs:18-89` — `locate` and `CorpusFindings`
- `cli/corpus-adapters/tests/research_agent_contract.rs:36-57,328-353` — tool-grant and template-row pins
- `skills/research/research-topic/SKILL.md:24-28,59-86,218-312,436-442` — preprocessor lines, knobs, dormant notice, `conduct`, count rule
- `skills/research/outputters/finding-outputter/SKILL.md` — the outputter to mirror
- `templates/topic-research-finding.md` — the finding template (no `depth`)
- `agents/researcher.md:1-8` — agent frontmatter shape
- `cli/config/src/catalogue.rs:183-186,214-225,287-318` — research and agent keys, `agent_name`, count test
- `skills/config/configure/SKILL.md:135-171,318-352` — agent and research documentation
- `cli/corpus/src/frontmatter_validation/schema.rs:19,189-239,305-312` — `SCHEMA`, topic-research rows, `OPTIONAL_EXTRAS`
- `cli/corpus/src/frontmatter_validation/mod.rs:388-400` — `check_empty_placeholders`
- `cli/corpus-adapters/src/frontmatter_validation.rs:67-124` — id resolution and `build_index`
- `cli/corpus-adapters/src/fs.rs:73-113` — recursive walk
- `cli/corpus-cli/tests/frontmatter_goldens.rs:449-485` — `highest_round` / `visible_finding_count` helpers

## Architecture Insights

- **The layering fits the recursion.** 0283's "content-free orchestration"
  continues ADR-0052 (the filesystem as message bus) and ADR-0045/0053 (a thin
  CLI over a hexagonal core). `outstanding` already makes `conduct`
  disk-derived and resume-equivalent, and 0283 extends the same verb rather
  than adding a new one.
- **The planner stays pure.** It sees only `RoundInputs`, so cap halving,
  over-cap trim, lineage-order dedupe, the level barrier and index retention
  can all be unit-tested with hand-built inputs, as the existing `round.rs`
  tests do. The trim warning maps naturally onto a new `Warning` variant,
  rendered on stderr per the work item. Note that current warnings go to the
  JSON `warnings` array, not stderr.
- **Confinement is single-role today.** A second confined role makes this
  the point to generalise `Researcher` / `Researchers` into a role-keyed
  identity with a scope per role, rather than bolting on a parallel
  `Composers` type.
- **Schema rows are the real type system.** Every field decision (`depth`,
  `follow_ups`, `level`, `id`) is decided by the row's extras, the global
  `OPTIONAL_EXTRAS`, and the empty-placeholder and quoting scanners, not by
  YAML types.

## Historical Context

- `meta/plans/2026-09-20-0282-tunable-depth-and-breadth.md` — `config get`
  became catalogue-aware. Clamping lives in prose. Three prose sites must
  flip when depth goes live. The misplaced-flag rule is "ignore with a note".
  User-facing text must not name "0283".
- `meta/plans/2026-09-23-0280-academic-source-profiles.md` (Phases 7–8) — the
  guard's design and accepted residuals: a researcher can overwrite sibling
  findings; the guard fails open if its binary is missing; its budget is 50ms
  p95 per hook call. It also covers the `outstanding` JSON contract and the
  index allocation rules.
- `meta/validations/2026-09-23-0280-academic-source-profiles-validation.md` —
  0280 is partially validated. The phase-10 gate that unblocks 0283 has not
  run.
- `meta/reviews/plans/2026-09-19-0279-iterative-accretion-and-finalise-review-1.md`
  (~l.285-291, ~l.771) — flagged earlier that the scan assumes a flat
  `<nn>-*.md` layout, and that every finding at any depth needs a `round`
  stamp.
- `meta/work/0280-academic-source-profiles.md` §Sibling contracts — recursion
  carries `source_profile` into deeper levels, and the cost model includes
  the per-profile multiplier.

## Related Research

- `meta/research/codebase/2026-09-20-0282-tunable-depth-and-breadth.md`
- `meta/research/codebase/2026-09-23-0280-academic-source-profiles.md`
- `meta/research/codebase/2026-09-08-0277-single-round-web-research-engine.md`

## Open Questions

- ❓ **`follow_ups: []`.** Options:
  - exempt `follow_ups` in `check_empty_placeholders`, alongside `tags`;
  - make it a per-row optional extra, and have a leaf omit it.

  The work item's semantics ("`[]` means pruned") favour the exemption.
- ❓ **Level-note `id`.** It must be unique corpus-wide. Precedent: manifests
  moved to slug-keyed ids (`frontmatter_goldens.rs:864-896`). One candidate is
  `<set-slug>/<stem>/<lineage>` or a flattened equivalent. Finding ids (the
  stem) already risk collision across sets; that is out of scope here but
  worth checking.
- ❓ **Optional `depth` on findings.** `OPTIONAL_EXTRAS` is global and is
  emitted by `frontmatter print-schema`. Adding `depth` there is the smallest
  change; a per-row optional list is cleaner.
- ❓ **`--concurrency` on `outline`.** Should it follow the existing "ignore
  with a note" rule, or does 0283 deliberately tighten it to rejection? The
  work item and one acceptance criterion say "rejected".
- ❓ **The researcher's finding rule.** Should it start requiring the `<nn>-`
  index to match the composer rule? Today `s/findings/x.md` passes.
- ❓ **The trim warning channel.** The work item says stderr. Existing
  planner warnings go to the JSON `warnings` array, which `conduct` surfaces.
  Stderr keeps stdout pure JSON, but `conduct` must then capture stderr.
- ⏱️ **Guard load.** The guard budget is 50ms p95 per hook call, and
  recursion multiplies hooked calls by up to 13× per pair at `depth: 3`. The
  lean-binary split noted in 0280 was never measured.
- **Unchecked.** I did not verify the `lint:dispatch-coherence` and
  `test:integration:skill-invocation` implications of adding a
  `config get research.topic.concurrency` preprocessor line and a
  `--depth` argument to `outstanding` in `SKILL.md`.
