---
date: "2026-03-29T00:00:00+0000"
type: plan-review
skill: review-plan
target: "meta/plans/2026-03-28-initialise-skill-and-review-pr-ephemeral-migration.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, correctness, usability, compatibility, safety, standards]
review_pass: 2
status: complete
---

## Plan Review: Initialise Skill & review-pr Ephemeral File Migration

**Verdict:** REVISE

The plan is well-structured with sound phasing, correct dependency ordering,
and strong adherence to existing plugin conventions. The core architectural
decisions — making `tmp` a configurable path, separating ephemeral from
persistent data, and preserving graceful degradation — are all correct.
However, the plan contains a design conflict between the root `.gitignore`
entry and the inner `meta/tmp/.gitignore` that would prevent `.gitkeep` from
being tracked, several gaps around post-migration cleanup of stale ephemeral
directories, and a `config-summary.sh` restructuring that needs more precise
flow control specification.

### Cross-Cutting Themes

- **Root vs inner `.gitignore` conflict** (flagged by: Architecture,
  Correctness) — The plan creates both a root `.gitignore` entry for
  `meta/tmp/` and an inner `.gitignore` with `!.gitkeep`. Git does not
  descend into directories ignored by a parent-level rule, so the inner
  `.gitignore`'s exclusions never take effect. The `.gitkeep` would not be
  tracked, defeating the goal of preserving the directory across clones.

- **Tmp directory as initialisation proxy** (flagged by: Architecture,
  Correctness, Usability) — Using the tmp directory's existence as a proxy
  for "has initialise been run" is fragile because `review-pr` itself creates
  `meta/tmp/` via `mkdir -p` after Phase 3. A more specific marker like
  `meta/tmp/.gitignore` would distinguish organic creation from proper
  initialisation.

- **Stale ephemeral directories after migration** (flagged by: Compatibility,
  Usability, Safety) — Existing `pr-review-*/` directories under
  `meta/reviews/prs/` will remain after the migration with no cleanup
  guidance, leaving un-gitignored ephemeral files alongside persistent review
  artifacts.

- **`.gitignore` dedup fragility** (flagged by: Correctness, Usability,
  Safety, Standards) — Exact-line matching for duplicate detection won't catch
  entries with trailing whitespace or missing trailing slashes, potentially
  producing duplicates on repeated runs.

### Tradeoff Analysis

- **Root `.gitignore` vs inner `.gitignore` for tmp**: Using only the inner
  `.gitignore` (removing the root entry) allows `.gitkeep` and `.gitignore`
  to be tracked while still ignoring tmp contents. Using only the root entry
  is simpler but means the directory won't survive fresh clones. The plan
  needs to choose one strategy.

### Findings

#### Major

- 🟡 **Correctness**: Root `.gitignore` entry conflicts with inner
  `.gitignore` exclusions
  **Location**: Phase 2: Create the Initialise Skill, steps 3-4
  The root `.gitignore` entry `meta/tmp/` prevents git from descending into
  the directory at all, so the inner `.gitignore`'s `!.gitkeep` and
  `!.gitignore` exclusions never take effect. The `.gitkeep` file would never
  be tracked, and the directory would not be preserved in fresh clones.

- 🟡 **Architecture/Correctness/Usability**: Tmp directory existence is a
  fragile proxy for initialisation state
  **Location**: Phase 4: Add Initialisation Detection to SessionStart Hook
  After Phase 3, `review-pr` creates `meta/tmp/` via `mkdir -p`, satisfying
  the proxy check without proper initialisation. Users who run `review-pr`
  before `initialise` would never see the hint. Checking for
  `meta/tmp/.gitignore` instead would be more specific.

- 🟡 **Correctness**: `config-summary.sh` restructuring omits trailing
  instruction line on no-config-but-uninitialised path
  **Location**: Phase 4: Add Initialisation Detection to SessionStart Hook
  The current script unconditionally appends "Skills will read this
  configuration..." (lines 134-136). On the new path 4 (no config, not
  initialised), this trailing instruction would be misleading. The plan
  should specify that the trailing line is only appended when config files
  are present.

