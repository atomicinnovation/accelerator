---
date: "2026-04-27T11:00:00+00:00"
type: plan
skill: create-plan
ticket: ""
status: draft
---

# Extend `create-work-item` to Enrich an Existing Work Item

## Overview

Extend `skills/work/create-work-item/SKILL.md` so it accepts an existing work
item (by path or number) as a starting point, treats it as a shell to enrich,
and runs the same five-step creation conversation while preserving the
existing file's identity fields and overwriting the existing path on save.
The no-arg and topic-string paths remain unchanged.

The work is driven test-first using the `skill-creator` workflow: new
scenarios are added to `evals/evals.json` before each implementation phase,
the previous skill is snapshotted as a baseline, and the skill-creator
benchmark scripts compare baseline vs updated runs to confirm the new
behaviour and guard against regressions in the existing topic-string flow.

## Current State Analysis

- `skills/work/create-work-item/SKILL.md:5` declares `argument-hint: "[topic
  or description]"`. Step 0 (lines 60–87) has a two-way branch (argument
  vs. no argument) and treats every argument as a topic string. There is no
  file-resolution logic in the skill body.
- `allowed-tools` (line 7) already permits
  `Bash(${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/*)`, which covers
  `work-item-read-field.sh` — no `allowed-tools` change is needed.
- The canonical two-form (path-like / numeric) resolution pattern is
  established in four peer skills:
  `skills/work/review-work-item/SKILL.md:35-50` (path-like, numeric, 0/1/N
  match handling), `skills/work/update-work-item/SKILL.md:43-55` (numeric
  multi-match → numbered options), `skills/work/refine-work-item/SKILL.md:61-75`
  (unparseable-frontmatter guard), and `skills/work/stress-test-work-item/SKILL.md:39-43`.
- `skills/work/scripts/work-item-read-field.sh` extracts any frontmatter
  field from a file and exits 1 on missing file, missing/unclosed frontmatter,
  or missing field. It is sufficient for reading the identity fields the
  enrichment path must preserve.
- `templates/work-item.md:1-11` defines the identity fields:
  `work_item_id`, `title`, `date`, `author`, `type`, `status`, `priority`,
  `parent`, `tags`. The body H1 is `# NNNN: Title`.
- `skills/work/create-work-item/evals/evals.json` contains 14 scenarios, all
  topic-string inputs. `evals/benchmark.json` is the most recent benchmark
  comparing the current skill (`with_skill`) to the prior version
  (`old_skill`). New evals must extend the file in the same shape.

### Key Discoveries

- The extension is a **clean branching addition to one file** — `SKILL.md` —
  with no scripts to write, no template changes, and no other skill changes
  (research §8).
- The existing 14 evals all use prose `expected_output` only.
  New evals add an `expectations: [string]` array (the harness's native
  field, verified against `claude-plugins-official/skill-creator/`) listing
  the invariants the grader subagent must check (no
  `work-item-next-number.sh` call, identity field preservation, target path
  equals input path, error strings on missing/unparseable input). All
  grading — including these invariants — is performed by the grader
  subagent reading the recorded transcript and outputs; there is no
  programmatic-assertion mechanism. Qualitative behaviour (gap analysis,
  augmentation framing) is also expressed as expectations in the same
  array.
- Skill-creator workflow snapshots the pre-edit skill as `old_skill` and
  runs `with_skill` against it for delta benchmarks; the existing
  `evals/benchmark.json` is the model to follow.
- `work-item-next-number.sh` scans the work items directory and returns
  `MAX(existing) + 1`, so calling it during enrichment would always return
  a *higher* number than the file already in place — a clear bug if
  preserved-id is not enforced (research §5, citing
  `skills/work/scripts/work-item-next-number.sh:43-73`).

## Desired End State

- `/create-work-item <path-or-number>` resolves to an existing file
  (path-like or numeric), reads it fully, and enters enrich-existing
  mode: gap-analysis-driven discovery (Step 1), self-excluded
  duplicate detection (Step 2), augmentation-framed proposal (Step 3),
  preserved-identity draft (Step 4), and an at-write identity-swap
  check + single confirmation gate before in-place overwrite
  (Step 5).
- Identity fields are preserved per the canonical Identity Field
  Rules in Quality Guidelines: `work_item_id`, `date`, `author` are
  immutable and substituted from the cached values into the draft
  at write time (textual substitution by the model);
  `status` defaults to the cached value but may transition if the
  user proposes one in Step 3; `title`, `type`, `priority`,
  `parent`, `tags` are proposable in Step 3 with explicit user
  agreement. `work-item-next-number.sh` is not called in
  enrich-existing mode. The H1 uses the cached `work_item_id`,
  never `XXXX`.
- `/create-work-item` (no argument) and `/create-work-item
  <topic-string-without-discriminators>` continue to behave exactly
  as today.
- `/create-work-item <numeric-or-path-like-input-that-doesn't-resolve>`
  prints a one-line warning naming the input, then falls back to
  topic-string handling — preserving the topic-string contract for
  inputs that look like discriminators but aren't existing items.
- Resolution errors that cannot fall back (multi-match without
  selection, unparseable frontmatter on a resolved file) produce
  canonical peer-skill messages and abort before any agent spawn
  or file write.
- The skill-creator benchmark in `evals/benchmark.json` shows the
  existing 14 scenarios pass on both `with_skill` and the pre-edit
  `old_skill` snapshot with `trigger_rate >= 0.9`, and every new
  scenario passes on `with_skill` with `trigger_rate >= 0.9`.

### Verification

- The skill-creator benchmark, run via `scripts.run_eval` and
  `scripts.aggregate_benchmark` against the workspace at
  `meta/eval-workspaces/create-work-item/`, produces a
  `benchmark.json` whose `with_skill` mean `trigger_rate` ≥
  `old_skill` mean `trigger_rate` on the existing 14 scenarios, with
  every new scenario at `with_skill` mean `trigger_rate >= 0.9`.
- Manual run of `/create-work-item <path-to-sparse-existing-file>`
  against a hand-prepared sparse work item enriches it in place
  without changing `work_item_id`, `date`, or `author`, and writes
  back to the same path; status either stays unchanged or
  transitions only via a Step 3-proposed-and-confirmed change.

## What We're NOT Doing

- Not introducing new scripts; `work-item-read-field.sh` is sufficient.
- Not changing `templates/work-item.md`.
- Not modifying any other skill file (`refine-work-item` continues to own
  decomposition / technical enrichment; this skill owns content gap-fill).
- Not implementing the auto status upgrade (research Open Question 2 — out
  of scope).
- Not implementing a before/after diff preview at write time (research
  Open Question 1 — left as a possible follow-up).
- Not changing `allowed-tools`, `disable-model-invocation`, or any
  frontmatter beyond `argument-hint`.
- Not adding `expectations` arrays to the existing 14 evals;
  invariant-listing applies to **new** scenarios only.

## Implementation Approach

The skill is a single Markdown file. The change strategy is incremental in
phases driven by the skill-creator TDD loop:

