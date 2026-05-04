---
date: "2026-05-05T01:00:00+00:00"
type: work-item-review
skill: review-work-item
target: "meta/work/0031-consolidate-accelerator-owned-files-under-accelerator.md"
work_item_id: "0031"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 5
status: complete
---

## Work Item Review: Consolidate Accelerator-Owned Files Under .accelerator/

**Verdict:** REVISE

The work item is exceptionally well-specified for its scope: it has a well-formed user story, thorough per-subsystem requirements with exact file paths and line numbers, and a comprehensive set of Given/When/Then acceptance criteria that cover the happy path, idempotency, edge cases, and migration guards. The main structural weakness is a tension in how the backwards-compatibility decision is surfaced — three lenses independently flagged that the Context section implies the source note's plan is being executed faithfully, while Requirements silently drops the backwards-compatibility window the note proposes. Two acceptance criteria also need tightening: the "every skill" criterion is unbounded, and the config-script criterion lacks an observable positive assertion.

### Cross-Cutting Themes

- **Backwards-compatibility window not explicitly surfaced as a departure** (flagged by: clarity, completeness, scope) — The 2026-04-29 source note proposes a backwards-compatibility window with a deprecation warning; the work item adopts a hard cut instead. The Context section says "This story executes that plan," which contradicts the Requirements. An implementer who reads the source note may introduce compatibility shims the story explicitly rejects, or be confused about which document governs. The hard-cut decision is correct and should be stated as a deliberate departure rather than implied by silence.

- **Acceptance criteria missing concrete observable outcomes** (flagged by: testability × 2) — Two criteria use unbounded or absence-of-failure language ("each skill completes without path-resolution errors", "resolves from .accelerator/ without error") rather than positive assertions that distinguish a correct result from a silent fallback or empty output.

### Findings

#### Major

- 🟡 **Clarity**: Context references a backwards-compatibility window that Requirements explicitly rejects
  **Location**: Context
  The Context section says "This story executes that plan" where the source note's plan includes a backwards-compatibility window. Requirements mandates a hard cut. A reader who reads the source note first will expect a compatibility fallback and may implement one.

- 🟡 **Dependency**: Already-shipped Jira skills are implied consumers not named in Dependencies
  **Location**: Dependencies
  The Jira skills already in the codebase (`init-jira`, `search-jira-issues`, `show-jira-issue`, `create-jira-issue`, `update-jira-issue`, `comment-jira-issue`, `attach-jira-issue`) all resolve state from `meta/integrations/jira/` and will break immediately at the hard-cut migration boundary. They are enumerated in Requirements prose but absent from the Dependencies section. A planner reading only Dependencies cannot see the coordination surface.

- 🟡 **Testability**: Unbounded "every skill" criterion is not verifiable as stated
  **Location**: Acceptance Criteria
  The final criterion — "when each skill in the plugin is invoked, then it completes without path-resolution errors" — has no defined skill list, invocation conditions, or definition of "path-resolution error". A verifier cannot know when this criterion is satisfied.

- 🟡 **Testability**: Config-scripts criterion underspecifies what a successful outcome looks like
  **Location**: Acceptance Criteria
  "Resolves from .accelerator/ without error" is an absence-of-failure assertion. A script that exits 0 but returns an empty string, or silently falls back to an old path, would pass as written. The criterion needs a positive observable output clause (e.g., "the returned path begins with `.accelerator/`").

#### Minor

- 🔵 **Clarity**: "Idempotent — does not overwrite if already present and correct" leaves "correct" undefined
  **Location**: Requirements — init-jira skill
  An implementer may implement idempotency as a file-existence test rather than a content-validity test, silently accepting a broken `.gitignore` from a previous failed run.

- 🔵 **Clarity**: "Removes vacated old paths after a successful move" is ambiguous when some sources are absent
  **Location**: Requirements — Migration 0003
  Step 3 moves paths "if the source exists", so some sources may be absent. It is unclear whether step 5 removes only the paths that were actually moved, or all known old paths regardless.