- 🟡 **Compatibility/Usability/Safety**: Ephemeral path migration leaves
  stale directories in existing consumer repos
  **Location**: Phase 3: Migrate review-pr Ephemeral Files
  Orphaned `pr-review-*/` directories under `meta/reviews/prs/` contain
  un-gitignored ephemeral files. No cleanup step or migration note is
  included.

- 🟡 **Usability**: Initialisation hint message lacks actionable wording
  **Location**: Phase 4: Add Initialisation Detection to SessionStart Hook
  The hint says "Run /accelerator:initialise" but this appears in hook
  context output. Using "Type `/accelerator:initialise` at the prompt" would
  be clearer for developers unfamiliar with slash commands.

- 🟡 **Correctness/Usability/Standards**: Exact-line gitignore dedup may
  produce duplicates
  **Location**: Phase 2: Create the Initialise Skill, step 4
  Entries with trailing whitespace or missing trailing slashes would not be
  matched, violating the idempotency goal. Should trim whitespace and check
  both `meta/tmp/` and `meta/tmp` variants.

#### Minor

- 🔵 **Architecture/Compatibility/Safety**: review-pr without initialise
  leaves ephemeral files un-gitignored for custom tmp paths
  **Location**: Phase 3: Migrate review-pr Ephemeral Files
  With default paths, the root `.gitignore` covers `meta/tmp/`. But custom
  `paths.tmp` values won't be gitignored until `initialise` runs.

- 🔵 **Usability**: No guidance on creating `.gitignore` when it doesn't
  exist
  **Location**: Phase 2: Create the Initialise Skill
  The testing strategy mentions this case but the skill instructions only say
  "handle it not existing". Should explicitly state to create the file.

- 🔵 **Compatibility**: `config-summary.sh` output contract changes
  **Location**: Phase 4: Add Initialisation Detection to SessionStart Hook
  Previously-silent sessions will now produce hook output. Should be
  documented in the CHANGELOG.

- 🔵 **Standards**: README directory table missing `tickets/` and `notes/`
  **Location**: Phase 5: Update Documentation
  The table lists 8 directories but omits these two configurable path keys.
  Phase 5 should add them.

- 🔵 **Safety**: No recovery guidance for partially-completed initialisation
  **Location**: Phase 2: Create the Initialise Skill
  The summary report should only be emitted after all steps complete, so an
  interrupted run does not produce a misleading "Initialisation complete"
  message.

- 🔵 **Standards**: Initialisation sentinel convention not documented
  **Location**: Phase 4: Add Initialisation Detection to SessionStart Hook
  The tmp directory doubles as an initialisation sentinel; a code comment
  should explain this.

#### Suggestions

- 🔵 **Compatibility**: Version bump and CHANGELOG entry needed for
  review-pr behavioral change
  **Location**: Phase 5: Update Documentation

- 🔵 **Usability**: Empty `argument-hint` gives no affordance
  **Location**: Phase 2: Create the Initialise Skill
  Consider `argument-hint: "(no arguments — safe to run repeatedly)"`.

- 🔵 **Architecture**: config-summary.sh coupling of config detection and
  initialisation concerns
  **Location**: Phase 4: Add Initialisation Detection to SessionStart Hook
  Acceptable for now; extract to a separate script if detection grows complex.

- 🔵 **Standards**: British English spelling (`initialise` vs `initialize`)
  **Location**: Phase 2: Create the Initialise Skill
  No action required if British English is the project convention.

### Strengths

- ✅ Phase ordering correctly respects dependency chains: infrastructure
  before consumer before migration before detection before documentation
- ✅ Making `tmp` a first-class configurable path maintains consistency with
  all other path keys and honours the existing override mechanism
- ✅ Clean separation of ephemeral and persistent data eliminates a real
  structural problem
- ✅ Preserving inline `mkdir -p` in existing skills maintains graceful
  degradation — skills work without requiring initialise
- ✅ Skill placement under `skills/config/` requires no `plugin.json` changes
- ✅ Frontmatter structure precisely mirrors the `configure` skill pattern
- ✅ Idempotency is a first-class design goal throughout
- ✅ The `!` backtick path resolution syntax is used consistently

### Recommended Changes