1. Each implementation phase begins by adding new scenarios to
   `evals/evals.json` with prose `expected_output` and an
   `expectations` array (list of strings) listing the invariants the
   grader subagent must verify. The pre-edit skill is snapshotted at the
   start of Phase 1 to provide a regression baseline for the existing 14
   scenarios across phases. The Phase 1 snapshot is **not** used as a
   meaningful comparison for new (enrich-mode) scenarios — those
   trivially fail on a skill that has no enrich-mode logic at all. To
   detect mid-plan regressions inside the enrich path, each phase
   re-snapshots the skill at its end (overwriting the previous
   `skill-snapshot/`) so the next phase compares its new scenarios
   against the immediately-preceding skill state.
2. The phase's `SKILL.md` edits are then made.
3. The skill-creator scripts (`scripts/run_eval.py`,
   `scripts/aggregate_benchmark.py`) are run to produce a benchmark
   showing each phase's new scenarios pass on `with_skill` and the
   existing 14 scenarios continue to pass on both `with_skill` and
   `old_skill`. Per-phase "fails on `old_skill`" applies only to
   scenarios that test behaviour the previous phase's snapshot already
   supported (typically not new enrich-mode scenarios in the phase
   that introduces them).

The five SKILL.md steps gain "enrich-existing" variants gated by mode
state set in Step 0 (see "Enrich-existing mode" definition in Phase 1
§4); the existing topic-string variants are left untouched in body and
intent.

### Eval harness notes

The skill-creator harness used here has the following observed contract,
verified against `claude-plugins-official/skill-creator`:

- The eval field for grader-checked invariants is `expectations:
  [string]` (a list of plain strings), graded by the grader subagent
  against the recorded transcript and outputs. There is no native
  programmatic-assertion mechanism; "the skill did not call X" is
  verified by the grader searching the transcript text for absence of
  X. Plan language must therefore avoid claiming "programmatic"
  checks — every check is grader-mediated.
- `runs_per_configuration` defaults to 3 in `run_loop.py`. The
  aggregator computes `mean`, `stddev`, `min`, `max` per scenario and
  `trigger_rate = passes / runs`. Plan success criteria therefore use
  `trigger_rate >= 0.9` on new scenarios rather than `pass_rate == 1.0`,
  to absorb LLM-grader variance.
- There is **no `--filter` flag** in `run_eval.py`; eval-set filtering
  is done externally (e.g. by pre-extracting a subset of scenarios into
  a temporary file). All references in this plan to filtered runs
  describe pre-filtered eval sets, not a CLI flag.
- The `files: [...]` array stages fixture paths to the executor
  subagent rather than auto-staging files into a working directory.
  Multi-file scenarios (e.g. id 17 multi-match) work because the
  executor receives both paths and is asked to simulate the glob
  result; filesystem state is not actually populated by the harness.
- The `old_skill` snapshot lives at `<workspace>/skill-snapshot/` and
  is referenced by the baseline subagent. The aggregator discovers
  configuration directories under `<workspace>/iteration-N/` at
  aggregation time.

---

## Phase 1: Eval Scaffolding and Argument Resolution

### Overview

Add eval scenarios for the path-like / numeric resolution branch and its
error paths, snapshot the pre-edit skill, and implement the Step 0
argument-resolution logic. After this phase, the enrichment branch is
*entered* correctly but does not yet diverge from the existing flow in
Steps 1–5.

### Changes Required

#### 1. Snapshot pre-edit skill as baseline

**Workspace location**: `meta/eval-workspaces/create-work-item/`
(outside `skills/` so plugin packaging globs do not pick it up;
gitignored — local-only artefact whose only consumer is the
skill-creator harness on this machine).

Before editing `SKILL.md`, copy the current skill directory to that
workspace as the `old_skill` baseline:

```bash
mkdir -p meta/eval-workspaces/create-work-item
cp -r skills/work/create-work-item \
  meta/eval-workspaces/create-work-item/skill-snapshot
```

The snapshot is what the baseline subagent reads when running the
`old_skill` configuration; the aggregator discovers configuration
directories under `meta/eval-workspaces/create-work-item/iteration-N/`
at aggregation time, per
`claude-plugins-official/skill-creator/scripts/aggregate_benchmark.py`.

**Re-snapshot at the end of each phase**: after a phase's SKILL.md
edits land and its evals pass, overwrite `skill-snapshot/` with the
new state so the next phase's `old_skill` reflects the immediately
preceding skill — turning each phase's "fails on `old_skill`" check
into a meaningful regression gate on the enrich path itself rather
than a tautology against a skill that lacked enrich mode entirely.

#### 2. Add resolution evals

**File**: `skills/work/create-work-item/evals/evals.json`

Append new entries with `id` continuing from 14 (so the existing 14 are
untouched). Each new entry uses the existing prose `expected_output`
shape **and** an `expectations: [string]` array listing invariants the
grader subagent must verify. New scenarios:

- `id: 15` — `enrich-existing-path-like` — argument
  `meta/work/0042-existing-sparse.md` resolves and skill enters enrich
  mode (signalled by mentioning the file's title / id in its first
  response, not asking topic-clarification questions).
- `id: 16` — `enrich-existing-numeric-single-match` — argument `42` resolves
  to the file in `meta/work/`.
- `id: 17` — `enrich-existing-numeric-multi-match` — `files` declares
  two fixtures (`0042-existing-sparse.md` and `0042-other-slug.md`) so
  the executor subagent treats the glob as returning both; skill lists
  numbered options and waits. (See "Eval harness notes" — the harness
  does not auto-stage files into a working directory; this scenario
  therefore exercises the prompt template's multi-match rendering, not
  a real glob result.)
- `id: 18` — `enrich-existing-missing-path-fallback` — argument
  `meta/work/9999-nope.md` is path-like but the file does not
  exist. Expectations: "Response includes a warning that names the
  input path explicitly", "Response indicates the input is being
  interpreted as a topic string", "Conversation continues into
  topic-string Step 0 logic (asks business-context questions about
  the input as a topic)".
- `id: 19` — `enrich-existing-unparseable-frontmatter` — fixture
  file resolves but its frontmatter is unclosed. Expectations:
  "Response prints the unparseable-frontmatter diagnostic
  (substring 'Could not parse frontmatter')", "Conversation
  aborts — no Step 1 questions follow". (No fallback — the file
  clearly resolved, so the user intended a path.)
- `id: 20` — `enrich-existing-numeric-no-match-fallback` —
  argument `9999` matches the numeric discriminator but the glob
  returns zero. Expectations: "Response includes a warning that
  names the input number explicitly", "Response indicates the
  input is being interpreted as a topic string", "Conversation
  continues into topic-string Step 0 logic".

Each scenario includes:
- `prompt`: simulation in the same shape as existing evals (id 1–14).
- `files`: a list of fixture paths under `evals/files/` (e.g.
  `evals/files/0042-existing-sparse.md`). The harness passes these
  paths to the executor subagent; multi-fixture scenarios (id 17)
  list both files so the executor can simulate the glob result.
