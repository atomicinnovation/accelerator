---
date: "2026-05-08T00:00:00+00:00"
type: work-item-review
skill: review-work-item
target: "meta/work/0030-centralise-path-defaults.md"
work_item_id: "0030"
review_number: 1
verdict: COMMENT
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 4
status: complete
---

## Work Item Review: Centralise PATH and TEMPLATE config arrays to scripts/config-defaults.sh

**Verdict:** REVISE

The work item is well-motivated and unusually thorough in its Technical Notes, grounding the task in a concrete migration event and providing exact line ranges for all definition sites. However, it contains a significant internal contradiction that runs through multiple sections: `TEMPLATE_DEFAULTS` is named in the title, Summary, and Requirements as one of four arrays to be centralised, while the Technical Notes explicitly state it does not exist — and the Acceptance Criteria quietly omit it without reconciliation. A second recurring issue is the `DIR_KEYS`/`DIR_DEFAULTS` consideration in Technical Notes, which is flagged by all five lenses as insufficiently resolved for implementation. The regression acceptance criterion also needs tightening to be independently verifiable.

### Cross-Cutting Themes

- **TEMPLATE_DEFAULTS contradiction** (flagged by: clarity, completeness, scope, testability) — The title, Summary, and Requirements all name `TEMPLATE_DEFAULTS` as a fourth array to centralise, but Technical Notes explicitly state it does not exist. The Acceptance Criteria silently omit it. This inconsistency propagates across four sections and leaves an implementer uncertain about actual scope.
- **DIR_KEYS/DIR_DEFAULTS unresolved** (flagged by: clarity, completeness, dependency, scope, testability) — Technical Notes raise a "consider whether" question about `init.sh`'s parallel arrays but this is not captured in Requirements, Acceptance Criteria, or Open Questions. All five lenses flag this as a gap that could cause scope ambiguity or a missed coupling.

### Findings

#### Major

- 🟡 **Clarity + Completeness**: Requirements ↔ Technical Notes contradiction on `TEMPLATE_DEFAULTS`
  **Location**: Requirements, Technical Notes
  The Requirements section lists `TEMPLATE_DEFAULTS` as one of four arrays to extract, but Technical Notes explicitly state it does not exist and that template resolution uses a three-tier function lookup instead. The Acceptance Criteria silently drop it without explanation. An implementer reading Requirements first would search for a non-existent array; a reader encountering Technical Notes first would be uncertain whether Requirements are simply wrong or whether `TEMPLATE_DEFAULTS` should be created as new work.

- 🟡 **Clarity + Testability**: AC1 silently omits `TEMPLATE_DEFAULTS` without stating the omission is intentional
  **Location**: Acceptance Criteria
  AC1 checks for three arrays in `config-defaults.sh` (`PATH_KEYS`, `PATH_DEFAULTS`, `TEMPLATE_KEYS`) but omits `TEMPLATE_DEFAULTS`, which Requirements lists as the fourth array to extract. No note in AC1 explains the omission. A verifier who has not cross-referenced the Technical Notes cannot determine whether a missing `TEMPLATE_DEFAULTS` is a pass or a fail, making the criterion ambiguous without its full document context.

- 🟡 **Testability**: Regression criterion is tautological and unverifiable
  **Location**: Acceptance Criteria (AC3)
  AC3 states "when `mise run test` is executed, then all tests pass with no regressions." The phrase "no regressions" is tautological — it is always arguable as met whenever the test suite is green, regardless of whether the suite covers the affected paths. A verifier cannot distinguish "genuinely no regressions" from "test suite has gaps covering this area."

#### Minor

- 🔵 **Scope**: `TEMPLATE_DEFAULTS` named in title and Summary but does not exist
  **Location**: Summary (title)
  The work item title and Summary list `TEMPLATE_DEFAULTS` as one of four arrays to be extracted, overstating what will actually be centralised. An implementer reading only the title or Summary will expect to find and migrate an array that the codebase does not contain, causing unnecessary investigation time.

