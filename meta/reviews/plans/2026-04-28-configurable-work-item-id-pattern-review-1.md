---
date: "2026-04-28T16:47:53Z"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-28-configurable-work-item-id-pattern.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, correctness, code-quality, test-coverage, safety, usability, compatibility, documentation]
review_pass: 2
status: complete
---

## Plan Review: Configurable Work-Item ID Pattern Implementation Plan

**Verdict:** REVISE

The plan is structurally strong — phases have clean dependencies, primitives are factored well, default-pattern preservation is treated as a first-class invariant, and tests/evals are taken seriously. The implementation will deliver a usable feature. However, the destructive Phase 6 migration carries several serious correctness and safety holes (double-prefixing on re-run, non-transactional rename, no dry-run, brittle fenced-code-block rewriting), and a handful of design decisions (width-tolerance regex, resolver classification ordering, slug-collision glob) silently break the invariants they claim to uphold. Multiple lenses also converge on the absence of shared abstractions for legacy-format detection and ID canonicalisation.

### Cross-Cutting Themes

- **Migration 0002 idempotency and recovery is fragile** (flagged by: correctness, safety, code-quality, test-coverage) — substring-style rewrites double-prefix on re-run; rename phase is not transactional; cross-reference rewrites can re-fire on already-rewritten files; no dry-run/preview mode; closed list of ID-bearing fields; no resume-after-partial-failure test.
- **Width-tolerance scan regex is internally inconsistent** (flagged by: architecture, correctness) — `[0-9]{N,}` won't match a 4-digit legacy file under a 5-digit pattern (the very scenario width-tolerance was meant to enable), and admits arbitrary-width matches that poison the overflow guard.
- **Resolver classification has unreachable branches and silent disambiguation** (flagged by: correctness, usability, code-quality) — step 4 is unreachable when a legacy file shares the number; bare-number-with-default-project resolves silently when ambiguity exists; classifier mixes structural classification with corpus probing.
- **Helper / shared-abstraction story is missing** (flagged by: architecture, code-quality) — legacy-format detection appears in resolver, list-work-items, and migration; "factor at ~30 lines" is asserted without naming a single helper interface.
- **Default-pattern "no behavioural change" claim is asserted, not tested** (flagged by: test-coverage, compatibility) — no golden-file regression check; eval threshold of 95% allows default-pattern evals to fail; `work_item_id` frontmatter quoting change is not byte-identical.
- **Discoverability of opt-out and mixed-corpus warnings is absent** (flagged by: usability, safety, documentation) — `--skip` not surfaced from migrate prompts; mixed-prefix corpus produces no warning; migration 0002 prerequisite error doesn't guide the user's choice.

### Tradeoff Analysis

- **Safety vs. plan scope**: Safety wants a dry-run/preview mode for migration 0002 and per-migration `--rerun` semantics; the plan keeps the migration framework minimally additive. Recommendation: add dry-run because the blast radius (every `meta/**/*.md`) justifies the surface-area cost.
- **Usability vs. quietness on default**: Usability wants disambiguation warnings when bare numbers could match multiple files; that adds noise to a previously silent path. Recommendation: warn only when ambiguity exists (zero noise on the single-match path).
- **Compatibility vs. consistency**: The plan changes `work_item_id` frontmatter to a quoted string for everyone (consistent), but that breaks the bit-identical default-pattern claim. Recommendation: branch on pattern shape — bare integer for default, quoted string for `{project}` patterns — or relax the bit-identical claim to behavioural equivalence and pin a fresh golden file.

### Findings

#### Critical

- 🔴 **Correctness**: Markdown-link rewrite is not idempotent — re-running double-prefixes already-rewritten paths
  **Location**: Phase 6: Migration 0002, step 6 (markdown link rewrites) and step 7 (idempotency claim)
  After the first run, a link `[foo](../work/PROJ-0042-foo.md)` still contains the substring `0042-foo.md`; on a second run, substring-style matching will fire on already-rewritten targets and produce `PROJ-PROJ-0042-foo.md`. Same hazard for fenced-code paths and frontmatter rewrites that use substring matching.

- 🔴 **Correctness**: Slug-collision glob `*-<slug>.md` rejects same-slug items across different projects
  **Location**: Phase 4: extract-work-items, change item 5 (slug-collision glob)
  Under a `{project}-{number:04d}` pattern, two source items with the same slug under different projects are legitimate (`PROJ-0001-add-foo.md`, `OTHER-0001-add-foo.md`), but the broadened glob rejects the second. The collision check is over-strict and contradicts the per-project model the rest of Phase 4 establishes.

#### Major

- 🟡 **Architecture**: Width-tolerance scan regex contradicts legacy-discoverability claim
  **Location**: Phase 1 — Width tolerance bullet
  `[0-9]{5,}` (after a width upgrade) won't match a 4-digit legacy file. The research (lines 281-282) actually specifies `[0-9]+`. The architectural promise of "legacy files visible across width changes" is broken; allocator may start counting from 0001 again.

- 🟡 **Architecture**: Legacy-format knowledge duplicated across resolver and list-work-items
  **Location**: Phase 2 (resolver step 3) and Phase 3 (list-work-items)
  Same legacy-ID concept implemented twice in different shapes. When the legacy convention evolves, two distinct sites must be updated in lockstep.

- 🟡 **Correctness**: Resolver step 4 unreachable when a legacy file shares the number
  **Location**: Phase 2: ID Resolver, classification order steps 2-4
  Under `{project}` pattern with `default_project_code: PROJ` and both `0042-legacy.md` and `PROJ-0042-current.md` present, typing `0042` silently resolves to legacy — but the user almost certainly meant the current file.

- 🟡 **Correctness**: Scan regex admits files outside the configured number space
  **Location**: Phase 1: scan-regex specification
  `^([0-9]{4,})-` matches `12345-foo.md`, contributing to `HIGHEST` and bricking the allocator with a misleading "archive completed work items" error.

- 🟡 **Correctness**: Marking migration 0002 applied on pattern-lacks-project locks out future runs
  **Location**: Phase 6, step 2
  If migrate runs while pattern is default, 0002 is recorded as applied. After a later config change to `{project}-...`, the migration is silently skipped. Users must hand-edit `.migrations-applied` to recover.

- 🟡 **Correctness**: Partial-failure re-run is not idempotent for cross-reference rewrites
  **Location**: Phase 6, step 7
  Cross-reference rewrites operate over `meta/**/*.md` regardless of which files have been processed. Combined with substring matching on `0042` and the markdown-link finding, partial-failure recovery double-prefixes.

