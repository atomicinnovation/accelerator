---
type: "plan-review"
id: "2026-09-26-0283-recursive-finding-deepening-review-1"
title: "Plan Review: Recursive Finding Deepening Implementation Plan"
date: "2026-09-26T20:50:42+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-26-0283-recursive-finding-deepening"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "security", "compatibility", "usability", "performance"]
review_number: 1
review_pass: 6
tags: ["research", "deep-research", "cli", "hooks"]
last_updated: "2026-09-27T12:41:02+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Recursive Finding Deepening Implementation Plan

**Verdict:** REVISE

The Rust core is strong. The tree derivation is pure and specified test by test, `Lineage` is a strict canonical value type, confinement generalises cleanly behind a `TopicLayout` port, and the build order keeps `depth: 1` intact until Phase 6. The weaknesses are at the seams between the planner, the adapter and the prose-orchestrated `conduct` loop. "Valid level note" has two owners, and so the loop can re-spawn without bound. Notes are joined to candidates by lineage alone. Index retention depends on a valid root note. Untrusted follow-up text is promoted into trusted task input. There are no critical findings, but 14 major findings after deduplication put the plan well over the revise threshold of 3.

### Cross-Cutting Themes

- **Two definitions of a valid level note** (flagged by: architecture, code-quality, correctness, test-coverage, performance). `conduct` checks spawns with `frontmatter validate`, while the adapter additionally requires `kind: level-note`, `level == lineage.level()` and a string-list `follow_ups`, and `is_answered` requires matching question and profile. A file that passes the first check but fails the second is re-reported by `outstanding`, is never marked failed, and is re-spawned forever. It also drops silently, with no diagnostic.
- **The CLI should own level-note identity** (flagged by: usability, architecture). `outstanding` emits `path` but not the corpus-unique id `<set-slug>.<stem>.<lineage>`, so the orchestrator must assemble the id itself. This contradicts the principle that the CLI allocates and the prose routes.
- **Depth going live needs a signal at the point of use** (flagged by: usability, compatibility). Removing the dormant notice means an existing `depth: 3` in team config silently becomes up to 13× the spawns per pair after upgrade, and nothing is printed when `conduct` starts.
- **Untrusted note content crosses trust boundaries** (flagged by: security, performance). Follow-ups become the next node's focus question. Questions and `known_questions` flow unbounded into the unconfined orchestrator's context after every batch.
- **Trims as a second warning channel** (flagged by: usability, architecture, correctness). Trims go to stderr, while other warnings sit in the JSON `warnings`. Because the loop re-runs `outstanding` after every batch, each trim is collected once per iteration.
- **Dual optionality and weak module homes** (flagged by: architecture, code-quality). Global `OPTIONAL_EXTRAS` sits beside per-row `optional_extras` with no rule for choosing between them. `normalised` moves into `tree.rs` although it concerns question equality. `TopicLayout` models two mutually exclusive path kinds as two booleans. Directory detection relies on `fs.read` returning `None`.

### Tradeoff Analysis

- **Security vs simplicity of the path-shape guard**: security wants `conduct` to diff `findings/` around each batch and quarantine unassigned writes. The work item deliberately accepts a prompt contract for cross-pair writes. Recommendation: keep path-shape confinement, but record the residual risk explicitly in What We're NOT Doing, and fix the cheap part (overflow of `highest + 1` from a crafted `.levels` name).
- **Transparency vs no human checkpoint**: usability and compatibility want a cost notice when depth > 1. The work item forbids pausing between levels. An informational line with no confirmation satisfies both.
- **Performance vs domain shape**: performance wants `known_questions` rendered once per pair rather than per node. Keep the domain `Node` shape and change only the rendering if the cost is judged material. At depth ≤ 3 the duplication is modest.

### Findings

#### Critical

None.

#### Major

- 🟡 **Correctness / Architecture / Code Quality**: The conduct loop can re-spawn forever when output validates but `outstanding` still reports it
  **Location**: Phase 4 §1 (`read_levels_directory`); Phase 6 §4 steps 5–6
  Step 5 checks spawns with frontmatter validation only, while `outstanding` applies stricter acceptance for notes (`kind`, `level`, `follow_ups` shape) and for findings (normalised question, exact profile). A file that fails only the stricter check is neither failed nor accepted, so it is quarantined and re-spawned without bound, and nothing reports why.
- 🟡 **Correctness**: Notes are matched to candidates by lineage alone, and `Complete` may include notes outside the tree
  **Location**: Phase 3 §1 (`LevelNote`, `derive` steps 2–3)
  After `1.md` is rewritten with different follow-ups, stale `2-k.md` notes satisfy the new candidates. The literal definition of `Complete` also sweeps in orphan notes, for example at dedupe-skipped or beyond-cap positions, and hands them to the composer.
- 🟡 **Correctness**: Index retention is lost when `1.md` is present but rejected and not yet quarantined
  **Location**: Phase 3 §1 (`root_question`); Phase 4 §1
  With `root_question` set to `None`, `index_for` misses while `highest` still counts the directory, so the pair restarts under a new stem and the old tree is orphaned. The root question should be read from `1.md` whenever it parses, whatever its validity.
- 🟡 **Security**: Follow-ups launder prompt injection into child researchers' trusted focus question
  **Location**: Phase 6 §2 (level-note outputter), §4 step 4
  A poisoned follow-up that embeds a URL becomes a focus question, and the Untrusted-Content Contract then permits fetching it. No human sits between levels.
- 🟡 **Security**: Untrusted node text reaches the unconfined orchestrator unbounded
  **Location**: Phase 4 §2; Phase 6 steps 2 and 7
  `question` and `known_questions` reach the main thread with no bounds on length or control characters. Stems in trim lines can carry newlines that forge `warning:` lines.
- 🟡 **Security**: The path-shape-only guard lets any researcher tamper with other pairs' trees and with planner state
  **Location**: Phase 2 §2–3; Phase 3 §2
  A compromised node can prune another pair's tree, steer `index_for`, or create `4294967295-x.levels` and overflow `highest + 1`.
- 🟡 **Usability / Architecture**: Level-note ids are assembled by hand rather than emitted by `outstanding`
  **Location**: Phase 4 §2; Phase 6 §4 step 4
  Add an `id` field to each `deepen` node, derived next to `Pair::level_note_path`.