1. **Resolve root vs inner `.gitignore` conflict** (addresses: root
   `.gitignore` conflicts with inner exclusions)
   Choose one strategy: either (a) remove `meta/tmp/` from the root
   `.gitignore` and rely solely on the inner `.gitignore` to ignore contents
   while tracking `.gitkeep` and `.gitignore`, or (b) keep the root entry
   and accept the directory won't be tracked (skills recreate it via
   `mkdir -p`). Option (a) is recommended since it preserves the directory
   across clones.

2. **Use `meta/tmp/.gitignore` as the initialisation sentinel** (addresses:
   fragile proxy, false negatives from `review-pr`)
   In Phase 4, check for `meta/tmp/.gitignore` (or the configured
   equivalent) instead of just the directory's existence. This file is only
   created by `initialise`, not by `mkdir -p`.

3. **Add stale directory cleanup guidance** (addresses: orphaned ephemeral
   directories)
   Add a migration note in Phase 3 or Phase 5 indicating that existing
   `{pr reviews directory}/pr-review-*/` directories can be safely deleted.
   Optionally, have `initialise` detect and report them.

4. **Specify `config-summary.sh` conditional logic precisely** (addresses:
   trailing instruction line, output contract change)
   Explicitly state that the "Skills will read this configuration..."
   trailing line is only appended when config files are present. Document the
   output contract expansion in the CHANGELOG.

5. **Strengthen `.gitignore` dedup logic** (addresses: exact-line fragility)
   Specify trimming whitespace and checking for both trailing-slash and
   no-trailing-slash variants. Move the edge case from the testing section
   into the skill instructions.

6. **Improve initialisation hint wording** (addresses: hint lacks actionable
   copy-paste)
   Change "Run" to "Type" and make the slash command reference explicit:
   "Type `/accelerator:initialise` at the prompt".

7. **Explicitly handle missing `.gitignore`** (addresses: no guidance on
   creation)
   Add to Phase 2 instructions: "If no `.gitignore` exists at the project
   root, create one containing the required entries."

8. **Add `tickets/` and `notes/` to README table** (addresses: incomplete
   directory table)
   Expand Phase 5 to include these two missing entries.

9. **Add version bump and CHANGELOG entry** (addresses: behavioral change
   in review-pr)
   Include a version bump and CHANGELOG entry in Phase 5 documenting the
   ephemeral file migration and new initialise skill.

10. **Emit summary only after all steps complete** (addresses: partial
    completion)
    Add to Phase 2 skill instructions that the "Initialisation complete"
    summary should only be output after all steps have finished.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is well-structured with clear phase ordering that
respects dependency chains. The architectural decisions — making tmp a
first-class configurable path, cleanly separating ephemeral from persistent
data, and placing the skill in the existing config category — are sound and
consistent with established patterns. The main architectural concern is the
use of tmp directory existence as a proxy for initialisation state, which
couples detection logic to a single directory rather than an explicit state
marker.

**Strengths**:
- Phase ordering correctly respects dependencies: infrastructure (tmp path
  key) before consumer (initialise skill) before migration (review-pr) before
  detection (SessionStart hook) before documentation
- The decision to make tmp a configurable path via config-read-path.sh rather
  than hardcoding it maintains consistency with all other path keys and
  honours the existing override mechanism
- Clean separation of ephemeral and persistent data by moving review-pr
  working files to the tmp directory eliminates a real structural problem
  where transient and committed artifacts were co-located
- The explicit decision NOT to remove inline mkdir -p from existing skills
  preserves graceful degradation
- Placing initialise under skills/config/ alongside configure requires no
  plugin.json changes

**Findings**:

- 🟡 **major** (medium confidence) — Tmp directory existence is a fragile
  proxy for initialisation state
  **Location**: Phase 4: Add Initialisation Detection to SessionStart Hook
  The plan uses the existence of the tmp directory as a proxy for whether
  initialise has been run. However, any skill writing to tmp (e.g., review-pr
  after Phase 3) will create it via mkdir -p, and users could also create it
  manually.