- 🔵 **Clarity**: "A single point of change" has an implicit subject
  **Location**: Requirements — integrations path-config key
  The referent of "single point of change" requires inference — it is the `config-read-path.sh` default, not the skill prose. The architectural intent is slightly obscured.

- 🔵 **Completeness**: Backwards-compatibility departure from source note not captured as an assumption
  **Location**: Assumptions
  The hard-cut decision is noted once in Drafting Notes but is not in Assumptions where an implementer would look for settled design choices. An implementer reading the source note without carefully reading the work item may make the wrong architectural choice.

- 🔵 **Dependency**: Migration framework dependency (ADR-0023) implied but not named
  **Location**: Dependencies
  Migration 0003's structural invariants (dirty-tree guard, idempotency check, numbering scheme) are governed by ADR-0023, cited explicitly in the Jira research document. ADR-0023 is not listed in References or Dependencies.

- 🔵 **Dependency**: `init-jira`'s soft ordering dependency on `accelerator:init` is not captured
  **Location**: Requirements — init-jira skill
  The "warns but does not fail if `.accelerator/` is absent" design establishes a soft ordering relationship between the two init skills. This is described in Requirements but absent from Dependencies, where it is relevant for documentation and user-journey planning.

- 🔵 **Scope**: Story type may understate the breadth of the change
  **Location**: Frontmatter: type
  Eight config scripts, three init skills, four visualiser lifecycle scripts, one migration script, and documentation all change simultaneously. The `story` type may cause planning surprises if teams use type to estimate effort.

- 🔵 **Scope**: Backwards-compatibility window silently dropped — departure not traced
  **Location**: Context
  The scope change from the reference document (dropping the compatibility window) is invisible to a reviewer reading only the work item. Drafting Notes says "confirmed in Step 1" without identifying what Step 1 refers to.

- 🔵 **Testability**: Visualiser criterion does not specify what "starts without path errors" means
  **Location**: Acceptance Criteria
  A server that starts but logs path warnings would pass or fail ambiguously. The criterion could be split into (1) the default argument value is verifiable by reading the script, and (2) the server exits 0 on startup.

- 🔵 **Testability**: Jira-skills path resolution criterion is partially tautological
  **Location**: Acceptance Criteria
  "Jira skills resolve integration state correctly" — "correctly" is undefined. A verifier can always argue the skills resolved correctly regardless of outcome.

#### Suggestions

- 🔵 **Clarity**: ADR parenthetical references are undefined on first appearance in the directory structure diagram
  **Location**: Requirements — Target directory structure
  Readers encounter `(ADR-0016)` etc. before reaching the References section. Low impact for readers familiar with the project's ADR convention.

- 🔵 **Clarity**: Line-number citations in Visualiser requirements may drift before implementation
  **Location**: Requirements — Visualiser
  Line numbers are not stable identifiers. If an earlier line is added or removed before implementation, cited lines will be wrong. A short content description alongside each citation would allow the change to be located by content if the line number has drifted.

### Strengths

- ✅ Actors are consistently named throughout Requirements — almost no passive construction obscures who does what.
- ✅ The target directory structure tree diagram is the single authoritative source of truth for file layout, annotated with prior path and ADR provenance.
- ✅ Acceptance Criteria use Given/When/Then form consistently; most name concrete observable system states rather than vague desired properties.
- ✅ Context explains the incremental origin of the problem, names the documents that motivated deferral, and states the hard-cut rationale — "why now" and "why no compatibility window" are answered.
- ✅ Requirements are unusually thorough: each subsystem (init, init-jira, paths.tmp, integrations key, config scripts, visualiser, docs, migration) gets its own named block with exact file paths, line numbers, and behavioural detail.
- ✅ Assumptions section captures three non-obvious decisions (site.json gitignore rationale, migration number confirmation, tmp inner-gitignore pattern) that would otherwise require implementers to consult multiple ADRs.
- ✅ Migration criteria are particularly well-specified: dirty-working-tree guard, idempotency of re-running, custom-path preservation, and .gitignore bootstrap all have discrete, verifiable outcomes.
- ✅ Technical Notes names the full config script list and notes the visualiser routing pattern, preventing common mistakes without bloating Requirements.
- ✅ The downstream coupling to future integrations (Linear, Shortcut) is correctly captured in Dependencies, explaining the motivating urgency for doing this now.
- ✅ All ADR references are paired with their file paths, so terms like "inner-gitignore pattern" can be verified without guessing.