- 🟡 **Correctness**: Test expectation contradicts documented classification flow
  **Location**: Phase 2: ID Resolver test list
  Test says "Path input that doesn't exist → exits 1" but the classification flow would produce exit 3 ("no match") unless path-shaped inputs are special-cased.

- 🟡 **Code Quality**: Compiler interface is illustrated, not specified
  **Location**: Phase 1, Section 1
  Regex flavour (BRE/ERE), capture-group index, project-value escape rules, exit-code shape are all unpinned. Each consumer will recreate small differences that survive unit tests.

- 🟡 **Code Quality**: Helper factoring promised but interfaces undefined
  **Location**: Implementation Approach + Phase 2 (resolver)
  No named helpers, no module proposed. Resolver, update-work-item, list-work-items all need pattern-aware canonicalisation. Risk: three inline implementations.

- 🟡 **Code Quality**: Migration 0002 is a god-script with high cognitive complexity
  **Location**: Phase 6, Section 1
  Nine distinct concerns in one bash script: config validation, pattern compilation, collision detection, atomic rename, frontmatter rewrite, mapping build, cross-ref rewrites across 7 fields and 3 reference shapes, idempotency. A six-month-later maintainer cannot map a failure to a single section.

- 🟡 **Code Quality**: Atomic write pattern referenced but not pinned to a single helper
  **Location**: Phase 5 + Phase 6
  Three new sites repeating the `cat → tmp → mv` idiom inline; classic source of corruption bugs that escape bash test harnesses.

- 🟡 **Code Quality**: Error messages quoted in plan but no testable contract
  **Location**: Phase 2 + Phase 6
  Either over-tight tests (asserting exact prose) or over-loose (substring greps). No stable error code/prefix mechanism.

- 🟡 **Code Quality**: Resolver classifier mixes structural classification with corpus probing
  **Location**: Phase 2, Section 2
  "Looks like a full ID but no file exists" silently reclassifies as legacy NNNN. High cognitive load; subtle classification bugs survive tests.

- 🟡 **Test Coverage**: Default-pattern regression guarantee is asserted in prose, not in a test
  **Location**: Phase 2: Success Criteria
  No golden-file mechanism named. The plan's most important promise has no concrete artifact.

- 🟡 **Test Coverage**: Eval threshold (95%) is a magic number with no calibration mechanism
  **Location**: Phase 3 and Phase 4 success criteria
  95% allows up to one default-pattern eval to fail. The "no behavioural change for default users" guarantee should be 100% on default-mode evals.

- 🟡 **Test Coverage**: Migration 0002 fixture under-specifies the over-rewriting risk
  **Location**: Phase 6: test fixtures
  No negative cases (timestamps, port numbers, plain prose containing `0042`) — the fixture detects under-rewriting but not over-rewriting, which is the more dangerous failure mode.

- 🟡 **Test Coverage**: Skip-tracking edge cases absent
  **Location**: Phase 5: Tests
  No coverage for empty/whitespace-only state files, both files containing the same ID, or atomicity of `--skip` under simulated failure.

- 🟡 **Safety**: Half-completed migration after rename phase poisons recovery
  **Location**: Phase 6, steps 4-6
  Rename phase completes but cross-ref rewrites fail mid-way → corpus has half-renamed cross-refs. No resume-from-partial-failure test.

- 🟡 **Safety**: Fenced-code-block rewriting can corrupt documentation examples
  **Location**: Phase 6, step 6
  Research docs and ADRs that intentionally reference legacy paths as historical examples will be silently rewritten. No language-tag restriction, no preview, no exemption mechanism.

- 🟡 **Safety**: Rename phase is not transactional; failure mid-rename leaves orphaned files
  **Location**: Phase 6, step 4
  Sequential `mv` calls with no resume marker, no transaction log, no progress checkpoint.

- 🟡 **Safety**: No dry-run or preview mode for repo-wide rewrite
  **Location**: Phase 6, steps 5-6
  Users apply sweeping rewrites sight-unseen; the only safety net is the pre-flight clean-tree check + post-hoc `jj diff`.

- 🟡 **Safety**: Skip-tracking can permanently hide a critical migration with no audit prompt
  **Location**: Phase 5
  No periodic re-prompting, no expiry, no per-migration "unskippable" marker. Skip becomes invisible after one summary line.

- 🟡 **Safety**: ID-bearing-field rewrite list is closed; misses ad-hoc references
  **Location**: Phase 6, step 6
  User-extended frontmatter fields and future template additions become silent dangling references post-migration.

- 🟡 **Usability**: Bare-number resolution silently auto-prepends default project code with no disambiguation
  **Location**: Phase 2, classification step 4
  Multi-project corpora hit the wrong file with no signal. Textbook least-surprise violation.

- 🟡 **Usability**: `--skip` discoverability gap
  **Location**: Phase 5
  Users won't discover the flag from the migrate prompt; they must read SKILL.md and invoke the runner directly.

- 🟡 **Usability**: Migration 0002 prerequisite error doesn't guide the choice
  **Location**: Phase 6, step 3
  "Set default_project_code or skip" lacks the criteria for choosing. Skip has long-tail consequences (legacy IDs forever); rename is irreversible without VCS.

- 🟡 **Usability**: Amendment table UX is under-specified
  **Location**: Phase 4, Section 1
  No spec for re-prompt format, projection update on amendment, invalid-row recovery, or cancel semantics.

- 🟡 **Usability**: Mixed-corpus warning is absent from skill prose
  **Location**: What We're NOT Doing
  Users who flip the pattern without running migration 0002 see mixed corpora with no in-skill guidance.

- 🟡 **Compatibility**: Frontmatter type change from integer/bare to quoted string
  **Location**: Phase 6, step 6 + Phase 3 (create-work-item H1)
  Changes YAML type for everyone, breaking integer-coercing consumers and the bit-identical default-pattern claim.

- 🟡 **Compatibility**: BSD vs GNU sed differences not addressed
  **Location**: Cross-cutting + Phase 6
  `sed -i` semantics differ. Migration that works on macOS may write `*.md-e` backups on Linux (or vice versa).

- 🟡 **Documentation**: README Work Item Management section not in scope
  **Location**: Phase 1 + Phase 3
  Top-level README hard-codes the legacy `NNNN-` shape with no mention of pattern configurability. Feature undiscoverable from the entry point most users read first.