- 🔵 **minor** (medium confidence) — Dual gitignore strategy for tmp creates
  overlapping ignore mechanisms
  **Location**: Phase 2: Create the Initialise Skill
  The plan creates both a root .gitignore entry for the tmp path and a
  self-contained .gitignore inside the tmp directory itself. These two
  mechanisms serve the same purpose, creating redundancy.

- 🔵 **minor** (medium confidence) — review-pr now depends on tmp path
  without ensuring directory exists
  **Location**: Phase 3: Migrate review-pr Ephemeral Files
  In consumer repositories that have not run initialise and do not have
  meta/tmp/ in their root .gitignore, ephemeral files could appear in git
  status.

- 🔵 **suggestion** (low confidence) — config-summary.sh restructuring
  increases coupling between config detection and initialisation concerns
  **Location**: Phase 4: Add Initialisation Detection to SessionStart Hook
  Combining them in one script is pragmatic but worth noting.

### Correctness

**Summary**: The plan is logically sound overall, with correct phasing and
well-identified line references in review-pr. The main correctness concerns
are around the config-summary.sh restructuring in Phase 4 and a potential
double-gitignoring conflict between the initialise skill's tmp directory
.gitignore and the root .gitignore entry.

**Strengths**:
- Line references in Phase 3 were verified against the actual SKILL.md and
  are accurate for all cited locations
- The phasing is correctly ordered — no phase depends on work not yet
  completed
- The idempotency design is correct: mkdir -p is inherently idempotent,
  .gitignore duplicate checking uses exact line match
- The observation that config-read-path.sh needs only a comment change is
  verified

**Findings**:

- 🟡 **major** (high confidence) — config-summary.sh restructuring omits
  trailing instruction line on the no-config-but-uninitialised path
  **Location**: Phase 4: Add Initialisation Detection to SessionStart Hook
  On path 4 (no config files AND not initialised), the trailing instruction
  "Skills will read this configuration..." would be misleading.

- 🔵 **minor** (medium confidence) — Redundant gitignoring: tmp directory
  contents are ignored by both root .gitignore and tmp/.gitignore
  **Location**: Phase 2: Create the Initialise Skill, step 4
  The root entry prevents git from descending into the directory, so the
  inner .gitignore's !.gitkeep never takes effect. The .gitkeep will never
  be tracked.

- 🔵 **minor** (high confidence) — Exact-line matching for .gitignore dedup
  may miss equivalent entries
  **Location**: Phase 2: Create the Initialise Skill, step 4
  An entry like meta/tmp/ would not match meta/tmp (without trailing slash).

- 🔵 **minor** (medium confidence) — Initialisation proxy check assumes tmp
  directory is never created organically
  **Location**: Phase 4: Add Initialisation Detection to SessionStart Hook
  Running review-pr before initialise would satisfy the proxy check.

### Usability

**Summary**: The plan introduces a well-structured initialisation skill that
significantly improves the first-run developer experience. A few usability
gaps remain around error feedback, discoverability, and the gitignore
matching strategy.

**Strengths**:
- Idempotency is a first-class design goal
- The decision to keep inline mkdir -p means review-pr still works without
  initialise — progressive disclosure
- The summary report format gives clear feedback
- Consistency with existing patterns maintained throughout

**Findings**:

- 🟡 **major** (high confidence) — Initialisation hint message lacks
  actionable copy-paste invocation
  **Location**: Phase 4: Add Initialisation Detection to SessionStart Hook
  Using "Type" rather than "Run" would make it clearer this is a prompt
  action.

- 🟡 **major** (medium confidence) — Exact-line-match gitignore dedup is
  fragile against whitespace and trailing newline variations
  **Location**: Phase 2: Create the Initialise Skill
  Repeated runs could append duplicate entries, undermining idempotency.

- 🔵 **minor** (high confidence) — No guidance on what to do when .gitignore
  doesn't exist
  **Location**: Phase 2: Create the Initialise Skill
  Should explicitly state to create the file if missing.

- 🔵 **minor** (medium confidence) — No migration guidance for existing
  ephemeral directories
  **Location**: Phase 3: Migrate review-pr Ephemeral Files
  Stale pr-review-*/ directories will remain after migration.

- 🔵 **suggestion** (medium confidence) — Empty argument-hint gives no
  affordance
  **Location**: Phase 2: Create the Initialise Skill
  Consider reinforcing the idempotency message in the hint.