- 🟡 **Usability / Compatibility**: An existing depth above 1 starts deep recursion on upgrade, with no notice at the point of use
  **Location**: Phase 6 §4–5; Migration Notes
  "No migration" misses users who set the dormant knob. Replace the dormant notice with an informational fan-out line.
- 🟡 **Usability**: A deeper re-run over already-composed pairs silently does nothing
  **Location**: Phase 6 §4 step 7; Phase 7 step 12
  This is the most natural way to try the feature, and it reads as a bug. Report pairs whose stamped depth is below the resolved depth.
- 🟡 **Compatibility**: The Python conformance suite reads `templates-schema.tsv` by position and is not updated
  **Location**: Phase 1 §2
  `test_conformance.py:_schema()` unpacks `fields[:8]`, so a column inserted after `extras` shifts `vocab`, `forbidden` and `linkkeys`. This was verified against the source.
- 🟡 **Performance**: Orchestrator context grows with every re-plan and every spawn prompt
  **Location**: Phase 6 steps 2, 4, 6; Performance Considerations
  At breadth 8, 3 profiles and depth 3 there are about 336 Task prompts and about 14 re-plans of the full JSON in one conversation.
- 🟡 **Performance**: A global cap of 24 lets recursion flood the serialised arXiv lock, turning waits into failures
  **Location**: Phase 5; Phase 6 step 4
  Beyond roughly 30 queued requests per 100 s budget at 3 s spacing, fetches fail with `lock_contention` and nodes are recorded as failed.
- 🟡 **Test Coverage**: The acceptance criterion demands unit tests for concurrency clamping, but the plan verifies it only by hand
  **Location**: Testing Strategy; Phase 5 §4
  Either amend the criterion alongside the misplaced-flag change or add a static `SKILL.md` contract test.
- 🟡 **Test Coverage**: Only two text-presence checks guard the prose that turns the feature on
  **Location**: Phase 6 Tests first
  Nothing asserts `--depth` on the `outstanding` call, `--kind level-note`, `level-note-outputter` routing, or the `.levels/` count exclusion.

#### Minor