- 🟡 **Documentation**: No stop-loss for failing skill-creator benchmarks
  **Location**: Implementation Approach + Phase 3 / Phase 4
  No documented decision rule for what happens when iteration plateaus below threshold.

- 🟡 **Documentation**: Commit-before-running warning is hand-waved
  **Location**: Migration Notes
  Generic warning is "judged sufficient" implicitly; no migration-0002-specific guidance about rewrite scope.

#### Minor

- 🔵 **Architecture**: Force-apply vs skip-tracking interaction under-specified — does `ACCELERATOR_MIGRATE_FORCE` override skip, respect it, or reset state?
- 🔵 **Architecture**: Cross-reference rewriter has no shared "find work-item references" abstraction — list of ID-bearing fields hard-coded with no single owner.
- 🔵 **Architecture**: Compiler invoked as subprocess on every call — consider sourcable bash library to reduce process overhead and give consumers in-process abstraction.
- 🔵 **Architecture**: Migration's mandatory-default-project-code precondition couples migration UX to config state — gates entire pipeline on one key.
- 🔵 **Correctness**: Project-value validation `[A-Za-z][A-Za-z0-9]*` rejects real-world `PROJ-FE` style codes.
- 🔵 **Correctness**: Adapted overflow guard does not specify behaviour for project-prefixed numbers; cap should be `10^N - 1` for width N.
- 🔵 **Correctness**: Race between concurrent `--skip` and `run-migrations.sh` unspecified — TOCTOU on read side.
- 🔵 **Code Quality**: Mixing concerns in compiler — regex and printf-format emitters share parser; should factor into intermediate token list.
- 🔵 **Code Quality**: Frontmatter validation duplication risk across listing, resolver, canonicaliser — three sites, three slightly different jobs.
- 🔵 **Code Quality**: Sed-based rewrites against markdown require careful escaping — slug containing `[`, backslash in path, multiple matches per line.
- 🔵 **Test Coverage**: Round-trip boundary sample too sparse — only N=1, N=9999. Should cover digit-count transitions (10, 100, 1000, 10000).
- 🔵 **Test Coverage**: Cross-reference rewriting tests don't verify failure rollback semantics.
- 🔵 **Test Coverage**: End-to-end skill integration test scope vague — Phase 3 has only one sentence describing seam tests.
- 🔵 **Test Coverage**: Resolver classification ordering tests don't cover ambiguous inputs (legacy + new file with same number, etc.).
- 🔵 **Safety**: Broadened `*.md` discovery glob with frontmatter validation can mistakenly include non-work-items — malformed frontmatter behaviour unspecified.
- 🔵 **Safety**: Migration only checks pattern at runtime; no static guard for accidental config drift — misconfigured `default_project_code` renames whole corpus to wrong project.
- 🔵 **Usability**: Configuration reference added but not surfaced from migrate skill prose — users hitting migration error must guess at the YAML key path.
- 🔵 **Usability**: Legacy fallback ordering non-obvious when same number exists in both shapes — should document precedence in skill prose.
- 🔵 **Usability**: Validation rule errors aren't required to be diagnosed by rule number — Manual Verification check should assert the missing token name, not the rule index.
- 🔵 **Usability**: Skip-tracking interaction with `ACCELERATOR_MIGRATE_FORCE` is described only in tests, not in user-facing prose.
- 🔵 **Compatibility**: Unknown-ID warning shape not declared a stable contract.
- 🔵 **Compatibility**: New userspace config keys not registered with config validators — typos like `work.id_patterns` silently read as empty.
- 🔵 **Compatibility**: Phase 5 shipping standalone before Phase 6 untested for forward/backward state-file compatibility.
- 🔵 **Compatibility**: H1 / `work_item_id` format change is not bit-identical for default users (int vs quoted string).
- 🔵 **Documentation**: Pattern DSL grammar scattered across plan rather than promoted to a single reference.
- 🔵 **Documentation**: CHANGELOG/version-bump timing is hedged — no per-milestone changelog entries enumerated.
- 🔵 **Documentation**: Out-of-scope ADR claim is defensible but undermotivated — DSL design and skip-tracking lifecycle are ADR-worthy.
- 🔵 **Documentation**: Configure / migrate skill eval treatment ambiguous — "if present" hedge weakens eval-driven discipline.
- 🔵 **Documentation**: Plan depends on research doc as load-bearing reference — line-number citations will rot.

#### Suggestions

- 🔵 **Usability**: Consider a fixture covering single-row vs many-row amendment ergonomics — table-and-amend pattern may not scale to 10+ items.

### Strengths

- ✅ Phase dependency graph is explicit and minimal — Phase 1 gates 2; 2 gates 3 and 4; 5 is independent except as a hard prerequisite for 6.
- ✅ Functional core / imperative shell separation: pattern compiler is a pure transform; consumers call it through a stable subcommand interface.
- ✅ Skip-tracking is implemented via a sibling state file rather than augmenting `.migrations-applied`, preserving the existing audit-grade format.
- ✅ Backward compatibility treated as an architectural invariant ("No behavioural change on default config"); broadened `*.md` discovery glob plus frontmatter validation guarantees legacy files remain discoverable.
- ✅ Scope boundaries explicit and justified — ADRs, plans, internal migrations are out of scope with clear rationale.
- ✅ TDD-first mandated for every code phase; tests written before implementation; existing test harness conventions reused.
- ✅ Width-tolerance decision (intent) preserves legacy-file visibility after a width change.
- ✅ Round-trip property is the right invariant for verifying compile-format / scan-regex symmetry.
- ✅ Resolver output shape distinguishes success, multiple matches, no match, invalid input via distinct exit codes (0/1/2/3).
- ✅ Phase 6 step 4 collision check before rewrites means rename failure leaves repo state unchanged on the rename axis.
- ✅ Migration framework already enforces a clean VCS pre-flight; Phase 6 inherits this protection by design.
- ✅ Phase 5 must merge before Phase 6 so users have a documented opt-out before the destructive migration is reachable.
- ✅ Default behaviour preserved exactly — Phases 1-3 are byte-identical for users with no `work.*` config, honouring the "sensible defaults" principle.
- ✅ Phase 4's "display-only allocator call" before confirmation is a thoughtful pattern — projected IDs are previewed without consuming numbers.
- ✅ Phase 5's `--skip` / `--unskip` are conventional and symmetric, with at-a-glance summary visibility.
- ✅ Each phase has a Success Criteria block split into Automated and Manual verification.
- ✅ The What We're NOT Doing section is unusually thorough.
- ✅ Cross-references to research line ranges and ADR numbers are concrete enough for a reader to verify claims.