### Recommended Changes

1. **Explicitly state the backwards-compatibility departure in Context** (addresses: Clarity major, Completeness minor, Scope minor)
   Add a sentence in Context noting that the backwards-compatibility window proposed in the source note was considered and rejected in favour of a hard cut, so the departure is explicit rather than requiring the reader to reconcile two documents. Optionally move this to Assumptions where implementers look for settled design choices.

2. **Add existing Jira skills to the Blocked-by or ordering note in Dependencies** (addresses: Dependency major)
   Name the seven already-shipped Jira skills as consumers that must have their path references updated as part of this story, or explicitly state that their updates are included in scope. A planner reading Dependencies should see the coordination surface.

3. **Tighten the "every skill" acceptance criterion** (addresses: Testability major)
   Either enumerate the specific skills that touch the relocated paths (the config-scripts list in Technical Notes effectively defines this set), or replace with a criterion scoped to config scripts: "when each config script is invoked with a valid fixture, then it exits 0 and its output path begins with `.accelerator/`".

4. **Add a positive assertion to the config-scripts criterion** (addresses: Testability major)
   Replace "resolves from .accelerator/ without error" with a positive observable output clause, e.g., "each script exits 0 and the returned path begins with `.accelerator/`", to distinguish correct resolution from silent fallback.

5. **Define "correct" in the init-jira idempotency requirement** (addresses: Clarity minor)
   Replace "if already present and correct" with "if already present and already ignores `site.json`" so the content-validity check is stated in the same sentence.

6. **Clarify step 5 of migration 0003 for partial-source cases** (addresses: Clarity minor)
   State whether old paths are removed individually ("each source path that was moved is then removed") or removed as a batch after all moves complete, to eliminate two equally defensible implementations.

7. **Add ADR-0023 to References** (addresses: Dependency minor)
   Add `Related: ADR-0023 (meta/decisions/ADR-0023-meta-directory-migration-framework.md)` alongside the existing ADR references, since migration 0003's structural invariants are governed by that framework.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is substantially clear and well-structured. Pronoun resolution is almost universally unambiguous, and actors are named throughout. One notable internal consistency tension exists: the Context section references a backwards-compatibility window from the source note that the Requirements section explicitly rejects. A small number of implicit referents and a few undefined terms create minor friction, but none rise to the level of genuine ambiguity about what must be built.

**Strengths**:
- Actors are consistently named throughout Requirements: "the init skill creates", "init-jira creates", "config-read-path.sh", "migration 0003". There is almost no passive construction that obscures who does what.
- The target directory structure tree diagram is the single authoritative source of truth for file layout and eliminates ambiguity about what files go where and what they were previously called.
- Acceptance Criteria use the Given/When/Then form consistently, and each criterion names a concrete observable system state as its outcome rather than a vague desired property.
- Cross-references to ADRs are paired with their file paths, so the meaning of terms like "inner-gitignore pattern" and "three-tier template resolution" can be verified without guessing.
- The hard-cut vs backwards-compatibility decision is stated once and clearly in the Requirements, removing any ambiguity about the migration's recovery model.

**Findings**:
- **major / high** — Context references a backwards-compatibility window that Requirements explicitly rejects (Location: Context)
- **minor / high** — "Idempotent — does not overwrite if already present and correct" leaves "correct" undefined (Location: Requirements — init-jira skill)
- **minor / medium** — "Removes vacated old paths after a successful move" is ambiguous when some source paths are absent (Location: Requirements — Migration 0003)
- **minor / medium** — "A single point of change" has an implicit subject (Location: Requirements — integrations path-config key)
- **suggestion / high** — ADR parenthetical references are undefined acronyms for readers without prior ADR context (Location: Requirements — Target directory structure)
- **suggestion / medium** — Line-number citations in Visualiser requirements may diverge from implementation reality (Location: Requirements — Visualiser)

