---
type: "work-item-review"
id: "0241-migrate-false-dirty-tree-detection-review-1"
title: "Work Item Review: False Dirty-Tree Detection on jj-Colocated Repositories"
date: "2026-09-23T01:57:40+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0241"
work_item_id: "0241"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 6
tags: []
last_updated: "2026-09-23T16:28:26+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: False Dirty-Tree Detection on jj-Colocated Repositories

**Verdict:** REVISE

The work item is structurally complete. It states the root cause precisely (jj's `@` is rewritten on every snapshot, whereas git's `HEAD` is stable) under one governing principle, and it points to concrete code sites. It is not ready to plan, for three reasons. The corpus-metadata consequence of moving the revision fact is left unresolved. The merge-commit revision value is undefined. The acceptance criteria verify only migrate out of the three consumers named in Requirement 2, and they check git against an unrecorded baseline. Scope also bundles the observed #97 defect with parity fixes that were inferred rather than observed.

### Cross-Cutting Themes

- **Corpus-metadata revision semantics unresolved** (flagged by: clarity, completeness, dependency, scope, testability): Requirement 1 changes the jj revision at the fact level, so corpus metadata stamping changes too. The item leaves this as an Open Question with no requirement, criterion or dependency entry, which makes the boundary of the work undefined.
- **Merge-commit revision value undefined** (flagged by: clarity, testability): Requirement 1 says "parent" in the singular, while the merge criterion asks for "all of the parents". No value format is given for the merge case, and the criterion does not require the value to be snapshot-invariant.
- **Requirement 2 consumers beyond migrate** (flagged by: testability, completeness, dependency): `work sync` dirtiness and the VCS status renderer are named as in scope, but no criterion covers them and no dependency entry records their owners.
- **Observed and inferred defects mixed together** (flagged by: scope, completeness, clarity): Requirements 2 and 3 are inferred parity work. The Summary presents them as observed, and bundling them delays the #97 fix.
- **Size threshold source ambiguous** (flagged by: clarity, testability): it is unclear whether the adapter honours the user's configured `snapshot.max-new-file-size` or a hard-coded 1 MiB. No boundary case is specified.
- **"Advances only when a new change is started" is too strong** (flagged by: clarity, testability): rebasing `@`, `jj edit`, or rewriting `@-` also change `@`'s parent. `jj describe` is not covered on the "unchanged" side.

### Findings

#### Major

- 🟡 **Scope / Dependency / Clarity**: Revision-fact change silently widens scope to corpus metadata
  **Location**: Open Questions / Requirements / Dependencies
  Requirement 1 changes the revision at the fact level, which also changes corpus document metadata stamping. That consumer has no stated outcome, no criterion and no dependency entry, and the Open Question gates the design (one fact or two).
- 🟡 **Clarity / Testability**: Merge-commit revision has no defined expected value
  **Location**: Requirements / Acceptance Criteria
  "The working-copy commit's parent" (singular) conflicts with "identifies all of the parents". No format is given (sorted join, digest, etc.), and nothing requires the merge value to stay stable under snapshot.
- 🟡 **Testability**: Requirement 2's three named consumers are verified only through migrate
  **Location**: Requirements / Acceptance Criteria
  A fix made at the migrate call site, instead of in the shared snapshot, would pass every criterion while `work sync` and the status renderer stay broken. Completeness raised the same gap as a minor finding.
- 🟡 **Testability**: Git no-regression criterion checks against an unrecorded baseline
  **Location**: Acceptance Criteria
  "Matches today's behaviour" is never stated per scenario. Once the change lands, no baseline remains to compare against.
- 🟡 **Scope**: Observed resume defect bundled with inferred, unobserved parity fixes
  **Location**: Requirements
  Requirement 1 alone fixes #97. Requirements 2 and 3 are inferred, reach more consumers, and could ship or be reverted on their own.
- 🟡 **Completeness**: Reproduction steps not captured as a single unit
  **Location**: Context
  Input, action, expected and actual outcome are spread across Context and AC1. The observed refusal output and the jj/git versions are missing.

#### Minor

- 🔵 **Clarity / Testability**: Size threshold source ambiguous, no boundary case
  **Location**: Requirements / Acceptance Criteria
  The item does not say whether the configured value or a fixed 1 MiB applies. No at-or-below-limit case and no "migrate proceeds" outcome are given.