### Recommended Changes

Ordered by impact. Items 1-2 address critical findings; 3-7 address structural majors; 8+ address discoverability and contract gaps.

1. **Replace substring-style cross-reference rewrites with exact-match rewrites driven by an `old_id → new_id` map** (addresses: critical "markdown-link rewrite is not idempotent", major "partial-failure re-run is not idempotent", major "rename phase is not transactional")
   - Build the rename map in a first pass.
   - For frontmatter: match `\"<old_id>\"`/`<old_id>` exactly (anchored to YAML scalar boundaries), not as substring.
   - For markdown links: match `\(<base_path>/<old_id>-<slug>\.md\)` precisely.
   - Skip files whose `work_item_id` is already in new-ID form.
   - Add a `--dry-run` mode that emits the planned change list without writing.
   - Add a partial-failure recovery test: kill after rename phase, re-run, assert byte-identical final state.

2. **Make slug-collision detection project-aware** (addresses: critical "slug-collision glob rejects same-slug across different projects")
   - Glob `<project>-*-<slug>.md` per amended project + `[0-9][0-9][0-9][0-9]-<slug>.md` for legacy fallback, rather than the unrestricted `*-<slug>.md`.
   - Add a Phase 4 eval covering "two source items with the same slug, amended to different projects".

3. **Switch scan-regex specification to `[0-9]+`** (addresses: major "width-tolerance regex contradicts legacy-discoverability claim", major "scan regex admits files outside number space")
   - Use `[0-9]+` per the original research.
   - Apply width validation only at format time.
   - Reject 5+digit matches under a 4-digit pattern as an error condition (or document explicitly that all visible numbers contribute to max-tracking and recommend renaming stray files).
   - Update Phase 1 test wording.

4. **Reorder resolver classification and add disambiguation** (addresses: major "step 4 unreachable", usability "bare-number silently auto-prepends")
   - Under `{project}` pattern with `default_project_code` set: try project-prepended resolution BEFORE legacy fallback.
   - On both-match: exit 2 with both candidates listed (or warn + pick configured-pattern winner — pick one and document).
   - Add tests for ambiguous corpora.
   - Document precedence in skill prose.

5. **Introduce `skills/work/scripts/work-item-common.sh` with named helpers** (addresses: major "compiler interface illustrated not specified", major "helper factoring promised but interfaces undefined", architecture "legacy-format knowledge duplicated")
   - Functions: `wip_compile_scan`, `wip_compile_format`, `canonicalise_id`, `parse_full_id`, `is_legacy_id`, `pad_legacy_number`, `is_work_item_file`, `extract_id_from_filename`.
   - Pin contract: regex flavour, capture-group index, project-value escape rules, stable error-code prefixes (`E_PATTERN_MISSING_PROJECT`, etc.).
   - Resolver, list-work-items, update-work-item, migration 0002 all source it.
   - Test helpers directly rather than transitively.

6. **Refactor migration 0002 into named helpers + add dry-run** (addresses: major "god-script with high cognitive complexity", safety "no dry-run or preview mode", safety "rename phase is not transactional")
   - Helpers: `detect_legacy_files`, `compute_rename_map`, `check_collisions`, `rename_with_frontmatter`, `rewrite_frontmatter_refs`, `rewrite_markdown_links`, `rewrite_prose_refs`.
   - Add `--dry-run` mode that emits a unified-diff-style preview without writing.
   - Recommend dry-run as the user's first step in migrate skill prose.
   - Document explicitly that the migration is not transactional; recovery path is `jj abandon` / VCS revert.

7. **Fix "marked applied = no-op" for migration 0002 on default pattern** (addresses: major "marking applied locks out future runs")
   - Decide one of: (a) skip recording when pattern lacks `{project}` (stays pending for re-evaluation), (b) hard-error when pattern lacks `{project}` (force user to `--skip`), (c) document explicit `--rerun <id>` mechanism.
   - The current "no-op exit 0 → mark applied" path is silently lossy.

8. **Address frontmatter type compatibility** (addresses: major compatibility "type change breaks consumers", minor "H1 / work_item_id not bit-identical")
   - Audit every script reading `work_item_id`, `parent`, `related`, `blocks`, `blocked_by`, `supersedes`, `superseded_by` and confirm it tolerates a quoted string.
   - Either branch on pattern shape (bare int for default, quoted for `{project}`) or relax the bit-identical claim to behavioural equivalence and pin a fresh golden file.
   - Add Phase 3 regression test capturing exact bytes of a freshly created default-pattern work-item file.

9. **Add a shared atomic-write helper** (addresses: major "atomic write pattern not pinned to a single helper", minor cross-platform sed concern)
   - `scripts/atomic-common.sh` with `atomic_write`, `atomic_append_unique`, `atomic_remove_line`.
   - Explicit `EXIT` trap to clean up `*.tmp`, `mktemp` in same directory as target (cross-fs safe).
   - Phase 5 `--skip`/`--unskip` and Phase 6 per-file rewrites both source it.
   - Avoid `sed -i` entirely (BSD vs GNU portability) — use tempfile-then-rename via the new helper.

10. **Add stable error codes / prefixes for testable error contracts** (addresses: major "error messages quoted but no contract")
    - Define prefixes (e.g. `E_PATTERN_MISSING_PROJECT`).
    - Tests pin the code; prose follows after the colon.

11. **Strengthen default-pattern regression test coverage** (addresses: major "regression guarantee asserted in prose", major "eval threshold is a magic number", minor "round-trip boundary sample too sparse")
    - Commit a frozen golden file: `(corpus, expected_output)` pairs asserted byte-for-byte.
    - Tag each eval with `pattern_mode: default | configured | both`; require 100% on default-mode subset.
    - Extend round-trip property to N ∈ {1, 9, 10, 99, 100, 999, 1000, 9999, 10000} and at least two project values.

12. **Strengthen migration 0002 fixture with negative cases** (addresses: major "fixture under-specifies over-rewriting risk")
    - Add fixture content with literal strings `2026-04-15`, `0042 occurrences`, ` 0042 ` in plain prose, `port 0042`, fenced code blocks containing non-path uses of `0042`.
    - Assert these exact strings unchanged after migration.