- 🔵 **suggestion** (low confidence) — tmp directory existence as proxy may
  produce false negatives
  **Location**: Phase 4: Add Initialisation Detection to SessionStart Hook

### Compatibility

**Summary**: The plan is largely additive and backward-compatible. The main
compatibility concern is that the review-pr ephemeral path migration is a
silent behavioral change that could leave stale directories in consumer repos.

**Strengths**:
- The config-read-path.sh change requires no functional modification
- Inline mkdir -p preserved in skills for backward compatibility
- No plugin.json manifest changes needed
- Idempotency design avoids common duplicate-entry pitfalls

**Findings**:

- 🟡 **major** (high confidence) — Ephemeral path migration leaves stale
  directories in existing consumer repos
  **Location**: Phase 3: Migrate review-pr Ephemeral Files
  No cleanup step or migration note is included.

- 🔵 **minor** (high confidence) — config-summary.sh output contract changes
  for hook consumer
  **Location**: Phase 4: Add Initialisation Detection to SessionStart Hook
  Previously-silent sessions will now produce hook output.

- 🔵 **minor** (medium confidence) — review-pr requires tmp directory to
  exist before first run without initialise
  **Location**: Phase 3: Migrate review-pr Ephemeral Files
  Custom paths.tmp values won't be gitignored until initialise runs.

- 🔵 **suggestion** (high confidence) — Version bump needed for behavioral
  change in review-pr
  **Location**: Phase 5: Update Documentation

### Safety

**Summary**: The plan is fundamentally a safety improvement — it separates
ephemeral data from persistent artifacts, reducing accidental commit risk.
Two minor concerns exist around partial completion recovery and post-migration
cleanup.

**Strengths**:
- The core motivation is itself a safety improvement
- Full idempotency design prevents corruption on repeated runs
- Graceful degradation via inline mkdir -p fallbacks
- Lightweight detection mechanism avoids complex state tracking

**Findings**:

- 🔵 **minor** (medium confidence) — No recovery path if initialise
  partially completes
  **Location**: Phase 2: Create the Initialise Skill
  Summary should only be emitted after all steps complete.

- 🔵 **minor** (medium confidence) — .gitignore append is not atomic and has
  a whitespace edge case
  **Location**: Phase 2: Create the Initialise Skill
  Functionally harmless but could create duplicate entries.

- 🔵 **minor** (high confidence) — No mention of cleaning up old ephemeral
  directories from previous location
  **Location**: Phase 3: Migrate review-pr Ephemeral Files
  Historical ephemeral data remains in the un-gitignored location.

- 🔵 **suggestion** (medium confidence) — review-pr depends on tmp for
  custom paths
  **Location**: Phase 3: Migrate review-pr Ephemeral Files
  Custom paths require initialise for full gitignore coverage.

### Standards

**Summary**: The plan demonstrates strong adherence to existing project
conventions across file organisation, naming, frontmatter patterns, and
configuration management.

**Strengths**:
- Skill placement follows the established category/skill/SKILL.md pattern
- Frontmatter mirrors the configure skill's structure
- Path key naming convention respected
- The ! backtick resolution syntax used consistently
- Backward compatibility maintained

**Findings**:

- 🔵 **minor** (high confidence) — README directory table missing tickets
  and notes entries
  **Location**: Phase 5: Update Documentation
  The table lists 8 directories but omits these two configurable path keys.

- 🔵 **minor** (medium confidence) — Initialisation detection heuristic uses
  only tmp directory
  **Location**: Phase 4: Add Initialisation Detection to SessionStart Hook
  Convention should be documented in a code comment.

- 🔵 **suggestion** (medium confidence) — Gitignore duplicate detection may
  miss variant formats
  **Location**: Phase 2: Create the Initialise Skill

- 🔵 **suggestion** (low confidence) — Skill name uses British English
  spelling
  **Location**: Phase 2: Create the Initialise Skill
  No action required if British English is the project convention.

## Re-Review (Pass 2) — 2026-03-29

**Verdict:** APPROVE

### Previously Identified Issues