- `expected_output`: prose description of the expected response.
- `expectations`: a list of strings the grader subagent verifies
  against the recorded transcript and outputs. All grading is
  LLM-mediated by the grader (there is no programmatic-check
  mechanism in the harness). Examples:
  - `"Response cites the file's existing title or work_item_id"`
  - `"Transcript shows no invocation of work-item-next-number.sh"`
  - `"Response includes the missing-path warning string verbatim"`
  - `"Outputs directory contains no work item file"`

The `expected_output` field continues to capture the qualitative gist,
exactly mirroring the existing eval authoring style in `evals.json:8`.

Fixture work item files must be created under
`skills/work/create-work-item/evals/files/`:

- `0042-existing-sparse.md` — frontmatter complete (work_item_id: 0042,
  title, date, author Toby Clemson, type story, status draft, priority
  medium, parent "", tags []) with body sections present but containing
  template-style `[bracketed placeholder]` text — i.e. mostly gaps.
- `0042-existing-rich.md` — fully-fleshed body, used by Phase 2 for the
  "rich content already" path.
- `0099-broken-frontmatter.md` — frontmatter opens with `---` but never
  closes; used by id 19.
- `0042-other-slug.md` — second `0042-*.md` fixture for id 17. The
  harness does not stage fixtures into a working directory; the eval's
  `files` array lists both `0042-*.md` paths and the executor subagent
  is asked to treat the glob as returning both — the scenario therefore
  exercises the prompt's multi-match rendering rather than a real glob.
  This is acceptable because in normal operation multi-match is a
  defensive branch (work-item-next-number.sh enforces uniqueness, so
  two `0042-*.md` files only co-exist after a manual or concurrent
  authoring race).

Additional fixtures introduced in later phases (listed here for
single-source-of-truth):

- `0042-existing-mixed.md` (Phase 2 id 21a) — one body section
  contains one substantive sentence followed by a residual
  `[bracketed]` placeholder; rest of body is normal.
- `0042-existing-instructional.md` (Phase 2 id 21b) — one body
  section contains the template's instructional prose verbatim
  (e.g. "Describe the business value of this work item here…").
- `0099-mismatched-id.md` (Phase 3 id 31) — frontmatter
  `work_item_id: 0099` paired with a prompt that caches `0042`,
  used to exercise the at-write integrity-mismatch abort.

#### 3. Update `argument-hint`

**File**: `skills/work/create-work-item/SKILL.md`
**Change**: line 5

```yaml
argument-hint: "[topic or existing work item path/number]"
```

(~42 chars; visibly conveys the discriminator without overflowing
slash-command picker / status-line render budgets.)

#### 4. Add Step 0 resolution branch

**File**: `skills/work/create-work-item/SKILL.md` (lines 60–87 region)

Step 0 retains its existing two-branch structure (argument vs.
no-argument), preserving alignment with `stress-test-work-item:39-64`
and `refine-work-item:42-55`. The argument branch is split internally
between "enrich-existing" (path-like / numeric reference resolves to
an existing file) and "topic-string" (everything else, including
discriminators that did not match a real file). The full resolution
pattern is the union of `review-work-item/SKILL.md:35-50` and
`refine-work-item/SKILL.md:61-75`:

```markdown
## Step 0: Parameter Check

When this command is invoked:

1. **If an argument was provided**:

   First, **try to resolve the argument as a reference to an existing
   work item**. The discriminator order is path-like → numeric.

   - **Path-like** — argument contains `/` or ends in `.md`: treat as
     a file path. Resolve relative to the user's current working
     directory if relative, or use the argument verbatim if absolute.
     - File exists and resolves: continue to frontmatter validation.
     - File does not exist: print
       `"No work item at <path> — interpreting as topic string. If
       that's wrong, abort and re-run with a different argument
       (or /list-work-items to find a valid path)."` and proceed to
       topic-string handling below. (Glob layout assumption: work
       items live directly under `{work_dir}` with filename
       `NNNN-<slug>.md`. A relative path is resolved against the
       user's current working directory; an absolute path is
       taken verbatim.)
   - **Numeric** — argument matches `^[0-9]+$`: zero-pad to 4 digits
     (or use as-is if already ≥4) and glob `{work_dir}/NNNN-*.md`.
     The glob is case-sensitive and does not recurse; nested-
     subdirectory work items will not resolve.
     - One match: continue to frontmatter validation.
     - Multiple matches: list them as numbered options and ask the
       user to select by number or specify the full path (matching
       `update-work-item:51`).
     - Zero matches: print `"No work item numbered NNNN found in
       {work_dir} — interpreting as topic string. If that's wrong,
       abort and re-run with a different argument (or
       /list-work-items to find a valid number)."` and proceed to
       topic-string handling below.

   **Frontmatter validation** (only reached after a file was
   successfully resolved). Run `work-item-read-field.sh work_item_id
   <path>` once as a canonical existence/parse check. If it fails
   because the frontmatter is missing or unclosed, print:

   ```
   Could not parse frontmatter in <path> — the file may be corrupted.
   Re-open it and check that the YAML frontmatter is bracketed by two
   `---` lines and contains all nine required fields.
   ```

   and exit without spawning agents or writing. (The file clearly
   resolved, so the user intended a path; no fallback applies. The
   wording follows the structural form of `refine-work-item:67-74`
   but intentionally omits its trailing `/update-work-item`
   cross-reference — when a file has resolved cleanly there's no
   reason to point the user at a different skill.)

   If validation passes, read the file fully (frontmatter and body)
   and cache the identity fields in conversation state per the
   canonical Identity Field Rules (see Quality Guidelines). For each
   *missing* optional field, use the template default — do not abort.
   Set the conversation into "enrich-existing mode" with
   `existing_work_item_path` cached, and skip directly to Step 1 in
   that mode — do not run the vagueness check.

   **Topic-string handling**: if no resolution succeeded (either the
   discriminators did not trigger, or they triggered with a fallback
   warning), [existing topic-string content unchanged].

