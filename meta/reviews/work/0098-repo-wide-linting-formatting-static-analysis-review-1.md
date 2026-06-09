---
type: work-item-review
id: "0098-repo-wide-linting-formatting-static-analysis-review-1"
title: "Work Item Review: Repo-Wide Linting, Formatting, And Static Analysis Guardrails"
date: "2026-06-09T18:14:24+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0098"
work_item_id: "0098"
reviewer: Toby Clemson
verdict: "APPROVE"
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-06-09T20:04:32+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Repo-Wide Linting, Formatting, And Static Analysis Guardrails

**Verdict:** REVISE

This is an unusually well-drafted tooling work item: "strict from day one" is
explicitly mapped to concrete per-tool configurations, open questions are
closed with dated decisions, and nearly every acceptance criterion names an
exact runnable command. The findings cluster around three structural gaps
rather than drafting quality: the acceptance criteria verify that tools
*pass* but not that they pass *under the mandated strictness* (lenient
configs and liberal suppressions would satisfy every AC), the single-merge
repo-wide remediation sweep carries delivery and coordination risk that the
recorded keep-unsplit decision accepts but does not mitigate, and several
cross-section inconsistencies (Biome domain lists, the open-ended Summary
scope, a Dependencies section contradicted by References) leave an
implementer and a verifier able to reach different conclusions about what
"done" means.

### Cross-Cutting Themes

- **AC1 silently drops the `project` Biome domain mandated by Requirements**
  (flagged by: clarity, completeness, testability) — the Requirements enable
  three domains (`react`, `test`, `project`); AC1 verifies only two. A
  verifier following AC1 literally would accept a configuration that violates
  the requirement, and the `project` domain governs Biome v2's type-aware
  analysis — the headline capability the work item's own Technical Notes cite.
- **The Summary's "other file types where it makes sense" is never resolved**
  (flagged by: clarity, completeness, scope) — Requirements enumerate a closed
  set of five surfaces but never state that the enumeration is the final
  answer. YAML, Markdown, TOML, JSON, and GitHub Actions workflows are neither
  included nor excluded, inviting scope negotiation mid-implementation.
- **The Dependencies section contradicts the rest of the document** (flagged
  by: completeness, dependency) — it reads "Related: none captured yet" while
  References explicitly names the 0090 relation, and the document's own
  Technical Notes describe a load-bearing external coupling on the mise
  registry that is captured nowhere.
- **The all-at-once repo-wide sweep is the item's highest delivery risk**
  (flagged by: scope, dependency, testability) — bundling five per-language
  workstreams behind a single merge point gates every guardrail on the
  slowest remediation stream, collides with in-flight work with no recorded
  coordination constraint, and has no sizing contingency if violation volume
  under maximally strict rule sets balloons.

### Findings

#### Critical

None.

#### Major

- 🟡 **Testability**: Pass-cleanly criteria don't verify configured strictness and place no bound on suppressions
  **Location**: Acceptance Criteria
  Every per-language criterion verifies that a tool exits cleanly, but none
  verifies that the configuration matches the mandated strictness, and none
  bounds suppression mechanisms — a lenient config plus liberal `# noqa` /
  `#[allow(...)]` / `// biome-ignore` suppressions passes every AC while
  delivering none of the intended guardrail strength.

- 🟡 **Clarity / Completeness / Testability** (merged): Biome domain list differs between Requirements and AC1
  **Location**: Acceptance Criteria
  The Frontend requirement enables `react`, `test`, and `project` domains,
  but AC1 verifies only "react + test domains", silently dropping `project` —
  the domain governing Biome v2's type-aware, multi-file analysis. The work
  could be signed off with a stated requirement unimplemented and unverified.

- 🟡 **Clarity / Completeness / Scope** (merged): Open-ended "other file types where it makes sense" never resolved by Requirements
  **Location**: Summary
  The Summary scopes the work open-endedly but the Requirements enumerate a
  closed set without stating it is the complete answer. An implementer and a
  reviewer can reasonably disagree on whether Markdown/YAML/TOML/JSON tooling
  is in scope, leading to unplanned scope growth or a "done" dispute.

- 🟡 **Scope**: Single task bundles four-plus independently deliverable per-language efforts plus a full-repo remediation sweep
  **Location**: Requirements
  Five independent toolchain workstreams — each completable, mergeable, and
  rollback-able alone — share a single merge point spanning ~700 Python files
  and ~160 shell scripts under maximally strict rules. A stall in one stream
  (pyrefly-strict remediation is plausibly weeks by itself) delays guardrails
  that could otherwise land and start paying off immediately.

- 🟡 **Dependency**: Repo-wide auto-fix sweep has uncaptured coordination constraint with in-flight work
  **Location**: Acceptance Criteria
  The fix-everything mandate implies mass mechanical commits that will
  collide with any concurrently in-flight work, yet no sequencing or
  coordination constraint is recorded anywhere — exactly the hidden
  "you can't merge until X" constraint the Dependencies section exists to
  surface.

#### Minor

- 🔵 **Completeness / Dependency** (merged): Dependencies section is placeholder text contradicted by the References section
  **Location**: Dependencies
  "Related: none captured yet" while References explicitly captures the 0090
  relation. Tooling and readers consulting the canonical dependency record
  conclude no relationships exist.

- 🔵 **Clarity**: 'Keep in sync' has no actor, trigger, or observable state
  **Location**: Requirements
  The `.editorconfig` duplication instruction for ruff/rustfmt names no
  mechanism for ongoing synchronisation — drift will go unnoticed and an
  implementer cannot tell whether a sync check is expected as part of this
  work.

- 🔵 **Dependency**: External tool-availability coupling on the mise registry not captured as a dependency
  **Location**: Dependencies
  The single-step CI design rests on the mise registry providing pinned
  versions of biome, ruff, pyrefly, shellcheck, and shfmt — pyrefly 1.0 in
  particular is stable only since May 2026 — but none of these appear in the
  Dependencies section.

- 🔵 **Clarity**: Ambiguous mapping of type-checks to the named lint tasks and AC5's 'full suite'
  **Location**: Requirements
  `tsc --noEmit` and pyrefly are mandated but it is unclear whether they live
  inside the named `lint:*` tasks, get their own tasks, and whether AC5's
  "full suite" includes them — AC6's "same checks" CI wiring inherits the
  same ambiguity.

- 🔵 **Clarity / Testability** (merged): Version-pinning criterion underspecified — owning manifest and pin semantics undefined
  **Location**: Acceptance Criteria
  AC7's "(mise/uv/package.json)" never says which manifest authoritatively
  owns each tool's version (in tension with "mise installs all of these
  tools"), and "pinned" is undefined — range constraints could be argued to
  satisfy it while still allowing the rule-set drift the criterion exists to
  prevent.