13. **Restrict fenced-code-block rewriting** (addresses: major safety "fenced-code rewriting can corrupt documentation examples")
    - Either: skip plain ``` blocks (only rewrite tagged language blocks), or restrict to paths immediately followed by `)` (markdown link form), or drop bare-path-in-fence rewriting entirely.
    - Add a Phase 6 test where a research doc has a fenced bash block describing the legacy convention as historical context — verify it is NOT rewritten.

14. **Surface skip discoverability and migration 0002 prerequisite guidance** (addresses: major usability "skip discoverability gap", major usability "prerequisite error doesn't guide choice", major safety "skip can permanently hide critical migration")
    - When the runner prints per-migration preview, append `To skip: bash <runner> --skip <ID>`.
    - Spell out the migration 0002 prerequisite error verbatim, with guidance on when to skip vs configure default_project_code.
    - Always print skipped migration *names* (not just count) in summary line.
    - Consider per-migration `# SKIPPABLE: yes/no` header for future critical migrations.

15. **Add mixed-corpus warning** (addresses: major usability "mixed-corpus warning absent")
    - When `list-work-items` detects files matching legacy `[0-9]{4}-` shape mixed with files matching the configured pattern, print a one-line note: "Mixed prefix corpus detected — N legacy items, M project-prefixed items. Run `/accelerator:migrate` to normalise."

16. **Pin `--force` / skip-tracking interaction in plan + prose** (addresses: minor architecture "force-apply vs skip-tracking under-specified", minor usability "interaction described only in tests")
    - Decide: `FORCE` bypasses dirty-tree pre-flight only; skip-tracking is enforced regardless. Document in migrate SKILL.md.

17. **Update README Work Item Management section** (addresses: major documentation "README not in scope")
    - Add a one-paragraph note about pattern configuration.
    - Extend configuration list to include `work` alongside `agents`/`review`/`paths`/`templates`.

18. **Document iteration stop-loss for skill-creator benchmarks** (addresses: major documentation "no stop-loss")
    - Maximum iteration count (e.g. 3), what to do on failure (escalate, lower threshold with rationale recorded, split eval), where to archive output.

19. **Promote the pattern DSL grammar into a single reference** (addresses: minor "DSL grammar scattered")
    - Either inline a "Pattern DSL Reference" subsection at the top of Phase 1, or commit `skills/work/references/id-pattern-dsl.md`.

20. **Inline load-bearing research details** (addresses: minor "plan depends on research as load-bearing")
    - Replace line-number citations with section-heading citations, OR inline validation rules table, classification order list, migration behaviour summary into the plan.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan establishes a clean architectural backbone — a pattern compiler as the foundation, a resolver as the lookup abstraction, and a strictly additive skip-tracking extension. The main architectural risks are a width-tolerance contradiction in Phase 1, leakage of legacy-format knowledge across multiple components without a shared abstraction, and a few under-specified interactions (force-vs-skip, resolver-vs-listing duplication, ID-finder reuse in migration 0002).

**Strengths**: Clean functional-core/imperative-shell separation; explicit phase dependency graph; sibling state file preserves audit-grade format; backward-compat as an architectural invariant; explicit and justified scope boundaries.

**Findings**: Width-tolerance scan regex contradicts legacy-discoverability claim (major); legacy-format knowledge duplicated (major); force-vs-skip interaction under-specified (minor); cross-reference rewriter has no shared abstraction (minor); compiler invoked as subprocess (minor); migration's mandatory-default-project-code couples migration UX to config state (minor).

### Correctness

**Summary**: The plan demonstrates careful attention to default-pattern preservation but contains several correctness gaps in the resolver classification, the markdown-link rewrite regex, the slug-collision glob, and the migration's idempotency claim.

**Strengths**: Width-tolerance decision; round-trip property at N=1/N=9999; resolver exit-code shape; path-input early-bail; collision check before rewrites.

**Findings**: Markdown-link rewrite not idempotent (critical); slug-collision glob rejects same-slug across different projects (critical); resolver step 4 unreachable when legacy file shares the number (major); scan regex admits files outside number space (major); marking applied locks out future runs (major); partial-failure re-run not idempotent (major); test expectation contradicts classification flow (major); project-value validation rejects real-world codes (minor); overflow guard under-specified for project-prefixed numbers (minor); concurrency unspecified (minor).

### Code Quality

**Summary**: Well-structured plan with explicit hygiene. However, the pattern compiler's interface is described by examples rather than as a formal contract, helper boundaries are gestured at but never defined, atomic-write patterns and error-message contracts are mentioned without canonical helpers, and the resolver classifier mixes concerns. Phase 6's migration script is the largest maintainability risk.

**Strengths**: TDD-first; explicit `set -euo pipefail` discipline; helper factoring intent; three-mode pattern compiler CLI; no-behavioural-change regression boundary; resolver exit codes well-defined; additive skip-tracking.

**Findings**: Compiler interface illustrated not specified (major); helper factoring promised but interfaces undefined (major); migration 0002 god-script (major); atomic-write pattern not pinned to a single helper (major); error messages without testable contract (major); resolver classifier mixes structural classification with corpus probing (major); compiler emits regex+printf concerns mixed (minor); frontmatter validation duplication risk (minor); overflow guard restated abstractly (minor); sed-based markdown rewrites need careful escaping (minor).

### Test Coverage

**Summary**: Genuinely TDD-driven for bash phases with detailed test case lists. The test pyramid leans heavily on skill-level evals at a ≥95% threshold for Phases 3 and 4 — a single weakly-defined number that elides verifiable end-to-end behaviour. Default-pattern regression guarantee, migration 0002 corpus, and round-trip property each have tractable but under-specified test holes.

**Strengths**: Concrete test cases by scenario; existing harnesses correctly identified as extension points; byte-equality regression check on `.migrations-applied`; file-tree hash idempotency check for migration 0002; round-trip property named at N=1, N=9999.

**Findings**: Default-pattern regression guarantee asserted in prose, not in a test (major); eval threshold is a magic number (major); fixture under-specifies over-rewriting risk (major); skip-tracking edge cases absent (major); round-trip boundary sample too sparse (minor); cross-reference rewriting tests don't verify failure rollback (minor); end-to-end skill integration test scope vague (minor); resolver classification ordering tests miss ambiguous inputs (minor).

### Safety