2. **If no argument was provided**: [existing content unchanged]
```

"Enrich-existing mode" replaces the placeholder `{enriching_existing}`
flag from earlier drafts of this plan. It is a conversation-state
concept — the model is told once at the end of Step 0 that subsequent
steps follow their "enrich-existing mode" sub-sections — rather than a
prompt-substitution variable. The existing rule numbers in the source
file (1, 2 of the current file) are preserved.

### Success Criteria

The skill-creator harness has no `--filter` flag; runs operate on
whichever `evals.json` is passed via `--eval-set`. To verify a subset
of scenarios, copy the relevant entries into a temporary eval file
(e.g. `/tmp/eval-1-to-14.json`) and pass that path. `runs_per_query`
defaults to 3, so per-scenario `trigger_rate` is `passes / 3`.

#### Automated Verification:

- [ ] All existing 14 eval scenarios pass on `with_skill` with
      `trigger_rate >= 0.9` (allowing one occasional grader miss out
      of three runs without blocking).
- [ ] New scenarios 15–20 pass on `with_skill` with `trigger_rate
      >= 0.9`.
- [ ] Existing 14 scenarios continue to pass on `old_skill`
      (the Phase 1 snapshot, which has no enrich-mode behaviour and
      so trivially fails 15–20 — that comparison is not informative
      and is not part of the success gate at this phase).
- [x] `bash skills/work/scripts/work-item-read-field.sh work_item_id evals/files/0042-existing-sparse.md` returns `0042`.
- [x] `bash skills/work/scripts/work-item-read-field.sh work_item_id evals/files/0099-broken-frontmatter.md` exits 1 (frontmatter unclosed).
- [ ] Markdown lint clean on `SKILL.md`: `make lint`. (No lint target in this project — SKILL.md reviewed manually.)
- [x] `evals/evals.json` ids are unique (note: Phase 2 introduces string IDs "21a"/"21b", so use
      `jq '.evals | map(.id | tostring) | length == (unique | length)' evals/evals.json`
      — prints `true`.

#### Manual Verification:

- [ ] `/create-work-item evals/files/0042-existing-sparse.md` prints a
      response that mentions the file's existing title and identifies
      it as the input being enriched, rather than asking
      topic-clarification questions.
- [ ] `/create-work-item meta/work/9999-nope.md` (path-like, missing)
      prints the fallback warning naming the path and proceeds into
      topic-string Step 0 (asks business-context questions about the
      pseudo-path treated as a topic).
- [ ] `/create-work-item 9999` (numeric, no match) prints the
      fallback warning naming the number and proceeds into
      topic-string Step 0.
- [ ] `/create-work-item evals/files/0099-broken-frontmatter.md` (file
      resolves but frontmatter is unclosed) prints the
      unparseable-frontmatter diagnostic and exits — no fallback.
- [ ] `/create-work-item add full-text search` (existing topic-string
      flow) still asks business-context questions in Step 1 — no
      regression.

---

## Phase 2: Enrich-Existing Variants for Steps 1–3

### Overview

Replace the broad-discovery and proposal-from-scratch behaviour in Steps 1
and 3 with gap-analysis and augmentation-framed variants when the
conversation is in **enrich-existing mode** (the state set at the end
of Step 0), and exclude the input file from Step 2's duplicate-detection
branch.

### Changes Required

#### 1. Add Step 1 enrich-existing variant

**File**: `skills/work/create-work-item/SKILL.md`
**Lines**: 89–105 region

Add a new sub-section under Step 1 covering the enrich-existing path:

```markdown
### In enrich-existing mode

Do not ask the broad discovery questions above. Instead:

1. Identify which body sections of the existing file are
   **substantive** (real content beyond `[bracketed placeholder]`
   text) and which are **gaps** (empty, placeholder-only, or missing
   entirely). Tag each gap with **exactly one** of the literal
   tokens — `empty` (section absent or contains no content),
   `placeholder-only` (section contains only template `[...]`
   blocks), `instructional-prose` (section contains the template's
   instructional prose carried over verbatim), or `partial` (section
   contains one or more substantive sentences alongside residual
   placeholders). Use the literal token; do not paraphrase. The
   eval grader pattern-matches on these exact strings.

2. Present the gap analysis briefly:

   ```
   I've read the existing work item (<resolved path>). Here's what
   looks complete and what still needs work:

   Complete: [section list]
   Gaps:
     - <Section> (<reason>)
     - ...

   I'll ask targeted questions about the gaps.
   ```

3. Ask only questions that address the identified gaps. Do not re-ask
   questions whose answers are already substantively present in the
   file.

4. If the existing content is rich enough that no obvious gaps remain,
   briefly confirm what the user wants to add or improve and proceed
   to Step 2.
```

#### 2. Add Step 2 self-exclusion note

**File**: `skills/work/create-work-item/SKILL.md`
**Lines**: 109–142 region

Insert one note in the duplicate-detection branch (around line 130):

```markdown
**In enrich-existing mode**: exclude the resolved input file from
the similarity scan. The {documents locator agent} search of
`{work_dir}` will find the file being enriched; do not surface it as
a "potential duplicate" of itself. Other near-duplicates discovered
are handled per the unchanged rules above.
```

The web-search-researcher spawn criterion is unchanged.

#### 3. Add Step 3 augmentation variant

**File**: `skills/work/create-work-item/SKILL.md`
**Lines**: 144–200 region

Add a new sub-section under Step 3:

```markdown
### In enrich-existing mode

Do not lead with a from-scratch proposal. Instead present a section-by-
section review and augmentation:

```
Here's how the existing work item reads against my research, with
proposed additions for the gaps:

**[Section name]** — [complete | needs improvement: <reason> | missing]
[existing content excerpt or note that it is missing]
[proposed addition or replacement, when applicable]

[repeat per section]

**Title**: [keep / propose new title with rationale]
**Type**: [keep existing <type> / propose change to <type> with rationale]
**Priority**: [keep / propose change with rationale]
**Parent**: [keep / propose change]
**Tags**: [keep / propose additions]
**Status**: [keep <cached> — say so if you'd like to transition it]
```

Apply the canonical Identity Field Rules (see Quality Guidelines):
propose only the fields marked Proposable above; never propose
changes to immutable fields; surface a status transition with
explicit confirmation only if the user makes a direct request for
one (the listing offers the affordance but never proposes a change
unsolicited).