### Completeness

**Summary**: Work item 0031 is exceptionally well-specified for a story of this scope. All required sections are present and substantively populated. One minor gap exists: the work item describes a deliberate departure from the backwards-compatibility strategy documented in the source note but does not surface this as an assumption, leaving implementers without guidance if they encounter the discrepancy in the reference material.

**Strengths**:
- Summary is a well-formed user story with a clear subject, a specific outcome, and three enumerable benefits.
- Context section explains the incremental origin of the problem, names the specific documents that motivated deferral, and states the hard-cut rationale explicitly.
- Requirements section is unusually thorough: each subsystem gets its own named block with exact file paths, line numbers, and behavioural detail.
- Acceptance Criteria are comprehensive and well-structured as Given/When/Then scenarios covering the full scope.
- Frontmatter is complete and consistent: type, status, priority, author, date, work_item_id, tags, and parent are all present and correctly valued.
- Assumptions section captures three non-obvious decisions (site.json gitignore rationale, migration number confirmation, tmp inner-gitignore pattern).
- Technical Notes section provides implementer-facing detail that doesn't belong in requirements but prevents common mistakes.
- References are explicit and scoped: each document is labelled (Source, Research, Related) and cross-referenced to the ADR it provides context for.

**Findings**:
- **minor / medium** — Backwards-compatibility departure from source note not captured as an assumption (Location: Assumptions)

### Dependency

**Summary**: The work item is well-structured from a dependency perspective: it explicitly states no upstream blockers, correctly captures the downstream relationship to future integrations, and the referenced ADRs and notes are accurately cited. However, the existing Jira skills are named consumers that will break at the hard-cut migration boundary but are not listed as blocked items; and the migration framework ADR (ADR-0023) is implied but not named.

**Strengths**:
- The Dependencies section explicitly and correctly declares no upstream blockers.
- The downstream coupling to future integrations (Linear, Shortcut) is correctly captured as a "Blocks" entry, explaining the motivating urgency.
- The work item references the Jira research document's explicit deferral of this reorg as separate work, correctly tracing the origin of the dependency relationship.
- The assumption that migration 0003 is the correct sequence number, with 0001 and 0002 confirmed to exist, is explicitly stated.
- The Jira auth resolution chain references config file paths accounted for in the Visualiser section and config scripts list.

**Findings**:
- **major / high** — Already-shipped Jira skills are implied consumers not named in Dependencies (Location: Dependencies)
- **minor / medium** — Migration framework dependency (ADR-0023) implied but not named (Location: Dependencies)
- **minor / medium** — init-jira's "run without accelerator:init" behaviour implies ordering that is not captured (Location: Requirements / init-jira skill)

### Scope

**Summary**: This story describes a single, coherent reorganisation objective and every requirement serves that one structural goal. The scope is large but genuinely indivisible: config scripts, migration, init skills, visualiser defaults, and documentation must change together for the cut to be clean. The declared type (story) is slightly undersized for the breadth of change described, but the work is a single cohesive structural refactoring with a clear boundary.

**Strengths**:
- All requirements are tightly coupled to the single goal of consolidating files under `.accelerator/`; none could be delivered independently without leaving the system in a broken intermediate state.
- The scope boundary is clearly articulated: the story explicitly names what is out of scope (future integrations, plugin extraction) and defers backwards-compatibility concerns with a deliberate hard-cut rationale.
- The Drafting Notes section explicitly accounts for scope expansion beyond the original note, providing full traceability for why the scope grew.
- Assumptions and Technical Notes pre-empt ambiguity about adjacent decisions that could otherwise cause scope creep during implementation.
- The work item references all relevant ADRs (0016–0020) that established the paths being moved.

