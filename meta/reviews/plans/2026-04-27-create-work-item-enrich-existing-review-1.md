---
date: "2026-04-27T15:25:38+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-27-create-work-item-enrich-existing.md"
review_number: 1
verdict: COMMENT
lenses: [architecture, code-quality, test-coverage, correctness, usability, compatibility, safety, standards]
review_pass: 3
status: complete
---

## Plan Review: Extend `create-work-item` to Enrich an Existing Work Item

**Verdict:** REVISE

The plan's core idea — adding a path/numeric resolution branch and an enrich-existing mode to `create-work-item` while preserving the topic-string and no-arg flows — is sound and well-anchored in peer-skill conventions. However, the plan assumes capabilities of the skill-creator eval harness (a structured `assertions` field, transcript/tool-use visibility for negative invariants, multi-fixture filesystem staging) that are nowhere verified, leaves several backwards-compatibility, identity-preservation, and safety details under-specified, and threads an undefined `{enriching_existing}` flag through five steps in a way that scatters mode-specific logic across the file. Address the harness verification, the silent reroute of numeric/path-like topic strings, the identity-field inconsistency, and the silent-overwrite clause before implementation.

### Cross-Cutting Themes

- **Eval harness assumptions are unverified** (flagged by: test-coverage, code-quality, compatibility, standards). The plan introduces a new `assertions` field, asserts negative tool-use checks ("did NOT call work-item-next-number.sh", "no file written"), relies on multi-fixture filesystem staging for id 17, and adopts a new `evals/files/` directory — none of which are demonstrated to be supported by the skill-creator scripts the plan invokes (`run_eval.py`, `aggregate_benchmark.py`). If the harness silently ignores `assertions`, every "programmatic" check degenerates to LLM-graded prose.
- **The Phase-1 snapshot baseline is a tautological gate** (flagged by: test-coverage, compatibility, architecture). Snapshotting once at the start of Phase 1 means new scenarios trivially fail on `old_skill` — that signal is meaningless and won't catch mid-plan regressions in the enrichment branch itself.
- **Silent backwards-compat breakage for numeric / path-like topic strings** (flagged by: compatibility, usability, code-quality). The discriminators reroute `/create-work-item 42`, `/create-work-item update README.md`, `/create-work-item add /healthz endpoint` etc. into enrichment mode with no warning and no escape hatch. Migration Notes claims none of this matters.
- **Identity-field list is internally inconsistent** (flagged by: correctness, standards, code-quality). Step 0 reads nine fields. Desired End State / Quality Guidelines preserve four. Step 3 augmentation explicitly proposes changes to type / priority / tags. The four-field eval (id 25) does not catch divergence on the rest.
- **Silent-overwrite-acceptable clause undermines the Step 5 safety story** (flagged by: safety, code-quality, usability, correctness). "If the user already issued an explicit approval at the end of Step 4, a silent overwrite is acceptable" gives the model latitude to skip the change-summary gate after a stale approval, while the path-existence guard is removed.
- **Architectural sprawl across the SKILL.md file** (flagged by: architecture, code-quality). `{enriching_existing}` branches appear in Steps 1, 2, 3, 4, 5 plus Quality Guidelines. The flag itself is never defined as a Markdown-skill mechanism; the resolution logic copy-pastes a fifth time across the work-item skill family without abstraction; and the boundary with `refine-work-item` (which already enriches existing work items) is asserted but not enforced.
- **In-place overwrite lacks the diff/double-confirmation pattern peers use** (flagged by: architecture, usability, safety). `refine-work-item` requires a unified diff and y/n confirmation for destructive content changes. This plan defers the diff as Open Question 1 even though enrich-mode is the clearest case for one.

### Tradeoff Analysis

- **Discoverability vs. command sprawl**: usability and architecture both suggest a separate `/enrich-work-item` skill (better discoverability, cleaner boundary with refine-work-item). The plan's implicit case for keeping it under `create-work-item` is shared Step 2/3 logic. If the in-file branching is kept, an explicit invariants table or a sibling "## Enrich-Existing Mode" section would mitigate the cohesion loss.
- **Safety confirmations vs. friction**: safety and correctness ask for an extra integrity check before write (re-read identity from disk, abort if drifted). Usability flags that two confirmation gates (Step 4 draft + Step 5 summary) are already redundant. These can both be addressed by collapsing to one gate that includes the identity re-check.

### Findings

#### Critical

- 🔴 **Test Coverage**: Hybrid-grading `assertions` array is unverified against the actual eval harness schema
  **Location**: Implementation Approach §1 and Phase 1 §2
  The plan introduces a new `assertions: [{text: ...}]` field and asserts it will be graded by skill-creator's grader subagent. The existing 14 evals show no such field. No harness code is cited as consuming it. If the field is silently ignored, all "programmatic" invariants degrade to prose grading.