- 🔵 **Clarity / Completeness**: Summary presents inferred defects as observed
  **Location**: Summary / Requirements
  The Summary says both defects force `ACCELERATOR_MIGRATE_FORCE=1`. The Assumptions say the exclude and size gaps are inferred. The Summary also omits Requirements 3 and 4.
- 🔵 **Testability / Clarity**: "Advances only when a new change is started" is overstated and under-tested
  **Location**: Requirements / Acceptance Criteria
  The parent also changes on rebase, `jj edit`, or a rewrite of `@-`. `jj describe` is not asserted on the "unchanged" side.
- 🔵 **Testability**: Refusal criterion does not check that owned paths are left out
  **Location**: Acceptance Criteria
  The fixture contains only foreign dirt, so an implementation that lists every dirty path would pass.
- 🔵 **Clarity**: Several overlapping, undefined terms for the resume anchor
  **Location**: Summary / Context / Requirements
  The anchor is called run base, run id, VCS revision fact, reported revision and run-base check in different sections, and none of these is defined.
- 🔵 **Clarity**: "Snapshot" used in two senses
  **Location**: Requirements
  It means both jj's operation and the adapter's dirty-path computation.
- 🔵 **Clarity**: Undefined domain terms and loose referents in criteria
  **Location**: Acceptance Criteria / Assumptions
  "Migrate scope", "the named decisions file", "the reported revision", "foreign dirt", "listed paths" and "described-but-not-closed" have no definition or clear referent.
- 🔵 **Scope**: Refusal path listing is an independent diagnostic enhancement
  **Location**: Requirements
  Requirement 4 does not fix false detection, and it is the only change to git output.
- 🔵 **Dependency**: Other consumers of the shared jj snapshot not captured as couplings
  **Location**: Dependencies
  `work sync` and the status renderer (e.g. 0198) will change output on jj, and neither is listed.
- 🔵 **Dependency**: Shared migrate dirty-path surface with 0286 lacks a coordination note
  **Location**: Dependencies
  The 0286 skill guidance may contradict the new refusal text, depending on which item lands first.
- 🔵 **Dependency**: Parity target depends on jj version and configuration
  **Location**: Dependencies
  The item does not say which pinned jj version defines "as `jj status` does".

#### Suggestions

- 🔵 **Dependency**: Persisted run ids from before the fix will not match the new revision
  **Location**: Dependencies / Assumptions
  Users stalled mid-migration at upgrade, which is the #97 population, may still need `ACCELERATOR_MIGRATE_FORCE` once. The item should either accept this and document it, or handle it.
- 🔵 **Testability**: Use `jj status` as an explicit differential oracle
  **Location**: Summary / Acceptance Criteria
  Assert that the reported dirty paths equal `jj status` output on each fixture.
- 🔵 **Scope**: Confirm the parent epic
  **Location**: Frontmatter: parent
  0136 (shell-to-Rust migration) may not be the natural home for a jj adapter behaviour bug.

### Strengths

- ✅ Context gives a precise causal account (`HEAD` is stable, `@` is rewritten per snapshot) and one governing principle that every requirement traces back to.
- ✅ AC1 is an executable reproduction of #97 that exercises the root cause, including the `jj status` snapshot that rewrites `@`.
- ✅ AC2 asserts both halves of the revision invariant (unchanged after edit plus snapshot, changed after `jj new`/`jj commit`).
- ✅ The exclude criterion covers `core.excludesFile` and `.git/info/exclude` separately, with a concrete precondition.
- ✅ Requirement 5 sets an explicit outer boundary on git behaviour. Adjacent bugs (0124, 0286) are cross-referenced rather than absorbed.
- ✅ The Assumptions and Drafting Notes state openly what was observed and what was inferred, and how "every consumer" was read.
- ✅ Technical Notes name concrete implementation sites, and Dependencies explain why each related item matters.

### Recommended Changes