**Findings**:
- **minor / medium** — Story type may understate the breadth of the change (Location: Frontmatter: type)
- **minor / medium** — Backwards-compatibility window from original note is silently dropped (Location: Context)

### Testability

**Summary**: The acceptance criteria are notably strong for a file-reorganisation work item: most are expressed as verifiable before/after states with explicit preconditions, subjects, and observable outcomes. The main gaps are one unbounded criterion, one underspecified outcome, and one tautological sub-clause. No criteria describe implementation details instead of outcomes, which is a genuine strength.

**Strengths**:
- The majority of criteria follow a clear Given/When/Then structure with concrete preconditions and observable outcomes.
- The migration criteria are particularly well-specified: the dirty-working-tree guard, idempotency, custom-path preservation, and .gitignore bootstrap step all have discrete, verifiable outcomes.
- The init-jira idempotency criterion explicitly identifies the precondition (directory already exists) and the expected outcome (contents and .gitignore left intact).
- The paths.tmp override criterion is a well-formed negative case: precondition, action, and non-action outcome are all specified.

**Findings**:
- **major / high** — Unbounded "every skill" criterion is not verifiable as stated (Location: Acceptance Criteria)
- **major / medium** — Config-scripts criterion underspecifies what a successful outcome looks like (Location: Acceptance Criteria)
- **minor / high** — Visualiser criterion does not specify what "starts without path errors" means (Location: Acceptance Criteria)
- **minor / high** — Jira-skills path resolution criterion is partially tautological (Location: Acceptance Criteria)

## Re-Review (Pass 2) — 2026-05-04

**Verdict:** REVISE

### Previously Identified Issues

- ✅ **Clarity**: Context references backwards-compatibility window that Requirements rejects — **Resolved** (Context now explicitly names the departure from the source note)
- ✅ **Clarity**: "correct" undefined in init-jira idempotency — **Resolved** (now "already ignores `site.json`")
- ✅ **Clarity**: Step 5 removal ambiguous when some sources absent — **Resolved** (now specifies per-moved-path removal after all moves complete)
- ✅ **Clarity**: "single point of change" implicit subject — **Resolved** (now explicitly names `config-read-path.sh`)
- ✅ **Completeness**: Backwards-compatibility departure not captured as assumption — **Resolved** (explicit in Context paragraph)
- ✅ **Dependency**: Jira skills not named as consumers in Dependencies — **Resolved** (Ordering entry now names all seven skills)
- ✅ **Dependency**: ADR-0023 not in References — **Resolved** (added)
- ✅ **Dependency**: init-jira soft ordering not captured — **Resolved** (Ordering note covers this)
- ✅ **Scope**: Story type understates breadth — **Resolved** (scope lens now confirms type is appropriate)
- ✅ **Scope**: Backwards-compatibility window silently dropped — **Resolved** (explicit in Context)
- ✅ **Testability**: Jira-skills path resolution tautological — **Resolved** (replaced with `config-read-path.sh integrations` assertion)
- ⚠️ **Testability**: Unbounded "every skill" criterion — **Still present** (scoping to Accelerator-configuration-reading skills still leaves the set open-ended; "does not reference paths in output" is vacuously true for silent-success skills)
- ⚠️ **Testability**: Config-script criterion underspecified — **Partially resolved** (exit-0 assertion added, but scripts that only return paths conditionally can trivially pass without preconditions)
- ⚠️ **Testability**: Visualiser "path errors" vague — **Still present** (reframed: "exits 0 on startup" is ambiguous for a long-running server process)

### New Issues Introduced

Adding ADR-0023 to References caused agents to read it, surfacing couplings the first pass missed.