- 🔵 **Scope**: Declared kind 'task' undersells a repo-wide, multi-toolchain effort
  **Location**: Frontmatter: kind
  Repo-wide tooling adoption across five language domains plus a full
  remediation sweep is story- or epic-shaped; the task label risks it being
  slotted into capacity sized for a small, bounded change.

- 🔵 **Testability**: CI criterion 'fails the build on any violation' has no falsification procedure
  **Location**: Acceptance Criteria
  Unlike 0090's explicit probe, AC6 defines no way to confirm fail-on-violation
  behaviour — a verifier can confirm the jobs exist but the blocking
  behaviour remains trust-based.

- 🔵 **Testability**: File-discovery scope for 'all Python' and 'all .sh scripts' is undefined, so excludes can silently narrow it
  **Location**: Acceptance Criteria
  ruff and shell runners honour configured excludes, making "all" effectively
  config-defined rather than filesystem-defined — legitimate excludes and
  scope-gaming become indistinguishable.

#### Suggestions

- 🔵 **Clarity / Dependency** (merged): No decision rule for pyrefly 'all' preset superseding 'strict'
  **Location**: Technical Notes
  The note that the v1.1 `all` preset "may supersede `strict` here" names no
  decider and no precedence rule; the roadmap watch-point is also captured
  nowhere that survives this item's closure.

- 🔵 **Scope**: No sizing contingency if the remediation sweep proves much larger than expected
  **Location**: Assumptions
  0090 models the better pattern — an explicit re-scope-to-epic contingency.
  Without one, a ballooning sweep forces an ad-hoc mid-flight re-scope or
  pressure to weaken the strict rule sets.

- 🔵 **Clarity**: Clippy 'short curated override list' — direction and content unspecified
  **Location**: Requirements
  The sentence never states whether overrides allow, downgrade, or deny lints,
  and "short" is unbounded — opposite strictness outcomes are readable from
  the same sentence.

### Strengths

- ✅ "Strict from day one" is explicitly translated into concrete per-tool
  configurations in Assumptions, defusing the most ambiguity-prone term in
  the work item and flagging the scope consequence if a harder reading was
  intended (noted by clarity, scope, testability).
- ✅ Acceptance criteria are phrased as exact, runnable commands with binary
  exit-status semantics (`cargo clippy --all-targets --all-features -- -D
  warnings`, `ruff format --check`, `shfmt -d`) — a verifier can execute them
  directly (noted by clarity, testability).