1. **Resolve the corpus-metadata Open Question and encode the answer** (addresses: revision-fact change widens scope; unresolved open question; corpus metadata coupling; no criterion for metadata)
   Decide between two options: metadata stamps `@`'s parent, or metadata keeps `@` and the run-base check gets its own anchor. Add a requirement and a criterion for the chosen behaviour, list corpus metadata as a coupled consumer, and remove the Open Question.
2. **Define the merge-case revision value** (addresses: merge-commit revision undefined; singular "parent" conflict)
   Rephrase Requirement 1 as "derived from the set of `@`'s parents". Specify the format, e.g. ids sorted and joined by `+`. Add "unchanged after edit plus snapshot" to the merge criterion.
3. **Decide on the split** (addresses: observed and inferred defects bundled; refusal path listing independent)
   Either keep this item to Requirement 1 (plus Requirement 4 if justified as a diagnostic aid for #97) and move the exclude and size parity work into a sibling bug, or keep the bundle and state why.
4. **Cover all snapshot consumers in the criteria** (addresses: Requirement 2 consumers verified only through migrate; consumers not captured as couplings)
   Check the exclude and size fixtures through migrate, `work sync` and the status renderer. Add Related entries for the owners of those features.
5. **Write out the git baseline explicitly** (addresses: git no-regression against an unrecorded baseline)
   Replace "matches today's behaviour" with the expected git outcome for the resume, exclude and refusal scenarios.
6. **Tighten the size and refusal criteria** (addresses: size threshold ambiguity; refusal owned-path exclusion)
   Honour the configured `snapshot.max-new-file-size`, falling back to 1 MiB when unset. Add a below-limit case and a custom-limit case. Add a mixed-tree refusal case that must not name owned paths.
7. **Restate the revision invariant precisely** (addresses: "advances only when a new change is started")
   State that the revision changes exactly when `@`'s parents change. Assert that `jj describe` leaves it unchanged and that rebasing `@` changes it.
8. **Add a Reproduction section and correct the Summary** (addresses: reproduction not captured as a unit; Summary presents inferred defects as observed)
   Give numbered steps, the observed refusal text from #97, and the jj/git versions. In the Summary, separate the observed defect from the inferred parity gaps.
9. **Record the pre-fix run-id transition** (addresses: persisted run ids from before the fix)
   State in Assumptions or Requirements how a run stalled before the upgrade behaves after it.
10. **Normalise terminology and note the remaining couplings** (addresses: overlapping anchor terms; two senses of "snapshot"; undefined terms; 0286 coordination; jj version coupling)
    Define the run base once, rename the adapter's snapshot to "dirty-path computation", and define foreign dirt and migrate scope. Add an ordering note for 0286 and name the pinned jj version as the parity target.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The item is largely clear, with a stated governing principle and named code locations. Its main defects are the singular-versus-plural parent conflict with the merge criterion, a Summary that frames a dirty-tree fix while Requirement 1 changes the revision fact for every consumer, and several overlapping or undefined terms.

**Strengths**:
- Context states the root cause precisely and gives a single governing principle.
- Each requirement names the concrete VCS behaviour it should match.
- The Drafting Notes and Assumptions separate observed defects from inferred ones.

**Findings**:
- 🟡 major / high — **Requirements** — Singular "parent" in Requirement 1 conflicts with the multi-parent merge criterion. The item gives no value format and does not say what "their order" refers to, so implementations could diverge on recorded run ids and metadata stamps.
- 🟡 major / medium — **Summary** — The Summary frames a dirty-tree fix, but Requirement 1 changes the revision fact for every consumer. It is unclear whether the metadata change is intended, so implementers could ship different scopes.
- 🔵 minor / high — **Summary** — Several overlapping, undefined terms for the resume anchor (run base, run id, VCS revision fact, reported revision, run-base check). Define it once.
- 🔵 minor / medium — **Requirements** — "Snapshot" is used both for jj's operation and for the adapter's dirty-path output. Rename the latter.
- 🔵 minor / medium — **Requirements** — The size threshold does not say whether it uses the configured jj value or a fixed 1 MiB.
- 🔵 minor / medium — **Summary** — The Summary presents inferred defects as observed and omits Requirements 3 and 4.
- 🔵 minor / medium — **Acceptance Criteria** — Undefined terms and loose referents: migrate scope, named decisions file, reported revision, foreign dirt, listed paths, "closed".
- 🔵 suggestion / low — **Requirements** — "Advances only when a new change is started" is broader than the criteria support. Rephrase it as "changes exactly when `@`'s parents change".

### Completeness

**Summary**: The item is structurally complete, with valid frontmatter and a causal Context. The gaps are specific to a bug: the reproduction is not written as one unit, the secondary defects have no observed reproduction, and the open question that shapes Requirement 1 is unresolved.

**Strengths**:
- Frontmatter is complete and consistent.
- Context explains why the defect occurs and states the governing principle.
- Requirements are numbered and specific, with an explicit git boundary.
- The Assumptions and Drafting Notes separate observation from inference.
- Dependencies are populated and annotated.
- Technical Notes point to concrete implementation sites.

**Findings**:
- 🟡 major / high — **Context** — Reproduction steps are not captured as a single unit. Add a Reproduction section with steps, the observed refusal output from #97, the expected behaviour, and the jj/git versions.
- 🔵 minor / high — **Requirements** — The exclude and size-parity defects have no observed outcome. Either reproduce them or label them as parity hardening.
- 🔵 minor / medium — **Acceptance Criteria** — The criteria cover only migrate among the named snapshot consumers.
- 🔵 minor / medium — **Open Questions** — The unresolved open question affects the scope of Requirement 1.

### Dependency

**Summary**: Related entries are well annotated and there are no hard upstream blockers. Couplings the body implies are missing: corpus metadata stamping as a decision gate, the other snapshot consumers, the shared surface with 0286, jj version behaviour, and run ids persisted before the fix.

**Strengths**:
- Each Related entry says why it is related.
- The corpus-metadata side effect is surfaced in Open Questions.
- Requirement 2 enumerates every snapshot consumer.

**Findings**:
- 🟡 major / high — **Open Questions / Dependencies** — Corpus metadata revision stamping is a coupled consumer and an unrecorded decision gate. List it, and make resolving the question a prerequisite for Requirement 1.
- 🔵 minor / medium — **Requirements / Dependencies** — Other consumers of the shared jj snapshot (`work sync`, the status renderer, e.g. 0198) are not captured as couplings.
- 🔵 minor / medium — **Dependencies** — The shared migrate dirty-path surface with 0286 has no coordination or ordering note.
- 🔵 minor / medium — **Dependencies** — The parity target depends on jj behaviour and configuration, and the pinned version is not named.
- 🔵 suggestion / medium — **Dependencies / Assumptions** — Run ids persisted before the fix hold an `@`-based value and will never match. Users stalled at upgrade may still need `ACCELERATOR_MIGRATE_FORCE` once.

### Scope

**Summary**: One principle unifies the item, and its boundaries against git, 0124 and 0286 are clear. It bundles the observed #97 revision defect with inferred parity fixes and a diagnostic enhancement, and the revision-fact change reaches corpus metadata with no stated boundary.

**Strengths**:
- A single stated principle links the requirements.
- Requirement 5 sets an explicit outer boundary.
- Adjacent defects are cross-referenced rather than absorbed.
- The title broadening is justified in the Drafting Notes.

**Findings**:
- 🟡 major / medium — **Requirements** — The observed resume defect is bundled with inferred, unobserved parity fixes. Consider a sibling bug for Requirements 2 and 3.
- 🟡 major / medium — **Open Questions** — The revision-fact change silently widens scope to corpus metadata. Decide whether it is in scope or out.
- 🔵 minor / medium — **Requirements** — The refusal path listing is an independent diagnostic enhancement. Split it out, or justify it.
- 🔵 suggestion / low — **Frontmatter: parent** — The parent epic 0136 may not be the natural home for this item.

### Testability

**Summary**: The item is well framed as a bug, and most criteria are concrete Given/When/Then statements. AC1 reproduces #97 faithfully. The gaps: the git criterion relies on an unrecorded baseline, the merge value is undefined, Requirement 2's consumers are checked only through migrate, and the corpus-metadata change has no criterion.

**Strengths**:
- AC1 is an exact reproduction that exercises the root cause.
- AC2 asserts both halves of the revision invariant.
- The exclude criterion covers both sources separately.
- AC6 uses named fixture paths across two scopes.
- Context states both the broken and the expected outcome.

**Findings**:
- 🟡 major / high — **Acceptance Criteria** — The git no-regression criterion checks against an unrecorded "today's behaviour". List the expected git outcomes.
- 🟡 major / high — **Acceptance Criteria** — The merge-commit revision has no defined value. Specify the format and snapshot invariance.
- 🟡 major / high — **Requirements** — Requirement 2's three consumers are verified only through migrate. Check the same fixture through all three.
- 🔵 minor / medium — **Acceptance Criteria** — The size-limit criterion has no boundary case and does not check honouring of a configured limit.
- 🔵 minor / medium — **Open Questions** — The corpus metadata revision change has no criterion either way.
- 🔵 minor / medium — **Acceptance Criteria** — The refusal criterion does not check that owned paths are left out. Add a mixed-tree case.
- 🔵 minor / medium — **Requirements** — "Advances only when a new change is started" is checked for only two commands. Add `jj describe` (unchanged) and a rebase of `@` (changed).
- 🔵 suggestion / low — **Summary** — Use `jj status` as an explicit differential test oracle.

## Re-Review (Pass 2) — 2026-09-23T14:55:17+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Scope / Dependency / Clarity**: Revision-fact change widens scope to corpus metadata — Resolved (Requirement 2 keeps metadata on `@`, with its own criterion)
- 🟡 **Clarity / Testability**: Merge-commit revision has no defined value — Resolved (sorted ids joined by `+`, snapshot-invariant)
- 🟡 **Testability**: Requirement 2 consumers verified only through migrate — Resolved (exclude and size criteria cover all three consumers)
- 🟡 **Testability**: Git criterion against an unrecorded baseline — Resolved
- 🟡 **Scope**: Observed defect bundled with inferred parity fixes — Still present (bundling kept by explicit decision; rationale recorded in Assumptions)
- 🟡 **Completeness**: Reproduction not captured as a unit — Resolved (the fixture that triggers the 0007 stall is still unstated)
- 🔵 **Clarity / Testability**: Size threshold source ambiguous — Partially resolved (the configured value is honoured, but the config layers and value formats are unspecified; raised to major by testability)
- 🔵 **Clarity / Completeness**: Summary presents inferred defects as observed — Resolved
- 🔵 **Testability / Clarity**: "Advances only when a new change is started" — Partially resolved (`jj edit` is listed but untested; parent rewrite in place is ambiguous)
- 🔵 **Testability**: Refusal did not exclude owned paths — Resolved
- 🔵 **Clarity**: Overlapping anchor terms — Partially resolved (terms are defined, but attributed to 0119, which does not define "run base" or "migrate scope")
- 🔵 **Clarity**: Two senses of "snapshot" — Resolved
- 🔵 **Clarity**: Undefined criterion terms — Resolved
- 🔵 **Scope**: Refusal path listing is separable — Still present (accepted with the bundling decision)
- 🔵 **Dependency**: Snapshot consumers not captured — Resolved
- 🔵 **Dependency**: 0286 coordination — Partially resolved (the note exists on this side only)
- 🔵 **Dependency**: jj version coupling — Resolved
- 🔵 **Dependency**: Pre-fix run ids — Resolved in Assumptions (not verified by a criterion)
- 🔵 **Testability**: `jj status` differential oracle — Resolved (it is now a criterion, but the consumer and normalisation are unspecified)

### New Issues Introduced

- 🟡 **Testability**: The criterion for the effective `snapshot.max-new-file-size` leaves the config source and value format unstated. The adapter cannot load `UserSettings`, so an implementation that reads one layer or one format would pass while diverging from `jj status`.
- 🔵 **Clarity / Testability**: The parity criterion does not name its scenarios, the component whose paths are compared, or how renames are counted.
- 🔵 **Testability**: No at-limit (exactly 1 KiB) case, and no criterion for the default global ignore fallback when `core.excludesFile` is unset.
- 🔵 **Testability**: The run-base criteria do not name where the run base is observed (e.g. the recorded run id in a new manifest).
- 🔵 **Testability / Clarity**: The refusal of runs stalled before the upgrade is untested. The release-notes sentence reads as a present fact with no owner.
- 🔵 **Dependency**: `lint:vcs-settings:check` could block Requirement 4 but is not listed as a coupling.
- 🔵 **Clarity**: One dirty-path function or two (`dirty_paths`, `working_copy_diff`). The size gap is described as both inferred from code and derived from the principle.
- 🔵 **Completeness**: The Summary opens in user-story form for a bug. The inferred gaps have no expected-versus-actual outline in Reproduction.

### Assessment

All four substantive pass-1 majors are resolved. Two majors remain, which meets the REVISE threshold of 2. One is the bundling finding, which was accepted deliberately. The other is new: how the effective `snapshot.max-new-file-size` is resolved without `UserSettings`. Specifying the config layers and value formats that count (or limiting scope to one layer) would clear the remaining substantive blocker. The rest is minor tightening.

## Re-Review (Pass 3) — 2026-09-23T15:29:48+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability**: Source and format of the effective `snapshot.max-new-file-size` — Resolved (checked against jj-lib/jj-cli 0.43.0: system, user, repo and workspace layers; value forms; `0` means unlimited; at-limit file reported; nothing written)
- 🟡 **Scope**: Observed defect bundled with inferred parity fixes — Still present, and stronger: requirement 4 has grown to reimplement jj's `secure_config` layout and carries a conditional lint blocker
- 🔵 **Clarity / Testability**: Parity oracle underspecified — Partially resolved (the component and rename counting are named; "edit criteria" and the order of `jj status` against the computation are still unclear)
- 🔵 **Testability**: At-limit case and default global ignore fallback — Resolved
- 🔵 **Testability**: Run-base observation point — Partially resolved (the point is named, but it needs a run that pre-flight would refuse; raised to major)
- 🔵 **Testability / Clarity**: Pre-upgrade stalled runs and release notes — Partially resolved (the criterion and CHANGELOG entry were added, but the CHANGELOG wording contradicts the Assumptions)
- 🔵 **Dependency**: `lint:vcs-settings:check` coupling — Partially resolved (listed, but "Blocked by: none" does not reflect it and nobody owns the decision)
- 🔵 **Dependency**: 0286 one-sided reconciliation — Resolved
- 🔵 **Clarity**: Terms attributed to 0119 — Resolved (the glossary does not yet say that "run id" is the stored form of the run base)
- 🔵 **Clarity**: One dirty-path function or two; source of the size gap — Resolved
- 🔵 **Completeness**: Summary framing; no outline for the inferred gaps — Resolved

### New Issues Introduced

- 🟡 **Clarity**: The CHANGELOG criterion says a pre-upgrade stalled run "is refused once", but the Assumptions say it is refused on every attempt until one forced run.
- 🟡 **Testability**: The run-base criteria read the value from "a migration run started at that point" in dirty states where pre-flight refuses a fresh run. The item should tie them to resume behaviour, or name a forced run.
- 🔵 **Testability**: "Exits cleanly" is undefined. Recovery after the forced run (a later resume without force) is not verified.
- 🔵 **Testability**: The no-writes guarantee is tested only for a legacy repo config, not for a legacy workspace config or a moved or copied repository. Behaviour for a malformed limit is unspecified.
- 🔵 **Testability**: Requirement 6 ("git unchanged") has no bounded check. The system layer, merges with 3 or more parents, `@` on the root commit, and a first-run refusal with no manifest are untested.
- 🔵 **Clarity**: The Context uses "the base is unchanged" where it means the parent commit. The workspace-only size variant contradicts itself. Requirement 3 omits the XDG fallback and relative-path resolution that the criteria test.
- 🔵 **Dependency**: The reimplemented `secure_config` layout is not in the External re-check list. The parity tests need the pinned jj CLI on PATH, outside the zero-spawn job.
- 🔵 **Completeness**: A conditional blocker has no Open Questions home. The pre-upgrade handling and CHANGELOG entry have no numbered requirement.

### Assessment

The size-limit question from pass 2 is resolved and verified against the jj 0.43.0 source. Three majors remain. Two are small wording fixes: the "refused once" contradiction, and how the run base is observed. The third, scope, has become substantive. With requirement 4 now reimplementing jj's `secure_config` discovery under a possible lint blocker, the case for shipping the #97 run-base fix separately from dirty-path parity is stronger than when bundling was decided.

## Re-Review (Pass 4) — 2026-09-23T15:46:41+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: CHANGELOG "refused once" contradicts the Assumptions — Resolved
- 🟡 **Testability**: Run base read from a run that pre-flight refuses — Partially resolved (the criteria are now behavioural, but the `jj new`/`jj commit` branch expects the wrong outcome; see new issues)
- 🟡 **Scope**: Bundling — Still present (split declined twice; now two scope majors: bundling, and requirement 4 being story-sized)
- 🔵 **Testability**: "Exits cleanly", recovery after force, no-writes coverage, malformed limit, git "unchanged", missing edge cases — Resolved
- 🔵 **Clarity**: "The base", workspace-only variant, Requirement 3 sources, "edit criteria" — Resolved
- 🔵 **Dependency**: Blocker is conditional, `secure_config` in the re-check list, jj CLI on PATH — Resolved
- 🔵 **Completeness**: Open Questions home; Requirement 7 — Resolved

### New Issues Introduced

- 🟡 **Testability**: `jj new`/`jj commit` after a stall moves the owned dirt into the new parent and leaves a clean tree. The run should proceed as a fresh run, not be "refused as stale".
- 🟡 **Testability**: The size matrix cannot be observed through `work sync`, which reports only dirty or clean. It needs a fixture where the oversize file is the only untracked file.
- 🟡 **Clarity**: It is unclear whether the fix covers all jj repositories or only jj-colocated ones. Requirement 3's git excludes presuppose a colocated `.git`.
- 🔵 **Testability**: The forced run in the stale-manifest criterion needs a defined second stall. "Guarded resume engages" has no observable signal. Untested cases: `~`-expanded excludes, user-over-system precedence, and a file exactly at 1 MiB. The legacy-config and copied-repo fixture has no values. The parity criterion's "abc" case has no defined outcome.
- 🔵 **Clarity**: Run base, run identity and run id are used interchangeably. "Every consumer" has two definitions. The renderer's failure mode is undefined. "Them" in Build system is ambiguous. Requirement 7's "later resumes" wording is vague. Guarded resume, pre-flight and stall are undefined terms.
- 🔵 **Dependency**: No existing test lane is confirmed for the jj-CLI parity tests. `jj-lib` alignment is assumed. The lint-exception blocker has no tracking item.
- 🔵 **Completeness**: The defect is not confirmed on jj 0.43.0. Step 1 relies on a fixture pointer.

### Assessment

All pass-3 majors except scope are resolved. Five majors remain. Two scope majors are the accepted no-split decision. The other three are real and small: two criteria I wrote in pass 3 expect unobservable or wrong outcomes (`jj new`/`jj commit` staleness, and `work sync` in the size matrix), and the colocated-versus-all-jj scope is undefined. Fixing those three would leave only the accepted scope findings among the majors.

## Re-Review (Pass 5, targeted: clarity, testability) — 2026-09-23T16:18:09+00:00

**Verdict:** REVISE

Only clarity and testability were re-run. Scope, completeness and dependency keep their pass-4 results: two accepted scope majors, and minors only for the other two.

### Previously Identified Issues

- 🟡 **Testability**: `jj new`/`jj commit` expected a stale refusal — Resolved (separate fresh-run criterion)
- 🟡 **Testability**: Size matrix unobservable through `work sync` — Resolved (a `work sync` criterion with only the oversize file present)
- 🟡 **Clarity**: Colocated only, or all jj repositories — Resolved (every jj repository with a git backend; criteria run in both layouts)
- 🔵 **Testability**: Forced-run second stall, resume signal, `~` excludes, user-over-system, 1 MiB boundary, legacy fixture values, the "abc" case in the parity check — Resolved
- 🔵 **Clarity**: Run base, run identity and run id; "every consumer"; renderer failure; "them"; Requirement 7 wording; undefined 0119 terms — Resolved

### New Issues Introduced

- 🟡 **Testability**: In every layer-precedence setup, the winning layer holds the smaller limit, so a "minimum across layers" implementation passes. The criteria need inverted setups.
- 🟡 **Clarity / Testability**: The invalid-limit criterion gives no outcome for migrate, which logs a warning and proceeds as if clean. Which consumers log a warning is unstated. The item also does not say where "abc" is set.
- 🟡 **Clarity**: "Owned dirt" is not tied to a matching run base, so Requirement 5 ("no owned path") appears to contradict the stale-run criterion ("naming the owned paths as foreign").
- 🔵 **Testability**: The no-write hash check has no bounded interval and its scope is too broad. `work sync` is checked only in the clean direction. The exclude sources are not checked against each other, and both XDG branches are not covered. The copy used in the parity check is itself a config-discovery edge case. `jj edit` to a sibling with the same parent, and rewriting one parent of a merge, are untested.
- 🔵 **Clarity**: "Snapshot" should be its own term. "Applied ledger" and "`JJ_CONFIG` rules" are undefined. The parity criterion's Given/When/Then structure is confused. The Dependencies consumer list omits the pre-flight. Non-git-backend jj repositories are neither excluded nor covered.

### Assessment

All three pass-4 majors are resolved. Three new majors remain, all precision gaps rather than design problems: layer setups that cannot tell precedence from minimum, migrate's outcome for an invalid limit, and a definition of owned dirt that ignores staleness. Each is a local edit.

## Re-Review (Pass 6, targeted: clarity, testability) — 2026-09-23T16:25:02+00:00

**Verdict:** REVISE

Only clarity and testability were re-run. Scope, completeness and dependency keep their pass-4 results.

### Previously Identified Issues

- 🟡 **Testability**: Layer setups cannot tell "later layer wins" from "minimum wins" — Resolved (setups where a later layer raises the limit were added)
- 🟡 **Clarity / Testability**: Migrate's outcome for an invalid limit — Resolved (each consumer's outcome and the warning are stated; the fail-open behaviour is stated explicitly)
- 🟡 **Clarity**: Owned-dirt definition ignores staleness — Resolved (a stale manifest owns nothing)
- 🔵 **Testability**: No-write interval, `work sync` in both directions, exclude-source interaction, XDG branches, parity on a copy, `jj edit` to a sibling, rewriting one merge parent — Resolved
- 🔵 **Clarity**: Snapshot term, applied ledger, `JJ_CONFIG` rules, pre-flight in the Dependencies list, non-git backends — Resolved