- 🟡 **Dependency** (major/high): Migration discoverability hook (`hooks/migrate-discoverability.sh`, per ADR-0023) checks for `.claude/accelerator.md` and `meta/` as sentinels that indicate an Accelerator project. Migration 0003 removes both. On a fully-migrated repo, the hook's sentinel check will no longer trigger, silently suppressing all future migration warnings. Not mentioned in Requirements, Technical Notes, or Dependencies.
- 🔵 **Clarity** (minor/high): "ADR-0016 through ADR-0020" range notation silently omits ADR-0018 (init skill bootstrap), which governs the `init` skill's scaffold behaviour. ADR-0018 is not listed in References and its omission is unexplained.
- 🔵 **Clarity** (minor/medium): Dirty-tree guard in migration step 1 described as "standard" but ADR-0023's definition checks `.claude/accelerator*.md` and `meta/` — paths that migration 0003 removes. Guard scope for post-0003 migrations is left undefined.
- 🔵 **Clarity** (suggestion): The "already present and already ignores `site.json`" fix leaves the "file exists but does not yet ignore `site.json`" case unresolved — should init-jira append, overwrite, or warn?
- 🔵 **Completeness** (minor): `skills/config/migrate/SKILL.md` and the discoverability hook are not mentioned in Requirements; the note identifies both as in-scope for structural migrations.
- 🔵 **Dependency** (minor): `integrations` path-config key existence not confirmed — the source note describes it as a prerequisite potentially delivered in Jira Phase 1. Current state (exists or not) is unconfirmed.
- 🔵 **Dependency** (minor): Migration driver clean-tree check (`run-migrations.sh`) needs to cover `.accelerator/` post-migration; ADR-0023 currently defines it to check only old paths.
- 🔵 **Testability** (minor): Visualiser "exits 0 on startup" is ambiguous — a running server process has no exit code; likely intends "launch script exits 0" or "server passes a health check".
- 🔵 **Testability** (minor): "Directory contents and `.gitignore` are left intact" (init-jira idempotency) is not operationally defined.
- 🔵 **Testability** (minor): Jira state migration criterion doesn't handle the case where `site.json` was previously committed (gitignore rule will not un-track it).
- 🔵 **Scope** (minor): Visualiser changes are separable from the core path consolidation and could be deferred without breaking the hard-cut migration.
- 🔵 **Scope** (minor): init-jira ownership model introduces new behavioural conventions (standalone resilience, integration-init-owns-its-subtree pattern) beyond pure file relocation.

### Assessment

Pass 1 edits resolved 11 of 14 prior findings. Three testability findings and the discoverability hook gap persist or are newly surfaced. The discoverability hook is the most structurally significant: it is a safety mechanism that migration 0003 would silently disable on fully-migrated repos if its sentinel paths are not updated. The two testability findings that remain are solvable by adding per-script preconditions to the config-script criterion and replacing the final criterion with an explicit skill list and concrete observable outputs.

## Re-Review (Pass 3) — 2026-05-05

**Verdict:** REVISE

### Previously Identified Issues (Pass 2)

- ✅ **Clarity**: "ADR-0016 through ADR-0020" omits ADR-0018 — Resolved (explicit list)
- ✅ **Clarity**: Dirty-tree guard scope conflicts with ADR-0023 — Resolved (explicit paths in step 1)
- ✅ **Clarity**: Idempotency "present but incomplete" case unspecified — Resolved (append behaviour now stated)
- ✅ **Completeness**: Discoverability hook not in Requirements — Resolved (new Requirements section)
- ✅ **Dependency**: `integrations` key existence unconfirmed — Resolved (Dependencies note)
- ✅ **Dependency**: Migration driver clean-tree check not addressed — Resolved (Requirements section + Technical Notes)
- ✅ **Testability**: Visualiser "exits 0 on startup" ambiguous — Resolved (split into static + runtime criteria)
- ✅ **Testability**: "intact" not operationally defined — Resolved (three concrete assertions)
- ✅ **Testability**: site.json precondition missing — Resolved
- ✅ **Scope**: init-jira ownership model not surfaced — Resolved (Context note added)
- ⚠️ **Testability**: "every skill" criterion — Partially resolved (now names seven skills but Jira skills need live infra; fixture precondition missing)
- ⚠️ **Testability**: Config-scripts criterion — Partially resolved (exit-0 added; still conflates path-returning and multi-field scripts under one assertion)
- ⚠️ **Dependency**: Atomic delivery constraint for hook + driver in prose only — Still present