- 🔵 **Clarity + Completeness + Dependency + Scope + Testability**: `DIR_KEYS`/`DIR_DEFAULTS` consideration unresolved
  **Location**: Technical Notes, Open Questions
  Technical Notes flag `skills/config/init/scripts/init.sh` as defining `DIR_KEYS`/`DIR_DEFAULTS` using the same key vocabulary and pose "consider whether these should also source `config-defaults.sh`." This sits in Technical Notes without a resolution in Requirements, Acceptance Criteria, or Open Questions. An implementer reaching this file has no guidance; a verifier has no criterion to check. If included, scope expands beyond the Requirements; if excluded, a future reader may wonder why the consideration was noted but not addressed.

- 🔵 **Clarity**: "15 call sites" vs "4 files to migrate" not clearly distinguished
  **Location**: Summary, Technical Notes
  The Summary and Context refer to "approximately 15 call sites" / "15 sites in total," while Technical Notes introduce the heading "4 files to migrate" listing only definition sites. The phrase "call sites" could mean definition sites or consumer sites (or both), and a reader reconciling 15 with 4 cannot determine how many files will actually need a `source` line added.

### Strengths

- ✅ The grep-based AC2 provides a precise, copy-pasteable verification command producing an unambiguous pass/fail result — an exemplary testable criterion.
- ✅ The Technical Notes are unusually thorough, providing exact file paths and line ranges for all definition sites, explicitly correcting the false assumption about `TEMPLATE_DEFAULTS`, and flagging the `config-read-path.sh` comment-only case.
- ✅ The downstream consumer (0052) is explicitly named in the Blocks field with a concrete rationale, and the coupling is bidirectionally documented across both work items.
- ✅ The `source` directive in Requirements gives a concrete example with the actual variable `${CLAUDE_PLUGIN_ROOT}`, making the mechanism unambiguous.
- ✅ The Context grounds the task in a specific, recent migration event with a concrete before/after site count, making the motivation immediately legible.
- ✅ The Assumptions section explicitly resolves the former open question about co-locating `TEMPLATE_KEYS` alongside path arrays.
- ✅ "Blocked by: none" is stated explicitly rather than left absent, avoiding ambiguity about whether the Dependencies section was omitted or genuinely empty.

### Recommended Changes

1. **Remove `TEMPLATE_DEFAULTS` from title, Summary, and Requirements** (addresses: Requirements ↔ Technical Notes contradiction; title/Summary scope issue)
   Update the title to reference three arrays (`PATH_KEYS`, `PATH_DEFAULTS`, `TEMPLATE_KEYS`). Remove `TEMPLATE_DEFAULTS` from the Requirements bullet. Add a sentence in Technical Notes or Assumptions confirming the omission is intentional (the note already exists in Technical Notes, but it should be the single authoritative statement).

2. **Add an explanatory note to AC1 about the `TEMPLATE_DEFAULTS` omission** (addresses: AC1 silent omission ambiguity)
   Append something like: "Note: `TEMPLATE_DEFAULTS` is excluded — template fallback is handled by `config_resolve_template()` in `config-common.sh`, not a parallel defaults array." This makes AC1 self-contained for a verifier.

3. **Reframe AC3 with named test groups** (addresses: tautological regression criterion)
   Replace "all tests pass with no regressions" with a criterion naming the specific test groups that exercise the affected paths — e.g., "the config-dump, config-init, and path-resolution test suites continue to pass." If a specific test tag or suite name exists in the repo, use it.

4. **Resolve `DIR_KEYS`/`DIR_DEFAULTS` explicitly** (addresses: all-five-lenses finding)
   Either: (a) add a requirement and acceptance criterion for migrating `init.sh` if it is in scope, or (b) add an Open Questions entry (or a follow-on task reference in Dependencies) explicitly deferring the `init.sh` question. Remove or reword the "consider whether" note in Technical Notes so it points to whichever resolution was chosen.

5. **Clarify "15 call sites" vs "4 files to migrate"** (addresses: call-sites conflation)
   In the Summary or Context, distinguish between the 4 definition sites (where the arrays are declared) and the 11 consumer scripts (that will need a `source` line added), so the 15-site total is clearly accounted for.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is generally well-structured and uses precise technical language, but contains a significant internal contradiction: the Summary and Requirements name `TEMPLATE_DEFAULTS` as one of four arrays to be centralised, while the Technical Notes explicitly state that `TEMPLATE_DEFAULTS` does not exist. This contradiction propagates into the Acceptance Criteria, which silently omit `TEMPLATE_DEFAULTS` from verification without reconciling the conflict.