- ✅ Open Questions are explicitly closed with a resolution date and pointer
  to Drafting Notes, which record each decision and even flag
  reviewer-flippable choices (ruff `ALL` vs Astral's curated select) —
  preserving the decision trail rather than silently deleting it (noted by
  clarity, completeness, testability).
- ✅ All expected sections are present and substantively populated, with
  complete, well-formed frontmatter; Context goes beyond restating the
  Summary with a concrete per-area inventory (completeness).
- ✅ Tool-specific jargon and coupling caveats (Biome domains, pyrefly
  presets, shfmt's .editorconfig-only-without-flags behaviour, ruff's
  .editorconfig gap) are consistently explained in Technical Notes and backed
  by References (clarity, dependency).
- ✅ Out-of-scope boundaries are explicit where they matter most: CI-only
  enforcement with pre-commit hooks excluded; stylelint dropped with its
  underpinning plain-CSS assumption on record (scope, dependency).
- ✅ The version-pinning criterion directly converts the ruff pre-1.0 drift
  coupling into an enforced constraint, and the config-vs-mechanical-commit
  separation makes the sweep's internal sequencing visible (dependency).

### Recommended Changes

1. **Strengthen the acceptance criteria to verify the mandated strictness, not just clean passes** (addresses: Pass-cleanly criteria don't verify configured strictness; Clippy curated override list unspecified)
   Add criteria pinning config content — e.g. "the Cargo `[lints.clippy]`
   table sets pedantic to warn with `priority = -1`", "every config-level
   ignore and inline suppression carries a comment naming the rule and
   rationale" — and state the direction of the clippy override list (allows
   or downgrades of named impractical pedantic lints).

2. **Align AC1 with the frontend requirement** (addresses: Biome domain list differs between Requirements and AC1)
   Amend AC1 to "react + test + project domains" or restate it as "all
   domains listed in Requirements are enabled in the Biome config" — or, if
   `project` was deliberately dropped, update the requirement and record why
   in Drafting Notes.

3. **Close the file-type scope boundary** (addresses: Open-ended 'other file types where it makes sense')
   Either drop the open-ended phrase from the Summary so the Requirements
   list is authoritative, or add an explicit out-of-scope statement naming
   the considered-and-excluded file types (YAML, Markdown, TOML, JSON,
   actionlint).

4. **Record the delivery shape for the sweep — or restructure it** (addresses: Single task bundles four-plus efforts; Uncaptured coordination constraint; No sizing contingency)
   If the keep-unsplit decision stands, at minimum: (a) decouple per-language
   CI jobs so each becomes blocking as its sweep completes rather than
   all-at-once, (b) add a Dependencies entry capturing the ordering
   constraint against in-flight work, and (c) borrow 0090's sizing
   contingency ("if remediation volume exceeds expectations, re-scope to
   per-language children with guardrail config landing first"). Otherwise
   re-shape as an epic with one child per language domain.

5. **Populate the Dependencies section** (addresses: Dependencies placeholder contradicted by References; mise registry coupling; pyrefly 'all' preset decision rule)
   Move the 0090 relation into Dependencies, add "External: mise registry
   must provide pinned versions of biome, ruff, pyrefly, shellcheck, shfmt"
   with a fallback (cargo:/npm:/pipx: backends), and record the pyrefly v1.1
   watch-point with an explicit precedence rule (e.g. "use `strict` for this
   item; evaluate `all` in a follow-up").

6. **Define the mise task mapping for type-checks** (addresses: Ambiguous mapping of type-checks to lint tasks)
   State which commands each named `lint:*` task runs (or that type-checks
   get their own tasks) and define AC5's "full suite" by reference to that
   mapping.

7. **Define pinning semantics and ownership** (addresses: Version-pinning criterion underspecified)
   Name, per tool, the manifest that owns its version; define "pinned" as
   exact versions with no range operators plus committed lockfiles; reconcile
   the "mise installs all of these tools" note with the uv/package.json
   mentions.

8. **Tighten the remaining verification gaps** (addresses: 'Keep in sync' has no actor; CI falsification procedure; file-discovery scope; kind undersells)
   State the .editorconfig sync mechanism (even if "manual convention, no
   enforcement"); add a 0090-style falsification probe ("introducing one
   representative violation per language causes the corresponding CI job to
   exit non-zero"); define file discovery as "every `.py`/`.sh` file under
   version control except <enumerated directories>"; and consider re-kinding
   to story if the unsplit decision stands.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is unusually precise for a tooling task: 'strict' is explicitly mapped to concrete per-tool configurations, jargon is well-referenced, and resolved open questions are traceable to dated decisions in Drafting Notes. The main clarity issues are cross-section inconsistencies — the Summary's open-ended 'other file types where it makes sense' versus the closed Requirements enumeration, and a Biome domain list that differs between Requirements (react, test, project) and AC1 (react + test). A handful of smaller ambiguities concern where type-checks live relative to the named lint tasks, the actorless '.editorconfig keep in sync' instruction, and which manifest authoritatively pins each tool's version.

**Strengths**:
- 'Strict from day one' is explicitly translated into concrete per-tool configurations in the Assumptions section, defusing the most ambiguity-prone term in the work item and flagging that scope grows materially if a harder interpretation was intended
- Open Questions are explicitly closed with a resolution date and a pointer to Drafting Notes, which record each decision and even flag reviewer-flippable choices (ruff ALL-plus-ignores versus Astral's curated-select guidance)
- Tool-specific jargon (Biome domains, pyrefly presets, shfmt's .editorconfig behaviour, clippy lint tables) is consistently explained in Technical Notes and backed by References links, so a competent developer new to the team can follow every term
- Acceptance criteria are phrased as exact commands with observable pass/fail outcomes (e.g. `cargo clippy --all-targets --all-features -- -D warnings`), leaving little interpretive room about what 'passing' means

**Findings**:

- 🟡 major (high confidence) — **Biome domain list differs between Requirements and AC1** (Acceptance Criteria)
  The Frontend requirement enables three Biome domains — `react`, `test`, and `project` — but acceptance criterion 1 verifies `biome ci` with only "react + test domains", silently dropping `project`. The two sections enumerate different configurations and a reader cannot tell which is intended.
  **Impact**: The `project` domain governs Biome v2's type-aware, multi-file analysis (which the work item's own Technical Notes highlight as a headline v2 capability), so the two readings produce materially different lint coverage and CI cost, and a verifier following AC1 literally would accept a configuration that violates the requirement.
  **Suggestion**: Align the two lists — either add `project` to AC1's parenthetical, or record in Drafting Notes why the project domain was deliberately excluded from verification.

- 🟡 major (medium confidence) — **Open-ended 'other file types where it makes sense' never resolved by Requirements** (Summary)
  The Summary scopes the work to the named surfaces "plus the wider codebase (Python, shell scripts, and other file types where it makes sense)", but the Requirements enumerate a closed set (frontend TS, Rust, Python, shell, CSS) and never state whether that enumeration is the final answer to "where it makes sense". File types such as Markdown, YAML, TOML, or JSON are neither included nor explicitly excluded anywhere in the work item.
  **Impact**: An implementer and a reviewer can reasonably hold different views on whether additional file-type tooling is in scope, leading either to unplanned scope growth or to a 'done' dispute when only the enumerated five surfaces are covered.
  **Suggestion**: Either state explicitly that the Requirements enumerate the complete set of covered file types, or add an out-of-scope note recording which other file types were considered and excluded.

- 🔵 minor (medium confidence) — **Ambiguous mapping of type-checks to the named lint tasks and AC5's 'full suite'** (Requirements)
  The Entry point requirement names four subtasks (`lint:frontend`, `lint:rust`, `lint:python`, `lint:shell`) plus an aggregate `lint`, while the Frontend requirement separately mandates `tsc --noEmit` "as a standalone type-check" and the Python requirement includes pyrefly. Acceptance criterion 5 then says `mise run lint` "runs the full suite", leaving it ambiguous whether the type-checks belong inside the named lint tasks, get their own tasks, and whether they are part of the aggregate's "full suite".
  **Impact**: If the type-checks sit outside the `lint` aggregate, `mise run lint` exiting zero does not mean what AC5's "full suite" implies, and AC6's "same checks" CI wiring inherits the same ambiguity.
  **Suggestion**: State which commands each named task runs (or that type-checks get their own tasks), and define "the full suite" by reference to that mapping.

- 🔵 minor (high confidence) — **'Keep in sync' has no actor, trigger, or observable state** (Requirements)
  The `.editorconfig` requirement instructs, for tools that don't read it (ruff, rustfmt), to "duplicate line-length/indent settings in their config and keep in sync" — but names no actor, trigger, or mechanism for the ongoing synchronisation. It is a desired property rather than an observable state, and no other section clarifies how sync is maintained or detected.
  **Impact**: Without a named mechanism, drift between `.editorconfig` and the duplicated settings will go unnoticed, and an implementer cannot tell whether a sync check is expected as part of this work.
  **Suggestion**: State the intended mechanism — e.g. a cross-referencing comment in each config, an automated consistency check, or an explicit statement that sync is by manual convention with no enforcement.

- 🔵 minor (medium confidence) — **Pinning locations (mise/uv/package.json) in tension with 'mise installs all of these tools'** (Acceptance Criteria)
  Acceptance criterion 7 requires tool versions pinned "(mise/uv/package.json)", while Technical Notes state "mise installs all of these tools from its registry". If mise installs everything, the role of uv and package.json pins is unclear — and the work item never says which manifest authoritatively owns each tool's version, or which wins if two pin the same tool.
  **Impact**: An implementer could pin a tool in one manifest while CI resolves it from another, defeating the criterion's stated purpose of preventing rule-set drift on upgrade.
  **Suggestion**: Name, per tool, the manifest that owns its version (or rephrase AC7 to require a single authoritative pin per tool), and reconcile the Technical Note if some tools are intentionally installed outside mise.

- 🔵 suggestion (medium confidence) — **Clippy 'short curated override list' — direction and content unspecified** (Requirements)
  The Rust requirement calls for "a short curated override list" in the `[lints.clippy]` table without saying what the overrides do — allow specific impractical pedantic lints, downgrade them, or deny additional ones — or what bounds "short". The Assumptions section implies the intent (pedantic warnings promoted to CI failures with curated exceptions) but never states the override direction.
  **Impact**: Minor interpretive latitude, but the direction matters for the strict-from-day-one mandate — an implementer allowing lints versus denying extras produces opposite strictness outcomes from the same sentence.
  **Suggestion**: Add one clause stating the overrides are allows/downgrades of named impractical pedantic lints, ideally each accompanied by a justification comment.

- 🔵 suggestion (medium confidence) — **No decision rule for pyrefly 'all' preset superseding 'strict'** (Technical Notes)
  Technical Notes say pyrefly's forthcoming `all` preset "is slated for v1.1 and may supersede `strict` here", but no decision rule states which preset governs if `all` ships before implementation — the Requirements mandate `strict` while the note hints at something stricter without naming who decides.
  **Impact**: An implementer encountering a released `all` preset cannot tell whether adopting it honours or violates the spec, and the Assumptions section warns that a harder strictness reading grows scope materially.
  **Suggestion**: State precedence explicitly — e.g. "use `strict` for this work item; evaluate `all` in a follow-up" or "adopt `all` if stable at implementation time".

### Completeness

**Summary**: Work item 0098 is structurally complete and unusually dense for a task: every expected section is present and substantively populated, the frontmatter is intact with a recognised kind and status, and the Requirements give per-tool configuration detail concrete enough to start implementation. The Acceptance Criteria section contains eight specific criteria that map closely to the requirements. Remaining gaps are minor: one unresolved scope phrase in the Summary, a placeholder Dependencies entry that contradicts the References section, and one acceptance criterion that drops a detail its corresponding requirement mandates.

**Strengths**:
- All expected sections (Summary, Context, Requirements, Acceptance Criteria, Open Questions, Dependencies, Assumptions, Technical Notes, Drafting Notes, References) are present and substantively populated — none are empty or placeholder-only except the Dependencies entry noted in findings
- Frontmatter is complete and well-formed: kind (task), status (draft), priority, tags, dates, author, and schema_version are all present with recognised values
- Requirements are exceptionally specific for a task — exact tools, versions, flags, configuration locations, CI invocation forms, and mise task names — an implementer could start without clarification on the covered languages
- The Open Questions section records that the four original questions were resolved and when, rather than silently deleting them, preserving the decision trail alongside Drafting Notes
- The Assumptions section explicitly documents how the ambiguous mandate 'strict from day one' was interpreted per tool, and flags the scope consequence if the interpretation is wrong
- Context explains the motivating problem (polyglot repo with no consistent guardrails) with a concrete per-area inventory, going well beyond restating the summary

**Findings**:

- 🔵 minor (medium confidence) — **Summary scope phrase 'other file types where it makes sense' is never resolved by Requirements** (Summary)
  The Summary scopes the work to the frontend, server, Python, shell, 'and other file types where it makes sense', but the Requirements section only resolves frontend (TS/React), Rust, Python, shell, CSS, and .editorconfig — leaving 'other file types' (e.g. YAML, JSON, TOML, Markdown) neither included nor explicitly ruled out anywhere in the work item.
  **Impact**: An implementer cannot tell whether the task is complete once the five enumerated areas pass, or whether they are expected to judge which additional file types 'make sense' — a question that would need to go back to the author.
  **Suggestion**: Either tighten the Summary to the enumerated languages, or add a Requirements bullet (or Assumptions entry) stating that other file types are explicitly out of scope for this task.

- 🔵 minor (high confidence) — **Dependencies section is placeholder text contradicted by the References section** (Dependencies)
  The Dependencies section contains only the placeholder phrasing 'Related: none captured yet', yet the References section explicitly captures a relation — '0090 (done) — tangential: a narrow CI gate for radius tokens that could consume the lint infrastructure this item establishes'. The two sections contradict each other, and the frontmatter carries no relates_to entry either.
  **Impact**: A reader checking Dependencies — the canonical place for relations — is told none exist, and the 'not yet' phrasing reads as an unfinished drafting stub on an item whose Open Questions are otherwise all resolved.
  **Suggestion**: Move the 0090 relation into the Dependencies section (and frontmatter relates_to if the schema supports it), or replace the placeholder with a definitive statement that no blocking or related dependencies exist.

- 🔵 minor (medium confidence) — **AC1 omits the 'project' domain mandated by the frontend requirement** (Acceptance Criteria)
  The frontend requirement mandates Biome with 'all applicable domains enabled (react, test, project)', but the corresponding acceptance criterion (AC1) only checks 'biome ci (strict config, react + test domains)' — the project domain is absent from the done-check.
  **Impact**: The work could be signed off as done with AC1 satisfied while a stated requirement (the project domain) goes unimplemented and unverified.
  **Suggestion**: Amend AC1 to read 'react + test + project domains' (or, if project was deliberately dropped, update the Requirements bullet and note the change in Drafting Notes).

### Dependency

**Summary**: Work item 0098 is a self-contained tooling task whose external couplings are richly described in Requirements and Technical Notes but almost entirely absent from the Dependencies section, which reads 'Related: none captured yet' despite the document itself naming a related work item (0090) and a load-bearing reliance on the mise registry. The most significant uncaptured coupling is the repo-wide fix-everything mandate, which implies mass mechanical commits across ~700 Python files and ~160 shell scripts that will collide with any concurrently in-flight work, with no sequencing or coordination constraint recorded anywhere.

**Strengths**:
- Internal ordering within the task is explicitly captured: Technical Notes recommends config commits distinct from per-language mechanical auto-fix commits, making the sweep's internal sequencing visible to the implementer
- The version-pinning acceptance criterion directly addresses the coupling to fast-moving external tools (ruff pre-1.0 with `ALL` implicitly enabling new rules on upgrade), turning a latent drift dependency into an enforced constraint
- Tool-level coupling caveats are documented in detail — shfmt only honouring .editorconfig when no style flags are passed, ruff/rustfmt not reading .editorconfig and needing duplicated settings — so the cross-tool configuration couplings are visible even though they live in Requirements/Technical Notes rather than Dependencies
- The stylelint drop is explicitly tied to its underpinning assumption (plain CSS, no SCSS), so the conditional coupling between the CSS footprint and the tool choice is on record in Assumptions

**Findings**:

- 🟡 major (medium confidence) — **Repo-wide auto-fix sweep has uncaptured coordination constraint with in-flight work** (Acceptance Criteria)
  The requirement to "fix all existing issues so every configured tool passes cleanly across the entire repository" and the acceptance criterion "no pre-existing violations remain anywhere in the repository at the time of merge" imply mass mechanical reformatting commits touching ~700 Python files, ~160 shell scripts, plus the frontend and server — yet the Dependencies section records no ordering or coordination constraint relative to concurrently in-flight work items, whose branches/workspaces will conflict with such a sweep.
  **Impact**: A repo-wide reformat landed without sequencing against in-flight work causes widespread rebase conflicts and planning disruption; this is exactly the kind of hidden "you can't start/merge until X" constraint the Dependencies section exists to surface.
  **Suggestion**: Add a Dependencies entry capturing the coordination constraint — e.g. "Ordering: the per-language auto-fix sweep commits should land when no (or minimal) work is in flight in the affected language areas, or in-flight items must rebase across the sweep" — and name any currently in-flight items that must merge first if known.

- 🔵 minor (high confidence) — **Dependencies section says 'none captured yet' while References names a related work item** (Dependencies)
  The Dependencies section states "Related: none captured yet", but the References section explicitly names a related work item — "0090 (done) — tangential: a narrow CI gate for radius tokens that could consume the lint infrastructure this item establishes" — so a known coupling is recorded in the wrong place and the canonical dependency record contradicts the document body.
  **Impact**: Tooling and reviewers that read the Dependencies section (or frontmatter relates_to) will conclude this item has no captured relationships, and the potential follow-on of folding 0090's standing radius grep gate into the lint infrastructure this item establishes is invisible to planning.
  **Suggestion**: Move the 0090 relationship into the Dependencies section as a Related entry (and frontmatter relates_to if used), noting the possible follow-on of consolidating 0090's CI grep gate into the new lint job structure.

- 🔵 minor (high confidence) — **External tool-availability coupling on the mise registry not captured as a dependency** (Dependencies)
  The work item's CI design rests on the Technical Notes claim that "mise installs all of these tools from its registry — CI setup stays a single jdx/mise-action step", making mise-registry availability (and version currency) for Biome 2.4.x, ruff 0.15.x, pyrefly 1.0, shellcheck, and shfmt an external coupling — but none of these tool dependencies appear in the Dependencies section, which is empty.
  **Impact**: If any tool (pyrefly 1.0 in particular, stable only since May 2026) is absent or lags in the mise registry, the single-step CI assumption breaks and the work stalls on an unrecorded external blocker discovered at implementation time.
  **Suggestion**: Record the external tool dependencies in the Dependencies section — at minimum "External: mise registry must provide pinned versions of biome, ruff, pyrefly, shellcheck, shfmt" — with a fallback noted (e.g. mise backends such as cargo:/npm:/pipx:) if a tool is missing from the registry.

- 🔵 suggestion (medium confidence) — **Vendor roadmap coupling on pyrefly v1.1 'all' preset left as a buried note** (Technical Notes)
  Technical Notes records that pyrefly's "all" preset is "slated for v1.1 and may supersede strict here" — a coupling to the vendor's roadmap that implies follow-on reconfiguration work, but it is captured only as an aside rather than as a tracked relationship or follow-on item.
  **Impact**: When pyrefly v1.1 ships, the intent to potentially adopt the harder preset will be invisible outside this item's Technical Notes, so the upgrade decision may be missed.
  **Suggestion**: Either note the pyrefly v1.1 watch-point under Dependencies (e.g. "External: pyrefly roadmap — re-evaluate strict vs all preset at v1.1") or raise a small follow-on item at completion so the roadmap coupling survives this item's closure.

### Scope

**Summary**: This task bundles four-plus independently deliverable per-language toolchain efforts (Biome/tsc, rustfmt/clippy, ruff/pyrefly, shellcheck/shfmt) plus a full-repository remediation sweep, mise task wiring, and CI integration into a single work item declared as a 'task'. The bundling is conscious — Drafting Notes record an explicit author decision to keep it unsplit — but the decision is recorded without any contingency for the case where remediation volume (ruff ALL + pyrefly strict over ~700 Python files; shellcheck --enable=all over ~160 scripts) proves very large, which is where the real delivery risk sits. Scope boundaries are otherwise well-articulated: enforcement boundaries (CI-only, no pre-commit), tool substitutions (Biome subsumes stylelint), and per-tool 'strict' mappings are all explicit.

**Strengths**:
- Scoping decisions are explicitly recorded rather than implicit: the Drafting Notes document that the single-task-vs-epic question was raised and resolved by the author on 2026-06-09, and the Assumptions section restates the unsplit decision — reviewers can see the bundle is deliberate, not accidental
- Out-of-scope boundaries are clearly stated where they matter most: CI-only enforcement with pre-commit hooks explicitly excluded, and stylelint explicitly dropped because Biome covers CSS — avoiding silent scope creep into overlapping tooling
- The open-ended 'strict from day one' mandate is bounded per-tool in Assumptions (Biome recommended+domains+warnings-as-errors, clippy pedantic-at-warn promoted in CI, ruff ALL minus documented ignores, pyrefly strict preset), with an explicit warning that a harder reading of 'strict' would grow scope materially
- Summary, Requirements, and Acceptance Criteria describe the same two-fold scope (durable guardrails + clean-slate remediation) consistently — each Requirements bullet has a matching AC gate

**Findings**:

- 🟡 major (medium confidence) — **Single task bundles four-plus independently deliverable per-language efforts plus a full-repo remediation sweep** (Requirements)
  The Requirements enumerate five independent toolchain workstreams (frontend Biome+tsc, Rust rustfmt+clippy, Python ruff+pyrefly, shell shellcheck+shfmt, CSS via Biome), each with its own config, its own remediation sweep, and its own blocking CI job — and each could be completed, merged, and rolled back without affecting the others. The only genuinely shared work is the aggregate `lint` mise task and the CI workflow wiring. The acceptance criterion that 'no pre-existing violations remain anywhere in the repository at the time of merge' implies a single merge point spanning ~700 Python files and ~160 shell scripts under maximally strict rule sets (pyrefly `strict` on a previously untype-checked codebase is plausibly weeks of remediation by itself), creating a long-lived, conflict-prone integration. The Drafting Notes record an explicit author decision to keep this unsplit, which is why this is rated medium confidence rather than high — but the recorded decision does not remove the delivery risk it accepts.
  **Impact**: A single all-languages merge gates every guardrail on the slowest remediation stream; a stall in (say) pyrefly-strict remediation delays Biome, clippy, and shellcheck enforcement that could otherwise land and start paying off immediately, and the mega-diff invites merge conflicts and review fatigue.
  **Suggestion**: Re-shape as an epic (or a small sequence of sibling tasks) with one child per language domain — config + remediation + blocking CI job landing together per language — plus a final child for the aggregate `mise run lint` task and documentation. If the unsplit decision stands, at minimum decouple the per-language CI jobs so each can become blocking as its sweep completes rather than all-at-once.

- 🔵 minor (medium confidence) — **Declared kind 'task' undersells a repo-wide, multi-toolchain effort** (Frontmatter: kind)
  The work item is declared `kind: task`, but it describes repo-wide tooling adoption across five language domains, a full-repository violation remediation sweep, new mise task infrastructure, and CI workflow changes — scope that, even kept unsplit, is epic- or at least story-shaped rather than task-shaped.
  **Impact**: The kind label drives planning expectations; treating this as a task risks it being slotted into capacity sized for a small, bounded change when the remediation sweep alone may dominate a sprint.
  **Suggestion**: If the unsplit decision stands, re-kind to a story (or epic, per the bundling finding) so the label matches the breadth the Requirements describe.

- 🔵 minor (medium confidence) — **Open-ended 'other file types where it makes sense' boundary is silently narrowed by Requirements** (Summary)
  The Summary scopes the work to Python, shell, 'and other file types where it makes sense', but the Requirements resolve this to exactly five domains without ever stating that other plausible candidates (YAML, Markdown, TOML, JSON, GitHub Actions workflows via actionlint) are out of scope. The Context's 'other candidates' list only resolves CSS.
  **Impact**: An implementer or reviewer reading the Summary's open-ended phrase cannot tell whether the omission of, e.g., YAML or Markdown linting is a deliberate exclusion or a gap, inviting scope negotiation mid-implementation.
  **Suggestion**: Add an explicit out-of-scope statement (e.g. 'file types beyond TS/JS/CSS, Rust, Python, and shell are out of scope for this item') or drop the open-ended phrase from the Summary so the Requirements list is authoritative.

- 🔵 suggestion (medium confidence) — **No sizing contingency if the remediation sweep proves much larger than expected** (Assumptions)
  The Assumptions accept fixing all violations across ~700 Python files and ~160 shell scripts within this single task, but record no fallback if the violation count under ruff `ALL` + pyrefly `strict` + shellcheck `--enable=all` proves unmanageable. The referenced sibling item 0090 models the better pattern: it records an explicit sizing contingency ('if the inventory is unexpectedly large, re-scope to an epic with per-file-group children').
  **Impact**: Without a recorded contingency, a ballooning sweep forces an ad-hoc mid-flight re-scope with no agreed shape, or pressure to weaken the 'strict from day one' rule sets to get the merge through.
  **Suggestion**: Borrow 0090's framing — add a one-line contingency to Assumptions or Drafting Notes stating that if remediation volume exceeds expectations, the item re-scopes to per-language children with the guardrail config landing first in each.

### Testability

**Summary**: The Acceptance Criteria are unusually strong on mechanical verifiability — nearly every criterion names an exact command with binary exit-status semantics, and the Assumptions section pins the subjective 'strict from day one' mandate to concrete per-tool configurations. The main weakness is that the criteria verify only that tools *pass*, not that they pass *under the mandated strictness*: config-level ignores, lenient configs, and inline suppressions could satisfy every criterion while defeating the work item's intent. A few smaller gaps (a dropped Biome domain in AC1, no falsification probe for the CI gate, undefined 'pinned') would tighten verification further.

**Strengths**:
- Almost every criterion specifies an exact, runnable command (`cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `tsc --noEmit`, `ruff check`, `ruff format --check`, `shfmt -d`, `mise run lint`) with unambiguous exit-status pass/fail semantics — a verifier can execute the criteria directly.
- The Assumptions section translates the inherently subjective 'strict from day one' goal into concrete per-tool configurations (recommended+domains+warnings-as-errors for Biome, pedantic-at-warn for clippy, literal ALL for ruff, strict preset for pyrefly), substantially reducing interpretation room when verifying.
- The sweep scope is grounded in the Context section with rough per-language counts (~700 Python files, ~160 shell scripts), so 'across the entire repository' is filesystem-determined rather than open-ended.
- All four original Open Questions are resolved with decisions recorded in Drafting Notes, so no criterion's verification is blocked on an unresolved choice.
- Tool-pass framing is appropriate for the task kind — infrastructure guardrail work is correctly verified by command outcomes rather than forced into behavioural Given/When/Then.

**Findings**:

- 🟡 major (high confidence) — **Pass-cleanly criteria don't verify configured strictness and place no bound on suppressions** (Acceptance Criteria)
  Every per-language criterion verifies that a tool exits cleanly, but none verifies that the configuration it runs under matches the strictness the Requirements mandate, and none bounds suppression mechanisms. AC2's clippy command passes equally well with no `[lints.clippy]` pedantic config at all; AC3's `ruff check` passes with arbitrarily broad ignore lists ('targeted, documented ignores' has no threshold for 'targeted'); and AC8's 'no pre-existing violations remain' can be satisfied wholesale via inline suppressions (`# noqa`, `#[allow(...)]`, `// biome-ignore`, `# shellcheck disable=`) rather than fixes.
  **Impact**: The criteria can be argued as met regardless of implementation quality — a lenient config plus liberal suppressions passes every AC while delivering none of the intended guardrail strength, so verification provides no signal on the work item's core 'strict from day one' intent.
  **Suggestion**: Add criteria that pin the config content and bound the escape hatches, e.g. 'the Cargo `[lints.clippy]` table sets pedantic to warn with `priority = -1`', 'every config-level ignore and every inline suppression carries a comment naming the rule and rationale', and 'the count of inline suppressions added by the sweep is enumerated in the PR description'.

- 🔵 minor (high confidence) — **AC1 drops the `project` domain that Requirements mandate** (Acceptance Criteria)
  The Requirements specify Biome with 'all applicable domains enabled (`react`, `test`, `project`)', but AC1 verifies only '(strict config, react + test domains)' — the `project` domain is absent from the criterion. A verifier following AC1 literally would pass the work with the `project` domain disabled.
  **Impact**: A portion of the mandated lint coverage (Biome's project-level, type-aware rules) is unverified by any criterion, so it can silently go unimplemented.
  **Suggestion**: Amend AC1 to read 'react + test + project domains', or restate it as 'all domains listed in Requirements are enabled in the Biome config'.

- 🔵 minor (medium confidence) — **CI criterion 'fails the build on any violation' has no falsification procedure** (Acceptance Criteria)
  AC6 requires that Main CI 'fails the build on any violation', but no procedure is defined for confirming this — unlike referenced work item 0090, whose AC4 specifies an explicit probe ('inserting a literal radius ... causes the gate step to exit non-zero'). 'Blocking jobs' is also verifiable only by interpretation (required status checks vs. absence of `continue-on-error`).
  **Impact**: A verifier can confirm the jobs exist and run the right commands, but cannot conclusively confirm the fail-on-violation behaviour without a defined check, leaving the criterion partially trust-based.
  **Suggestion**: Add a falsification probe in the 0090 style — e.g. 'introducing one representative violation per language causes the corresponding CI job to exit non-zero' — or restate the criterion as a structural check: 'each lint job invokes the same mise task as AC5 and has no continue-on-error or allow-failure setting'.

- 🔵 minor (medium confidence) — **File-discovery scope for 'all Python' and 'all .sh scripts' is undefined, so excludes can silently narrow it** (Acceptance Criteria)
  AC3 ('across all Python') and AC4 ('all `.sh` scripts') do not define how files are discovered or whether exclusions (generated code, vendored files, workspace checkouts) are permitted. Since ruff and the shell-script runner honour configured excludes, a verifier running the named commands would silently respect any exclude list, making 'all' effectively config-defined rather than filesystem-defined.
  **Impact**: The scope of the sweep — and therefore what AC8's 'no pre-existing violations remain anywhere' means — cannot be conclusively verified, because legitimate excludes and scope-gaming are indistinguishable.
  **Suggestion**: State the discovery rule and the permitted exclusion set explicitly, e.g. 'every `.py`/`.sh` file under version control except <enumerated directories>, with any exclude entry in tool config justified by an adjacent comment'.

- 🔵 suggestion (medium confidence) — **'Pinned' versions criterion lacks a definition of pinning** (Acceptance Criteria)
  AC7 requires 'all tool versions are pinned (mise/uv/package.json)' but does not define what counts as pinned — an exact version, a caret/tilde range, a lockfile-resolved entry, or a major-version constraint could each be argued to satisfy it. The Technical Notes' rationale (ruff `ALL` enables new rules on upgrade) implies exact pinning is intended, but the criterion doesn't say so.
  **Impact**: A verifier could pass the criterion with range constraints that still allow rule-set drift — the exact failure mode the criterion exists to prevent.
  **Suggestion**: Define pinning explicitly, e.g. 'each tool is declared at an exact version (no range operators) in mise config / package.json, and lockfiles are committed', and enumerate the tools the criterion covers (biome, typescript, rust toolchain, ruff, pyrefly, shellcheck, shfmt).

## Re-Review (Pass 2) — 2026-06-09

**Verdict:** REVISE

All five lenses were re-run against the revised work item.

### Previously Identified Issues

- 🟡 **Testability**: Pass-cleanly criteria don't verify configured strictness / no bound on suppressions — **Resolved** (new config-strictness AC and suppression-policy AC; testability now rates strictness "checkable rather than arguable")
- 🟡 **Clarity/Completeness/Testability**: Biome domain list differs between Requirements and AC1 — **Resolved** (AC1 now verifies react + test + project)
- 🟡 **Clarity/Completeness/Scope**: Open-ended "other file types where it makes sense" — **Resolved** (Summary tightened; explicit out-of-scope bullet added; all lenses now cite the boundary as a strength)
- 🟡 **Scope**: Single task bundles four-plus per-language efforts plus full-repo sweep — **Still present** (re-flagged major/medium; the recorded keep-unsplit decision stands and mitigations are now documented, but the structural delivery risk remains)
- 🟡 **Dependency**: Sweep coordination constraint with in-flight work uncaptured — **Resolved** (Ordering entry added; residual suggestion about how affected in-flight items get identified)
- 🔵 **Completeness/Dependency**: Dependencies placeholder contradicted by References — **Resolved** (section populated; 0090 moved out of References; `relates_to` added)
- 🔵 **Clarity**: '.editorconfig keep in sync' has no actor — **Resolved** (manual-convention mechanism stated; new gap: the requirement has no covering AC)
- 🔵 **Dependency**: mise-registry coupling uncaptured — **Resolved** (External entry with backend fallback; new gap: npm supply leg for biome/typescript lacks a symmetric entry)
- 🔵 **Clarity**: Type-check-to-task mapping ambiguous — **Resolved** (Entry point now maps each task to exact commands; new nit: orphaned "full suite" definition)
- 🔵 **Clarity/Testability**: AC pinning underspecified — **Partially resolved** (exact-version semantics and per-tool ownership defined, but the enumeration omits the Rust toolchain — escalated to a new major)
- 🔵 **Scope**: Kind 'task' undersells the effort — **Still present** (deliberately not addressed, per the author's unsplit decision; recorded in Drafting Notes)
- 🔵 **Testability**: CI criterion lacked a falsification procedure — **Resolved** (per-language violation probe added; new gap: probe leaves no verifiable artefact)
- 🔵 **Testability**: File-discovery scope undefined — **Partially resolved** (version-controlled-files definition and workspaces/ exclusion added; coverage is still verified only via tool exit codes)
- 🔵 **Clarity/Dependency**: No decision rule for pyrefly 'all' preset — **Resolved** ('strict governs regardless' precedence stated; new gap: the committed follow-up isn't in the done-definition)
- 🔵 **Scope**: No sizing contingency — **Resolved** (re-scope-to-per-language-children contingency added to Assumptions)
- 🔵 **Clarity**: Clippy override direction unspecified — **Resolved** (allows of named impractical pedantic lints, each with justification comment)

### New Issues Introduced

- 🟡 **Clarity / Scope** (merged): Singular "sweep PR" and "at the time of merge" conflict with the staged-landing language added to the CI requirement and Dependencies Ordering entry — if delivery is staged, two ACs lose their referent exactly when the planned contingency is exercised
- 🟡 **Clarity**: "Every tool" pin criterion enumerates six tools but omits the Rust toolchain (rustfmt/clippy) — either toolchain pinning is silently out of scope (allowing the drift the AC exists to prevent) or its pin location is unspecified
- 🔵 **Clarity / Testability** (merged): Biome "warnings escalated" verification locus ambiguous — Requirements permit CLI flag or severity config, but the strictness AC checks "committed configs" only
- 🔵 **Testability**: AC2's clippy invocation (`--all-targets --all-features`) diverges from the `lint:rust` mapping ("clippy with `-D warnings`") — acceptance could pass against a command nothing durably enforces
- 🔵 **Testability**: The zero-violations AC is tautological given the per-tool ACs — it can never fail independently
- 🔵 **Testability**: The CI falsification probe requires no record (e.g. links to the failing probe runs), so acceptance takes the implementer's word
- 🔵 **Testability**: "All Python"/"all .sh" coverage is still verified only via tool exit codes — no check that the tools' file set matches `git ls-files` minus exclusions
- 🔵 **Testability**: The .editorconfig duplication requirement has no covering acceptance criterion
- 🔵 **Completeness**: The pyrefly follow-up commitment in Dependencies ("to be raised at completion") is not captured in any acceptance criterion
- 🔵 **Dependency**: npm supply leg for biome/typescript pins has no External entry symmetric to the mise-registry one
- 🔵 **Clarity**: CSS coverage scope ambiguous between repo-wide and frontend-only (latent pre-existing issue, newly surfaced)
- 🔵 **Clarity** (suggestion): Dangling definition — "the full suite" is defined in the Entry point bullet but no longer used anywhere
- 🔵 **Completeness** (suggestion): Documentation target for the lint entry point is unnamed
- 🔵 **Dependency** (suggestion): No mechanism stated for identifying in-flight items affected by each sweep

### Assessment

The Review 1 findings are substantially addressed: 11 of 16 resolved outright,
2 partially resolved, 2 standing by explicit author decision (the unsplit
bundle and the task kind), and the verdict-driving strictness-verification gap
is closed. However, the revision itself introduced a delivery-shape
inconsistency — staged-landing language in Requirements/Dependencies versus
single-PR/single-merge framing in the ACs — and a pin-enumeration gap (Rust
toolchain), which together with the standing scope finding keep the mechanical
verdict at REVISE (3 major findings ≥ threshold of 2). Both new majors are
small, targeted edits: state the delivery shape and make the affected ACs
well-defined under it, and name the Rust toolchain's pin location. Once those
land, the remaining findings are minor polish and the item is ready for
implementation planning — the scope major reflects an accepted, documented
author decision rather than an open defect.

## Re-Review (Pass 3) — 2026-06-09

**Verdict:** REVISE

All five lenses were re-run against the work item after the pass-2 fixes.

### Previously Identified Issues

- 🟡 **Clarity/Scope**: Singular "sweep PR"/"time of merge" vs staged-landing language — **Resolved** (Delivery shape bullet added; ACs rephrased to hold under either shape; both lenses now cite this as a strength)
- 🟡 **Clarity**: "Every tool" pin omits the Rust toolchain — **Resolved** (existing `mise.toml` pin named in the AC)
- 🔵 **Clarity/Testability**: Biome warnings-escalation locus — **Resolved** ("either committed locus counts")
- 🔵 **Testability**: AC2 clippy invocation diverged from `lint:rust` mapping — **Resolved** (mapping carries the full invocation)
- 🔵 **Testability**: Zero-violations AC tautological — **Resolved** (replaced by coverage AC + staged formulation; residual nit: Requirements and the coverage AC disagree on whether the `workspaces/` exclude itself needs a justification comment)
- 🔵 **Testability**: CI probe left no artefact — **Resolved** (probe runs linked from PR descriptions; superseded by a deeper finding on probe granularity, below)
- 🔵 **Testability**: Coverage verified only via exit codes — **Resolved** for ruff and the shell tools (new gap: pyrefly's file set is excluded from the cross-check, below)
- 🔵 **Testability**: .editorconfig requirement had no AC — **Resolved**
- 🔵 **Completeness**: pyrefly follow-up not in done-definition — **Resolved** (AC added; dependency lens cites the double capture as a strength)
- 🔵 **Dependency**: npm supply leg missing — **Resolved**
- 🔵 **Clarity**: CSS scope ambiguous — **Resolved** (frontend-only, explicitly)
- 🔵 **Clarity**: Dangling "full suite" definition — **Resolved**
- 🔵 **Completeness**: Documentation target unnamed — **Resolved** (`mise.toml` description fields; residual suggestion: the AC wording itself could name the locus)
- 🔵 **Dependency**: In-flight identification mechanism — **Resolved**
- 🟡 **Scope**: Single task bundles four per-language tracks — **Still present** (third consecutive pass; documented author decision)
- 🔵 **Scope**: Kind 'task' undersells — **Still present** (now folded into the bundling finding's suggestion)

### New Issues Introduced

- 🟡 **Testability**: CI falsification probe is per-language, not per-tool — each job runs 2–3 tools, so one probe violation leaves a silently mis-wired second tool (e.g. pyrefly checking no files, tsc missing from the task) unverified
- 🟡 **Testability**: No coverage cross-check for pyrefly's file set — the git-ls-files cross-check names only ruff and the shell tools; pyrefly does its own file discovery and could vacuously pass over unchecked files
- 🔵 **Clarity**: Requirements say every exclude entry needs a justification comment; the coverage AC exempts `workspaces/` — the two rules conflict for the item's only expected exclusion
- 🔵 **Clarity**: Unstated whether CI jobs invoke the `lint:*` mise tasks or duplicate their commands in workflow YAML
- 🔵 **Clarity/Completeness** (merged): `.shellcheckrc` role undefined — neither the Requirements nor the strictness AC says which locus carries `--enable=all`, and AC5 omits the shell config entirely
- 🔵 **Clarity**: 'Sweep' is used before being defined and shifts between commit-level and PR-level meanings, making the suppression-enumeration AC's coverage arguable
- 🔵 **Dependency**: GitHub Actions / `jdx/mise-action` is a third external supply channel absent from Dependencies
- 🔵 **Dependency** (low confidence): Whether a non-zero job blocks merges may depend on uncaptured branch-protection/required-check settings
- 🔵 **Scope**: The re-scope contingency has no trigger threshold (e.g. a report-only violation count before the first sweep PR)
- 🔵 **Testability**: Suppression-marker list omits pyrefly (`# pyrefly: ignore`, `# type: ignore`) and tsc (`// @ts-expect-error`) syntaxes
- 🔵 Suggestions: 'from day one' ambiguous under staged delivery; the 0090 gate-consolidation follow-on lacks the raise-or-decline mechanism its pyrefly sibling has; the entry-point documentation AC isn't self-sufficient

### Assessment

All fourteen addressable pass-2 findings are resolved, and the lenses are now
operating at a substantially finer grain than pass 1 — the work item's scope
boundaries, strictness definitions, and verification procedures are repeatedly
cited as strengths. The mechanical verdict remains REVISE (3 majors ≥
threshold 2), but its composition has changed: one major is the standing
scope-bundling finding, re-flagged on every pass and answered by a documented
author decision, and the two new majors are narrow verification-strength gaps
(per-tool probe granularity; pyrefly in the coverage cross-check), each a
one-line AC edit. Recommendation: apply those two edits plus the
workspaces-comment alignment, after which only the accepted scope finding
remains at major severity — below the two-major threshold — making the work
item eligible for COMMENT and ready for implementation planning.

### Closing Note — 2026-06-09

The pass-3 recommendation was applied the same day: the falsification probe
was tightened to per-check-command granularity, pyrefly was added to the
coverage cross-check, the exclude-comment rule was aligned, and the remaining
pass-3 minors and suggestions were swept (`.shellcheckrc` locus, CI jobs
executing the mise tasks, "sweep" defined, suppression markers made
open-ended, `jdx/mise-action` captured, contingency trigger checkpoint,
branch-protection assumption, 0090 follow-on disposition, "from day one"
anchored). With those fixes the only finding remaining at major severity is
the scope-bundling concern, which is answered by the documented author
decision to keep the item unsplit — below the two-major REVISE threshold.
The closing verdict is therefore recorded as **COMMENT**: the work item is
acceptable and ready for implementation planning; the standing scope
observation is noted for the record.

**Manual approval — 2026-06-09**: the author (Toby Clemson) manually
approved the work item, overriding the closing verdict from COMMENT to
APPROVE. The standing scope-bundling observation is accepted as a
documented decision; no findings remain open.