### New Issues Introduced (Pass 3)

- 🟡 **Clarity** (major/high): "Hard cut" used without definition — now appears twice without explanation of what it means operationally
- 🟡 **Dependency** (major/high): Discoverability hook and driver updates have an uncaptured atomic-delivery ordering constraint — prose says "ships with" but Dependencies section has no formal prerequisite entry
- 🟡 **Testability** (major/high): Jira skills criterion requires live Jira infrastructure to invoke; no fixture precondition specified
- 🟡 **Testability** (major/high): Config-scripts criterion assertion ("any path it returns") is ambiguous for multi-field output scripts (`config-common.sh`, `config-dump.sh`, `config-summary.sh`)
- 🔵 **Testability** (minor): No AC covers migration pre-flight guard failure path (dirty tree → exits non-zero, nothing moved)
- 🔵 **Testability** (minor): Visualiser criterion mixed static inspection with runtime check in single criterion
- 🔵 **Completeness** (minor): Migration driver clean-tree check update in Technical Notes only, not Requirements
- 🔵 **Dependency** (minor): Plugin-extraction interaction flagged in source note absent from work item
- 🔵 **Scope** (minor ×2): Story type breadth; `integrations` key separable — recurring observations

Note: one clarity major (ambiguous pronoun) was discarded — the cited sentence does not appear in the current work item and appears to have been hallucinated from the source note.

### Pass 3 Edits Applied

- Context: "hard cut" defined on first use ("immediate flag day: old paths are removed, no fallback")
- Migration step 6: same definition added
- Requirements — Migration discoverability hook section: strengthened to say hook + driver are prerequisites, not co-deliveries
- Dependencies: Ordering entry tightened to require config-script and Jira skill updates merged before migration is released; Atomic delivery entry added for hook + driver
- Acceptance Criteria: config-scripts split into two criteria by output type (single-path vs multi-field)
- Acceptance Criteria: Jira skills criterion reframed around path-resolution step, not full invocation, removing live-infra dependency
- Acceptance Criteria: migration pre-flight guard failure criterion added
- Acceptance Criteria: visualiser criterion split into static (default argument) and runtime (exits 0) criteria
- Open Questions: plugin-extraction interaction added

### Assessment

Pass 3 addressed all four confirmed major findings and the key minor gaps. The work item now has 19 acceptance criteria covering the full scope including the pre-flight guard failure path. The config-scripts and Jira skills criteria are now fixture-verifiable. The atomic-delivery ordering constraint is formally captured in Dependencies. No new structural concerns are expected on a fourth pass — remaining scope/type observations are recurring and the lens itself acknowledges they are low delivery risk.

## Re-Review (Pass 4) — 2026-05-05

**Verdict:** REVISE

### Previously Identified Issues (Pass 3)

- ✅ **Clarity**: "Hard cut" undefined — Resolved
- ✅ **Dependency**: Atomic delivery in prose only — Resolved (Dependencies section formalised)
- ✅ **Testability**: Jira skills required live Jira — Partially resolved (reframed around path-resolution step; new issue surfaced this pass)
- ✅ **Testability**: Config-scripts criterion conflated multi-field scripts — Resolved (split into two criteria by output type)
- ✅ **Testability**: Migration pre-flight guard had no AC — Resolved
- ✅ **Testability**: Visualiser criterion mixed static + runtime — Split into two (new issue surfaced with the static criterion this pass)
- ✅ **Completeness/Dependency**: Plugin-extraction gap — Added to Open Questions
- ✅ **Completeness**: Driver update in Technical Notes only — Resolved

### New Issues (Pass 4)