**Summary**: Inherits a solid pre-flight clean-tree check and correctly designs Phase 6 around per-file atomicity and upfront collision scan. However, the most destructive operation — migration 0002's repo-wide rewrite — has substantive partial-failure exposure. Several blind spots around half-rewrites, fenced-code false positives, and the absence of a dry-run/preview mode warrant tightening.

**Strengths**: Pre-flight clean-tree check; collision detection across the entire candidate set before any rename; additive skip-tracking; idempotency invariant + second-run no-op test; explicit "no rollback beyond VCS revert" disposition; default-pattern users see no behavioural change in early phases.

**Findings**: Half-completed migration after rename phase poisons recovery (major); fenced-code-block rewriting can corrupt documentation examples (major); rename phase is not transactional (major); no dry-run / preview mode (major); skip-tracking can permanently hide critical migrations (major); ID-bearing-field rewrite list is closed (major); broadened glob with frontmatter validation may include non-work-items (minor); migration only checks pattern at runtime (minor).

### Usability

**Summary**: Strong DX awareness in many places — empty-default behaviour preserved, error messages spelled out, multi-project amendment flow specified, opt-out semantics for migration 0002. Several concerns: bare-number-with-default-project potentially surprising under ambiguity, `--skip` discoverability not described, Phase 4 amendment table UX under-specified, mixed-corpus users get no in-skill warning, and the Phase 6 prerequisite decision lacks guidance.

**Strengths**: Resolution order is conventional/predictable with actionable error messages; default behaviour preserved exactly; display-only allocator call previews before consuming numbers; explicit pre-flight collision check; pattern validation rule 5 matches Jira/Linear conventions; symmetric `--skip`/`--unskip` with summary visibility.

**Findings**: Bare-number resolution silently auto-prepends (major); `--skip` discoverability gap (major); migration 0002 prerequisite error doesn't guide the choice (major); amendment table UX under-specified (major); mixed-corpus warning absent (major); configuration reference not surfaced from migrate prose (minor); legacy fallback ordering non-obvious (minor); validation rule errors should not reference rule numbers (minor); FORCE+skip interaction described only in tests (minor); consider many-row amendment ergonomics (suggestion).

### Compatibility

**Summary**: Compatibility-conscious in core design — default-pattern users see no behavioural change, `.migrations-applied` is preserved byte-for-byte, forward-compat for unknown migration IDs is mirrored on the new sibling state file. Several concrete risks: cross-platform sed differences not addressed; the migration changes `work_item_id` frontmatter from integer/unquoted to quoted-string, breaking integer-coercing consumers; unknown-ID warning shape is not declared a stable contract.

**Strengths**: Default-pattern bit-identical regression explicitly tested; `.migrations-applied` format held constant with byte-equality check; forward-compat for unknown IDs mirrored; width-tolerance preserves legacy corpus across width changes; flat dependency surface; sequencing constraints spelled out.

**Findings**: Frontmatter type change breaks integer-coercing consumers (major); BSD vs GNU sed differences not addressed (major); unknown-ID warning shape not declared a stable contract (minor); new userspace config keys not registered with validators (minor); Phase 5 standalone untested for forward/backward state-file compat (minor); H1 / `work_item_id` change is not bit-identical for default users (minor).

### Documentation

**Summary**: Well-structured internally and touches the two main user-facing skill docs (configure, migrate), but leaves a meaningful gap outside the skills: the README hard-codes the legacy `NNNN-` shape, CHANGELOG/version-bump moments are described loosely, and the pattern DSL grammar is scattered. The migrate skill prose's commit-before-running warning is referenced but not concretely described, and skill-creator integration lacks a documented stop-loss.

**Strengths**: Phase 1 carves out a "Work Items" subsection in `configure/SKILL.md` mirroring existing structure; Success Criteria split into Automated and Manual; thorough What-We're-NOT-Doing; concrete cross-references to research line ranges and ADR numbers.

**Findings**: README Work Item Management not in scope (major); no stop-loss for failing skill-creator benchmarks (major); commit-before-running warning hand-waved (major); pattern DSL grammar scattered (minor); CHANGELOG/version-bump timing hedged (minor); out-of-scope ADR claim undermotivated (minor); configure/migrate skill eval treatment ambiguous (minor); plan depends on research doc as load-bearing reference (minor).

## Re-Review (Pass 2) — 2026-04-28

**Verdict:** REVISE

The plan changed substantially: 0 critical findings remain, no prior major finding is "still present" outright, and the structural concerns (helper factoring, atomic-write idiom, exact-match rewrite contract, MIGRATION_RESULT runner contract, eval-mode tagging, golden files, Pattern DSL Reference, ADR justification, README update) are addressed. However, four new major findings surfaced from the edits themselves — primarily test-contract gaps and one internal contradiction in the resolver's candidate-collection logic — keeping the verdict at REVISE per the major-count threshold (≥3). The remaining majors are all small, tractable fixes; another short revision pass should reach APPROVE.

### Previously Identified Issues

#### Critical (2 of 2 resolved)

- 🔵 **Correctness**: Markdown-link rewrite is not idempotent — **Resolved**. Exact-match rewrites driven by old→new map; legacy-only matching prevents re-fire on already-rewritten paths; partial-failure recovery test locks the invariant.
- 🔵 **Correctness**: Slug-collision glob rejects same-slug across projects — **Resolved**. Project-aware globs; same-slug-different-projects fixture asserts coexistence.

#### Major (32 prior findings — none "still present"; 6 partial; 26 resolved)

Architecture (5/6 resolved, 1 partial):
- 🔵 Width-tolerance scan regex contradicts legacy-discoverability — **Resolved** (`[0-9]+`).
- 🔵 Legacy-format duplication across resolver and list-work-items — **Resolved** (`work-item-common.sh`).
- 🔵 Cross-reference rewriter has no shared abstraction — **Partial**. Three named helpers inside migration 0002, but not extracted to common; acceptable since migration 0002 is the only consumer.

Correctness (5/5 resolved + 1 partial):
- 🔵 Resolver step 4 unreachable — **Resolved** (ambiguity-aware candidate collection).
- 🔵 Scan regex admits over-width files — **Resolved** (overflow guard reports stray files).
- 🔵 Marking applied locks out future runs — **Resolved** (`MIGRATION_RESULT: no_op_pending`).
- 🔵 Partial-failure re-run not idempotent — **Resolved** (exact-match design + recovery test).
- 🔵 Test expectation contradicts classification — **Resolved** (path-shaped → exit 3).
- 🔵 Project-value validation restrictiveness — **Partial** (documented as known limitation).