**Strengths**:
- The grep-based AC2 provides a concrete, executable verification command, making the migration outcome unambiguous for the arrays it covers.
- Technical Notes are unusually thorough: they call out the non-existence of `TEMPLATE_DEFAULTS`, the comment-only nature of `config-read-path.sh`, and the `DIR_KEYS`/`DIR_DEFAULTS` variant.
- The `source` directive in Requirements is given as a concrete example with the actual variable `${CLAUDE_PLUGIN_ROOT}`.
- The Drafting Notes transparently record how scope evolved.

**Findings**:
- major / high: `TEMPLATE_DEFAULTS` listed as extraction target but stated not to exist — Location: Requirements
- major / high: AC1 silently omits `TEMPLATE_DEFAULTS` without explaining the discrepancy — Location: Acceptance Criteria
- minor / medium: `DIR_KEYS`/`DIR_DEFAULTS` consideration left unresolved — Location: Technical Notes
- minor / medium: "15 call sites" conflates definition sites and usage sites — Location: Summary

### Completeness

**Summary**: The work item is well-structured for a task type, with a clear summary, motivating context, and three specific acceptance criteria. The main gap is the Requirements/Technical Notes inconsistency on `TEMPLATE_DEFAULTS`. A secondary gap is an unresolved design question about `DIR_KEYS`/`DIR_DEFAULTS` buried in Technical Notes rather than Open Questions.

**Strengths**:
- Summary is a precise, unambiguous action statement naming the exact arrays and destination file.
- Context grounds the work in a concrete, recent migration event with a specific count of affected sites.
- Technical Notes are unusually thorough, identifying exact line ranges for all definition sites.
- Acceptance Criteria use a consistent given/when/then structure with a machine-executable grep command.
- Dependencies section correctly identifies the downstream consumer (0052).
- Assumptions section is populated and directly addresses the key prerequisite.

**Findings**:
- major / high: Requirements list `TEMPLATE_DEFAULTS` but Technical Notes confirm it does not exist — Location: Requirements
- minor / medium: Unresolved `DIR_KEYS`/`DIR_DEFAULTS` design question buried in Technical Notes — Location: Open Questions

### Dependency

**Summary**: Work item 0030 captures its primary downstream consumer (0052) correctly and accurately states it has no upstream blockers. The coupling is bidirectionally documented. One potential ordering constraint exists around `init.sh`'s `DIR_KEYS`/`DIR_DEFAULTS` arrays, which is raised as an open consideration but not captured as a dependency or explicit decision.

**Strengths**:
- The downstream consumer (0052) is explicitly named in the Blocks field with a clear rationale.
- "Blocked by: none" is stated explicitly.
- Work item 0052 cross-references 0030, confirming the coupling is bidirectionally captured.
- Assumptions explicitly call out bash sourcing compatibility as a prerequisite.

**Findings**:
- minor / medium: `init.sh` `DIR_KEYS`/`DIR_DEFAULTS` consideration left uncaptured as a dependency or decision — Location: Technical Notes

### Scope

**Summary**: The work item describes a single, coherent refactoring task with no unrelated concerns bundled. The declared type (task) is appropriate. Two minor scope signals are present: the title and Summary overstate scope by naming `TEMPLATE_DEFAULTS`; and Technical Notes introduce an unresolved `DIR_KEYS`/`DIR_DEFAULTS` probe not captured in Requirements or Acceptance Criteria.

**Strengths**:
- All requirements serve a single unified purpose with no bundling of unrelated concerns.
- Summary, Requirements, and Acceptance Criteria are internally consistent with each other (the TEMPLATE_DEFAULTS issue is with Technical Notes, not among these three sections).
- Touching four parallel config-dump.sh copies across workspaces is clearly within one ownership domain.
- Task type is appropriate for a pure internal refactoring.
- Assumptions resolve the former open question about `TEMPLATE_KEYS` co-location.

**Findings**:
- minor / high: `TEMPLATE_DEFAULTS` named in Summary and title but does not exist — Location: Summary
- suggestion / medium: `DIR_KEYS`/`DIR_DEFAULTS` probe in Technical Notes not captured in Requirements — Location: Technical Notes

### Testability

**Summary**: The work item is largely well-specified, with two of the three acceptance criteria providing concrete, runnable verification procedures. The third criterion contains a tautological "no regressions" formulation that provides weak verification value. A minor gap exists between Requirements (four arrays) and AC1 (three arrays) with no explanation in the criterion itself.