- 🔴 **Test Coverage**: Negative tool-use assertions assume transcript visibility the harness may not provide
  **Location**: Phase 1 §2 (id 26 enrich-existing-no-next-number-call) and Phase 1 (ids 18, 20)
  Multiple new scenarios assert the *absence* of a tool call ("work-item-next-number.sh is not called", "No agents spawned", "No work item file was written"). The existing id 8 sidesteps this by asking the model to self-report — unreliable for a negative invariant. The most central plan invariant (don't call next-number in enrich path) sits on the riskiest test pattern.

#### Major

- 🟡 **Architecture**: Domain boundary between create-work-item and refine-work-item is asserted but not architecturally enforced
  **Location**: Overview / What We're NOT Doing
  Refine-work-item's `enrich`/`sharpen` operations already mutate existing work items. After this plan a sparse work item has two correct entry points with subtly different conventions, and future requirements ("never replace existing prose silently") will need to be applied in two places.

- 🟡 **Architecture**: Resolution pattern is now copy-pasted across five skills with no shared abstraction
  **Location**: Phase 1 §4
  The plan replicates the regex, error strings, and zero/one/many handling rather than extracting a `work-item-resolve-ref.sh` helper or marking the duplication as accepted technical debt with a follow-up.

- 🟡 **Architecture**: Internal flag threaded through five steps creates pervasive intra-file coupling
  **Location**: Implementation Approach / Phases 1–3
  After this change every step has two parallel branches whose invariants must stay aligned (no XXXX in Step 4 ↔ no next-number call in Step 5 ↔ Quality Guidelines bullet). The most likely next change (auto-status-upgrade, diff preview) will require coordinated edits across 2–3 sub-sections.

- 🟡 **Code Quality**: `{enriching_existing}` flag semantics and lifecycle are not specified
  **Location**: Phase 1 §4 (Step 0) and throughout
  The placeholder syntax `{enriching_existing}` mimics existing `{work_dir}` / `{documents locator agent}` placeholders that are filled by config scripts at load time, but a runtime conditional flag is a different concept and the Markdown convention for it isn't established.

- 🟡 **Code Quality**: Branching by `{enriching_existing}` flag scattered across five steps creates maintenance burden
  **Location**: Phases 2 and 3
  The base SKILL.md is ~325 lines; adding five "When `{enriching_existing}` is set" sub-sections plus an expanded Step 0 will push it past the size where a reader can hold the flow in their head.

- 🟡 **Test Coverage**: Single Phase-1 baseline snapshot cannot meaningfully fail Phase 2–4 scenarios
  **Location**: Implementation Approach §1
  The Phase 1 snapshot has no enrich-existing logic at all, so it will trivially fail every Phase 2–4 enrich scenario for reasons unrelated to what those phases add. The "fails on old_skill" criterion becomes vacuous; mid-plan regressions in the enrich branch are invisible.

- 🟡 **Test Coverage**: Multi-match scenario relies on filesystem state the eval harness may not provide
  **Location**: Phase 1 §2 (id 17)
  The plan acknowledges uncertainty ("if simulation is needed, follow the pattern in eval id 9") but does not resolve it. Id 9's inline simulation does not actually exercise a glob.

- 🟡 **Test Coverage**: Single-run-per-configuration design exposes LLM-graded prose assertions to flake
  **Location**: Implementation Approach + benchmark.json
  `runs_per_configuration: 1` plus 15 new prose-heavy assertions plus a "fails on old_skill" gate is a flaky combination. The existing benchmark already shows old_skill drifting from 1.0 to 0.8 on one scenario despite no behavioural change.

- 🟡 **Test Coverage**: Self-not-duplicate assertion (id 23) is too loose to mutation-test the change
  **Location**: Phase 2 §3
  A negative check on absence of a phrasing is satisfied by any paraphrase. Pair it with a positive assertion that Step 3 augmentation framing is reached and the input file's preserved id/title is referenced.

- 🟡 **Test Coverage**: Gap-analysis tests do not cover the boundary cases that drive the heuristic
  **Location**: Phase 2 §1
  Ids 21 and 22 test only "mostly gaps" and "fully rich". The classification is hardest in the mixed-substantive-plus-placeholder and instructional-prose-carried-over cases — exactly the false-positive risk the spec calls out, with no eval covering it.

- 🟡 **Correctness**: Status-change mechanism is referenced but never defined
  **Location**: Phase 3, Step 4 enrich-existing variant
  Step 4 says status is preserved "unless the user explicitly requested a status change during the conversation", but no step defines how / when / where the user can make that request. Step 3 explicitly excludes status from proposed changes.

- 🟡 **Correctness**: Inconsistent definition of which identity fields are preserved
  **Location**: Current State Analysis vs Desired End State vs Quality Guidelines
  Step 0 reads nine fields. Quality Guidelines preserves four. Step 3 invites changes to type/priority/tags. Step 4 only re-asserts the four-field set. Title and parent fall in the gap.

- 🟡 **Correctness**: Per-field reads via `work-item-read-field.sh` confound 'missing field' with 'unparseable frontmatter'
  **Location**: Phase 1, Step 0
  The script exits 1 in three cases: file missing, frontmatter missing/unclosed, field absent. A sparse user-authored file that omits `parent` or `tags` will trigger 'No <field> field found', and the plan does not specify the recovery branch.

- 🟡 **Correctness**: Path-like resolution semantics (cwd vs work_dir vs absolute) are unspecified
  **Location**: Phase 1, Step 0 path-like discriminator
  The plan uses three different relative-path forms in eval prompts (`meta/work/...`, `evals/files/...`, bare `0042-...md`). Without a canonical resolution rule, identical inputs may resolve to different files depending on where the conversation started.

- 🟡 **Standards**: Identity-field list is internally inconsistent between Step 0 and Quality Guidelines
  **Location**: Phase 1 §4 (lines 281–283) and Phase 3 §3 (lines 559–562)
  Step 0 reads nine fields; Quality Guidelines preserves four. The plan never explains why title/type/priority/parent/tags are read but not preserved. Eval id 25 only checks the four.

- 🟡 **Standards**: Unparseable-frontmatter diagnostic diverges from the canonical refine-work-item wording
  **Location**: Phase 1, Step 0 (lines 270–276)
  The plan drops the "and contains all nine required fields" clause that's in `refine-work-item/SKILL.md:67-74`, while claiming to follow that pattern verbatim.

- 🟡 **Usability**: Single-slot argument-hint conflates three distinct inputs
  **Location**: Phase 1 §3
  `[topic, description, or existing work item number/path]` is more ambiguous than peer hints (review-work-item's `[path to work item file]`). A user reading the hint cannot predict what `/create-work-item 42` will do.

- 🟡 **Usability**: Silent behaviour change for `/create-work-item <number>` and `/create-work-item <path-like-string>`
  **Location**: Migration Notes and Phase 1 §4
  The plan claims migration is "Not applicable" but users who type `/create-work-item 42`, `/create-work-item update README.md`, or `/create-work-item add /healthz endpoint` today get a topic interpretation; tomorrow they get an enrichment attempt or a missing-file error.

- 🟡 **Usability**: Discoverability of enrich-mode is poor — a separate `/enrich-work-item` would be more obvious
  **Location**: Overall
  The command name says "create"; users searching for "enrich" or "fill in gaps" will reach for refine-work-item or update-work-item and never find the new behaviour.

- 🟡 **Compatibility**: Numeric and path-like discriminators silently reroute legitimate topic strings
  **Location**: Phase 1 §4
  Same root cause as the usability silent-behaviour-change finding. The plan claims topic-string contract is preserved verbatim; it isn't, for inputs matching `^[0-9]{1,4}$` or containing `/` or ending `.md`.

- 🟡 **Compatibility**: Single Phase-1 snapshot is not a meaningful baseline for new scenarios
  **Location**: Implementation Approach §1
  Same finding as the test-coverage one: the "fails on old_skill" check is trivially satisfied and doesn't gate regressions in the enrich path itself.

- 🟡 **Safety**: Silent overwrite licensed by stale Step 4 approval
  **Location**: Phase 3 §2 (Step 5 enrich-existing variant)
  No temporal bound on "explicit approval at the end of Step 4". After further iteration the original approval is no longer scoped to what is about to be written; combined with the removed path-existence guard, an out-of-date draft can land on disk silently.

- 🟡 **Safety**: Path-existence guard removed without a substitute integrity check
  **Location**: Phase 3 §2 step 3
  "Overwrite is intentional" only holds if Step 0 resolved correctly. There's no checksum or "this file's current work_item_id is 0042 — confirm?" guard between resolution and write.

#### Minor

- 🔵 **Architecture**: Self-exclusion of the input file leaks the enrich-existing mode into the documents-locator interaction contract
  **Location**: Phase 2 §2
  The exclusion is a Claude post-filter rather than a documents-locator argument; the eval (id 23) checks for absence of a phrasing rather than a structural property.

- 🔵 **Architecture**: In-place overwrite path lacks the destructive-change safeguards present in peer skills
  **Location**: Phase 3 §2
  `refine-work-item` requires diff + double confirmation for destructive changes; this plan defers diff (Open Question 1) and softens confirmation (silent-overwrite clause).

- 🔵 **Architecture**: Skill snapshot path is outside the workspace and not version-controlled by the plan
  **Location**: Phase 1 §1
  `cp -r skills/work/create-work-item skills/work/create-work-item-workspace/skill-snapshot` — lifecycle, gitignore status, and reproducibility across contributors is unspecified.

- 🔵 **Code Quality**: Snapshot location uses a sibling-directory naming that mirrors the skills tree
  **Location**: Phase 1 §1
  `skills/work/create-work-item-workspace/` will look like a skill to globs in plugin packaging or CI lint paths. Consider `meta/eval-workspaces/...` or `.eval-cache/...`.

- 🔵 **Code Quality**: Identity-preservation rules duplicated across Step 0, Step 4, Step 5, and Quality Guidelines
  **Location**: Phases 1 §4 and 3 §§1–3
  Four restatements of the same invariant set. Make Quality Guidelines canonical and have steps reference it by name.

- 🔵 **Code Quality**: Discriminator regex `^[0-9]{1,4}$` excludes valid 5-digit topics but creates a future cliff
  **Location**: Phase 1 §4
  A future overflow at 9999 silently routes inputs to topic-string interpretation with no error.

- 🔵 **Code Quality**: Error message references `/update-work-item` as the diagnostic source
  **Location**: Phase 1 §4
  Couples create-work-item's error message to update-work-item's diagnostic behaviour; violates the "What We're NOT Doing — not modifying any other skill" stance.

- 🔵 **Code Quality**: 'Silent overwrite is acceptable' clause undercuts the explicit confirmation rule
  **Location**: Phase 3 §2
  Quality Guidelines: "Never write a file without explicit user approval"; this clause introduces a fuzzy exception for the most destructive operation.

- 🔵 **Test Coverage**: Strict `pass_rate: 1.0` on every new scenario contradicts variance noted elsewhere
  **Location**: Phase 4 Success Criteria
  At runs_per_configuration=1 the existing benchmark already has old_skill drift; holding 15 new scenarios to perfect 1.0 invites flake-driven false blocks or post-hoc relaxation.

- 🔵 **Test Coverage**: Fixture creation is not itself testable and lacks a verification step
  **Location**: Phase 1 §2
  Sparse-but-not-empty / broken-frontmatter / placeholder-only — properties that are easy to get wrong and would make evals pass for the wrong reasons.

- 🔵 **Test Coverage**: Identity-preservation eval does not cover the 'user requested status change' branch
  **Location**: Phase 3 §1, id 29
  Add a 30th scenario where the conversation includes "change status to in-progress" and assert the draft reflects it.

- 🔵 **Test Coverage**: No test that the topic-string flow still calls `work-item-next-number.sh`
  **Location**: Testing Strategy
  All four existing scenarios that touch next-number explicitly forbid it. After this change, the topic-string flow's *required* call to next-number is unprotected.

- 🔵 **Correctness**: Title-update vs H1-sync interaction is under-specified
  **Location**: Phase 3 step 4 (line 503)
  Step 3 augmentation doesn't include "Title:" in its proposed-change frame; Quality Guidelines says H1 stays in sync with "(possibly updated) title". The mechanism for updating title is undefined.

- 🔵 **Correctness**: Numeric multi-match is implausible given `work-item-next-number.sh`'s uniqueness guarantee
  **Location**: Phase 1, id 17
  Multi-match exists as a defensive branch; the plan should say so and simulate the result inline rather than rely on real filesystem state.

- 🔵 **Correctness**: Silent-overwrite escape hatch can skip the change-summary invariant
  **Location**: Phase 3 §2 step 2
  Step 4 doesn't actually require a change-summary block; Step 5 presupposes one was shown. Both clauses presuppose each other but neither contains the obligation.

- 🔵 **Correctness**: Removed path-existence guard widens TOCTOU window
  **Location**: Phase 1 / Phase 3 §2
  Long Step 4 conversations open a window where the resolved file can be deleted, moved, or replaced. A re-read at write time closes it.

- 🔵 **Usability**: Two confirmation gates (Step 4 draft preview + Step 5 change summary) create double-friction
  **Location**: Phase 3 §2
  Either always-redundant or always-skipped given the silent-overwrite clause. Pick one model.

- 🔵 **Usability**: `Complete: … / Gaps: …` listing is not actionable without context
  **Location**: Phase 2 §1 (lines 358–368)
  Tag each gap with a one-word reason mirroring the per-section annotations Step 3 already specifies.

- 🔵 **Usability**: Multi-match flow inherits review-work-item's pattern but plan does not specify the selection prompt format
  **Location**: Phase 1 §4
  Pick `update-work-item`'s exact wording ("select by number or specify the full path") to keep selection mechanics aligned across the family.

- 🔵 **Compatibility**: Eval id reuse / ordering assumption may break harness contracts
  **Location**: Phase 1 §2
  Verify how `--filter '15..20'` resolves in `run_eval.py` (id-keyed vs index-keyed) before committing to the numbering.

- 🔵 **Compatibility**: argument-hint string growth may exceed downstream display budgets
  **Location**: Phase 1 §3
  The new hint is ~57 chars vs ~26. Some surfaces may truncate or validate.

- 🔵 **Compatibility**: Glob `{work_dir}/NNNN-*.md` assumes case-sensitive single-directory layout
  **Location**: Phase 1 §4
  Spell out the layout assumption and confirm parity with `update-work-item`/`review-work-item`'s globs.

- 🔵 **Safety**: Recovery story not surfaced — jj working copy is the implicit safety net
  **Location**: Migration Notes
  A one-line recovery note (`jj restore <path>`) bounds the perceived blast radius of a wrong-file overwrite.

- 🔵 **Safety**: Identity preservation enforced only in prose — no programmatic guardrail at write time
  **Location**: Phase 3 §1
  The script and the cached values from Step 0 are both already present; one extra substitution step closes the gap.

- 🔵 **Standards**: Missing-path and numeric-no-match messages drop the offer-to-list affordance present in peer skills
  **Location**: Phase 1 §4
  Add an assertion to ids 18 / 20 that the response also references `/list-work-items`.

- 🔵 **Standards**: Success-message wording diverges from the existing create-work-item confirmation form
  **Location**: Phase 3 §2
  Lock in `Work item updated: <path>` with a verbatim-string assertion on id 28, or reuse `update-work-item`'s confirmation idiom.

- 🔵 **Standards**: Fixture directory `evals/files/` is asserted as a convention without precedent
  **Location**: Phase 1 §2
  Verify against the skill-creator harness where it expects fixtures.

- 🔵 **Standards**: Path/work_dir placeholder spelling inconsistent across peer skills
  **Location**: Phase 1, Step 0
  Pick a canonical spelling (`{work_dir}` is the more compact and matches existing usage).

- 🔵 **Standards**: Re-numbering existing rules into a three-branch list breaks alignment with the two-branch peer pattern
  **Location**: Phase 1 §4 (lines 287–294)
  Keep Step 0 as a binary decision (existing-file-ref vs. anything-else); preserves rule numbering and structural consistency.

#### Suggestions

- 🔵 **Usability**: Draft preview should show what changed, not just the full updated work item
  **Location**: Phase 3 §1
  Pull the deferred diff-preview work into this plan (it's a direct usability multiplier for enrich), or precede the full draft with a per-section change summary.

- 🔵 **Safety**: Partial-write atomicity not addressed
  **Location**: Phase 3 §2
  Either document the atomicity assumption explicitly or write to a sibling temp path and rename over the original.

### Strengths

- ✅ Single-file change with explicit "no scripts, no template, no peer-skill modifications" boundary — bounded scope.
- ✅ Existing two invocation forms (no-arg, topic-string) are preserved verbatim in body and intent.
- ✅ Identity-field preservation is treated as an architectural invariant with explicit guards and dedicated programmatic eval assertions.
- ✅ Resolution-error contract is canonicalised against four peer skills, keeping user-facing error semantics consistent across the work-item skill family.
- ✅ TDD-via-evals discipline: each phase adds scenarios before SKILL.md edits, with a snapshotted baseline as `old_skill`.
- ✅ The hybrid-grading idea (prose for qualitative behaviour + structured assertions for invariants) is the right architecture for distinguishing flake-prone judgement calls from deterministic invariants.
- ✅ Coverage of the Step 0 error matrix (missing path, numeric-no-match, multi-match, unparseable frontmatter) is comprehensive — each error path has its own scenario.
- ✅ Identity-preservation evals (ids 25, 27, 29) target exact failure modes flagged in research (next-number drift, XXXX leakage, status regression).
- ✅ Allowed-tools assessment is correct: `work-item-read-field.sh` is already covered.
- ✅ Self-exclusion rule prevents the file being enriched from being misidentified as its own duplicate.
- ✅ Errors abort before any agent spawn or write, satisfying the no-side-effects-on-error invariant.
- ✅ The H1 form `# NNNN: <title>` and the identity-field list match `templates/work-item.md` verbatim.

### Recommended Changes

Ordered by impact. Address all critical and structural items before implementation begins.

1. **Verify the skill-creator eval harness contract before committing to the hybrid-grading design** (addresses: Hybrid-grading `assertions` array unverified; Negative tool-use assertions assume transcript visibility; Eval id reuse assumption; Fixture directory `evals/files/` convention).
   Read `run_eval.py` and `aggregate_benchmark.py`. Confirm: (a) whether `assertions` is a recognised field and how it factors into pass/fail counts; (b) how `--filter '15..20'` resolves (id-keyed vs index-keyed); (c) where fixture files should live; (d) whether the harness exposes a tool-use log the grader can inspect for negative invariants. If any of (a)–(d) is unsupported, either contribute the support upstream as a prerequisite or redesign the affected scenarios so each invariant becomes a discrete eval whose `expected_output` is narrow enough for the grader to score binarily on artifact contents (printed paths, written file presence) rather than transcript inspection.

2. **Resolve the silent backwards-compat regression for numeric / path-like topic strings** (addresses: Numeric and path-like discriminators silently reroute legitimate topic strings; Silent behaviour change for `/create-work-item <number>`; Single-slot argument-hint conflates three inputs; Migration Notes "Not applicable" claim).
   Pick one: (a) require an explicit prefix (e.g. `--existing` or `@<path>`) for enrichment, keeping the topic-string contract intact; (b) fall back to topic-string interpretation when a numeric input has zero matches, with a one-line note ("No work item numbered 9999 — interpreting as topic string. Press Ctrl-C to abort."); or (c) prompt-disambiguate when the discriminator matches ("It looks like you want to enrich existing work item 0042. Or did you want a new work item *about* `42`? (enrich/new)"). Update Migration Notes to name affected inputs explicitly.

3. **Re-snapshot the baseline at the end of each phase, or drop the per-phase "fails on old_skill" criterion** (addresses: Single Phase-1 baseline cannot meaningfully fail Phase 2–4 scenarios; Single Phase-1 snapshot is not a meaningful baseline; Skill snapshot path lifecycle unspecified).
   Either: re-snapshot before each phase so each phase's new evals demonstrate the *delta* that phase introduces; OR restrict the `old_skill` gate to scenarios 1–14 and replace the trivial "fails on old_skill" check on 15–29 with a within-`with_skill` cross-phase comparison. Specify the snapshot location (e.g. `meta/eval-workspaces/...` outside the published skill tree) and whether it is committed or gitignored.

4. **Pin down the identity-field contract and reflect it consistently across Step 0, Step 3, Step 4, Step 5, and Quality Guidelines** (addresses: Inconsistent definition of preserved identity fields; Identity-field list internally inconsistent; Identity-preservation rules duplicated across four places; Title-update vs H1-sync interaction).
   State once in Quality Guidelines: (a) immutable in enrich-mode (work_item_id, date, author); (b) preserved unless explicitly changed in Step 3 (status); (c) proposable for change in Step 3 (title, type, priority, parent, tags). Have Step 0 read only what's needed; Step 3's augmentation frame must enumerate the proposable fields explicitly (including Title); Step 4's preserved-fields list and the Quality Guidelines bullet must reference the canonical rule rather than restating it. Extend eval id 25 to cover all immutable fields.

5. **Define the status-change mechanism, or drop it** (addresses: Status-change mechanism referenced but never defined).
   Either remove the "unless the user explicitly requested a status change" clause from Step 4 (status always preserved verbatim), or add a Step 3 sub-clause that surfaces a proposed status transition when the user spontaneously raises one, and add an eval covering the override path.

6. **Collapse Step 4 + Step 5 confirmations to a single mandatory gate that includes an at-write integrity check** (addresses: Silent overwrite licensed by stale Step 4 approval; Path-existence guard removed without substitute; Two confirmation gates create double-friction; Silent-overwrite escape hatch; TOCTOU between Step 0 and Step 5; Identity preservation only enforced in prose).
   Single gate in Step 5: re-read the resolved file's `work_item_id` via `work-item-read-field.sh`. Abort if absent or different from the value cached in Step 0. Show a per-section change summary (added / modified / preserved). Require explicit y/n confirmation. Drop "silent overwrite is acceptable" entirely. This pattern matches `update-work-item`'s existing idiom and is cheap (one bash call).

7. **Specify the path-like resolution semantics and the frontmatter parsing algorithm** (addresses: Path-like resolution semantics unspecified; Per-field reads confound 'missing field' with 'unparseable frontmatter').
   Step 0: "Path-like arguments are resolved against the current working directory if relative, taken literally if absolute." Frontmatter parsing: (1) one canonical existence/parse check (e.g. read the file directly or run one `work-item-read-field.sh` call and intercept stderr); the unparseable-frontmatter diagnostic is gated on that single check. (2) Per-field reads after parse-validation use template defaults for missing fields (status → draft, parent → '', tags → []) without aborting. Add an eval covering a sparse but well-formed input that omits `tags`.

8. **Strengthen eval coverage on the boundary cases that drive the heuristics** (addresses: Gap-analysis tests don't cover boundary cases; Self-not-duplicate assertion too loose; Strict pass_rate: 1.0 contradicts variance; Fixture creation isn't testable; No test that topic-string flow still calls next-number; Identity-preservation eval doesn't cover user-status-change branch).
   Add: (a) two boundary-case fixtures for gap analysis (mixed substantive+placeholder; instructional-prose-carried-over); (b) positive assertion to id 23 that Step 3 augmentation framing is reached; (c) bump `runs_per_configuration` to ≥3 for new scenarios with a probabilistic threshold (with_skill mean ≥0.95, old_skill mean ≤0.5); (d) fixture sanity checks in Phase 1 success criteria; (e) a topic-string scenario that runs through Step 5 with next-number permitted, asserting it is called exactly once; (f) a status-change scenario complementing id 29.

9. **Document the architectural choices and recovery story explicitly** (addresses: Resolution pattern copy-pasted across five skills; Internal flag threaded through five steps; Domain boundary with refine-work-item; Recovery story not surfaced; In-place overwrite lacks destructive-change safeguards).
   Add an "Architectural Tradeoffs" section to the plan: (a) why this is a mode of `/create-work-item` rather than a peer `/enrich-work-item`; (b) why the resolution pattern is duplicated rather than extracted (or a follow-up ticket reference); (c) the boundary with `refine-work-item` (this skill = body content gap-fill; refine = decomposition / sharpening / Technical Notes); (d) recovery via `jj restore <path>`. Consider lifting enrich-mode into a "## Enrich-Existing Mode" sibling section that runs end-to-end after Step 0 sets the flag, cross-referencing the topic-string steps for shared logic — this keeps each branch readable independently.

10. **Tighten lower-level details before drafting the SKILL.md edits** (addresses: Snapshot location naming; Discriminator regex 5-digit cliff; Error message references `/update-work-item`; Diagnostic divergence from refine-work-item; Multi-match implausibility; Multi-match selection prompt format; Argument-hint length; Glob layout assumption; Standards drift on missing-path / placeholder spelling / success message; Re-numbering Step 0; Diff preview deferral; Partial-write atomicity).
   Snapshot under `meta/eval-workspaces/...`; widen numeric regex to `^[0-9]+$` and let the no-match branch handle out-of-range; drop the cross-skill `/update-work-item` reference from the error message; restore the "and contains all nine required fields" clause from refine-work-item's diagnostic; mark multi-match as a defensive branch and simulate the glob result inline; copy `update-work-item`'s exact selection wording; trim argument-hint to ~42 chars; spell out the layout assumption in Step 0; add `/list-work-items` references to the assertions for ids 18/20; lock in `Work item updated: <path>` with a verbatim assertion on id 28; pick `{work_dir}` as the canonical placeholder; keep Step 0 as a two-branch decision; pull the diff preview into scope or document the atomicity assumption.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan extends one Markdown skill with a parallel 'enrich-existing' mode threaded via an internal flag through all five steps, preserving the existing topic-string flow untouched. The branching is locally clean and the no-op surface (no new scripts, no template change, no peer-skill change) is appealing, but the design creates a non-trivial cohesion question (does this skill now own two distinct workflows?) and a boundary question against `refine-work-item`, which already owns enrichment of existing work items. Replicating the four-skill resolution pattern inline rather than extracting a shared helper also reinforces coupling-by-copy across the skill family.

**Strengths**:
- Clean single-file change scope with explicit acknowledgement that no scripts, templates, or peer skills need modification.
- Existing two invocation forms (no-arg, topic-string) preserved verbatim, with regression baseline via `old_skill`.
- Identity-field preservation is treated as an architectural invariant with explicit guards and dedicated programmatic eval assertions.
- Resolution-error contract canonicalised against four peer skills, keeping user-facing error semantics consistent.
- Skill-creator TDD workflow with snapshot-as-baseline gives an explicit before/after architectural fitness function.

**Findings**:
- 🟡 **major**: Domain boundary between create-work-item and refine-work-item asserted but not enforced. Refine-work-item's `enrich`/`sharpen` operations already mutate existing work items; users will have two correct entry points with subtly different conventions.
- 🟡 **major**: Resolution pattern copy-pasted across five skills with no shared abstraction. Plan explicitly cites the duplication but doesn't extract or document it as accepted debt.
- 🟡 **major**: `{enriching_existing}` flag threaded through five steps creates pervasive intra-file coupling and weakens cohesion.
- 🔵 **minor**: Self-exclusion in Step 2 leaks the enrich-existing mode into the documents-locator interaction contract.
- 🔵 **minor**: In-place overwrite path lacks destructive-change safeguards (diff + double confirmation) present in `refine-work-item`.
- 🔵 **minor**: Skill snapshot path lifecycle unspecified — outside workspace, not version-controlled, no reproducibility guarantee.

### Code Quality

**Summary**: The plan extends a single Markdown skill file with an enrichment branch by adding 'When `{enriching_existing}` is set' sub-sections to four of the five steps plus a Step 0 resolution branch. The branching strategy keeps the existing topic-string flow untouched at the cost of duplicating step-level structure and scattering enrichment-specific logic across five locations, which raises maintenance and readability concerns. The TDD-style eval workflow with a baseline snapshot is a strong quality discipline, but the cross-step pseudo-flag (`{enriching_existing}`) and the implicit ordering rules around it are under-specified and risk drift over time.

**Strengths**:
- Test-first discipline: each phase adds eval scenarios before SKILL.md edits with a snapshotted `old_skill` baseline.
- Reuses an established resolution pattern from four peer skills rather than inventing one.
- Reuses `work-item-read-field.sh` rather than introducing new scripts.
- Identity preservation rules made explicit in Quality Guidelines.
- Out-of-scope items explicitly listed under "What We're NOT Doing".
- Per-phase Success Criteria split automated vs. manual verification cleanly.

**Findings**:
- 🟡 **major**: Branching by `{enriching_existing}` flag scattered across five steps creates maintenance burden — Markdown analogue of a flag argument threading through multiple methods.
- 🟡 **major**: `{enriching_existing}` flag semantics and lifecycle not specified — placeholder syntax mimics `{work_dir}` config-script-substitution but is conceptually a runtime flag.
- 🔵 **minor**: Snapshot location `skills/work/create-work-item-workspace/` looks like a skill directory; risks being picked up by globs in plugin packaging.
- 🔵 **minor**: Identity-preservation rules duplicated across Step 0, Step 4, Step 5, Quality Guidelines — drift risk.
- 🔵 **minor**: Numeric multi-match scenario relies on simulated harness state — fragile.
- 🔵 **minor**: Discriminator regex `^[0-9]{1,4}$` excludes 5-digit topics, creating a future cliff at 9999.
- 🔵 **minor**: Unparseable-frontmatter error references `/update-work-item` — cross-skill coupling violates the "no other skill modified" stance.
- 🔵 **minor**: 'Silent overwrite is acceptable' undermines the explicit confirmation rule in Quality Guidelines.

### Test Coverage

**Summary**: The plan adopts a TDD-via-evals approach with structured assertions for invariants and prose for qualitative behaviour, which is conceptually well-suited to a Markdown-driven skill. However, several test-engineering concerns are unresolved: the harness's actual support for the `assertions` field is asserted but never verified against skill-creator code; the multi-match scenario (id 17) and several invariant checks ('did NOT call work-item-next-number.sh', 'no file written') depend on transcript/tool-use telemetry that the existing eval shape does not appear to expose; and the once-and-done baseline snapshot strategy may not correctly grade Phase 2–4 scenarios that did not exist when the snapshot was taken. LLM-graded prose assertions also introduce variance risk that the plan does not address with multiple runs or determinism controls.

**Strengths**:
- Phasing aligns red→green transitions with each SKILL.md change.
- Hybrid assertions array is conceptually right for distinguishing programmatic invariants from qualitative judgements.
- Existing 14 scenarios preserved untouched and re-run every phase.
- Fixture files give a deterministic substrate for resolution branches.
- Comprehensive coverage of the Step 0 error matrix.
- Identity-preservation tests target exact mutation-sensitive failure modes.

**Findings**:
- 🔴 **critical**: Hybrid-grading `assertions` array unverified against the actual eval harness schema.
- 🔴 **critical**: Negative tool-use assertions assume transcript visibility the harness may not provide.
- 🟡 **major**: Single Phase-1 baseline snapshot cannot meaningfully fail Phase 2–4 scenarios.
- 🟡 **major**: Multi-match scenario (id 17) relies on filesystem state the eval harness may not provide.
- 🟡 **major**: Single-run-per-configuration design exposes LLM-graded prose assertions to flake.
- 🟡 **major**: Self-not-duplicate assertion (id 23) is too loose to mutation-test the change.
- 🟡 **major**: Gap-analysis tests don't cover boundary cases (mixed-substantive, instructional-prose-carried-over).
- 🔵 **minor**: Strict `pass_rate: 1.0` on every new scenario contradicts variance noted elsewhere.
- 🔵 **minor**: Fixture creation not itself testable, no verification step.
- 🔵 **minor**: Identity-preservation eval doesn't cover the 'user requested status change' branch.
- 🔵 **minor**: No test that the topic-string flow still calls `work-item-next-number.sh`.

### Correctness

**Summary**: The plan's Step 0 resolution branch is logically sound and consistent with peer skills, and the core invariant (no work-item-next-number.sh call in the enrichment path) is well-protected. However, there are several correctness gaps: the set of fields described as 'preserved' diverges between the plan's own Current State Analysis and the Desired End State / Step 4 spec; the status-change mechanism referenced in Step 4 is undefined; sparse fixture files may trigger spurious frontmatter errors because work-item-read-field.sh exits non-zero on a missing field; and the path-like discriminator's resolution semantics (relative to cwd or work_dir) are not pinned down.

**Strengths**:
- Step 0 branch order consistent with the canonical pattern in three peer skills.
- All three error branches abort before any agent spawn or file write.
- Plan correctly forbids work-item-next-number.sh in the enrichment path; codified twice and tested.
- Path-existence guard removal in Step 5 explicitly justified.
- Self-exclusion of input file from Step 2 duplicate detection.

**Findings**:
- 🟡 **major**: Status-change mechanism referenced but never defined.
- 🟡 **major**: Inconsistent definition of which identity fields are preserved (4 vs 9 vs Step 3-proposable).
- 🟡 **major**: Per-field reads via `work-item-read-field.sh` confound 'missing field' with 'unparseable frontmatter'.
- 🟡 **major**: Path-like resolution semantics (cwd vs work_dir vs absolute) unspecified.
- 🔵 **minor**: Title-update vs H1-sync interaction under-specified.
- 🔵 **minor**: Numeric multi-match implausible given uniqueness guarantee — should be marked defensive.
- 🔵 **minor**: Silent-overwrite escape hatch can skip the change-summary invariant.
- 🔵 **minor**: Removed path-existence guard widens TOCTOU window between Step 0 and Step 5.

### Usability

**Summary**: The plan extends create-work-item with an enrich-existing branch in a way that broadly aligns with peer-skill conventions (path/numeric resolution, error message phrasing, multi-match selection), preserves backward compatibility for the no-arg and topic-string flows, and adds sensible identity-preservation guardrails. However, the overloaded single-slot argument-hint creates real discoverability ambiguity, the silent behaviour change for users currently typing `/create-work-item 42` as a topic is glossed over, and a couple of UX details — the gap-analysis presentation, the conditional double-confirmation in Steps 4/5, and the mismatch with update-work-item's diff/y-n confirmation idiom — risk friction or surprise.

**Strengths**:
- Reuses canonical resolution pattern; users who know one peer skill can predict resolution behaviour.
- Error-message strings copied verbatim from peer skills.
- Backward compatibility preserved for no-arg and topic-string forms (claim — see findings).
- Identity preservation well-specified.
- Gap-analysis framing is a thoughtful affordance.

**Findings**:
- 🟡 **major**: Single-slot argument-hint conflates three distinct inputs.
- 🟡 **major**: Silent behaviour change for `/create-work-item <number>` and path-like inputs — Migration Notes claim doesn't hold.
- 🟡 **major**: Discoverability of enrich-mode is poor — a separate `/enrich-work-item` would be more obvious.
- 🔵 **minor**: Two confirmation gates (Step 4 + Step 5) create double-friction.
- 🔵 **minor**: `Complete: … / Gaps: …` listing not actionable without per-section context.
- 🔵 **minor**: Multi-match selection prompt format unspecified — adopt update-work-item's exact wording.
- 🔵 **suggestion**: Draft preview should show what changed, not just the full updated work item — pull diff preview into scope.

### Compatibility

**Summary**: The plan is largely backwards-compatible: the new path-like / numeric discriminators in Step 0 do not collide with any of the 14 existing eval topic strings, no new bash patterns are required (work-item-read-field.sh is already covered by the existing allowed-tools glob), and the no-arg / topic-string flows are explicitly preserved. However, several real compatibility concerns remain: the discriminators can silently reroute legitimate user topics that happen to be a small integer or contain a slash, the snapshot-as-old_skill baseline is constructed once but reused across phases that add scenarios the baseline cannot meaningfully address, the eval id-numbering convention may not be id-stable in the harness, and the argument-hint change is a contract surface that downstream tooling may render or validate.

**Strengths**:
- Allowed-tools assessment correct.
- Existing 14 evals' topic strings don't collide with new discriminators.
- Identity-field preservation contractually pinned; next-number contractually never called in enrich path.
- Migration Notes correctly identifies no on-disk format changes.
- No new agent spawns; topic-string flow steps 1–5 textually untouched.

**Findings**:
- 🟡 **major**: Numeric and path-like discriminators silently reroute legitimate topic strings (`42`, `update README.md`, `add /healthz endpoint`).
- 🟡 **major**: Single Phase-1 snapshot is not a meaningful baseline for new scenarios across phases.
- 🔵 **minor**: Eval id reuse / ordering assumption may break harness contracts (filter range semantics).
- 🔵 **minor**: argument-hint string growth (~57 chars) may exceed downstream display budgets.
- 🔵 **minor**: Glob `{work_dir}/NNNN-*.md` assumes case-sensitive single-directory layout — worth spelling out.

### Safety

**Summary**: The plan introduces in-place overwrite semantics for an existing user file but defers most data-loss safeguards. For a developer-tool skill operating in a jj-tracked workspace the blast radius is bounded — VCS provides an implicit recovery path — but the plan does not surface this explicitly, removes the existing path-existence guard without a substitute, and leaves several confirmation/atomicity questions unaddressed. With small, low-cost additions (a pre-write confirmation gate and a brief recovery note) the safety posture would match peer skills.

**Strengths**:
- Identity-field preservation enforced at multiple checkpoints with eval coverage.
- Enrich path explicitly forbids work-item-next-number.sh; eval id 26 catches regression.
- Resolution errors abort before any agent spawn or write.
- Self-exclusion rule in Phase 2 §2.

**Findings**:
- 🟡 **major**: Silent overwrite licensed by stale Step 4 approval — no temporal bound.
- 🟡 **major**: Path-existence guard removed without substitute integrity check (no checksum or work_item_id re-read).
- 🔵 **minor**: Recovery story not surfaced — `jj restore <path>` is the implicit safety net.
- 🔵 **minor**: Identity preservation enforced only in prose — no programmatic guardrail at write time.
- 🔵 **suggestion**: Partial-write atomicity not addressed.

### Standards

**Summary**: The plan does a creditable job of citing the four peer skills as the source of the resolution and frontmatter-guard patterns, but its adopted text introduces several subtle divergences from those peers — the missing-path message, the unparseable-frontmatter diagnostic, the success-message wording, and an internal inconsistency in the identity-field list. None individually breaks anything, but together they erode the cross-skill consistency the plan's own Current State Analysis makes a virtue of.

**Strengths**:
- Plan explicitly anchors Step 0 branching to four named peer skills.
- Discriminator order and multi-match wording match update-work-item / review-work-item.
- H1 form `# NNNN: <title>` and the identity-field list match `templates/work-item.md` verbatim.

**Findings**:
- 🟡 **major**: Identity-field list internally inconsistent between Step 0 (9 fields) and Quality Guidelines (4 fields).
- 🟡 **major**: Unparseable-frontmatter diagnostic diverges from the canonical refine-work-item wording (drops "and contains all nine required fields" clause).
- 🔵 **minor**: Missing-path / numeric-no-match messages drop the offer-to-list affordance present in review-work-item / refine-work-item.
- 🔵 **minor**: Success-message wording (`Work item updated: <path>`) diverges from existing create-work-item form and from update-work-item's confirmation idiom.
- 🔵 **minor**: Fixture directory `evals/files/` asserted as convention without precedent in the cited peer skills.
- 🔵 **minor**: `<path>` vs `{work_dir}` placeholder spelling inconsistent across peer skills.
- 🔵 **minor**: Re-numbering existing rules into a three-branch list breaks the two-branch peer pattern.

## Re-Review (Pass 2) — 2026-04-27

**Verdict:** REVISE

The revision substantively addresses all 50+ findings from Pass 1: every prior critical and major item is either resolved or explicitly acknowledged as a deferred tradeoff with rationale. The harness alignment, identity-field canonicalisation, single-confirmation gate, per-phase re-snapshot, and gap-analysis tagging are particularly strong improvements. However, the revision introduced **5 new major findings** — most centred on a recurring theme: the plan claims runtime guarantees that a Markdown-only skill cannot actually deliver. Address these before implementation; the scope is much narrower than Pass 1.

### Previously Identified Issues (Pass 1)

#### Architecture
- 🟡 **Architecture**: Domain boundary between create-work-item and refine-work-item — Partially resolved (Tradeoffs section articulates the boundary; still enforced only by prose discipline)
- 🟡 **Architecture**: Resolution pattern duplicated across five skills — Partially resolved (acknowledged with follow-up commitment)
- 🟡 **Architecture**: Mode flag threaded through five steps — Partially resolved (renamed, structural coupling remains by design — Tradeoffs documents it)
- 🔵 **Architecture**: Self-exclusion leaks into documents-locator contract — Resolved
- 🔵 **Architecture**: In-place overwrite lacks destructive-change safeguards — Resolved
- 🔵 **Architecture**: Skill snapshot path outside workspace — Resolved

#### Code Quality
- 🟡 **Code Quality**: Branching scattered across 5 steps — Partially resolved (acknowledged as tradeoff; structure unchanged)
- 🟡 **Code Quality**: `{enriching_existing}` flag semantics unspecified — Resolved (now documented as conversation state)
- 🔵 **Code Quality**: Snapshot location naming — Resolved
- 🔵 **Code Quality**: Identity-preservation rules duplicated — Mostly resolved (Step 3 still partially restates — see new finding)
- 🔵 **Code Quality**: Discriminator regex 5-digit cliff — Resolved
- 🔵 **Code Quality**: Cross-skill `/update-work-item` reference in error — Resolved
- 🔵 **Code Quality**: Silent overwrite clause — Resolved (removed entirely)

#### Test Coverage
- 🔴 **Test Coverage**: Hybrid-grading `assertions` array unverified — Resolved (now `expectations: [string]` with verified harness contract)
- 🔴 **Test Coverage**: Negative tool-use assertions assume transcript visibility — Resolved (framed as grader-mediated transcript inspection)
- 🟡 **Test Coverage**: Single Phase-1 baseline can't fail Phase 2-4 scenarios — Resolved (per-phase re-snapshot)
- 🟡 **Test Coverage**: Multi-match scenario relies on filesystem state — Resolved (documented as prompt-rendering test, defensive branch)
- 🟡 **Test Coverage**: Single-run-per-configuration exposes flake — Resolved (`runs_per_query=3`, `trigger_rate >= 0.9`)
- 🟡 **Test Coverage**: Self-not-duplicate (id 23) too loose — Resolved (positive+negative paired)
- 🟡 **Test Coverage**: Gap-analysis boundary cases — Resolved (ids 21a, 21b added)
- 🔵 **Test Coverage**: Strict pass_rate: 1.0 — Resolved (0.9 threshold)
- 🔵 **Test Coverage**: Fixture creation not testable — Resolved (sanity check added)
- 🔵 **Test Coverage**: Identity-preservation user-status branch — Resolved (id 30)
- 🔵 **Test Coverage**: Topic-string still calls next-number — Resolved (id 32)

#### Correctness
- 🟡 **Correctness**: Status-change mechanism undefined — Resolved (Step 3 + Quality Guidelines + id 30)
- 🟡 **Correctness**: Inconsistent identity-field definition — Resolved (canonical 3-group rule)
- 🟡 **Correctness**: Per-field reads confound missing-field with unparseable — Resolved (one canonical existence/parse check)
- 🟡 **Correctness**: Path-like resolution semantics unspecified — Resolved (cwd-relative if relative, literal if absolute)
- 🔵 **Correctness**: Title-update vs H1-sync — Resolved (Title now in Step 3 proposable list)
- 🔵 **Correctness**: Numeric multi-match implausible — Resolved (marked defensive)
- 🔵 **Correctness**: Silent-overwrite escape hatch — Resolved (clause removed)
- 🔵 **Correctness**: Removed path-existence guard widens TOCTOU — Partially resolved (at-write integrity check narrows but does not close it — see new finding)

#### Usability
- 🟡 **Usability**: Single-slot argument-hint — Resolved (trimmed)
- 🟡 **Usability**: Silent behaviour change — Resolved (zero-match fallback warns + falls through)
- 🟡 **Usability**: Discoverability of enrich-mode — Partially resolved (mitigation via description-frontmatter is weak — see new finding)
- 🔵 **Usability**: Two confirmation gates — Resolved (single Step 5 gate)
- 🔵 **Usability**: Gap-analysis listing not actionable — Resolved (per-gap reason tags)
- 🔵 **Usability**: Multi-match wording unspecified — Resolved (copies update-work-item)
- 🔵 **Usability**: Draft preview should show changes — Partially resolved (per-section change summary in Step 5; full diff still deferred)

#### Compatibility
- 🟡 **Compatibility**: Discriminators silently reroute topic strings — Resolved (zero-match fallback)
- 🟡 **Compatibility**: Single Phase-1 snapshot — Resolved (per-phase re-snapshot)
- 🔵 **Compatibility**: Eval id reuse — Resolved (jq monotonicity check)
- 🔵 **Compatibility**: argument-hint length — Resolved (trimmed to ~42 chars)
- 🔵 **Compatibility**: Glob case-sensitivity — Still present (not addressed in revision)

#### Safety
- 🟡 **Safety**: Silent overwrite licensed by stale Step 4 approval — Resolved
- 🟡 **Safety**: Path-existence guard removed without substitute — Resolved (at-write integrity check; though scope is narrower than implied — see new finding)
- 🔵 **Safety**: Recovery story not surfaced — Resolved (Recovery section)
- 🔵 **Safety**: Identity preservation prose-only — Partially resolved ("programmatic substitution" overclaims — see new finding)
- 🔵 **Safety**: Partial-write atomicity not addressed — Resolved (Write atomicity section)

#### Standards
- 🟡 **Standards**: Identity-field list internally inconsistent — Resolved
- 🟡 **Standards**: Unparseable-frontmatter diverges from refine-work-item — Partially resolved (plan claims verbatim match, still diverges — see new finding)
- 🔵 **Standards**: Missing-path/numeric-no-match drops offer-to-list — Resolved (now in fallback warning)
- 🔵 **Standards**: Success-message wording diverges — Resolved (locked in via id 28 expectation)
- 🔵 **Standards**: Fixture directory convention — Resolved (verified against skill-creator schema)
- 🔵 **Standards**: Path/work_dir placeholder inconsistent — Resolved (`{work_dir}` canonical)
- 🔵 **Standards**: Re-numbering broke two-branch peer pattern — Resolved (two-branch structure restored)

### New Issues Introduced

#### Major (5)

- 🟡 **Test Coverage**: Eval id 31 cannot be exercised under the harness contract the plan documents
  **Location**: Phase 3 Success Criteria, id 31
  The harness has no runtime file-mutation mechanism, so the cached `work_item_id` and the on-disk `work_item_id` will always agree within a single executor run. Forcing the cached value via prompt instruction tests instruction-following, not the integrity check. Either drop id 31 in favour of manual verification, or restructure as a transcript check that the at-write `work-item-read-field.sh` call appears at Step 5 immediately before the Write tool call.

- 🟡 **Correctness / Safety**: "Programmatically substitute the cached immutable identity fields" is not a defined operation in a Markdown skill
  **Location**: Phase 3 §2 (Step 5 step 5)
  The skill is a Markdown prompt with no scripted post-processing pass; the only actor that can perform substitution is the model itself, and the only verification is the grader subagent. The plan's own Eval Harness Notes warns against using "programmatic" wording precisely because every check is grader-mediated. The current wording overclaims the safety guarantee and may confuse the model into invoking a non-existent script. Reword as: "Immediately before invoking Write, re-read the cached immutable fields from conversation state and overwrite the corresponding lines in the draft frontmatter with those exact values, even if they appear unchanged. This is a textual substitution the model performs."

- 🟡 **Safety**: At-write integrity check verifies only `work_item_id`, not `date` / `author` / full-content drift
  **Location**: Phase 3 §2 (Step 5 step 2) and Quality Guidelines Identity Field Rules
  If a concurrent editor modifies `date`, `author`, `status`, or body content while leaving `work_item_id` unchanged, the integrity check passes and the overwrite proceeds. Worse: the immutable-field substitution then rewrites `date`/`author` from the stale Step 0 cache, regressing identity to a now-incorrect prior state. Either extend the check to compare a content fingerprint (mtime, full-frontmatter hash, full file SHA) captured at Step 0, or explicitly scope and document that the check only catches identity-swap (not concurrent edits) and rely on VCS recovery.

- 🟡 **Safety**: Confirmation gate behaviour for non-y/non-n input is unspecified
  **Location**: Phase 3 §2 (Step 5 step 3)
  The y/n gate has no defined behaviour for empty input, "yes", "YES", "sure", "go ahead", or "looks good but please also...". An ambiguous response is a fail-open risk on a destructive overwrite. Add explicit fall-through: any response that is not exactly `y` or `n` (case-insensitive, after trimming) is treated as `n` and the skill returns to iteration with `Did not recognise <response> as y/n — staying in review.`

- 🟡 **Compatibility**: Phase 4 snapshot-restoration command is broken
  **Location**: Phase 4 §1
  Three issues in the shell snippet: (a) `rm -rf` of the snapshot directory immediately followed by a redirect that does not `mkdir -p` the parent — the redirect will fail with "No such file or directory"; (b) `HEAD~N` is a literal placeholder, not an executable revision specifier; (c) the snippet writes only `SKILL.md` whereas Phase 1's snapshot was a full `cp -r` of the directory — the baseline subagent's contract differs between phases. Replace with a directory-level restore that mirrors Phase 1, e.g. `mkdir -p meta/eval-workspaces/create-work-item/skill-snapshot && jj file show -r <pre-plan-rev> ... ` or `cp -r` from the pre-Phase-1 worktree.

#### Minor (≈15) — see Per-Lens Results below for the full list

Selected high-signal items:

- 🔵 **Test Coverage**: Negative tool-use expectations rely on grader's transcript completeness assumptions; tighten lexical patterns (id 26, id 32, id 23).
- 🔵 **Test Coverage**: Gap-tag wording is grader-fragile — tighten SKILL.md to require the exact tokens `empty`/`placeholder-only`/`instructional-prose`/`partial`.
- 🔵 **Test Coverage**: Existing-14 parity gate at Phase 4 satisfiable by flat regression — add per-scenario gate.
- 🔵 **Code Quality**: Step 3 still partially restates Identity Field Rules — should defer purely.
- 🔵 **Code Quality**: Stale `{enriching_existing}` token in Architectural Tradeoffs section title.
- 🔵 **Correctness**: "Spontaneously requests" status-change trigger under-specified — tighten to "explicit, direct request".
- 🔵 **Correctness**: H1 sync rule wording inconsistent across locations.
- 🔵 **Usability**: "Press Ctrl-C to abort" hint may not behave cleanly mid-skill — replace with positive prompt.
- 🔵 **Usability**: Integrity-mismatch abort discards the user's enrichment draft — dump draft before aborting.
- 🔵 **Usability**: Per-section change summary risks being skimmed past — lead with what's changing.
- 🔵 **Usability**: Status-change trigger phrased so users may not realise the affordance — show status in Step 3 listing.
- 🔵 **Compatibility**: Fallback warning preamble is itself a stdout contract change — document in Migration Notes.
- 🔵 **Compatibility**: Glob case-sensitivity carried over from Pass 1 — still unaddressed.
- 🔵 **Compatibility**: `expectations` field may break older harness versions — pin a minimum.
- 🔵 **Safety**: Integrity-check failure path has only one eval (id 31, which is itself unrealisable) — add ids 31a (file-deleted) and 31b (frontmatter-corrupted).
- 🔵 **Safety**: Pre-write snapshot recommended but not enforced — add a `jj status <path>` check to the Step 5 confirmation.
- 🔵 **Standards**: Plan claims verbatim parity with refine-work-item's unparseable-frontmatter message but truncates the `/update-work-item` clause — reword to acknowledge intentional divergence.
- 🔵 **Standards**: `Aborting overwrite:` error prefix has no precedent in peer skills (`Error: ...` is the convention).

### Cross-Cutting Theme in New Findings

The dominant new theme — flagged by **correctness** and **safety** independently, and implicit in **test-coverage** finding 1 — is that the revision strengthened the safety story by introducing language that overclaims what a Markdown-only skill can actually enforce at runtime: "programmatic substitution", "at-write integrity check", "abort on drift", "id 31 verifies the abort fires". In each case the underlying mechanism is the model following prose instructions, with a grader subagent reviewing the transcript after the fact. This is fine — but the plan's wording invites readers and reviewers to believe the guarantees are stronger than they are, and the corresponding eval scenarios cannot test what they claim to test. Either soften the wording to match the grader-mediated reality (preferred), or introduce a genuinely programmatic substrate (a small `work-item-write-with-identity.sh` helper script that takes draft body + cached identity fields as inputs and produces the final file) and rebuild the integrity guarantees on top of it.

### Assessment

The plan is materially closer to ready than it was at Pass 1 — every prior major issue is addressed, and the architectural choices are now visible decisions rather than implicit assumptions. The remaining issues are scoped: 5 majors all sit in Phase 3 §2 (Step 5 wording + integrity check semantics), Phase 3 Success Criteria (id 31), and Phase 4 §1 (broken shell snippet). A focused third pass addressing those five — plus the high-signal minors — would close the verdict to APPROVE.

## Re-Review (Pass 3) — 2026-04-27

**Verdict:** COMMENT — Plan is acceptable but could be improved; see below for the implementation punch list.

The Pass 3 revision substantively closes all 5 majors flagged in Pass 2. Step 5 now has explicit fail-safe semantics for non-y/n input, the at-write check is honestly scoped as identity-swap-only, "programmatic substitution" is reframed as a model-performed textual edit, id 31 is restructured as a transcript-only existence check (with full failure-mode coverage in manual verification), and the Phase 4 shell snippet is reproducible. The plan is implementation-ready.

The remaining issues are scoped polish — one factual error (a line citation), several format-spec ambiguities in the new Step 5 confirmation message, and a handful of new-convention disclosures that should be made explicit. A focused implementation pass can address all of these alongside the SKILL.md edits; a fourth review pass is not required.

### Previously Identified Issues (Pass 2)

#### Architecture
- 🔵 **Architecture**: Phase 4 baseline-restoration fragile — Resolved
- 🔵 **Architecture**: Identity Field Rules location couples mode-conditional with mode-agnostic — Resolved
- 🟡 **Architecture**: Domain boundary with refine-work-item — Partially resolved (carry-over from Pass 1, no new test coverage)
- 🟡 **Architecture**: Resolution pattern duplicated — Partially resolved (acknowledged with follow-up)
- 🟡 **Architecture**: Mode flag through 5 steps — Partially resolved (carry-over)

#### Code Quality
- 🔵 **Code Quality**: Step 3 still partially restates Identity Field Rules — Resolved (Steps 0/3/4 now defer purely)
- 🔵 **Code Quality**: Stale `{enriching_existing}` token in Tradeoffs title — Resolved
- 🔵 **Code Quality**: Phase 4 snapshot-restore writes through removed dir — Resolved

#### Test Coverage
- 🟡 **Test Coverage**: Id 31 cannot be exercised under harness contract — Resolved (restructured as transcript existence check; failure modes moved to manual verification)
- 🟡 **Test Coverage**: Negative tool-use expectations rely on transcript completeness — Partially resolved (id 31 tightened; id 26 still loose)
- 🔵 **Test Coverage**: Gap-tag wording grader-fragile — Resolved (literal tokens now mandated in SKILL.md instruction; expectations aligned)
- 🔵 **Test Coverage**: Multi-match tests prompt rendering not heuristic — Carry-over (acknowledged limitation)
- 🔵 **Test Coverage**: Existing-14 parity gate satisfiable by flat regression — Resolved (per-scenario gate added)
- 🔵 **Test Coverage**: Verbatim warning string brittle — Resolved (loosened to substring/semantic)

#### Correctness
- 🟡 **Correctness**: At-write integrity TOCTOU — Resolved (reframed as best-effort identity-swap guard with explicit residual window note)
- 🟡 **Correctness**: "Programmatically substitute" not defined — Resolved (reworded as model-performed textual edit)
- 🔵 **Correctness**: "Spontaneously requests" under-specified — Resolved (tightened to "explicit, direct request"; eval id 30a covers oblique-mention guard)
- 🔵 **Correctness**: H1 sync wording inconsistent — Resolved
- 🔵 **Correctness**: Topic-string fall-through control flow relies on prose — Carry-over (acceptable)
- 🔵 **Correctness**: Fallback warning leaves discriminator-input ambiguous — Carry-over (warning now invites re-run)

#### Usability
- 🔵 **Usability**: Ctrl-C hint may not behave cleanly — Resolved (reworded to "abort and re-run with a different argument")
- 🔵 **Usability**: Integrity-mismatch discards user's draft — Resolved (abort messages now dump draft inline)
- 🔵 **Usability**: Per-section change summary risks being skimmed — Resolved (leads with what's changing)
- 🔵 **Usability**: Status-change trigger phrased so users may not realise — Resolved (Step 3 listing shows status with "say so if you'd like to transition" affordance)
- 🔵 **Usability**: Discoverability via description-frontmatter weak — Carry-over (accepted)
- 🔵 **Usability**: Zero-padding rule may surprise — Carry-over (accepted)

#### Compatibility
- 🟡 **Compatibility**: git show redirect / HEAD~N broken — Resolved (rewritten with `git archive` against resolved jj revision, mkdir -p, shape parity check)
- 🔵 **Compatibility**: Fallback warning preamble is stdout contract change — Still present (Migration Notes does not mention)
- 🔵 **Compatibility**: Glob case-sensitivity carried over — Resolved (now documented)
- 🔵 **Compatibility**: Regenerated benchmark.json schema may diverge — Still present
- 🔵 **Compatibility**: `expectations` field may break older harness versions — Still present
- 🔵 **Compatibility**: Verbatim warning string couples skill prose to grader — Resolved

#### Safety
- 🟡 **Safety**: Integrity check verifies only work_item_id — Resolved (honestly scoped; concurrent edits routed to VCS)
- 🟡 **Safety**: "Programmatic substitution" is grader-mediated — Resolved (reworded)
- 🟡 **Safety**: Confirmation gate non-y/n behaviour unspecified — Resolved (explicit fail-safe added)
- 🔵 **Safety**: Pre-write snapshot recommended but not enforced — Carry-over (acceptable; jj auto-snapshots)
- 🔵 **Safety**: Single eval covers integrity-check failure path — Resolved (3 manual sub-cases added)
- 🔵 **Safety**: Step 4 'looks good' rule prescriptive only — Effectively resolved (Step 5 fail-safe covers it)

#### Standards
- 🔵 **Standards**: Plan claimed verbatim parity with refine-work-item — Resolved (now honestly described as structural form with intentional omission)
- 🟡 **Standards**: `Aborting overwrite:` prefix without precedent — Partially resolved (now `Error:` matching update-work-item, but cited line range is wrong — see new finding)
- ✅ **Standards**: Identity Field Rules new convention — Resolved (now explicitly self-flagged as new with backfill follow-up note)
- 🔵 **Standards**: Fallback warning format diverges from peer offer-to-list — Carry-over (not explicitly addressed)

### New Issues Introduced

#### Major (1)

- 🟡 **Compatibility / Standards**: Cited line range for `update-work-item` Error: prefix is wrong
  **Location**: Phase 3 §2 Step 5 step 2 — error-prefix justification
  Plan cites `update-work-item/SKILL.md:88-95`. Lines 88-95 are the tag-operations script delegation block; the actual `Error:` precedent is at update-work-item:60-65 (frontmatter validation: `"Error: <filename> has no YAML frontmatter..."`) and 125-128 (`"Error: work_item_id cannot be changed..."`). The convention is real but the citation is misattributed.
  **Fix**: Replace the citation with `update-work-item/SKILL.md:60-65, 125-128`.

#### Minor (~17) — bundled by theme

**Phase 4 shell snippet still brittle (cross-cutting: code-quality, compatibility, usability)**
- 🔵 No `set -euo pipefail`; `<pre-plan-rev>` is a literal-replace footgun with no missing-var guard.
- 🔵 `git -C "$(jj root)" archive` assumes jj-on-git colocation; pure jj-native checkouts will not have a working git command at the jj root. `<pre-plan-rev>` must be a git commit hash, not a jj change id.
- 🔵 Manual `<pre-plan-rev>` substitution is friction; user has to `jj log` and pick the right commit a week after starting Phase 1. Recommend tagging via bookmark at Phase 1 start (`jj bookmark set create-work-item-pre-plan -r @-`) and referencing the bookmark in Phase 4.

**Step 5 confirmation prompt format under-specified (code-quality, usability, compatibility)**
- 🔵 `<count>`, `(<terse list or "none">)`, `[+ any unmodified proposable fields]` placeholders have ambiguous rendering rules. Either commit to one verbatim form or annotate the placeholders with rendering rules.
- 🔵 Always-shown `(status: <cached> unchanged)` couples the format to a single render path; consider showing it only when no other frontmatter changed.
- 🔵 Section count requirement is hard to operationalise consistently across runs; consider dropping the count and listing all preserved sections deterministically.

**Step 5 fail-safe message wording (usability, standards, correctness)**
- 🔵 "What change would you like before I overwrite?" presumes intent; user typing "yes" expecting acceptance gets pushed into invented feedback. Use a neutral re-prompt: `"Reply 'y' to overwrite <path>, 'n' to keep iterating, or describe a change."`
- 🔵 update-work-item:201-207 uses re-prompt-once-then-decline policy; this skill uses one-strike treat-as-decline. Either align with peer policy or flag the divergence as deliberate.
- 🔵 Trim semantics for confirmation input not fully specified — define as "after stripping leading/trailing ASCII whitespace (spaces, tabs, CR, LF)".

**Newly-introduced conventions deserve explicit disclosure (standards × 3)**
- 🔵 Step 5 change-summary block format has no peer precedent (peer skills use unified diffs). Either flag as new convention with rationale or align with diff form.
- 🔵 "Apply the canonical X (see Quality Guidelines)" defer-by-name cross-reference style is new in the work-item skill family. Acknowledge alongside the existing Identity Field Rules disclosure.
- 🔵 Named-rule sub-structure inside Quality Guidelines (3 named groups with nested sub-bullets) diverges from peer skills' flat-bullet shape. Flatten or document.

**Other minors**
- 🔵 **Test Coverage**: id 30a's "asks clarifying question" expectation could pass for a model that proposes a transition with confirmation. Add paired negative: "Response does NOT name a specific status value as a proposed transition before the user clarifies".
- 🔵 **Test Coverage**: No eval covers the confirmation fail-safe (P3-M4) — add a scenario where simulated user says "yes" and verify it routes to fail-safe with no Write tool invocation.
- 🔵 **Test Coverage**: id 26 negative tool-use expectation still says "Transcript shows no invocation". Tighten to lexical search: "Transcript contains zero occurrences of the literal string `work-item-next-number.sh` between Step 0 and Step 5."
- 🔵 **Correctness**: Step 5 step 5 line-based substitution may break if Step 4 reordered/refactored frontmatter. Reword as key-targeted: "overwrite the value of the `work_item_id`, `date`, `author` keys in the draft's YAML frontmatter, regardless of layout".
- 🔵 **Correctness**: Step 5 step 8 ("stay in Step 4 / Step 5 review") conflates the two steps' return states. Specify: on `n`, return to start of Step 4 (re-offer draft); on unrecognised, stay at Step 5 prompt and re-prompt.
- 🔵 **Architecture**: Step 5 enrich variant is now ~100 lines and concentrates write-path logic. Consider promoting the at-write check and confirmation interpretation into named Quality Guidelines entries that Step 5 references.
- 🔵 **Usability**: Migration Notes still references "hit Ctrl-C" inconsistently with Step 0 fallback wording change.
- 🔵 **Usability**: Inline draft in abort messages has no upper bound; for long drafts the abort header scrolls off-screen. Consider writing to a tmp file or wrapping in fenced code block with reason both above and below.
- 🔵 **Compatibility**: Stdout fallback preamble still not mentioned in Migration Notes.

### Cross-Cutting Themes (Pass 3)

- **Pass 4 polish surface centres on Step 5**: 5 of the new minors target the Step 5 enrich variant's confirmation message, fail-safe wording, or sub-section length. Implementation should treat Step 5 as the touch-up zone.
- **Phase 4 shell snippet wants a script**: 3 lenses independently flagged the `<pre-plan-rev>` substitution and the absence of `set -euo pipefail` as friction. Consider extracting to `scripts/restore-snapshot.sh` with proper error handling.
- **Pass 3 introduced 4 new conventions** without explicit disclosure: change-summary block format, fail-safe wording, defer-by-name cross-reference style, named-rule Quality Guidelines structure. Add a single "New conventions introduced" subsection to the Tradeoffs block listing all four (Identity Field Rules already there) — this future-proofs the standards story.

### Assessment

The plan is ready to implement. The 1 major finding is a single-line citation fix; the ~17 minors are scoped polish that can be addressed during the SKILL.md edits in Phase 1–3. No structural rework is needed. A focused fourth review pass is not required — verify the citation fix and sample 2-3 of the higher-signal minors as part of implementation review instead.