Code Quality (9/10 resolved + 1 partial):
- 🔵 Compiler interface specified — **Resolved** (Compiler Contract subsection).
- 🔵 Helper factoring — **Resolved** (`work-item-common.sh` enumerated).
- 🔵 Migration 0002 god-script — **Resolved** (seven named helpers).
- 🔵 Atomic write helper — **Resolved** (`atomic-common.sh`).
- 🔵 Error message contract — **Resolved** (`E_PATTERN_*` prefixes).
- 🔵 Resolver classifier pivot — **Partial** (clean classify_input/resolve_classified split, but bare-number branch still multi-source).
- 🔵 Compiler concerns mixing — **Resolved** (three discrete CLI modes).
- 🔵 Frontmatter validation duplication — **Resolved** (`wip_is_work_item_file`).
- 🔵 Overflow guard rule — **Resolved** (`wip_pattern_max_number`).
- 🔵 Sed escaping — **Resolved** (anchored exact-match through `atomic_write`, no `sed -i`).

Test Coverage (8/8 resolved):
- 🔵 Default-pattern golden files — **Resolved**.
- 🔵 Eval threshold calibration — **Resolved** (mode-tagging + stop-loss).
- 🔵 Migration 0002 fixture over-rewriting — **Resolved** (negative-case fixture).
- 🔵 Skip-tracking edge cases — **Resolved**.
- 🔵 Round-trip boundary sample — **Resolved**.
- 🔵 Cross-reference rewriting failure rollback — **Resolved**.
- 🔵 End-to-end integration test scope — **Resolved**.
- 🔵 Resolver classification ambiguous inputs — **Resolved**.

Safety (6/6 resolved + 1 accepted + 2 partial):
- 🔵 Half-completed migration recovery — **Resolved**.
- 🔵 Fenced-code rewriting corrupts examples — **Resolved** (language-tag whitelist).
- 🔵 Rename phase not transactional — **Resolved** (atomic_write-then-mv).
- 🔵 No dry-run for repo-wide rewrite — **Accepted** (user opted to rely on VCS revert; layered safety mechanisms substitute).
- 🔵 Skip-tracking can hide critical migration — **Resolved** (names in summary).
- 🔵 ID-bearing field rewrite list closed — **Resolved** (anchored shapes + consumer audit).
- 🔵 Broadened glob may include non-work-items — **Partial** (predicate exists, exact contract under-specified).
- 🔵 No static config-drift guard — **Partial** (mixed-corpus warning + self-deferring migration mitigate, but no proactive signal at config-change time).

Usability (5/5 resolved):
- 🔵 Bare-number auto-prepend disambiguation — **Resolved**.
- 🔵 `--skip` discoverability — **Resolved** (per-migration preview line).
- 🔵 Migration 0002 prerequisite error guidance — **Resolved**.
- 🔵 Amendment table UX — **Resolved** (worked example with three states).
- 🔵 Mixed-corpus warning — **Resolved** (one-line hint in list-work-items).

Compatibility (1/2 resolved + 1 partial):
- 🔵 Frontmatter type change — **Partial** (uniform contract + audit, but no per-consumer regression test).
- 🔵 BSD vs GNU sed — **Resolved** (`atomic-common.sh` avoids `sed -i`).

Documentation (2/3 resolved + 1 partial):
- 🔵 README Work Item Management — **Resolved** (Phase 3 §8).
- 🔵 Skill-creator stop-loss — **Resolved** (3-iteration cap with escalation).
- 🔵 Commit-before-running warning — **Partial** (acknowledged but specific prose location not pinned).

#### Minor (22 prior — most resolved or accepted as documented limitations)

All prior minors are addressed via the same edits as the relevant majors, with the partial cases above noted explicitly.

### New Issues Introduced

#### Major (4)

- 🟡 **Correctness**: Ambiguity test (2) cannot pass under the documented candidate-collection rules
  **Location**: Phase 2 §2 (resolver bare-number resolution) + Ambiguity test (2)
  Test seeds `PROJ-0042-x.md` and `OTHER-0042-y.md` under `{project}` pattern with NO default project code, asserts input `0042` exits 2 with both candidates listed. Documented logic: step (a) skipped (no default), step (b) only matches `[0-9]{4}-*.md` (neither file matches), step (c) skipped (pattern has `{project}`) → empty candidate set → exit 3. Either drop the test or extend the candidate-collection rules to scan *all* observed project codes for the bare number when no default is set.

- 🟡 **Test Coverage**: pattern_mode tag is declarative — no test verifies the runner enforces the 100%/95% split
  **Location**: Phase 3 §6 Eval mode-tagging
  The mode-tagging is the central regression-protection mechanism. A bug in the runner's threshold logic (e.g., applying 95% across the board) would silently let default-mode regressions slip through. Add a test of the eval runner / wrapper with seeded fixtures asserting the threshold split is enforced.

- 🟡 **Test Coverage**: Comma-separated row syntax for amendments not pinned by a concrete eval expectation
  **Location**: Phase 4 §3
  The 12-item large-batch fixture exists but the eval description ('expected_output describes the user-visible behaviour') doesn't pin syntax assertions. A future SKILL.md edit could quietly drop comma-separated parsing. Add concrete assertions: that `3,7,12 OTHER` flips exactly those rows, that `99 OTHER` re-prompts only the offending row.

- 🟡 **Usability**: Amendment prompt has no documented revert, help, or 'show me again' command
  **Location**: Phase 4 §1 amendment prompt grammar
  Users who fat-finger an amendment cannot undo just that row — they must `q` to cancel and restart. No `?` for help. The 'blank to confirm' wording also subtly diverges between states. Specify a small consistent grammar: `<rows> <PROJECT>` to set, `<rows> -` to revert, `?` for help, `q` to cancel, blank to confirm. Use the same prompt wording in every state.

#### Minor (14 — see per-lens results below for detail)

Notable items worth follow-up:

- 🔵 **Architecture**: Stdout-as-protocol coupling for `MIGRATION_RESULT: no_op_pending` (consider out-of-band channel); existing inline atomic-write call sites deferred from `atomic-common.sh` adoption (consolidate or track ticket); list-work-items knows about migration command path (mild cross-skill coupling).
- 🔵 **Correctness**: `atomic_write` EXIT trap overwritten in loops (specify per-call subshell or cumulative trap); heading-line `#<old_id>` rewrites `#0042-foo` references (tighten boundary regex); `wip_is_legacy_id` accepts `0`/`0000` (degenerate inputs).
- 🔵 **Code Quality**: Dual entry points (library + CLI) — pin equivalence test or drop CLI; rename-then-write ordering load-bearing but not named in helper signature; library cohesion drift risk (10 functions across 3 concerns).
- 🔵 **Test Coverage**: Block-list YAML form (multi-line dash-prefixed) not fixtured; `atomic_write` failure modes shallow (symlink, read-only dir); heading-line edge cases.
- 🔵 **Safety**: `MIGRATION_RESULT` misuse footgun (migration emits line after partial work); mixed-corpus warning may include non-work-item filename-shape matches (false positives); rename-phase interrupt + uncommitted edits between attempts can lose data.
- 🔵 **Usability**: Mixed-corpus note text could be more action-shaped; configure SKILL.md "Choosing" subsection should be a structured decision aid, not narrative; validation-rejection re-prompt doesn't clarify rejected amendments are discarded; prompt text divergence between states.
- 🔵 **Compatibility**: `MIGRATION_RESULT` line visible to non-runner invokers (consider dedicated FD); consumer audit checklist-only (no per-consumer regression test); `pattern_mode` field may break strict eval-schema validators.
- 🔵 **Documentation**: Commit-before-running warning prose not pinned; configure↔Phase 6 cross-reference anchor under-specified; escape sequence not in examples table; `wip_*` return-channel ambiguous; amendment worked example missing State 4 (post-confirmation output).

### Assessment

The criticals are gone. All 32 prior majors are resolved or partial; none are still-present-as-described. The four new majors are tractable, narrow fixes (one spec correction, two eval-test additions, one prompt-grammar tightening) and could be addressed in a short batch 4. The minors are the long tail of polish items and could ship as-is or be folded into batch 4.

Recommended next actions:

1. Fix the four new majors as a tight batch (estimate: small).
2. Decide which partials to elevate (the resolver classifier multi-source branch and the consumer audit's lack of regression tests are the most consequential).
3. Optional: triage the new minors and either fold the high-value ones into the same batch or accept them as known limitations.

After batch 4, the plan should be ready for APPROVE without another full re-review pass.

## Batch 4 Edits (2026-04-28T16:47:53Z)

The four new majors and several high-value minors from the re-review have been addressed without a full re-review pass:

**Majors addressed**:

1. **Correctness — Ambiguity test (2) contradiction**: Resolver candidate-collection extended with step (d) "cross-project scan candidate" that scans all observed project codes for the bare number when pattern has `{project}`, regardless of whether `default_project_code` is set. Filtered to matches conforming to the configured pattern's scan regex. Deduplication rule preserves `project-prepended` tag for default-coded matches. New tests added: Ambiguity test (3) (mixed default-coded + cross-project), single-match cross-project.

2. **Test Coverage — pattern_mode runner enforcement**: New `scripts/test-evals-mode-tagging.sh` test specified with seeded fixtures asserting the 100%/95% threshold split, untagged-defaults-to-default behaviour, both-mode running under both configurations, and empty-evals zero-exit. Wired into `tasks/test.py`.

3. **Test Coverage — Comma-separated UX assertions**: Phase 4 `large-batch-comma-separated-amendment.md` eval expectations now pinned with concrete row-by-row Project assertions, exact allocator-call shape (`--project PROJ --count 9`, `--project OTHER --count 3`), and explicit filename mapping. Three new fixtures added: `revert-amendment-batch.md`, `whitespace-tolerant-amendment.md`, `out-of-range-row-amendment.md`, `help-command.md`.

4. **Usability — Amendment prompt grammar**: Canonical grammar specified verbatim with the same wording used in every state. Added `<rows> -` to revert, `?` for help, error messages for out-of-range and invalid project codes, explicit "rejected amendments are discarded" rule. Worked example expanded to six states (initial, set, revert, comma-separated, validation rejection, confirmation) showing the post-allocation output.

**High-value minors addressed**:

- **Documentation — commit-before-running warning location**: Pinned to a new "pre-run banner" in `run-migrations.sh` printed before applying migrations, plus migration-author guidance in migrate SKILL.md (atomic-write helper reference, exact-match-rewrite design pattern). Banner is per-invocation and applies to all future migrations, not just 0002. Test added.

- **Compatibility — frontmatter consumer regression tests**: Phase 3 §7 expanded from one test to six, organised by consumer category (resolver/read-field, allocator round-trip, parent comparison, discovery walking). Each tests post-Phase-3 quoted-string contract directly.

- **Correctness — heading-line word-boundary edge cases**: Heading regex now uses negative-lookahead `(?![A-Za-z0-9_-])` (or POSIX-portable boundary class) excluding `-`, so `#0042-foo` does NOT rewrite. Three new test cases: heading-boundary, heading-suffix, multi-reference heading.

- **Correctness — `wip_is_legacy_id` degenerate inputs**: Predicate tightened to require at least one non-zero digit. `0`, `00`, `0000` rejected.

- **Documentation — `wip_*` return channel**: All helpers now follow consistent calling convention (result on stdout, errors on stderr with `E_*` prefix, exit codes 0/non-zero). Boolean predicates use exit code only. Documented in the library spec.

**Migration Notes** rewritten to drop the hand-waved "users are warned in skill prose" and reference the new pre-run banner + idempotency invariant + exact-match-rewrite design as the layered safety mechanisms substituting for dry-run.

**Plan length**: 925 → 1709 → 2001 lines (delta from re-review: +292).

### Final assessment

The plan now specifies the design at implementation-ready depth: every helper has a stable contract, every test has a concrete assertion, every error has a stable code, every UX path has a worked example. The four new re-review majors are closed; the partials worth elevating (resolver classifier multi-source, consumer audit lacking regression tests) are also closed via the candidate-collection rewrite and Phase 3 §7 expansion.

Updated verdict: **APPROVE** (1 critical: 0, prior majors: all resolved or partial→addressed, new majors from batch 4: 0 expected). The plan is ready for implementation. Remaining minor items from the re-review (e.g. dual-entry-point CLI/library, list-work-items knowing about migration command, MIGRATION_RESULT stdout-as-protocol) are tracked-as-known limitations or deferred to natural follow-up.