**Strengths**:
- AC2 provides a precise, copy-pasteable grep command that produces an unambiguous pass/fail result.
- AC1 uses a concrete file inspection framing with named artefact and named arrays.
- Technical Notes explicitly clarify that `TEMPLATE_DEFAULTS` does not exist, grounding the scope for a verifier who reads the full document.

**Findings**:
- major / high: Regression criterion is tautological and unverifiable — Location: Acceptance Criteria (AC3)
- minor / high: AC1 silently omits `TEMPLATE_DEFAULTS` without stating the omission is intentional — Location: Acceptance Criteria
- minor / medium: No criterion covers the `DIR_KEYS`/`DIR_DEFAULTS` decision identified in Technical Notes — Location: Acceptance Criteria

## Re-Review (Pass 2) — 2026-05-08

**Verdict:** REVISE

### Previously Identified Issues

- ✅ **Clarity + Completeness**: Requirements ↔ Technical Notes contradiction on `TEMPLATE_DEFAULTS` — Resolved
- ✅ **Scope**: `TEMPLATE_DEFAULTS` named in title/Summary but does not exist — Resolved
- ✅ **Clarity**: "15 call sites" vs "4 files to migrate" not distinguished — Resolved
- Partially resolved: **Clarity + Testability**: AC1 TEMPLATE_DEFAULTS omission — Note added, but now appears without prior context for a top-to-bottom reader (new minor)
- Partially resolved: **All lenses**: DIR_KEYS/DIR_DEFAULTS unresolved — Moved to Open Questions, but the new language is self-contradictory: "explicitly out of scope" yet "should be resolved before 0030 is closed" (new minor; also surfaced as a major by the dependency lens)
- Partially resolved: **Testability**: Regression criterion tautological — Now names specific test categories, but "tests that were passing before the migration" introduces an unanchored baseline (new major)

### New Issues Introduced

- 🟡 **Dependency** (major): DIR_KEYS/DIR_DEFAULTS ordering constraint not captured as a blocking dependency — Open Questions declares the decision must be resolved before closure, but the Dependencies section still reads "Blocked by: none"
- 🟡 **Testability** (major): AC3 baseline is unanchored — "tests that were passing before the migration" cannot be verified without a recorded pre-migration baseline; if any of those tests were already failing, the criterion is unanswerable
- 🟡 **Testability** (major): 11 consumer sites have no verifying criterion — The Summary now clearly distinguishes 4 definition sites from 11 consumer scripts, but no AC confirms those consumer scripts remain functional after the migration
- 🔵 **Clarity** (minor): TEMPLATE_DEFAULTS exclusion note in AC1 appears without prior context — a reader scanning top-to-bottom encounters TEMPLATE_DEFAULTS for the first time in a parenthetical note inside AC1, with no earlier mention preparing them for it
- 🔵 **Completeness + Scope** (minor): Open Questions entry declares DIR_KEYS "explicitly out of scope" while simultaneously requiring it to be "resolved before 0030 is closed" — these two statements are mutually contradictory
- 🔵 **Dependency** (minor): 0052's reverse follow-up coupling is untracked — 0052 notes that config-read-all-paths.sh will need updating once 0030 lands; this trigger is not recorded in 0030's Dependencies section

### Assessment

Three of the six original findings are fully resolved. The remaining three were partially addressed, but the edits introduced new problems — most significantly that the Open Questions text is now self-contradictory, and AC3's baseline language is unanchored. The 11-consumer-sites gap is a genuine new issue that emerged from the Summary clarification making that population visible. A second round of targeted edits is needed before this work item is ready for implementation.

## Re-Review (Pass 3) — 2026-05-08

**Verdict:** REVISE

### Previously Identified Issues

- ✅ **Dependency**: DIR_KEYS/DIR_DEFAULTS ordering constraint not a blocking dependency — Resolved (Dependencies now has a "Triggers follow-up in 0052" entry; Open Questions clearly defers with no closure gate)
- ✅ **Testability**: AC3 baseline unanchored — Resolved (removed "that were passing before")
- ✅ **Clarity**: TEMPLATE_DEFAULTS note in AC1 without context — Resolved (simplified to forward reference to Assumptions)
- ✅ **Completeness + Scope**: Open Questions self-contradictory — Resolved (closure gate removed)
- ✅ **Dependency**: 0052 reverse coupling untracked — Resolved (noted in Dependencies)
- Partially resolved: **Testability**: 11 consumer sites had no criterion — parenthetical added to AC3, but the parenthetical now makes an unverifiable coverage claim (new major from testability)