### New Issues Introduced

- 🟡 **Testability**: The `jj edit` sibling fixture cannot tell an unchanged run base from a stale one: both give the same refusal.
- 🟡 **Testability**: No criterion covers the user-layer sources (`config.toml`, `conf.d`, `~/.jjconfig.toml`) or the rule that `JJ_CONFIG` replaces them.
- 🟡 **Testability**: The copied-repository size expectation is excluded from the parity check, so nothing confirms that jj 0.43.0 behaves as asserted.
- 🔵 **Testability**: The `work sync` fixture location is unspecified, so the "clean" checks could pass without the fix. The system layer's source is unnamed. The parity check does not require the same environment. No criterion covers the non-git-backend exclude branch, a `0` limit in an overriding layer, or an invalid value in an overridden layer.
- 🔵 **Clarity**: The Drafting Notes and the Acceptance Criteria preamble state the backend scope differently from Context. "Derived from the VCS revision" in the run-base definition clashes with Requirement 2. The Snapshot term refers to criterion wording that no longer exists. Owned dirt covers only staleness, not every unusable-manifest case in 0119. The decisions-file references in the stale-run criterion are unclear. The parity inclusion set is loose. One criterion has an undefined antecedent. The system layer is undefined.

### Assessment

All three pass-5 majors are resolved, and clarity has no majors left. Testability raised three new majors. Each pass has resolved every prior major, but requirement 4's surface keeps producing new edge cases: config layers, legacy paths, copied repositories, value forms. This is the pattern scope flagged in passes 3 and 4. The remaining gaps are real, but they are test-design detail that could equally be settled while writing the plan.

## Verdict Change — 2026-09-23T16:28:26+00:00

**Verdict:** APPROVE

Toby Clemson approved the work item after pass 6 and moved it to `ready`. The pass-6 testability findings (the `jj edit` sibling fixture, the user-layer config sources, the copied-repository check) and the minor findings stay open as test-design input for the implementation plan. The accepted scope findings on bundling stand as deliberate decisions.