- 🔵 **Test Coverage**: No test pins how notes at lineages no candidate occupies are treated (Phase 3). This overlaps the lineage-join correctness finding.
- 🔵 **Test Coverage**: Adapter rejection conditions are partly untested, and the test location is undecided (Phase 4 §1).
- 🔵 **Test Coverage**: Deepened-set goldens could pass without any note being read (Phase 1 §6).
- 🔵 **Test Coverage**: Guard escape and fail-closed tests are not carried over to `.levels/` or the composer (Phase 2 §4).
- 🔵 **Test Coverage**: Index retention tests skip multi-profile pairs, root-less directories and the minimum rule (Phase 3 §2).
- 🔵 **Correctness**: `--depth -1` will be rejected by clap with exit 2, not 1. `cli.rs` sets no `allow_hyphen_values` (Phase 4 §2).
- 🔵 **Correctness**: Note quarantine omits the rule against overwriting an existing marker (Phase 6 §4 steps 3, 5).
- 🔵 **Correctness**: `known_questions` is ambiguous at the barrier level, and the `3-1-1` fixture cannot reach level 3 as described (Phase 3 §1).
- 🔵 **Correctness / Usability / Architecture**: Trim warnings repeat on every loop iteration and use a second channel (Phase 4 §2; Phase 6 steps 2, 7).
- 🔵 **Security**: The composer's lack of fetch and its read scope rest only on its tool list (Phase 6 §1; Phase 2 §3).
- 🔵 **Usability**: The failure summary is unspecified for composer failures and new guard refusals (Phase 6 steps 5, 7).
- 🔵 **Usability**: There is no per-batch progress feedback during multi-batch runs (Phase 6 step 6).
- 🔵 **Usability**: A name configured for both roles is silently confined as the researcher (Phase 2 §3).
- 🔵 **Usability**: The level-note template is the only topic-research template users cannot override (What We're NOT Doing).
- 🔵 **Compatibility**: Older plugin versions misread deepened sets in a shared corpus (Migration Notes).
- 🔵 **Performance**: `known_questions` is repeated per node, so the payload is quadratic per tree (Phase 3 §1; Phase 4 §2).
- 🔵 **Performance**: Batches are fully synchronous and have no fill order (Phase 5 §2; Phase 6 step 4).
- 🔵 **Architecture / Code Quality**: The lineage/level consistency rule lives in the adapter, not the domain (Phase 4 §1).
- 🔵 **Architecture / Code Quality**: Directory detection depends on the incidental behaviour of `fs.read` returning `None` (Phase 4 §1).
- 🔵 **Architecture / Code Quality**: Two optional-extras mechanisms coexist with no stated distinction (Phase 1 §1).
- 🔵 **Code Quality**: `plan_item` and `index_for` gain branches with no planned extraction (Phase 3 §2).
- 🔵 **Code Quality**: Tuple returns and mixed `usize`/`u32` appear in the new domain API (Phase 3 §1; Phase 4).

#### Suggestions

- 🔵 **Architecture / Code Quality**: Move `normalised` to a `question` module or `Question` type, not `tree.rs`.
- 🔵 **Architecture / Code Quality**: Replace `TopicLayout`'s two booleans with `classify() -> Option<TopicFile>`.
- 🔵 **Code Quality**: The level-note outputter should reference finding-outputter's body contract, not copy it.
- 🔵 **Correctness / Test Coverage**: `Lineage::child(0)` builds a non-canonical lineage. Use `NonZeroU32`.
- 🔵 **Test Coverage**: Pin one full `deepen` node object by whole-JSON equality, and add an overflow case to the invalid-depth test.
- 🔵 **Test Coverage**: Boundary tests for `Depth::new(0)`, `check_extras` missing an optional extra, and `follow_ups: ""`.
- 🔵 **Usability**: Document that hand-run `outstanding` defaults `--depth` to 1 and does not read config.
- 🔵 **Performance**: Double validation per note per batch. Let `outstanding` report rejected notes.
- 🔵 **Compatibility**: Record in the release notes that `is_finding_path` now narrows to indexed stems.

### Strengths

- ✅ Tree derivation is a pure function over hand-built notes, with one test per work-item example: cap halving, trim-before-dedupe, stable positions, no backfill, and the per-pair barrier.
- ✅ `Lineage` is canonical-only, round-trips, and orders numerically (`2-2 < 2-10 < 3-1-1`).
- ✅ Confinement generalises through a `TopicLayout` port. The composer gets default-deny commands and a narrower write scope. `is_finding_path` narrows to indexed stems.
- ✅ Resume and fresh runs share one path, because `outstanding` re-derives from disk after every batch.
- ✅ The `outstanding` JSON changes only add fields, `--depth` defaults to 1, and `depth` is optional on the finding row, so legacy findings still validate.
- ✅ The build order goes inside out, and every phase ends with `mise run` green while `depth: 1` stays unchanged until Phase 6.
- ✅ `every_allocated_level_note_path_is_one_the_guard_admits` ties the planner to the guard.
- ✅ Error codes, role-specific refusal text and the failure next-step (a smaller `--depth`) are actionable.

### Recommended Changes

1. **Give one owner to "this file counts", and make the loop terminate** (addresses: the unbounded re-spawn finding, the lineage/level rule in the adapter, double validation)
   Add `LevelNote::from_frontmatter(...) -> Result<LevelNote, NoteRejection>` in `corpus`. Have `outstanding` report rejected notes and their reasons in its output. In `conduct` step 6, treat any node or pair that `outstanding` reports again after a successful spawn as failed, with reason "wrote a file `outstanding` does not accept".
2. **Join notes to candidates by question, and define `Complete` exactly** (addresses: the lineage-only join, orphan notes, the missing orphan-note test)
   Carry `question` on `LevelNote`. A note whose normalised question differs from its candidate's is treated as missing. `Complete` holds exactly the reached candidates that have notes. Add tests for mismatched, dedupe-skipped and beyond-cap notes.
3. **Read the root question from any parseable `1.md`** (addresses: lost index retention, the marker-overwrite finding)
   Fall back to `.1.md.invalid` and its suffixed forms. Specify that note quarantine uses the same no-overwrite rule as findings. Add a test for an unquarantined invalid `1.md` keeping its index.
4. **Emit `id` on each `deepen` node** (addresses: hand-assembled level-note ids)
   Derive it beside `Pair::level_note_path`, assert it in the CLI tests, and have `conduct` inject it verbatim.
5. **Bound and sanitise untrusted note text** (addresses: prompt-injection laundering, unbounded orchestrator input, tampering via crafted `.levels` names)
   In the adapter, reject follow-ups and questions that are multi-line, contain control characters or a URL, or exceed a length bound. Restrict the `.levels` stem to the slug alphabet. Use `checked_add` in the planner for `highest + 1`. Add to the level-note outputter and the deepen prompt that node questions are plain research questions, and that the profile, not the question, decides which sources are legitimate. Record the residual risk of cross-pair writes in What We're NOT Doing. Add an attended step that seeds an adversarial `1.md`.
6. **Add a point-of-use depth notice and a migration note** (addresses: silent deep recursion on upgrade, the silent deeper re-run, cross-version corpora)
   Replace the dormant notice with an informational line when depth is above 1, giving the per-pair multiplier, the pair count and the concurrency. In the summary, name pairs composed at a smaller depth that are not re-deepened. Add Migration Notes entries covering a configured `depth` above 1 going live, and deepened sets needing every collaborator on this version.
7. **Fix the TSV column placement** (addresses: the positional read in `test_conformance.py`)
   Either append `optional_extras` as the last column or switch `_schema()` to read columns by header name. List `test_conformance.py` in Phase 1.
8. **Add static contract tests for the Phase 6 prose, and settle the clamping criterion** (addresses: the two Test Coverage majors)
   In `research_agent_contract.rs`, assert that `--depth` appears on the `outstanding` call, that `--kind level-note` is used, that `level-note-outputter` is routed, and that `.levels/` is excluded from the count. Also assert the absence of the whole dormant sentence and that the knob rules name `concurrency`. Amend the work item's clamping acceptance criterion in Phase 5.
9. **Address orchestrator scale and arXiv contention** (addresses: the two Performance majors, the fill-order finding)
   Specify a batch fill order: deepen nodes before composers, and shallower levels first. Cap how many arXiv spawns share one batch. Consider an option to limit `outstanding` output to ready spawns. Add token volume and arXiv contention to Performance Considerations and to a Phase 7 step.
10. **Tidy the minor items** (addresses: the remaining minor findings)
    Add `allow_hyphen_values` on `--depth`, and deduplicate trim warnings across iterations. Add the guard escape tests for `.levels/` and the composer, the retention tests, and a negative golden. Name composer failures as `<stem> (composer)`. Add a warning when both roles share a name. Resolve the `known_questions` barrier wording and fix the `3-1-1` fixture.

## Per-Lens Results

### Architecture

**Summary**: The plan follows the existing hexagonal layering and makes `outstanding` the single source of truth for what gets spawned next. The main structural weakness is that "a valid level note" is defined in two places, and that gap can stall the loop. The note id scheme and the note-consistency rules also sit outside the functional core.

**Strengths**: pure `tree::derive`; resume and fresh runs share one path; the `TopicLayout` port preserves dependency direction; the composer's lack of fetch is enforced twice; value types carry domain vocabulary; the inside-out build order.

**Findings**:
- 🟡 major / high — Phase 4 §1, Phase 6 steps 5–6 — **Two definitions of a valid level note can stall or loop conduct.** A note at `2-1.md` stamped `level: 1` passes `conduct`'s check but reads as missing to `outstanding`, so it is re-spawned indefinitely. Give the rule one owner, and treat a node reported again after a successful spawn as failed.
- 🔵 minor / high — Phase 4 §1 — **The lineage/level consistency rule lives in the imperative shell.** Move it to `LevelNote::from_frontmatter` in `corpus`.
- 🔵 minor / high — Phase 6 step 4, Phase 4 §2 — **The level-note id scheme is computed in prose, not by `outstanding`.** Emit `id` per node.
- 🔵 minor / medium — Phase 4 §1 — **Directory detection depends on the incidental behaviour of `FileReader::read`.** Make directories explicit in the port.
- 🔵 minor / medium — Phase 1 §1 — **Two optional-extras mechanisms coexist with no stated distinction.**
- 🔵 suggestion / medium — Phase 3 §1 — **Question normalisation moves into the tree module.** Give it its own module.
- 🔵 suggestion / low — Phase 2 §3 — **`TopicLayout` grows one predicate per path kind.** Use `classify() -> Option<TopicPathKind>`.
- 🔵 suggestion / low — Phase 3 §2, Phase 4 §2 — **Trims form a second warning channel beside `Round.warnings`.** Explain the split or mirror trims into the JSON.

### Code Quality

**Summary**: The layering is good and the domain value types are well named. The risks are at the seams: validity is defined twice and invalid notes are dropped silently, directories are detected through `read -> None`, `TopicLayout` uses two booleans, and dense functions grow with no planned extractions.

**Strengths**: the derivation is pure; `Lineage` avoids primitive obsession; the glossary table; `Role` + `ConfinedAgent` replace special-casing; named constants; the `Stage` enum makes the next action explicit.

**Findings**:
- 🟡 major / medium — Phase 4 §1, Phase 6 step 5 — **Level-note validity is defined in two places, and invalid notes are dropped silently.** Put the rules in one domain function and emit `warning:` lines with reasons.
- 🔵 minor / high — Phase 4 §1 — **Directory detection relies on the implicit `fs.read` → `None` convention.**
- 🔵 minor / medium — Phase 2 §3 — **`TopicLayout` models mutually exclusive kinds as two booleans.**
- 🔵 minor / medium — Phase 3 §2 — **`plan_item` and `index_for` grow more branches with no planned extractions.** Extract `stage_for` and an `IndexHolder` source.
- 🔵 minor / medium — Phase 3 §1, Phase 4 — **Tuple returns and mixed integer types.** Use `Derivation { state, trims }`, one count type and `Trim::trimmed()`.
- 🔵 suggestion / medium — Phase 3 §1 — **`normalised` is a weak fit for `tree.rs`.**
- 🔵 suggestion / low — Phase 1 §1 — **Two coexisting optionality mechanisms.**
- 🔵 suggestion / low — Phase 6 §2 — **The level-note outputter duplicates finding-outputter's body contract.**

### Test Coverage

**Summary**: The Rust layers are planned test-first with thorough, well-targeted tests. The weak points are the Phase 6 orchestration prose (two text checks and attended runs), an acceptance criterion requiring clamping unit tests that the plan moves to manual checks, and edge cases at the adapter and derivation boundaries.

**Strengths**: a derivation table that maps to the work-item examples; the `Lineage` rejection and ordering tests; two-level guard testing; the planner tied to the guard predicate; whole-JSON equality kept; a negative test for the exemption; contract tests in lockstep.

**Findings**:
- 🟡 major / high — Testing Strategy, Phase 5 §4 — **The acceptance criterion demands unit tests for concurrency clamping.** Amend the criterion or add a static contract test.
- 🟡 major / medium — Phase 6 Tests first — **Only two text-presence checks guard the prose that turns the feature on.** Add structural assertions in `research_agent_contract.rs`.
- 🟡 major / medium — Phase 3 `derive` tests — **No test pins how notes at non-candidate lineages are treated.**
- 🔵 minor / high — Phase 4 §1 — **Adapter rejection conditions are untested, and the test location is undecided.** Add a table test in `topic_research_outstanding.rs`.
- 🔵 minor / medium — Phase 1 §6 — **The deepened-set goldens could pass without any note being read.** Add a negative companion.
- 🔵 minor / medium — Phase 2 §4 — **Guard escape and fail-closed tests are not carried over.** Cover a symlinked `.levels`, a `..` escape, an unparseable config for the composer, and the composer using Edit.
- 🔵 minor / medium — Phase 3 §2 — **Index retention tests skip multi-profile pairs, root-less directories and the minimum rule.**
- 🔵 suggestion / medium — Phase 4 §2 — **New node fields are not pinned by whole-JSON equality, and there is no overflow depth case.**
- 🔵 suggestion / low — Phases 1–2 — **Boundary values are untested**: `Depth::new(0)`, `child(0)`, `check_extras` missing an optional extra, `follow_ups: ""`.

### Correctness

**Summary**: The tree derivation is well specified, and the node counts and batch sequences check out. The risks are at the seams: the loop has no guard against re-spawning accepted-but-rejected work, index retention depends on a valid root note, and notes are joined to candidates by lineage alone.

**Strengths**: the barrier is checked before the depth limit; trimming happens before dedupe, with stable positions and no backfill; numeric `Ord`; `read` is called before `list` for `.levels`; `.levels` names feed `highest`; the loop terminates in normal operation; the `PairTrim` subtraction cannot underflow.

**Findings**:
- 🟡 major / high — Phase 6 steps 5–6 — **The loop can re-spawn forever when output validates but `outstanding` still reports it.** This covers notes (`kind`, `level`) and findings (`is_answered` question and profile match).
- 🟡 major / medium — Phase 3 §1, Phase 4 §1 — **Index retention is lost when `1.md` is present but rejected and not yet quarantined.** Read the root question from any parseable `1.md`.
- 🟡 major / medium — Phase 3 §1 — **Notes are matched by lineage alone, and `Complete` may include off-tree notes.** Carry `question` on `LevelNote`.
- 🔵 minor / medium — Phase 4 §2 — **clap probably rejects `--depth -1` with exit 2, not 1.** Add `allow_hyphen_values`.
- 🔵 minor / medium — Phase 6 steps 3, 5 — **Note quarantine omits the no-overwrite rule.**
- 🔵 minor / low — Phase 3 §1 — **`known_questions` is ambiguous at the barrier level, and the `3-1-1` fixture cannot reach level 3.**
- 🔵 minor / medium — Phase 6 steps 2, 7 — **Trim warnings repeat on every loop iteration.** Deduplicate them.
- 🔵 suggestion / low — Phase 2 §1 — **`Lineage::child(0)` builds a non-canonical lineage.** Use `NonZeroU32` or derive `level`.

### Security

**Summary**: The guard work is well built. The new risk comes from recursion: follow-up questions shaped by untrusted web content become trusted task input and reach the unconfined orchestrator. The guard still judges writes by path shape alone, while up to 24 concurrent agents can reach any pair's tree.

**Strengths**: the composer gets default-deny commands and a narrow scope; `Lineage::parse` and `is_level_note_path` are strict; `is_finding_path` narrows; default names are matched first; the composer marks notes as untrusted; the `locate` defences are unchanged.

**Findings**:
- 🟡 major / medium — Phase 6 §2, step 4 — **Follow-ups launder prompt injection into the child's trusted focus question.** Forbid URLs and directives in follow-ups, sanitise them in the adapter, and add a warning to the deepen prompt and an adversarial attended step.
- 🟡 major / medium — Phase 4 §2, Phase 6 steps 2, 7 — **Untrusted node text reaches the unconfined orchestrator unbounded.** Apply bounds and sanitisation, restrict the stem alphabet, and label the text as opaque data in the prose.
- 🟡 major / medium — Phases 2–3 — **The path-shape-only guard lets researchers tamper with other pairs' trees and with planner state.** Diff `findings/` around each batch, use `checked_add` for `highest + 1`, and state the residual risk.
- 🔵 minor / medium — Phase 6 §1, Phase 2 §3 — **The composer's lack of fetch depends only on its tool list.** Add a `WebFetch|WebSearch` matcher or document the constraint.

### Compatibility

**Summary**: Backward compatibility is handled well: output changes only add fields, `--depth` defaults to 1, and `depth` is optional. The plan misses one positional consumer of the TSV, users who had set a dormant `depth` above 1, and collaborators on older plugin versions.

**Strengths**: the `outstanding` contract only adds fields; `depth` is optional and older validators tolerate it; `agents.composer` is registered; the launcher pins the binary to the plugin version; the public API is snapshotted.

**Findings**:
- 🟡 major / high — Phase 1 §2 — **The Python conformance suite reads `templates-schema.tsv` by position.** Update `_schema()` or append the column last.
- 🟡 major / medium — Phase 6 §4–5, Migration Notes — **An existing depth above 1 starts deep recursion on upgrade without notice.**
- 🔵 minor / medium — Migration Notes — **Older plugin versions misread deepened sets in a shared corpus.**
- 🔵 suggestion / low — Phase 2 §2 — **Narrowing `is_finding_path` could refuse legacy unindexed paths.** Put it in the release notes.

### Usability

**Summary**: The default experience is intact and the new surfaces follow existing conventions. The gaps are in feedback at depth above 1: a deeper re-run silently does nothing, no fan-out or cost signal replaces the dormant notice, and the orchestrator assembles ids by hand.

**Strengths**: recursion is opt-in; the misplaced-flag rule is consistent; refusal text depends on the role; there is an `E_` error for `--depth`; the `stage` discriminator is self-describing; failure next steps are actionable; the multipliers are documented.

**Findings**:
- 🟡 major / high — Phase 6 step 7, Phase 7 step 12 — **A deeper re-run over already-composed pairs silently does nothing.**
- 🟡 major / medium — Phase 6 steps 1–2 — **Removing the dormant notice leaves no signal of fan-out or cost at the point of use.**
- 🟡 major / medium — Phase 4 §2, Phase 6 step 4 — **Level-note ids are hand-assembled by the orchestrator.**
- 🔵 minor / medium — Phase 4 §2 — **Trim warnings use a second channel alongside the JSON `warnings`.**
- 🔵 minor / medium — Phase 6 steps 5, 7 — **The failure summary format is unspecified for composer failures and new refusals.**
- 🔵 minor / medium — Phase 6 step 6 — **There is no per-batch progress feedback.**
- 🔵 minor / medium — Phase 2 §3 — **A name configured for both roles is silently resolved as the researcher.**
- 🔵 minor / medium — What We're NOT Doing — **The level-note template cannot be overridden.**
- 🔵 suggestion / low — Phase 4 §2 — **A hand-run `outstanding` defaults to depth 1 regardless of config.**

### Performance

**Summary**: The Rust side is linear in small, bounded inputs. The bottlenecks are in the prose loop: the JSON re-emitted after every batch grows quadratically per tree, batches are synchronous with no fill order, and a global cap of 24 lets many arXiv agents compete for the serialised lock.

**Strengths**: derivation is linear and bounded at 21 nodes; the guard's hot path stays cheap; resume respawns only what failed; the cap applies at every depth; no extra stat pass is needed.

**Findings**:
- 🟡 major / medium — Phase 6 steps 2, 4, 6 — **Orchestrator context grows with every re-plan and every spawn prompt.**
- 🟡 major / medium — Phases 5–6 — **A global cap of 24 lets recursion flood the serialised arXiv lock.**
- 🔵 minor / high — Phase 3 §1, Phase 4 §2 — **`known_questions` is repeated per node, so the payload is quadratic.**
- 🔵 minor / medium — Phase 5 §2, Phase 6 step 4 — **Batches are fully synchronous and have no fill order.**
- 🔵 suggestion / medium — Phase 6 step 5, Phase 4 §1 — **Each note is validated twice per batch.**

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-09-27

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Correctness / Architecture / Code Quality**: The conduct loop can re-spawn forever — Partially resolved (single note-acceptance owner and attempted set in place, but a stem can move between re-plans; see new issues)
- 🟡 **Correctness**: Notes are matched by lineage alone, and `Complete` includes off-tree notes — Resolved
- 🟡 **Correctness**: Index retention is lost for an unaccepted `1.md` — Resolved
- 🟡 **Security**: Follow-ups launder prompt injection — Partially resolved (the gate misses schemeless domains and Unicode format characters; `researcher.md` still lets the focus question license a fetch)
- 🟡 **Security**: Untrusted node text reaches the orchestrator unbounded — Resolved
- 🟡 **Security**: The path-shape-only guard allows cross-pair tampering — Partially resolved (overflow and name crafting closed; the residual-risk wording understates forged-note pre-emption)
- 🟡 **Usability / Architecture**: Level-note ids are assembled by hand — Resolved
- 🟡 **Usability / Compatibility**: Depth goes live without notice — Still present by decision (point-of-use notice declined; the Migration Notes entry has no step that writes the release note)
- 🟡 **Usability**: A deeper re-run silently does nothing — Resolved
- 🟡 **Compatibility**: The Python conformance suite reads the TSV by position — Resolved (the TSV is retired; the shape mapping is raised below)
- 🟡 **Performance**: Orchestrator context growth — Partially resolved (`--limit` bounds each plan; the `--attempted` list is re-emitted in full, quadratically)
- 🟡 **Performance**: arXiv lock flooding — Resolved by decision (measured in Phase 7; the step's setup is too light, raised below)
- 🟡 **Test Coverage**: Clamping acceptance criterion — Resolved
- 🟡 **Test Coverage**: Phase 6 prose guarded by two text checks — Resolved
- 🔵 All other pass-1 minors and suggestions — Resolved, except that trims still use the stderr channel (partially resolved: deduplicated, but outside the JSON), `known_questions` is still timing-dependent at the barrier level, and the adapter test location is still undecided

### New Issues Introduced

- 🟡 **Correctness**: `SpawnRef` is keyed by a stem that can be reallocated between re-plans. `index_for` (`round.rs:454-458`) gives new focus areas `next_index` in outline order. A pair that fails without leaving an index holder, while a later pair writes, moves to a new stem, so it is offered again. This falsifies "attempted at most once per invocation". Verified against the source.
- 🟡 **Correctness / Architecture**: The plain-question gate applies to a note's own `question`, so a trusted outline question containing `://`, or longer than 300 characters, makes every root note `NotAPlainQuestion`. That pair can never deepen.
- 🟡 **Security**: The plain-question gate is bypassable with schemeless domains (`evil.example/q`), Unicode format and separator characters (Cf, Zl, Zp), and `researcher.md:46` still lets the focus question license a fetch.
- 🔵 **Compatibility / Correctness**: "The four review rows" misnames the `reviewer` owners. The `plan` row (`schema.rs:48`) carries `reviewer`, and a literal reading would make it required on every plan. Verified.
- 🔵 **Compatibility / Correctness / Test Coverage**: Phase 0's Python rewrite claims the assertions are unchanged, but JSON rows carry a bool and arrays where the code compares `"yes"` and calls `.split()`. `print_schema_emits_the_three_banks` lives in `frontmatter_goldens.rs`, not in the Python file. Phase 1 never moves the 18-row golden to 19.
- 🔵 **Correctness**: `checked_add` guards `highest` but not `next_index += 1`. A holder at `MAX-1` still overflows on the first allocation.
- 🔵 **Usability / Correctness**: A note rejected before the run is quarantined and re-spawned without a reason, which contradicts Phase 7 steps 11 and 15. The step 6 reason and quarantine assume a note exists.
- 🔵 **Usability / Architecture**: Schema-invalid notes lose their validation reason (`rejected: null`).
- 🔵 **Usability**: The configure docs claim the level-note template cannot be overridden, but the resolver honours `<paths.templates>/topic-research-level-note.md`.
- 🔵 **Architecture**: Loop termination depends on the conversation accumulating `--attempted` refs, not on disk state.
- 🔵 **Performance**: The `--attempted` list is regenerated in full on every re-plan, which is quadratic in generated tokens. Phase 7 step 16's single arXiv pair peaks at 8 nodes, too few to show contention.
- 🔵 **Architecture / Code Quality**: `extras` silently narrows to required extras (consider `required_extras`). `print-schema` row mapping can drift unless the row is destructured exhaustively. `Node::rejected` has two writers. There is no `Stem` value type. `from_frontmatter` takes positional `Option`s. `LevelsDirectory` has no constructor.
- 🔵 **Compatibility**: No phase writes the `CHANGELOG.md` entries or the `docs-site` `outstanding` contract update that the Migration Notes call for.
- 🔵 **Test Coverage**: The legacy-finding CLI test expects `shallower` at depth 1 rather than 3. The window tests miss cross-pair ordering and truncation. The note-rejection table misses an empty or multi-line `question` and a multi-byte boundary. There is no static test for compose routing.
- 🔵 **Correctness**: Research and deepen can coexist at depth 1 (a directory without an accepted root). `--limit` lacks `allow_hyphen_values`.
- 🔵 Suggestions: spawn scheduling in its own module; Phase 5 could use the window directly; `LONGEST_PLAIN_QUESTION` locked to the outputter prose by a test; exemptions named as data; `is_dir` in its own trait; a profile-name alphabet check.

### Assessment

The revision resolved 12 of 14 pass-1 majors fully or by explicit decision, and nearly every minor. The core design is now sound: one owner for note acceptance, question-matched trees, a domain-owned spawn window, and a single schema source. Three new majors remain, all narrow and cheap to fix:

- stable stems within an invocation, so `--attempted` stays a true identity;
- the plain-question gate applied to follow-ups only;
- the gate hardened against schemeless hosts and Unicode format characters, plus the `researcher.md` contract wording.

The minors are mostly precision fixes to the Phase 0 text (the `reviewer` rows, the Python shape mapping, the golden count) and small specification gaps. One more targeted pass after these edits should reach COMMENT or APPROVE.

## Re-Review (Pass 3) — 2026-09-27

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Correctness**: `SpawnRef` keyed by a reallocatable stem — Resolved for one run; concurrent runs are raised below
- 🟡 **Correctness / Architecture**: The plain-question gate rejects trusted roots — Resolved
- 🟡 **Security**: The plain-question gate is bypassable — Partially resolved (schemeless host and path now caught; Unicode tag block, U+061C and fullwidth `．`/`／` still pass; the "outside the focus question" wording in `researcher.md:46` is still there)
- 🔵 All pass-2 minors (the `reviewer` rows, Python types, goldens, allocator overflow, rejections found at the start, schema-invalid reasons, template docs, trims, test gaps, structural items, CHANGELOG and docs-site) — Resolved
- 🔵 **Security / Correctness**: Cross-pair tampering — Partially resolved (pre-emption of unoffered nodes is detected; overwrites, races within a batch and interrupted-run forgeries are not)

### New Issues Introduced

- 🟡 **Security**: `FailsValidation(String)` carries attacker-chosen frontmatter values to the unconfined orchestrator. Violation messages quote values verbatim (`violation.rs:119-177`, e.g. `status: '{value}' not in vocab`), and `rejected` is not on step 2's list of opaque data. Verified.
- 🟡 **Architecture / Correctness**: The CLI creates the run ledger but the prose must delete it, and `research-topic`'s `allowed-tools` (`SKILL.md:13-19`) grants no command that deletes a file. Verified.
- 🟡 **Architecture / Correctness**: Two `conduct` runs on one set, or two runs sharing a run id within one second, silently discard or share each other's ledger. `attempted` resets, spawns are offered again, and "terminates by construction" fails.
- 🟡 **Code Quality / Architecture / Correctness**: The `RunLedger` API is inconsistent. `window` takes a non-optional ledger despite the "no ledger" text, `read_run_ledger` calls a two-argument `resumed_for`, and the ledger enters twice (`RoundInputs` and `window`), coupling `round.rs` and `spawn_window.rs` in both directions. `notes_at_start` has no stated data source, and its `SpawnRef` type admits `Pair` refs.
- 🟡 **Test Coverage / Architecture / Correctness**: Nothing specifies or tests what happens when the ledger write fails, or whether stdout is emitted before the write. A swallowed failure brings back unbounded re-offering.
- 🔵 **Correctness / Architecture**: `attempted` records what was offered, not what was spawned. A repeated plan call drops a batch and misreports it as "wrote no note".
- 🔵 **Security / Correctness**: `unexpected` compares refs, not content, so overwrites and notes refused at the start that are later forged go unflagged.
- 🔵 **Security**: `INVISIBLE_FORMAT_CHARACTERS` is hand-picked. It misses U+E0000–E007F, U+061C, U+180E, U+FFF9–FFFB and the variation selectors, and has no NFKC fold for fullwidth hosts.
- 🔵 **Security**: `researcher.md:46` still says "outside the focus question", which contradicts the new sentence.
- 🔵 **Code Quality**: Ledger `indexes` are keyed by an untyped `String`. A corrupt ledger is discarded silently. `IndexHolder { name }` contradicts "every stem is a `Stem`". The plain-question gate deserves its own named predicate. Several references are stale (the Phase 2 snapshot line, the Testing Strategy window bullet, the legacy bullet).
- 🔵 **Usability**: The summary has no fixed section order and trims have no summary line. The `NoteRejection` `Display` texts are unspecified, and step 15 quotes a Rust variant name. A leftover ledger is unexplained and discarded silently.
- 🔵 **Test Coverage**: No test that notes present before the first plan are not `unexpected`. No `RunId` alphabet test, and no test tying `RunId` to `Timestamp For Filename`. No `Stem` round-trip test. The whole-JSON depth-1 shape is unspecified. The `unexpected` summary line is untested.
- 🔵 **Compatibility**: CHANGELOG lines for Phases 2 and 5 land only in Phase 6, although each merge cuts a prerelease. The published guard block-message table (`research.md:258-270`) is not updated for the composer.
- 🔵 **Performance**: The arXiv step does not seed trees to guarantee a 24-node batch, and counts failures per node rather than per fetch. The token volume of spawn prompts is not accounted for. `notes_at_start` should be built from the listing already read.

### Assessment

Pass 3 resolved all 3 pass-2 majors for the single-run case and nearly every minor. The domain model is now cohesive. Four of the five new majors are about the run ledger: its lifecycle, concurrency, API shape and failure behaviour. That is the expected cost of a new stateful component. One design pass on the ledger would fix all four together:

- a start / continue / end lifecycle, with `end-run` as its own verb;
- refusal of a foreign run's ledger;
- split planner and window inputs;
- a fail-closed write before any output.

The fifth major is a one-line fix: `FailsValidation` carries only the violation code and field. After those, the plan should reach COMMENT.

## Re-Review (Pass 4) — 2026-09-27

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Security**: `FailsValidation` launders attacker text — Resolved (code plus schema-named key only; opaque-data list extended)
- 🟡 **Architecture / Correctness**: Ledger lifecycle split, with no tool to delete the ledger — Resolved (`end-run` verb under the existing allowance)
- 🟡 **Architecture / Correctness**: Concurrent runs share or discard each other's ledger — Resolved (random-suffixed `RunId`, `E_TOPIC_RESEARCH_RUN_SUPERSEDED`); the residual in-flight overlap is a minor below
- 🟡 **Code Quality / Architecture / Correctness**: Inconsistent `RunLedger` API — Resolved (`PinnedIndexes` / `Attempts` / `RunLedger`); `Digest` placement is a minor below
- 🟡 **Test Coverage / Architecture / Correctness**: Ledger write failure unspecified — Resolved (write before render, fail closed, tested)
- 🔵 Pass-3 minors — Resolved, except that the plain-question gate still misses variation selectors, the Hangul fillers, U+3002, ports and backslashes, and `unexpected` has two edge cases (below)

### New Issues Introduced

- 🟡 **Compatibility**: The new Unicode crates break the `corpus` cargo-pup rule. `cli/pup.ron:63-77` allows only `std`/`core`/`alloc`, `kernel::Error` and `crate` imports in `corpus`, so `is_plain_question` in `question.rs` fails `pup:check`. Verified.
- 🔵 **Architecture / Code Quality**: `Digest` is declared in `spawn_window.rs` but carried by `LevelNote` (tree.rs) and `Round`, which recreates a module cycle. `from_frontmatter` has no digest input.
- 🔵 **Code Quality / Correctness**: `run_outstanding` resolves `start` before planning, but `start` needs a `Round`. The initial `batch` value and whether `continue_as` clears `offered` are unspecified. `record` takes the round redundantly.
- 🔵 **Correctness**: A note that `conduct` itself quarantines and re-researches after `QuestionDisagreesWithCandidate` is falsely `unexpected`, because its digest changed. A note appearing later for an attempted node that failed is never flagged.
- 🔵 **Architecture / Correctness / Usability**: The `--start` warning and the Migration Notes say the discarded run was interrupted, but it may still be live, and its in-flight batch can overlap. Step 7 does not say what happens after a non-zero `end-run`.
- 🔵 **Architecture / Usability / Test Coverage**: `--spawned` flag constraints are unspecified (it should require `--run` and conflict with `--start`). "Terminates by construction" should be qualified as depending on correct acknowledgement.
- 🔵 **Security / Correctness**: Gate gaps and over-rejection. It misses `Default_Ignorable_Code_Point` characters (variation selectors, CGJ, Hangul fillers), U+3002, `:port` and `\`. It over-rejects `Node.js/Deno`. Whether length is counted before or after NFKC is unspecified, and the gate and `normalised` disagree on folding.
- 🔵 **Security**: The residual-risk paragraph omits forgeries within a batch, and forged findings for pending `research`/`compose` pairs. Ledger fields should be re-parsed through their value types, with a length bound on `RunId`. Ancestor `known_questions` should render candidate text.
- 🔵 **Compatibility**: `notices:check` will drift, and the workspace-dependency convention applies. Phase 4 has no CHANGELOG line for its CLI surface or the ledger file, and the `.gitignore` edit sits in no phase. Several published `research.md` statements go stale.
- 🔵 **Test Coverage**: No test covers `record` adding digests, a superseded exit leaving the ledger bytes unchanged, `--start` over a corrupt ledger, a `NotAPlainQuestion` position above 1, rows isolating `//`/Co/Cn/Zl, or a direct test of all `Display` texts. There is no attacker-named-key test. Two tests are misfiled, and the pair-drop rule has no test.
- 🔵 **Code Quality**: Two normalisers coexist (`normalised` and `NormalisedQuestion::of`). `batch`/`offered` form a data clump, and `PendingBatch` would fix it. There is no `Violation::schema_key` in the domain. Also `Stem`'s dual index fields, the unnamed position in `NotAPlainQuestion`, `(Stem, Lineage)` as an anonymous tuple, and misplaced `question.rs` items.
- 🔵 **Usability**: Error codes lack recovery text. "Refused as {clause}" reads awkwardly. `end-run` needs a run id a user doesn't know. `NotAPlainQuestion` should name the rule it tripped.
- 🔵 **Performance**: `rate_limited` failures after a late lock acquisition are unlogged. Seeded follow-ups must be distinct. The rescan cost is per plan times batches.

### Assessment

Every pass-3 major is resolved, and the plan is acceptable to implement. The one new major is mechanical: move the Unicode classification out of `corpus`, either behind a port implemented in `corpus-adapters` (where the note bytes are already read) or through a justified widening of `pup.ron`. The minors are precision and edge-case items. None blocks implementation, but two are worth fixing first, because each would otherwise be discovered only in attended runs:

- the false `unexpected` on re-researched notes;
- the start-before-plan ordering.

## Re-Review (Pass 5, targeted: compatibility, correctness, security) — 2026-09-27

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Compatibility**: The Unicode crates break the `corpus` pup rule — Resolved (std-only `UnicodeText` port; `UnicodeTables` in `corpus-adapters`; licences accepted; `unicode-normalization` already locked)
- 🔵 **Compatibility**: Notices, CHANGELOG, `.gitignore` and `research.md` — Resolved
- 🔵 **Correctness**: Start ordering, batch 0, `Digest` placement, a re-researched note falsely `unexpected`, a late note for a failed attempt, supersession wording, the gate gaps — Resolved
- 🔵 **Security**: Gate gaps, `schema_key`, ledger re-validation, candidate text — Resolved. The residual-risk text is partially resolved (see the answered-pairs major)

### New Issues Introduced

- 🟡 **Security / Correctness**: `answered_at_start` never grows, so every legitimately answered pair is `unexpected` from the second plan after its batch. `record` must fold `window.round.answered` in, as it does for `notes_seen`.
- 🟡 **Correctness**: The batch number never advances in a normally acknowledged run. `acknowledge` keeps the number, and `replace` increments only over a non-empty batch. A replayed `--spawned n` therefore acknowledges a batch that was never spawned.
- 🟡 **Correctness**: The `?` host marker refuses ordinary questions such as "What changed in Python 3.12?" and "… Vue.js?".
- 🟡 **Compatibility**: Phase 3 changes `RoundInputs` and `Round::plan`, but their only consumers (`corpus-adapters/src/topic_research.rs:83` and `corpus-cli/src/topic_research.rs:25`) are updated only in Phase 4, so Phase 3 does not compile on its own. Verified. Partly pre-existing: `levels` and `depth` were always added in Phase 3.
- 🔵 **Correctness**: The spawns `replace` displaces are undefined, and the rationale that they have always written is overstated. The rebuild of `pins` keys is unspecified: `read_run_ledger` has no `UnicodeText`.
- 🔵 **Security**: The claim that NFKC maps U+FF61 is inaccurate (it maps to U+3002, which the gate then maps to `.`). The host-label alphabet is undefined, so non-ASCII IDN hosts with a path could pass.
- 🔵 **Compatibility**: `delete_run_ledger` has no port. The copied `DerivedCoreProperties` table needs a Unicode-3.0 notice and version pinning. The `sha2` and `rand` dependency additions are unlisted. Consumer repositories get no ignore rule for the ledger.

### Assessment

All 4 majors are single-rule or phase-placement fixes, not design changes:

- accumulate answered stems;
- advance the batch number on acknowledgement;
- let `?` count as a host marker only when a query token follows it;
- move the `corpus-adapters` and `corpus-cli` wiring, including `UnicodeTables`, into Phase 3.

The verdict follows the configured threshold of 3 majors. Once these land, the plan is expected to stand at COMMENT.

## Re-Review (Pass 6, targeted: compatibility, correctness, security) — 2026-09-27

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Security / Correctness**: `answered_at_start` never grows — Resolved (`answered_seen` folded in by `record`)
- 🟡 **Correctness**: The batch number never advances — Resolved (all six scenarios walked: normal, replay, repeat, `--limit` split, empty final offer, superseded)
- 🟡 **Correctness**: The `?` host marker over-rejects — Resolved (`Python 3.12?`, `Vue.js?`, `v2.0?`, `U.S.` and `e.g.` pass)
- 🟡 **Compatibility**: Phase 3 does not compile on its own — Resolved (verified: the two call sites are the only consumers, and `RoundInputs: Default`)
- 🔵 Pass-5 minors — Resolved, except that the Unicode version-match test is partially resolved (only `unicode-normalization` is verified to export `UNICODE_VERSION`)

### New Issues Introduced

- 🔵 **Security**: The narrowed `?` rule lets query strings through when a non-word character follows the `?` (`?/`, `?%41`, `?&`). Inverting it closes this: `?` is a host marker unless whitespace, end of text or closing punctuation follows.
- 🔵 **Security**: A percent-encoded dot (`evil%2Eexample/q`) passes, so `%` followed by two hex digits should be refused.
- 🔵 **Correctness**: The JSON `batch` field is not pinned to the pending number after `record`.
- 🔵 **Correctness**: A changed, unacknowledged offer reports its own spawns' legitimate writes as `unexpected`. The fix is to excuse the pending batch's refs in the window.
- 🔵 **Correctness**: Two gate over-rejections: unspaced CJK clauses joined by `。`, and numeric dotted tokens (`1.85:1`, `3.5/5`).
- 🔵 **Correctness / Security**: A paragraph splits the rejection test table, leaving nine rows orphaned.
- 🔵 **Compatibility**: NFKC folding changes depth-1 question matching, although the plan says depth-1 behaviour is unchanged.
- 🔵 **Security**: The residual-risk text omits the case where an already-answered finding is overwritten.

### Assessment

No majors remain in the three lenses most affected by recent edits. The plan is ready to implement. The minors are small text or rule tweaks, and none changes the design.

## Approval — 2026-09-27

**Verdict:** APPROVE

The reviewer approved the plan after the 9 pass-6 minors were applied. Those
last edits were not re-reviewed. One fact is still open for implementation
time: confirm that a `unicode-general-category` release is built on the same
Unicode version as `unicode-normalization` 0.1.25 (17.0.0).