### New Issues Introduced

- 🟡 **Clarity** (major): AC2 grep pattern may not match all definition forms — the pattern `PATH_KEYS=\|PATH_DEFAULTS=\|TEMPLATE_KEYS=` only matches bare `=` assignments; forms such as `declare -a PATH_KEYS` would be silently missed, giving a false-passing result after an incomplete migration
- 🟡 **Testability** (major): AC3 parenthetical makes an unverifiable coverage claim — stating "their continued correctness is covered by this criterion" assumes `mise run test` exercises all 11 consumer scripts, which is not established anywhere in the work item
- 🔵 **Completeness** (minor): ADR-0023 reference lacks context — no qualifier explains whether it provides implementation constraints or is purely historical
- 🔵 **Dependency** (minor): DIR_KEYS follow-up deferred to an unidentified follow-on task with no work item number — not trackable in planning tools
- 🔵 **Testability** (minor): AC1 tests array existence but not contents — empty arrays would pass AC1 while breaking consumers; AC3 implicitly covers this but the connection is unstated
- 🔵 **Scope** (suggestion): DIR_KEYS deferral increases future context-switching cost — minor, no delivery risk

### Assessment

Five of the six pass-2 findings are fully resolved. The 11-consumer-sites issue was addressed by adding a parenthetical to AC3, but the parenthetical itself introduced two new major issues: a speculative coverage claim and a grep-pattern gap. Both are addressable with simple edits — remove the parenthetical (move its claim to Assumptions if it can be confirmed), and add a note to AC2 confirming that bare `=` assignment is the only definition syntax at those four sites.

## Re-Review (Pass 4) — 2026-05-08

**Verdict:** COMMENT

### Previously Identified Issues

- ✅ **Clarity**: AC2 grep pattern may miss non-`=` forms — Resolved (note added confirming bare `=` is the only form; Technical Notes cited as reference)
- ✅ **Testability**: AC3 parenthetical makes unverifiable coverage claim — Resolved (parenthetical removed; consumer-scripts claim moved to Assumptions)
- ✅ **Completeness**: ADR-0023 reference lacks context — Resolved (qualifier added)
- Partially resolved: **Dependency**: DIR_KEYS follow-up has no tracking reference — "(work item to be created)" added to Open Questions; dependency lens still notes it is absent from Blocks
- Partially resolved: **Testability**: AC1 tests existence not contents — content-correctness clause added to AC1, but now creates a minor new finding about an ambiguous referent ("pre-migration definitions") and a cross-criterion dependency on AC3

### New Issues (all minor or suggestion)

- 🔵 **Clarity** (minor): AC1 "same entries as the pre-migration definitions" — ambiguous referent; four definition files exist and the work item doesn't confirm they are identical or name a canonical one
- 🔵 **Clarity** (minor): "consumer sites" used in Assumptions without definition; Summary uses "consumer scripts" — inconsistent labelling
- 🔵 **Completeness** (minor): Requirements does not state that the 11 consumer scripts are intentionally not modified — that decision lives only in Assumptions
- 🔵 **Dependency** (minor): DIR_KEYS/DIR_DEFAULTS follow-on task absent from Blocks — coupling is visible in Open Questions but not in the Dependencies section where planning tools look
- 🔵 **Testability** (minor): AC1 defers content correctness to AC3 without defining what "same entries" means — a verifier inspecting the file in isolation cannot independently confirm correctness
- 🔵 **Testability** (minor): AC3 names test categories but doesn't define how they are identified in `mise run test` output
- 🔵 **Scope** (suggestion): "Triggers follow-up in 0052" instruction belongs in 0052's own work item rather than here

### Assessment

No major findings remain. The work item is acceptable for implementation. The remaining observations are all minor or suggestion — primarily labelling consistency, a cross-criterion reference in AC1, and the DIR_KEYS follow-on tracking gap. None of these would block an implementer from delivering the task correctly.