The refinement loop in this step (challenging untestable criteria,
vague requirements, etc.) applies equally to existing and proposed
content.
```

### Success Criteria

#### Automated Verification:

- [ ] Existing 14 eval scenarios still pass on `with_skill` with
      `trigger_rate >= 0.9` (no regression to topic-string flow).
- [ ] New scenarios 21–24b pass on `with_skill` with `trigger_rate
      >= 0.9`.
- [ ] New scenarios 21–24b fail on `old_skill` (the snapshot
      re-taken at the end of Phase 1, which has Step 0 resolution
      but no enrich variants in Steps 1–3 — so these scenarios
      genuinely test the Phase 2 delta).
- [ ] Existing 14 scenarios continue to pass on `old_skill`.
- [ ] `make lint` passes on `SKILL.md`. (No lint target in this project — SKILL.md reviewed manually.)

The new scenarios are:

- `id: 21` — `enrich-existing-gap-analysis` — input
  `0042-existing-sparse.md`. Expectations include: "Response contains
  a `Complete:` listing and a `Gaps:` listing", "Each gap is tagged
  with one of empty / placeholder-only / instructional-prose /
  partial", "Step 1 follow-up questions target only sections listed
  under Gaps", "Transcript shows no broad-discovery questions (the
  default Step 1 vagueness check)".
- `id: 21a` — `enrich-existing-gap-mixed-section` — input
  `0042-existing-mixed.md` (one section contains one substantive
  sentence plus a residual `[bracketed]` placeholder). Expectations:
  "The mixed section is classified as a gap with reason `partial`",
  "Step 1 asks a follow-up about that section". Boundary case for
  the heuristic.
- `id: 21b` — `enrich-existing-gap-instructional-prose` — input
  `0042-existing-instructional.md` (one section contains the
  template's instructional prose verbatim, e.g. "Describe the
  business value here..."). Expectations: "The instructional-prose
  section is classified as a gap with reason
  `instructional-prose`". Boundary case.
- `id: 22` — `enrich-existing-rich-confirms` — input
  `0042-existing-rich.md`. Expectations: "Response confirms there
  are no obvious gaps", "Response asks what the user wants to add
  or improve", "Response does not re-ask the default Step 1
  discovery questions".
- `id: 23` — `enrich-existing-self-not-duplicate` — input file is
  surfaced by a simulated documents-locator result. Expectations
  (positive + negative paired so a paraphrased regression cannot
  pass via absence alone): "Response continues into Step 3
  augmentation framing referencing the input file by its preserved
  work_item_id and title", "Response does NOT enumerate the
  duplicate-handling options ('Proceed with new', 'Exit and
  update', 'Continue creating linked')".
- `id: 24` — `enrich-existing-augmentation-proposal` — input
  `0042-existing-sparse.md`. Expectations: "Step 3 output annotates
  each section with one of `[complete]`, `[needs improvement: ...]`,
  `[missing]`", "Output includes per-field lines for Title, Type,
  Priority, Parent, Tags marked `keep` or `propose change`",
  "Output does not include the from-scratch 'Suggested type' /
  'Requirements I'd suggest' framing from the topic-string flow".

#### Manual Verification:

- [ ] On a hand-prepared sparse work item, the gap analysis
      correctly identifies missing acceptance criteria as a gap
      tagged `empty` or `placeholder-only`, and asks only about it.
- [ ] On a richly-fleshed input file, the skill confirms
      completeness and asks for improvements without redundant
      discovery.
- [ ] On a section containing one substantive sentence plus a
      placeholder, the heuristic tags it `partial` and the
      follow-up question references the residual placeholder.

---

## Phase 3: Identity Preservation, Write-Back, and Quality Guidelines

### Overview

Add enrich-existing variants to Step 4 (draft) and Step 5 (write) so the
final draft uses the preserved identity fields, the H1 carries the real
NNNN, `work-item-next-number.sh` is never called in the enrichment path,
and the file is overwritten in place. Update the Quality Guidelines to
record the preservation rules.

### Changes Required

#### 1. Add Step 4 enrich-existing variant

**File**: `skills/work/create-work-item/SKILL.md`
**Lines**: 202–239 region

Add a new sub-section under Step 4:

```markdown
### In enrich-existing mode

1. Produce a complete updated draft incorporating existing content
   plus the additions approved in Step 3.
2. Apply the canonical Identity Field Rules (see Quality Guidelines)
   when filling frontmatter — the rules are the single source of
   truth for which fields are immutable, preserved, or proposable.
3. Apply the H1 sync rule (see Quality Guidelines).
4. Apply the Script avoidance rule (see Quality Guidelines).
5. Present the full updated draft, framed as an update preview:

   ```
   Here's the updated work item. The Step 5 confirmation will name
   the file path and ask for explicit approval before any write:

   [draft content]
   ```

6. Continue to challenge during review — apply the same rules as the
   normal-path Step 4. Iterate the draft until the user is happy
   with it. Do **not** treat a "looks good" mid-iteration as approval
   to write — that approval is gated by Step 5's single confirmation.
```

#### 2. Add Step 5 enrich-existing variant

**File**: `skills/work/create-work-item/SKILL.md`
**Lines**: 241–271 region

Add a new sub-section under Step 5:

```markdown
### In enrich-existing mode

1. Do **not** call `work-item-next-number.sh`. The target path is the
   resolved `existing_work_item_path` cached in Step 0.

2. **At-write identity-swap check** (best-effort guard, not a
   transactional lock): immediately before the confirmation prompt,
   re-read the target file's `work_item_id` from disk via
   `bash work-item-read-field.sh work_item_id <existing_work_item_path>`.
   This catches the case where the resolved path no longer points at
   the same work item (file deleted, replaced, or its frontmatter
   corrupted between Step 0 and Step 5). It does **not** catch
   concurrent body / `date` / `author` edits — those are recoverable
   via VCS (see Recovery section), not by this skill.
   - If the script exits non-zero (file gone, frontmatter
     unparseable since Step 0), abort with: `"Error: <path> is no
     longer present or its frontmatter is unparseable. Your
     proposed draft is below — copy it before re-running
     /create-work-item: <draft>"`.
   - If the on-disk `work_item_id` differs from the value cached in
     Step 0, abort with: `"Error: <path> changed identity since
     Step 0 (was <cached>, now <current>). Your proposed draft is
     below — copy it before re-running /create-work-item to refresh:
     <draft>"`.

   The error prefix `Error:` matches `update-work-item`'s convention
   (`update-work-item/SKILL.md:88-95`); peer alignment is preferable
   to a novel `Aborting overwrite:` form. The aborted message
   includes the proposed draft inline so the user can recover their
   work without redoing the conversation.

3. **Single confirmation gate**: present the path and a per-section
   change summary, then require explicit y/n approval before any
   write. (This is the *only* approval gate for the write — Step 4
   intentionally does not authorise it.) Lead with what's
   *changing* — Modified and Added sections / fields appear before
   Preserved ones, and any frontmatter field whose value differs
   from the Step 0 cached value is shown with explicit before→after
   to surface unexpected drift.

   ```
   I'm about to overwrite <existing_work_item_path>.

   Sections changed:
     Added:    [...]
     Modified: [...]

   Frontmatter changed:
     <field>: <cached> → <new>     # repeated per modified field
     (status: <cached> unchanged)  # always show status explicitly

   Sections preserved verbatim: <count> (<terse list or "none">)
   Frontmatter fields preserved verbatim: work_item_id, date, author
     [+ any unmodified proposable fields]

   Proceed? (y/n)
   ```