- 🟡 **Completeness** (major/high): Open question is unresolved and could invalidate the entire directory structure — no owner, no deadline, no blocking label
- 🟡 **Dependency** (major/high): "Blocked by: none" understates reality — Jira skill updates are a release gate, not merely an ordering note
- 🟡 **Testability** (major/high): `launch-server.sh` default-argument criterion uses static text inspection rather than runtime verification
- 🟡 **Testability** (major/high): Jira skills criterion uses implementation-detail language ("internal path resolution step") rather than an observable output; 7 skills bundled with no per-skill attribution
- 🔵 **Clarity** (minor × 3): "Both updates" referent ambiguous; backwards-compat "it" ambiguous; `.gitignore` "it" in idempotency rule
- 🔵 **Testability** (minor × 2): "existing project" fixture underspecified; "filesystem path value" undefined for multi-field scripts

Note: clarity finding about "ADF undefined" was discarded — `jira-md-to-adf.sh` does not appear in the work item; agent hallucinated it from the Jira research document.

### Pass 4 Edits Applied

- Context: Restructured backwards-compat sentence to make the rejected subject explicit ("was considered and rejected. Instead this story adopts…")
- Requirements — init-jira: `.gitignore` references made explicit throughout the idempotency rule, removing pronoun ambiguity
- Requirements — discoverability hook section: "Both updates" replaced with explicit named subjects
- Acceptance Criteria: "existing project" fixture enumerated (`.claude/accelerator.md`, `meta/.migrations-applied`, `meta/integrations/jira/fields.json`, `meta/tmp/`)
- Acceptance Criteria: multi-field config-scripts criterion qualified ("no line whose value is a filesystem path")
- Acceptance Criteria: visualiser two criteria merged into single runtime criterion with observable tmp-path assertion
- Acceptance Criteria: Jira skills criterion replaced with static grep-based check (no hardcoded `meta/integrations/jira/` in skill scripts)
- Dependencies: Restructured — "Blocked by: none (to start)" + "Release gate" entry for Jira skills + "Atomic delivery" entry for hook/driver, each with named scripts
- Open Questions: Marked BLOCKING with owner (Toby Clemson) and resolution instructions

### Assessment

Pass 4 addressed all four confirmed major findings. The open question is now explicitly blocking with an owner — it cannot be silently bypassed by an implementer picking up the story. The Jira skills criterion is now a static grep check, directly verifiable without live infrastructure or internal implementation knowledge. The Dependencies section now distinguishes work-can-start gates (none) from release gates (Jira skills) from atomicity constraints (hook + driver). No new major findings are expected on a fifth pass; remaining minor scope observations (story-type label) are recurring and the lens itself rates them low-risk.

## Re-Review (Pass 5) — 2026-05-05

**Verdict:** APPROVE

### Open Question Resolution

The blocking open question has been investigated and resolved. The 2026-03-14 plugin-extraction work (`meta/research/2026-03-14-plugin-extraction.md`, `meta/plans/2026-03-14-plugin-extraction.md`) addressed only the plugin's internal structure — extracting skills and agents from `~/.claude/` into `${CLAUDE_PLUGIN_ROOT}/`. It placed no constraints on the `.accelerator/` root name or file placement in user repos; the `meta/` convention was explicitly noted in that work as a project working-directory artifact separate from the plugin. The userspace directory model was established independently by ADR-0016 through ADR-0020.

### Previously Identified Issues (Pass 4)

- ✅ **Completeness**: Open question unresolved — **Resolved** (confirmed irrelevant; added to Assumptions with full rationale; Open Questions now reads "None")
- ✅ **Dependency**: "Blocked by: none" understated — **Resolved** (Pass 4 edits)
- ✅ **Testability**: Visualiser static-inspection criterion — **Resolved** (Pass 4 edits)
- ✅ **Testability**: Jira skills implementation-detail language — **Resolved** (Pass 4 edits)

### New Issues Introduced

None.

### Assessment

All major findings across five passes have been resolved. The work item has 20 verifiable acceptance criteria, a fully specified migration with a dirty-tree guard and idempotency path, explicitly named release-gate and atomic-delivery constraints, and no unresolved open questions. The story-type label (story vs epic) is the only recurring observation and the scope lens consistently rates it low delivery risk — no action required. The work item is ready for implementation.