- ✅ **Architecture**: Tmp directory existence is a fragile proxy for
  initialisation state — **Resolved**. Sentinel changed to
  `meta/tmp/.gitignore` with explicit code comment.
- ✅ **Architecture**: Dual gitignore strategy creates overlapping ignore
  mechanisms — **Resolved**. Root `.gitignore` entry removed; inner
  `.gitignore` is now the sole mechanism, with clear explanation of why.
- ✅ **Architecture**: review-pr depends on tmp path without ensuring
  directory exists — **Resolved**. `mkdir -p` retained in review-pr;
  initialise handles the `.gitignore` setup.
- ✅ **Architecture**: config-summary.sh coupling — **Resolved**. Accepted as
  pragmatic; flow is well-specified with clear branching.
- ✅ **Correctness**: config-summary.sh trailing instruction on uninitialised
  path — **Resolved**. Path 4 explicitly excludes the trailing instruction.
- ✅ **Correctness**: Root .gitignore prevents inner .gitignore exclusions —
  **Resolved**. Root entry removed entirely.
- ✅ **Correctness**: Exact-line .gitignore dedup — **Resolved**. Whitespace
  trimming now specified.
- ✅ **Correctness**: Proxy check assumes tmp not created organically —
  **Resolved**. Sentinel is now `tmp/.gitignore`, not directory existence.
- ✅ **Usability**: Hint lacks actionable wording — **Resolved**. Changed to
  "Type `/accelerator:initialise` at the prompt".
- ✅ **Usability**: Gitignore dedup fragility — **Resolved**. Whitespace
  trimming and variant checking specified.
- ✅ **Usability**: No guidance on missing .gitignore — **Resolved**.
  Explicit instruction to create the file if absent.
- ✅ **Usability**: No migration guidance for stale directories — **Resolved**.
  Phase 3j adds migration note with cleanup command.
- ✅ **Usability**: Empty argument-hint — **Resolved**. Now reads "(no
  arguments — safe to run repeatedly)".
- ✅ **Usability**: Proxy false negatives — **Resolved**. Sentinel change
  addresses this.
- ✅ **Compatibility**: Stale directories after migration — **Resolved**.
  Phase 3j + CHANGELOG entry.
- ✅ **Compatibility**: config-summary.sh output contract change — **Resolved**.
  Flow restructured with 5-point conditional logic; CHANGELOG documents it.
- ✅ **Compatibility**: Version bump needed — **Resolved**. Phase 5 includes
  version bump and CHANGELOG.
- ✅ **Safety**: No recovery for partial completion — **Resolved**. Summary
  only emitted after all steps complete.
- ✅ **Safety**: .gitignore whitespace edge case — **Resolved**. Whitespace
  trimming specified.
- ✅ **Safety**: No cleanup for old ephemeral directories — **Resolved**.
  Migration note added.
- ✅ **Standards**: README missing tickets/notes — **Resolved**. Phase 5 adds
  both rows.
- ✅ **Standards**: Sentinel convention not documented — **Resolved**. Code
  comment text specified.
- ✅ **Standards**: Gitignore variant formats — **Resolved**. Whitespace
  trimming specified.
- ✅ **Standards**: British English spelling — **Resolved**. Confirmed as
  project convention.

### New Issues Introduced

- 🔵 **Correctness** (suggestion): The trailing-slash variant check
  description mentions checking for `.claude/accelerator.local.md` "as-is"
  but this is a file path where a trailing-slash variant doesn't apply. Minor
  wording ambiguity; no functional impact since the only current entry is a
  file path.

- 🔵 **Compatibility** (minor): review-pr without initialise still leaves
  ephemeral files visible in `git status` / `jj status` because the inner
  `.gitignore` isn't present. Mitigated by the SessionStart hook nudging
  users to initialise, and by the existing cleanup guideline in the skill.

### Assessment

All 24 findings from the initial review have been resolved. The plan is now
architecturally sound, with a clean single-mechanism gitignore strategy, a
robust initialisation sentinel, well-specified conditional flow in
config-summary.sh, and comprehensive documentation including migration notes
and a version bump. The two new observations are low-severity and do not
warrant further revision. The plan is ready for implementation.