4. **Confirmation interpretation** (fail-safe):
   - Exactly `y` or `Y` (after trimming whitespace): proceed to
     step 5.
   - Exactly `n` or `N`: go to step 8.
   - Anything else (empty input, "yes", "go ahead", "looks good but
     also...", paragraph of feedback): treat as `n`. Print:
     `"Did not recognise <response> as y/n — staying in review.
     What change would you like before I overwrite?"` and go to
     step 8.

5. **On `y` — substitute the cached immutable fields, then write**:
   immediately before invoking the Write tool, re-read the cached
   immutable identity fields (`work_item_id`, `date`, `author`)
   from conversation state and overwrite the corresponding
   frontmatter lines in the draft text with those exact values,
   even if they appear unchanged. This is a textual substitution
   the model performs in the draft buffer — it exists to defend
   against the model having drifted those values during Step 4
   iteration. (This is grader-mediated, not transactional; the
   eval scenarios verify the *written file's* values match the
   cached values, which is the only test the harness can support.
   Status, title, and proposable fields use the Step 3-approved
   values.)

6. Write the file. The path-existence guard from the normal flow
   does not apply — overwrite is the intended behaviour, and the
   identity-swap check in step 2 just verified the file is still
   the one resolved in Step 0 (within the small TOCTOU window
   between check and write — concurrent edits in that window are
   recoverable via VCS only).

7. Print:

   ```
   Work item updated: <existing_work_item_path>
   ```

8. **On `n` or unrecognised**: stay in Step 4 / Step 5 review and
   iterate. Do not re-run the identity-swap check until the next
   `y`; do not write.
```

#### 3. Update Quality Guidelines

**File**: `skills/work/create-work-item/SKILL.md`
**Lines**: 273–323 region

Add a new "**Identity Field Rules**" group under Quality Guidelines,
after the existing "All frontmatter fields…" bullet. This is the
canonical home for the rules — Steps 0, 3, 4, and 5 reference these
rules by name rather than restating them. The taxonomy is a **new
convention** specific to enrich-existing mode; no peer skill currently
groups identity fields this way. If the convention proves useful, a
follow-up ticket can backfill it into `refine-work-item` (which also
mutates existing work items).

```markdown
**Identity Field Rules** (apply in enrich-existing mode):

- **Immutable** — `work_item_id`, `date`, `author`. Cached from the
  source file in Step 0; never proposed for change; the model
  substitutes the cached values back into the draft frontmatter at
  write time (Step 5 step 5) as a defence against drift during
  Step 4 iteration. This is a textual substitution the model
  performs, not a separate process — the eval suite verifies the
  written file's values match the cached values (id 25), which is
  the strongest guarantee the grader-mediated harness can provide.
- **Preserved unless explicitly changed** — `status`. Defaults to
  the cached value. May only change if the user makes an explicit,
  direct request during the conversation (e.g. "set status to
  in-progress", "move this to in-progress"). The model must not
  propose a status change unsolicited; if the user makes only an
  oblique reference (e.g. "this is now in flight"), the model
  asks a clarifying question rather than infer the transition. A
  proposed transition is shown explicitly (e.g. "draft →
  in-progress") and requires confirmation in Step 3 before
  acceptance. The `status: draft` default applies only to
  newly-created work items, not to enrichment.
- **Proposable in Step 3** — `title`, `type`, `priority`, `parent`,
  `tags`. Default to the cached values; can be replaced after
  explicit user agreement in Step 3's augmentation review.

**H1 sync**: the body H1 is `# <work_item_id>: <title>`, using the
cached immutable `work_item_id` (4-digit, never `XXXX`) and the
title as confirmed or replaced in Step 3 (cached title if the user
chose `keep`, new title if the user accepted a proposed change).

**Script avoidance**: `work-item-next-number.sh` is never called in
enrich-existing mode. The number is already cached. The path-existence
guard in Step 5 does not apply — overwrite is the intended behaviour
once the at-write identity-swap check passes.
```

### Success Criteria

#### Automated Verification:

- [ ] Existing 14 eval scenarios still pass on `with_skill` with
      `trigger_rate >= 0.9`.
- [ ] New scenarios 25–32 (including 30a) pass on `with_skill`
      with `trigger_rate >= 0.9`.
- [ ] New scenarios 25–31 (including 30a) fail on `old_skill`
      (the snapshot re-taken at the end of Phase 2, which has
      Step 0 + Steps 1–3 enrich variants but no Step 4 / Step 5 /
      identity-field enforcement). Id 32 (topic-string regression
      test) is expected to *pass* on both `old_skill` and
      `with_skill` — it tests behaviour the snapshot already
      supports.
- [ ] Existing 14 scenarios continue to pass on `old_skill`.
- [ ] `make lint` passes on `SKILL.md`.

The new scenarios are:

- `id: 25` — `enrich-existing-preserves-identity` — full conversation
  through Step 5. Expectations: "Written file's frontmatter
  `work_item_id` matches the cached value", "Written file's `date`
  matches the cached value", "Written file's `author` matches the
  cached value", "Written file's `status` matches the cached value
  (where the conversation did not propose a transition)".
- `id: 26` — `enrich-existing-no-next-number-call` — full conversation
  through Step 5. Expectations: "Transcript shows no invocation of
  work-item-next-number.sh at any point in the conversation".
- `id: 27` — `enrich-existing-h1-real-number` — full conversation
  through Step 5. Expectations: "Written file's body H1 begins with
  `# <cached work_item_id>:`", "Written file contains no literal
  `XXXX` token".
- `id: 28` — `enrich-existing-overwrites-path` — full conversation
  through Step 5. Expectations: "The Write tool's target path equals
  the cached `existing_work_item_path`", "The post-write
  confirmation message reads `Work item updated: <path>` verbatim".
- `id: 29` — `enrich-existing-status-preserved` — input file with
  `status: in-progress`, conversation does not propose any status
  change. Expectations: "Written file's `status` is `in-progress`",
  "Transcript contains no proposed status transition".
- `id: 30` — `enrich-existing-status-changed` — input file with
  `status: draft`. The simulated user message contains an explicit,
  direct request: "set status to in-progress". Expectations: "Step
  3 surfaces the proposed transition `draft → in-progress` and asks
  for explicit confirmation", "Written file's frontmatter `status`
  is `in-progress`", "Transcript shows the model did not propose a
  status change before the user asked for one".
- `id: 30a` — `enrich-existing-oblique-status-mention` — input file
  with `status: draft`. The simulated user message contains an
  oblique remark: "this is now in flight". Expectations: "Model
  asks a clarifying question about the user's status intent rather
  than inferring a transition", "Written file's `status` (if the
  conversation reaches Step 5) is whatever the user confirms after
  the clarifying question". Guards against eager-inference
  regressions.
- `id: 31` — `enrich-existing-identity-swap-attempted` — full
  conversation through Step 5; transcript-only check. Because the
  harness has no runtime file-mutation, no executor run can produce
  a genuine cached-vs-on-disk divergence within a single
  conversation. The scenario therefore verifies only that the
  *check is performed*, not that it fires: Expectations
  "Transcript at Step 5 shows a `work-item-read-field.sh
  work_item_id <path>` invocation immediately before the
  confirmation prompt is rendered", "Step 5 confirmation prompt
  appears after the read". The ability of the check to actually
  abort on drift is covered by Phase 3 manual verification (see
  Manual Verification below).
- `id: 32` — `topic-string-flow-still-calls-next-number` — full
  conversation through the **topic-string** flow (no
  discriminator), exercising Step 5 with `work-item-next-number.sh`
  *permitted* (unlike existing ids 7, 8, 11, 14 which prohibit it).
  Expectations: "Transcript shows exactly one invocation of
  `work-item-next-number.sh` during Step 5", "Written file's
  `work_item_id` is the value returned by that invocation",
  "Written file's body H1 begins with `# <work_item_id>:`". This
  protects the topic-string flow against accidental regression
  caused by the enrich-existing self-exclusion logic leaking into
  the default path.

#### Manual Verification:

- [ ] After running `/create-work-item meta/work/0042-existing-sparse.md`
      end-to-end on a sandbox copy of `meta/work/`, the file on disk
      has the same `work_item_id`, `date`, and `author` as before, an
      updated body, and a confirmation message naming the same path.
- [ ] On an input file with `status: in-progress` and no proposed
      change, the enrichment preserves it; on a fresh `status: draft`
      input, it stays `draft` unless the user explicitly proposes a
      transition during Step 3.
- [ ] **At-write identity-swap check covers all three failure
      modes** (the harness cannot exercise these — they are
      manual-only):
      - Delete the resolved file between Step 4 approval and Step
        5 confirmation; expect the "no longer present or
        unparseable" abort with the proposed draft inline.
      - Corrupt the resolved file's frontmatter (remove the closing
        `---`) between Step 4 and Step 5; expect the same abort.
      - Replace the resolved file with one whose `work_item_id`
        differs from the cached value; expect the
        identity-mismatch abort naming both values.
- [ ] On the y/n gate, type "yes", then "looks good", then a
      paragraph of feedback — each is treated as `n` with the
      "did not recognise" message; only `y` proceeds.

---

## Phase 4: End-to-End Benchmark

### Overview

Run the full skill-creator benchmark against the updated skill, regenerate
`evals/benchmark.json`, and confirm parity with the snapshotted baseline
on the existing 14 scenarios while every new scenario shows a positive
delta.

### Changes Required

For Phase 4 the relevant `old_skill` baseline is the original pre-edit
snapshot taken at the start of Phase 1 (not the per-phase rolling
snapshot used during development). The Phase 4 benchmark answers two
questions: (1) does the change preserve the existing 14-scenario
behaviour, and (2) do all 18 new scenarios pass on the final skill?
Restore the pre-edit snapshot under `skill-snapshot/` before running.

#### 1. Run the full benchmark

**Working dir**: `skills/work/create-work-item/evals/`

```bash
# Restore the original pre-edit skill directory as the old_skill
# baseline. The snapshot must be the full skill directory (matching
# the Phase 1 §1 cp -r), not just SKILL.md — the baseline subagent
# reads the directory.

# 1. Find the jj revision immediately preceding the first plan
#    commit (the commit that introduces the Phase 1 SKILL.md edits).
#    Replace <pre-plan-rev> below with that revision id from
#    `jj log skills/work/create-work-item/SKILL.md`.
PRE_PLAN_REV=<pre-plan-rev>

# 2. Restore the snapshot directory in full. Use jj's `restore`
#    against a temporary checkout, or extract via git tooling that
#    jj's git backend supports:
rm -rf meta/eval-workspaces/create-work-item/skill-snapshot
mkdir -p meta/eval-workspaces/create-work-item
git -C "$(jj root)" archive "$PRE_PLAN_REV" \
  skills/work/create-work-item \
  | tar -x -C meta/eval-workspaces/create-work-item
mv meta/eval-workspaces/create-work-item/skills/work/create-work-item \
   meta/eval-workspaces/create-work-item/skill-snapshot
rm -rf meta/eval-workspaces/create-work-item/skills

# 3. Verify shape parity with Phase 1's snapshot:
test -f meta/eval-workspaces/create-work-item/skill-snapshot/SKILL.md
test -d meta/eval-workspaces/create-work-item/skill-snapshot/evals

# 4. Run the benchmark.
python -m scripts.run_eval \
  --skill-name create-work-item \
  --workspace meta/eval-workspaces/create-work-item \
  --iteration N
python -m scripts.aggregate_benchmark \
  meta/eval-workspaces/create-work-item/iteration-N \
  --skill-name create-work-item
```

(`jj`'s git backend exposes `git archive` against any revision id;
this reproduces a full directory tarball. If your local tooling
prefers a different recipe, the only requirement is that
`meta/eval-workspaces/create-work-item/skill-snapshot/` ends up
identical to `skills/work/create-work-item/` at the pre-plan
revision, mirroring Phase 1 §1's `cp -r`.)

The aggregation produces a fresh `benchmark.json` and `benchmark.md`
following the schema seen in `skills/work/create-work-item/evals/benchmark.json`.

#### 2. Update the persisted benchmark

**File**: `skills/work/create-work-item/evals/benchmark.json`

Replace with the regenerated benchmark. The metadata block's
`evals_run` array should list every new scenario name added in Phases
1–3 in addition to the existing 14.

#### 3. Optional: review qualitative outputs

Use the skill-creator viewer
(`<skill-creator-path>/eval-viewer/generate_review.py`) to sanity-check
the textual outputs of the new scenarios. This is a manual step and is
not required for plan completion if the benchmark numbers are clean.

### Success Criteria

#### Automated Verification:

- [ ] **Per-scenario** parity gate on the existing 14: each
      scenario's `with_skill` mean `trigger_rate` is no more than
      one run (`1/3 ≈ 0.33`) below its `old_skill` mean
      `trigger_rate`. This catches per-scenario regressions in the
      topic-string flow that an aggregate-mean gate would absorb.
- [ ] **Aggregate** parity gate on the existing 14:
      `with_skill` mean ≥ `old_skill` mean. Both are expected to
      be near 1.0; this is a sanity check.
- [ ] Every new scenario (15–32, including 21a/21b, 30a) shows
      `with_skill` mean `trigger_rate >= 0.9` across
      `runs_per_query = 3` (so up to one occasional grader miss
      out of three runs is tolerated; two misses fail the gate).
- [ ] Every new scenario *that exercises enrich-mode behaviour
      not present in the pre-edit skill* (15–31, 30a, but NOT 32)
      shows `old_skill` mean `trigger_rate < 0.5` against the
      pre-edit snapshot — should hold trivially; check exists to
      detect a new eval so loosely worded the baseline
      accidentally satisfies it. Id 32 (topic-string regression
      test) is expected to pass on `old_skill` as well.
- [ ] `runs` array in `benchmark.json` includes a `with_skill` entry
      for every scenario in `evals/evals.json`.
- [ ] `make lint` passes on the workspace.

#### Manual Verification:

- [ ] Spot-check 3 new scenarios in the eval viewer — the gap analysis
      reads naturally and the augmentation framing in Step 3 is clear.
- [ ] Run `/create-work-item` on at least one real sparse work item in
      a sandbox copy of `meta/work/`, confirm the resulting file is
      structurally complete and the original identity fields are
      preserved.
- [ ] Run `/create-work-item add a brand-new feature` (no path) on the
      same sandbox to confirm the topic-string flow is unaffected.

---

## Architectural Tradeoffs

The plan makes several deliberate design choices worth documenting so
future maintainers can judge whether the reasoning still holds.

- **Mode of `/create-work-item` rather than a peer `/enrich-work-item`
  skill.** The two flows share Step 2 (research / duplicate detection)
  and the Step 3 challenge loop; lifting enrichment into a separate
  skill would either duplicate that prose or create a delegation
  chain. Discoverability cost is real: a user with a sparse work item
  may search for "enrich" and miss `/create-work-item <path>`. We
  mitigate by extending the skill `description:` frontmatter to
  mention enrichment explicitly. A separate `/enrich-work-item`
  alias is cheap to add later if discovery turns out to be a
  recurring problem.
- **Resolution pattern duplicated, not extracted.** The path-like /
  numeric resolver is now present in five skills (`review-`, `update-`,
  `refine-`, `stress-test-`, and now `create-`). Extracting a
  `work-item-resolve-ref.sh` helper is the cleaner long-term fix but
  is out of scope here — calling it would coordinate edits to four
  other SKILL.md files and their evals. **Follow-up**: open a separate
  ticket to consolidate once the fifth copy lands.
- **Boundary with `refine-work-item`.** This skill (`create-`) owns
  *content gap-fill* in enrich mode — populating empty or
  placeholder-only body sections through the same five-step
  conversation that drives create-from-scratch. `refine-work-item`
  owns *technical decomposition and sharpening* — splitting a story
  into subtasks, adding Technical Notes, tightening acceptance
  criteria into testable form. The skills overlap when a sparse work
  item also needs sharpening, but the entry-question is different:
  "what's missing?" → `create-work-item`; "is this work
  decomposable / testable?" → `refine-work-item`. The Step 3
  augmentation framing in this plan stays in the gap-fill register;
  it should not start proposing subtasks or technical splits.
- **Enrich-existing mode as conversation state, not a Markdown
  template variable.** The original draft used a `{enriching_existing}`
  placeholder, which collides with config-script substitution
  (`{work_dir}`, `{documents locator agent}`). The final plan uses
  the prose phrase
  "in enrich-existing mode" everywhere, set once at the end of
  Step 0. This is a Markdown-skill convention worth replicating in
  future skills that need conditional sub-sections.
- **In-place overwrite without a unified diff.** `refine-work-item`
  shows a unified diff before destructive content changes; this plan
  uses a per-section change summary (added / modified / preserved)
  plus the at-write identity-swap check instead. The diff is a
  strictly better affordance — pulling it into scope is the right
  next follow-up for this skill.

## Testing Strategy

### Unit Tests

The skill is Markdown; behaviour is exercised through the eval scenarios
rather than unit tests. The fixture files under `evals/files/` provide the
deterministic substrate for the resolution scenarios.

### Integration Tests

The skill-creator benchmark is the integration test surface. Each phase's
scenarios target the SKILL.md change introduced in that phase so red→green
transitions are visible per phase.

### Manual Testing Steps

1. Create a sandbox copy of `meta/work/` and add a hand-crafted sparse
   work item (frontmatter complete, body sections containing only
   `[bracketed]` placeholders). **Commit / snapshot the sandbox in jj
   first** so any unintended overwrite is recoverable via
   `jj restore <path>`.
2. Run `/create-work-item <path>` and follow the conversation through.
3. Confirm: the H1 carries the original NNNN; frontmatter
   `work_item_id`, `date`, and `author` are unchanged; `status` is
   unchanged unless you proposed a transition; the file at the same
   path is updated in place; the Step 5 `Work item updated: <path>`
   message names the same path.
4. Repeat with the numeric form (`/create-work-item NN`).
5. Repeat with a missing path (`/create-work-item meta/work/9999.md`)
   and a missing number (`/create-work-item 9999`) — both should
   print the fallback warning and proceed into topic-string Step 0.
6. Repeat with a broken-frontmatter file to confirm the
   no-fallback abort path.
7. Repeat with a no-arg invocation and a topic-string invocation to
   confirm the existing flows are unaffected (and that the
   topic-string flow still calls `work-item-next-number.sh`).

## Performance Considerations

No new agent spawns are introduced. The enrich-existing path actually
spawns *fewer* agents in some cases (Step 1 has no broad discovery
questions; Step 2 still spawns the documents-locator and conditionally
the web-search-researcher). The Step 0 frontmatter validation runs
`work-item-read-field.sh` once for the existence/parse check; per-field
extraction happens lazily in conversation context as fields are
referenced. The Step 5 at-write identity-swap check adds one more call.
Total ≤3 invocations of `work-item-read-field.sh` per enrichment —
negligible.

## Recovery

In-place overwrite is irreversible at the filesystem level. Recovery
relies on the VCS:

- This repo uses jujutsu (jj). The working copy is auto-snapshotted on
  every `jj` command, so an unintended overwrite is recoverable via
  `jj restore <path>` (or `jj diff <path>` to inspect first).
- For non-jj users running this skill in a git checkout,
  `git checkout HEAD -- <path>` restores the file to the last
  committed state.
- Recommend: ensure the target work item is committed / snapshotted
  before invoking enrichment, especially on hand-edited content that
  has not yet been committed.

The Step 5 at-write identity-swap check (re-reading `work_item_id` from
disk and aborting on drift) is an inline safeguard against overwriting
a file that changed identity between Step 0 and Step 5; it does not
substitute for the VCS-based recovery path above.

## Write atomicity

The Write tool used by Claude Code performs the standard
write-to-temp-then-rename atomic replacement on POSIX filesystems, so
a mid-write failure cannot leave a truncated file in place. This is an
implementation detail of the Write tool rather than a contract the
skill enforces; if the underlying tool changes semantics, the skill's
recovery story falls back entirely to the VCS path described above.

## Migration Notes

This plan changes the meaning of three classes of input to
`/create-work-item`:

- **Numeric (`^[0-9]+$`)** — previously a topic, now an existing-item
  reference. With the zero-match fallback (this plan), inputs that
  don't resolve to an existing item produce a one-line warning and
  fall through to topic-string interpretation, so the regression is
  observable but not silent: a user who typed `42` as a topic sees
  the warning and can either accept the fallback or hit Ctrl-C.
- **Path-like (contains `/` or ends `.md`)** — same fallback contract.
- **Anything else** — unchanged.

There is no on-disk format change; existing work items remain valid
input on both the topic-string path and the enrichment path. No
user-side migration is required, but users who routinely passed
numeric or path-like topic strings should expect the fallback
warning. The change is documented in the CHANGELOG entry for the
release that ships this skill.

## References

- Research: `meta/research/2026-04-27-create-work-item-open-from-existing.md`
- Skill being extended: `skills/work/create-work-item/SKILL.md`
- Existing evals: `skills/work/create-work-item/evals/evals.json`
- Existing benchmark: `skills/work/create-work-item/evals/benchmark.json`
- Resolution pattern source: `skills/work/review-work-item/SKILL.md:35-50`
- Multi-match selection pattern: `skills/work/update-work-item/SKILL.md:43-55`
- Unparseable-frontmatter guard: `skills/work/refine-work-item/SKILL.md:61-75`
- Field reader: `skills/work/scripts/work-item-read-field.sh`
- Work item template: `templates/work-item.md`
- Skill-creator workflow: `~/.claude/plugins/cache/claude-plugins-official/skill-creator/unknown/skills/skill-creator/SKILL.md`
