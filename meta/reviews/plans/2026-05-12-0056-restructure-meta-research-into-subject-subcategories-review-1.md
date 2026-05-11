---
date: "2026-05-12T00:00:00+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-12-0056-restructure-meta-research-into-subject-subcategories.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, correctness, test-coverage, code-quality, safety, compatibility, documentation]
review_pass: 3
status: complete
---

## Plan Review: 0056 — Restructure `meta/research/` into Subject Subcategories

**Verdict:** REVISE

The plan is well-structured, test-first, and demonstrates strong situational
awareness of the existing migration framework, the DIR_DEFAULTS shadowing
trap, and the configure/SKILL.md drift. However, all seven lenses surfaced
critical or major concerns that converge on the same underlying issue: the
migration's correctness and safety guarantees do not hold under realistic
userspace shapes — overridden paths, partial migration states, `.gitkeep`/
`.DS_Store` residue, prefix-shaped false positives, and content imported
later. The visualiser cross-boundary contract (bash → Rust JSON keys) and
`templates.research` rename are also under-addressed. Seven critical
findings and ~24 major findings — well above the REVISE threshold.

### Cross-Cutting Themes

These appeared across multiple lenses and deserve top-billing in the
revision:

- **Path-prefix matching has no boundary anchor** (flagged by:
  Architecture, Correctness, Code-Quality) — `meta/research-templates/`,
  `meta/researchers.md`, `meta/research-cache/` will match `meta/research`
  and be silently corrupted. The non-matching fixture cases listed in the
  plan would *fail* under the rewriter logic as specified.

- **Value preservation breaks under user overrides** (Architecture,
  Correctness, Compatibility) — a user with `paths.research: docs/research`
  has files moved to `docs/research/codebase/` but the rewritten
  `paths.research_codebase: docs/research` points one level too shallow.
  The same problem applies to independent overrides on
  `paths.design_inventories` / `paths.design_gaps` (silently re-nested
  under `OLD_RESEARCH`).

- **Step 1 cleanup leaves stale legacy directories** (Correctness,
  Safety, Test-Coverage) — `*.md` / `*/` globs miss `.gitkeep`,
  `.DS_Store`, dotfiles, and non-`.md` siblings; `rmdir … || true`
  silently swallows the resulting ENOTEMPTY. Phase 7's hand-cleanup of
  `.DS_Store` does not generalise to userspace.

- **Cross-step partial-failure recovery is implicit** (Correctness,
  Safety) — Steps 1/2/3 are not atomic. If Step 3 fails partway,
  `.migrations-applied` is *not* updated (script exits non-zero) but
  files are moved, config is rewritten, and an unknown subset of inbound
  refs are rewritten. The user's recovery is `jj op restore`, but the
  dirty-tree pre-flight does not cover Step 3's scan corpus.

- **0001/0002/0003 patterns lifted by copy-paste rather than extracted**
  (Architecture, Code-Quality) — 0056 becomes the 4th carrier of
  `_move_if_pending` and the 3rd of a `probe_paths_key`-shaped reader.
  Phase 5 generalises 0002's id-rename rewriters to path-prefix
  rewriters in two sentences while the actual logic is materially
  different.

- **Visualiser surface is under-scoped** (Architecture, Compatibility,
  Documentation) — Phase 8.2 says "inspect `src-tauri/`" but the real
  cross-boundary contract is `write-visualiser-config.sh` emitting JSON
  `doc_paths.research` / `doc_paths.design_gaps` /
  `doc_paths.design_inventories`, with `server/src/docs.rs::config_path_key()`
  reading the same string keys, plus two JSON fixtures and a TypeScript
  test all containing legacy literals.

- **`probe_legacy_path` uses `config-read-path.sh` after Phase 1
  retired the legacy keys** (Architecture, Code-Quality) — 0003
  explicitly avoided this because the migration is itself rewiring the
  resolver. Plan cites 0003's `probe_paths_key` in Key Discoveries but
  the Phase 3 code sample does not actually use that pattern.

### Tradeoff Analysis

- **Migration scope vs. user discoverability**: Documentation argues
  for richer post-migration output (file counts, new-key explanation);
  Compatibility cautions that any structured-notification format
  becomes a fragile contract. Recommendation: emit a single summary
  line documented as informational/human-readable only.

- **Atomicity vs. framework simplicity**: Safety wants a staged
  Step 3 (materialise edits before commit); Code-Quality already
  flagged the migration framework's growing per-migration surface
  area. Recommendation: take an explicit `jj snapshot` at entry and
  document the resulting op as the single rollback point, rather than
  building tmp-staging infrastructure.

### Findings

#### Critical

- 🔴 **Correctness**: Prefix matching has no path-boundary anchor — `meta/research-templates/` will be falsely rewritten
  **Location**: Phase 5 — config-driven inbound-link rewriting
  The frontmatter/scalar/markdown-link prefix matcher as specified will
  incorrectly rewrite `meta/research-templates/foo.md` to
  `meta/research/codebase-templates/foo.md`. The non-matching fixture
  lists this exact shape as required to remain byte-identical.

- 🔴 **Correctness**: `NEW_*` paths derived from `OLD_RESEARCH` even when user already has new keys in config — mixed-state runs corrupt placement
  **Location**: Phase 3 — Step 0 capture + Step 1 moves
  A user who hand-edited `paths.research_codebase: docs/research/codebase`
  but hasn't moved files will have `probe_legacy_path` fall back to the
  default `meta/research`, then the script computes new locations from
  that, ignoring the user's stated `paths.research_codebase`.

- 🔴 **Architecture**: Value-preserving config rewrite breaks resolution under user overrides
  **Location**: Phase 3 (Step 0/1) + Phase 4 (Step 2)
  Files move to `${OLD_RESEARCH}/codebase` but the user's verbatim
  `paths.research: docs/research` becomes `paths.research_codebase:
  docs/research` — the new key points one directory level too shallow.

- 🔴 **Architecture**: Design-inventories/design-gaps overrides silently ignored at move time
  **Location**: Phase 3 — Step 0 capture + Step 1 moves
  `NEW_INV="${OLD_RESEARCH}/design-inventories"` nests under
  `OLD_RESEARCH` regardless of whether the user customised
  `paths.design_inventories`. Independent overrides are silently
  discarded.

- 🔴 **Safety**: Dirty-tree pre-flight does not cover Step 3's scan corpus
  **Location**: Phase 5 — inbound rewriting + Migration Notes
  `run-migrations.sh` only checks `meta/`, `.claude/accelerator*.md`,
  `.accelerator/`. Step 3 scans every directory in `accelerator:paths`
  (potentially `docs/`, `custom/work/`, etc.). Uncommitted user work in
  configured-but-non-default paths gets commingled with migration edits.

- 🔴 **Safety**: No rollback if Step 2 or Step 3 fails partway — repo left in inconsistent state
  **Location**: Phase 3-5 atomicity + Migration Notes
  Steps 1/2/3 are non-atomic. On partial Step 3 failure,
  `.migrations-applied` is not updated (script exited non-zero) but
  half the rewrites have landed. Re-run idempotency is the only
  protection and is not exhaustively tested.

- 🔴 **Compatibility**: Upgrade-without-migrate window leaves skills resolving to empty paths
  **Location**: Phase 2 — Skill consumers
  Between plugin upgrade and `accelerator:migrate`, every consumer of
  the renamed keys (research-codebase, research-issue, extract-adrs,
  extract-work-items, init, visualise, documents-locator) will resolve
  `research_codebase` to a directory that doesn't yet exist on disk.
  The SessionStart hook only warns; it does not block.

- 🔴 **Compatibility**: Bash → Rust JSON contract for `doc_paths.{research,design_gaps,design_inventories}` is not updated
  **Location**: Phase 8.2 — visualiser frontend
  `write-visualiser-config.sh` emits JSON keys `doc_paths.research`,
  `doc_paths.design_gaps`, `doc_paths.design_inventories`, and
  `server/src/docs.rs::config_path_key()` reads them. Plan says
  "inspect `src-tauri/`" but the real surface includes the bash writer,
  Rust resolver string constants, two JSON fixtures, and a TypeScript
  test.

#### Major

- 🟡 **Correctness**: `rmdir … || true` leaves legacy parent directories behind when `.DS_Store`, `.gitkeep`, or non-`.md` files remain — `meta/design-inventories/` (with `.DS_Store`) on Mac userspaces will persist post-migration.

- 🟡 **Correctness**: Step 1/Step 2/Step 3 partial-failure recovery semantics undocumented — cross-step idempotency invariants only implicit.

- 🟡 **Correctness**: `paths.research_issues` insertion rule underspecified — sed pipeline has no anchor for a brand-new key (nested-YAML vs flat-dotted? indent? sibling position?).

- 🟡 **Correctness**: Duplicate-key handling missing — if user config has both `paths.research:` and `paths.research_codebase:` (partial pre-migration), sed rewrite produces a duplicate key.

- 🟡 **Correctness**: Move loops glob `*.md` / `*/` only — `.gitkeep`, `.DS_Store`, non-`.md` siblings are not moved, blocking cleanup.

- 🟡 **Architecture**: `probe_legacy_path` uses `config-read-path.sh` after Phase 1 retired the legacy keys — couples migration correctness to undocumented short-circuit behaviour. 0003 explicitly avoided this.

- 🟡 **Architecture**: Scan-corpus union via `accelerator:paths` misses out-of-corpus user docs (top-level README, ARCHITECTURE.md). User repos get partial inbound-rewrite coverage with no diagnostic.

- 🟡 **Architecture**: Lifted-paste of 3 patterns from 0001/0002/0003 entrenches duplication — bug fixes (e.g. conflict-message format) must be back-ported by hand across N migrations.

- 🟡 **Code Quality**: 4-clause sed pipeline elided behind `# …` in the plan — the hardest, most error-prone block in Phase 4 is under-specified.

- 🟡 **Code Quality**: DIR_KEYS/DIR_DEFAULTS shadow patched in two places rather than eliminated — perpetuates the smell that caused the original bug.

- 🟡 **Safety**: Conflict detection happens during Step 1 mid-loop — by the time `_move_if_pending` raises, prior siblings have already moved. 0002 collision-checks up-front; 0056 should too.

- 🟡 **Safety**: `rmdir` silent-swallow conflates two failure modes (non-empty-OK vs permission error). Either should emit a diagnostic.

- 🟡 **Safety**: Step 3 cannot distinguish a live reference from a quoted/historical one. Work items, ADR rationale, CHANGELOG-shaped narrative inside `meta/work/` gets silently mutated.

- 🟡 **Safety**: No-VCS case passes pre-flight silently (vcs="", dirty=""); claimed `jj op restore` recovery is unavailable. Should fail closed.

- 🟡 **Safety**: Phase 7 is simultaneously the integration test and the production-destructive step against plugin meta/. No isolated rehearsal.

- 🟡 **Compatibility**: `templates.research` rename has no migration handling — userspace overrides of `templates.research` silently stop being applied.

- 🟡 **Compatibility**: Notification line format treated as AC contract but only plain echo to merged stdout/stderr; format drift will silently break any downstream parser.

- 🟡 **Compatibility**: Inbound-link rewriting is once-only — content imported later (branch merges, sibling-repo copies) retains legacy paths.

- 🟡 **Compatibility**: Dropping `design_inventories`/`design_gaps` from `EXCLUDED_KEYS` widens `accelerator:paths` surface; downstream consumers expecting stable key set may surprise.

- 🟡 **Test Coverage**: Prose rewriter only tested in fenced code blocks; real corpus has backtick-inline, bullet-list-bare, and narrative-prose shapes none of which have fixtures.

- 🟡 **Test Coverage**: Fixture diversity narrow — no `.gitkeep`/`.DS_Store`/partial-state/mixed-config/destination-non-empty cases.

- 🟡 **Test Coverage**: `config.local.md` rewrite path implemented but no test fixture exercises it.

- 🟡 **Test Coverage**: No test covers refs inside the files being moved (a research file linking to another research file) — the most common in-tree shape.

- 🟡 **Test Coverage**: AC #11/#12 (research-codebase/research-issue write to new paths) verified by grep + manual run only, not by automated path-resolution assertion.

- 🟡 **Documentation**: `configure/SKILL.md` line 724 narrative hardcodes `meta/research/` and `paths.research:` example — not enumerated in Phase 6 or Phase 8.

- 🟡 **Documentation**: Four template files (`templates/{adr,work-item,plan,research}.md`) contain legacy `meta/research/` body references; plugin `templates/` is outside the default scan corpus (`paths.templates: .accelerator/templates`).

- 🟡 **Documentation**: `paths.research_issues` introduction buried in CHANGELOG — no narrative explaining the new bucket's purpose.

- 🟡 **Documentation**: Visualiser server fixtures (`config.valid.json`, `config.optional-override-null.json`), `test-launch-server.sh`, and `LifecycleClusterView.test.tsx` contain legacy path literals — not enumerated in Phase 8.

- 🟡 **Documentation**: Historical CHANGELOG entries (lines 98, 155, 165, 170) reference paths that will no longer exist; readers grepping for legacy paths land in orphan contexts.

#### Minor

- 🔵 **Correctness**: Plain `mv` interleaved with `config-read-path.sh` invocations inside a jj working copy can produce mid-migration auto-snapshots.

- 🔵 **Correctness**: Notification fires misleadingly with `value preserved: <default>` when user has an empty-value legacy key.

- 🔵 **Correctness**: Scan corpus filters to `[ -d $v ]` — directories configured but not yet created are skipped silently.

- 🔵 **Architecture**: Template key rename (`templates.research` → `templates.codebase-research`) coupled to path-key rename without separate migration rationale.

- 🔵 **Architecture**: Phase 7 plugin-self-migration state-file lifecycle vs. userspace-clone behaviour not addressed.

- 🔵 **Architecture**: Phase 8.2 visualiser scope vague — "inspect … if hardcoded literals anywhere".

- 🔵 **Code Quality**: Phase 1 vs Phase 6 fragmentation — `configure/SKILL.md` legend and `documents-locator` legend both mirror PATH_KEYS but are split across phases.

- 🔵 **Code Quality**: `build_scan_corpus` uses fragile bullet-string slicing; `printf "${array[@]}"` will trip `set -u` on empty results.

- 🔵 **Code Quality**: `rmdir … 2>/dev/null || true` silently swallows non-empty-dir errors with no log diagnostic.

- 🔵 **Test Coverage**: VCS rename test only covers jj, not git (AC requires both).

- 🔵 **Test Coverage**: Notification-line test uses `contains` not exact-format equality.

- 🔵 **Test Coverage**: Phase 7 grep/find checks verify shape but not rewrite-correctness of specific files.

- 🔵 **Test Coverage**: AC #9 (documents-locator subcategory groups) verified by grep only, not invocation.

- 🔵 **Test Coverage**: `scripts/research-metadata.sh` literal check (AC #19) has no enumerated test.

- 🔵 **Test Coverage**: Idempotency assertion checks content but not `jj diff --name-only` empty.

- 🔵 **Test Coverage**: design-inventories fixture lacks internal cross-links.

- 🔵 **Test Coverage**: `research_issues` addition test conflates two behaviours (paths-block-present vs absent).

- 🔵 **Safety**: `config.local.md` typically untracked; pre-flight filters `^??` so uncommitted edits there pass unseen.

- 🔵 **Safety**: No dry-run is fine in general, but Step 3's blast radius warrants a pre-mutation intent summary.

- 🔵 **Compatibility**: DIR_DEFAULTS shadow remains as a forward-compat hazard.

- 🔵 **Compatibility**: Forward-compat: older plugin versions don't recognise `paths.research_issues`/`paths.research_codebase`.

- 🔵 **Compatibility**: Phase 7's assertion of "three Renamed lines" contradicts the rewrite-only-if-explicit-override rule when plugin config relies on defaults.

- 🔵 **Documentation**: Four explicit research output groups in documents-locator may over-categorise; consider sub-bullets or "omit empty groups" instruction.

- 🔵 **Documentation**: README meta/ flat-row table loses tree-nesting visualisation for the four research subcategories.

- 🔵 **Documentation**: Migration user-facing output is thin (3 lines + move logs); no completion summary.

- 🔵 **Documentation**: documents-locator legend bullet ordering not aligned to PATH_KEYS as other surfaces are.

- 🔵 **Documentation**: Phase 8 final-verification grep regex `meta/research/[0-9]` only catches date-prefixed references.

### Strengths

- ✅ Strict TDD ordering — tests precede implementation across phases 1-6
- ✅ Step 0 capture explicitly precedes Step 2 rewrite — temporal coupling identified correctly
- ✅ `_move_if_pending` reuse from 0003 — proven idempotent four-state helper
- ✅ Scan corpus derived from `accelerator:paths` — config-driven rather than convention-driven
- ✅ DIR_KEYS/DIR_DEFAULTS shadowing surfaced explicitly in Current State Analysis
- ✅ AC refinements (template-key rename, wider callers, drift fixes) negotiated up-front
- ✅ configure/SKILL.md drift (`integrations`, `design_inventories`, `design_gaps`) bundled into the same atomic commit
- ✅ migrate/SKILL.md `meta/.migrations-*` stale text bundled — opportunistic doc hygiene
- ✅ Phase 7 as integration test against the plugin's own meta/
- ✅ Idempotency, conflict detection, VCS-rename preservation surfaced as explicit success criteria
- ✅ Pattern sources cited with file:line anchors
- ✅ Plain `mv` (not `jj mv` / `git mv`) deliberately chosen to preserve rename history
- ✅ "What We're NOT Doing" section explicit about scope boundaries (visualiser docType strings, historical CHANGELOG entries)

### Recommended Changes

Prioritised by impact. Address criticals before majors; consider grouping
related fixes:

1. **Resolve the value-preservation vs filesystem-move contradiction.**
   (addresses: "Value-preserving config rewrite breaks resolution",
   "NEW_* paths derived from OLD_RESEARCH", "Design-inventories/design-gaps
   overrides silently ignored", "templates.research rename silently
   invalidates")

   Decide: does the rewrite append `/codebase` (or `/design-inventories` /
   `/design-gaps`) when preserving a user value, or does the filesystem
   move target the user's existing `paths.design_inventories` /
   `paths.design_gaps` rather than nesting unilaterally under
   `OLD_RESEARCH`? Update Phase 3, Phase 4, the work item AC #3, and the
   test fixtures in lockstep. Same decision needs to land for
   `templates.research` (extend the migration to rewrite `templates.*`,
   or descope it).

2. **Anchor prefix matching on path-segment boundaries.**
   (addresses: "Prefix matching has no boundary anchor", "Over-rewrite
   risk in quoted/historical contexts" partial)

   Require matched prefix to be followed by `/`, `"`, `'`, whitespace,
   `)`, `]`, `#`, or end-of-line. Apply uniformly across frontmatter
   scalar/list, markdown-link, and prose rewriters. Add explicit
   non-matching fixtures: `meta/researchers.md`, `meta/research-templates/`,
   `meta/research-archive/`.

3. **Replace `probe_legacy_path` with 0003's column-0-anchored awk reader.**
   (addresses: "probe_legacy_path uses config-read-path.sh after Phase 1
   retired the legacy keys")

   Phase 3 code sample should match the References section's pattern
   source (0003:66-102). Read directly from
   `.accelerator/config.{md,local.md}` for the legacy keys, with
   hardcoded legacy defaults as fallback.

4. **Up-front collision check before any `mv` in Step 1.**
   (addresses: "Conflict detection happens too late",
   "Duplicate-key handling missing")

   Enumerate all `(old, new)` pairs, run a single
   pre-loop `check_collisions` pass, exit non-zero with per-conflict
   diagnostics before any mutation. Apply analogously to config rewrite:
   if `paths.research` and `paths.research_codebase` both present in
   user config, refuse to proceed.

5. **Fix Step 1 cleanup to handle `.gitkeep`, `.DS_Store`, dotfiles,
   non-`.md` siblings.**
   (addresses: "rmdir leaves legacy parent dirs behind", "Move loops
   don't handle non-.md siblings", "Silent rmdir failure conflates two
   failure modes")

   Move all regular files (not just `*.md`). Delete `.gitkeep` and
   `.DS_Store` rather than re-homing. After cleanup, if legacy dir
   non-empty, emit one informational line per remaining file and skip
   rmdir cleanly. Add fixtures for each case.

6. **Extend dirty-tree pre-flight to cover Step 3's scan corpus.**
   (addresses: "Dirty-tree pre-flight does not cover Step 3", "config.local.md
   may be untracked", "No VCS at all — rollback story collapses")

   Either extend `run-migrations.sh` to check every path returned by
   `config-read-all-paths.sh`, or have 0056 perform its own pre-flight
   at Step 3 entry. Fail closed when no VCS detected (require
   `ACCELERATOR_MIGRATE_FORCE=1` to bypass). Snapshot `config.local.md`
   to `.accelerator/state/0056-backup-config.local.md` before rewriting.

7. **Specify the `paths.research_issues` insertion algorithm explicitly.**
   (addresses: "research_issues insertion rule underspecified",
   "research_issues addition test conflates two behaviours")

   Document: (a) detect nested-YAML vs flat-dotted form; (b) for
   nested, insert after the last sibling key matching the indent;
   (c) for flat-dotted, append a `paths.research_issues:` line.
   Split the single test into two test cases (paths-block-present vs
   absent).

8. **Address skills upgrade-without-migrate window.**
   (addresses: "Upgrade-without-migrate window leaves skills resolving
   to empty paths")

   Either hard-block (not just warn) in the SessionStart hook when 0056
   is pending, OR add a one-line pre-flight check at each affected
   skill ("Run /accelerator:migrate before using this skill"), OR
   document the hazard prominently in CHANGELOG + migrate/SKILL.md.
   Decide once; apply uniformly.

9. **Expand visualiser scope in Phase 8.2 explicitly.**
   (addresses: "Bash → Rust JSON contract not updated", "Plan leaves
   Rust-resolver coupling vague", "Visualiser server fixtures contain
   legacy literals")

   Enumerate exact files and string identifiers:
   - `skills/visualisation/visualise/scripts/write-visualiser-config.sh`
     (abs_path callers + JSON `--arg` list)
   - `skills/visualisation/visualise/server/src/docs.rs::config_path_key()`
     (returned string constants)
   - `server/tests/fixtures/config.valid.json`,
     `config.optional-override-null.json`
   - `server/scripts/test-launch-server.sh`
   - `frontend/src/routes/lifecycle/LifecycleClusterView.test.tsx`

   Decide: do JSON wire keys move to `research_codebase` etc., or
   stay as legacy strings with the Rust side mapping back? Document
   the decision.

10. **Spell out the 4-clause sed pipeline (Phase 4) or replace with
    structured rewriter.**
    (addresses: "4-clause sed pipeline elision under-specifies")

    Either expand the elided `# … 4-clause sed pipeline from 0001 …`
    inline in the plan with all clauses (per-key, per-form), or
    replace the sed-pipeline approach with an awk/bash function that
    tokenises lines and encodes ordering in data.

11. **Take a snapshot at Step 3 entry for rollback.**
    (addresses: "No rollback if Step 2 or Step 3 fails partway",
    "No isolated rehearsal before destructive Phase 7")

    Run `jj op log` snapshot named "pre-0056" (or equivalent for git)
    immediately before invoking Step 3. Document the op-id as the
    single recovery point. For Phase 7, do a clone-and-rehearse pass
    against a tmp directory before running against the plugin's
    working copy.

12. **Extract the lifted patterns into a shared `migrate-common.sh`
    module.**
    (addresses: "Three-way copy-paste from 0001/0002/0003 entrenches
    duplication", "Generalising 0002 rewriters non-trivial")

    Move `_move_if_pending`, `probe_paths_key`-shaped awk reader, and
    the inbound-rewriter trio into `skills/config/migrate/scripts/
    migrate-common.sh`. If the team consciously rejects extraction
    (e.g., migrations should be snapshot-frozen), state that tradeoff
    explicitly in the plan.

13. **Strengthen test fixtures for the diverse real-world shapes.**
    (addresses: "Fixture diversity narrow", "Prose rewriter only tested
    in fenced code blocks", "config.local.md rewrite path untested",
    "No test covers refs inside files being moved", "VCS rename only
    tested for jj not git")

    Add fixtures: `.gitkeep` in moved categories, `.DS_Store` siblings,
    partial-state re-run, mixed-config (only one key overridden),
    destination-non-empty, prose backtick-inline, prose bullet-list,
    prose narrative, config.local.md-only override, moved-file
    internal links, .git-only VCS.

14. **Address documentation surface gaps.**
    (addresses: "configure/SKILL.md line 724", "Four template files
    reference legacy paths", "paths.research_issues introduction
    buried")

    Add `configure/SKILL.md:724` to Phase 6's edit list. Add
    `templates/{adr,work-item,plan,research}.md` to Phase 8's edit
    list (and rewrite their body references). Expand the CHANGELOG
    entry's `paths.research_issues` paragraph to explain the new
    bucket's purpose for `research-issue` outputs.

15. **Emit a migration completion summary line.**
    (addresses: "Three notification lines + move logs is thin
    communication", "Inbound-link rewriting once-only")

    Print one final summary at the end of the migration: file counts
    per destination, inbound rewrites count, new key introduction,
    pointer to CHANGELOG. Document this as informational/human-readable
    only (not a downstream contract).

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is well-structured, follows established migration
framework patterns, and articulates phase ordering clearly. However,
there is a load-bearing architectural inconsistency between
filesystem-move targets and config-key value preservation that breaks
path resolution when users have customised the legacy paths, plus
concerns about how the migration probes legacy values after Phase 1 has
already mutated the path-key registry, and about the migration
unilaterally nesting design-inventories/design-gaps under
`OLD_RESEARCH` irrespective of explicit user overrides.

**Strengths**:
- TDD-first phase ordering consistent with framework reliability posture
- Phase 7 as integration test on plugin's own meta/
- Clear pattern-source traceability with file:line references
- Scan-corpus generalisation from hardcoded glob to config-driven union
- DIR_KEYS/PATH_KEYS updated in lockstep per Key Discoveries
- Plan addresses pre-existing drift opportunistically

**Findings**: See Critical and Major sections above (8 findings total).

### Correctness

**Summary**: Migration ordering invariant identified correctly (capture →
moves → config-rewrite → inbound-rewrite) and 0003's `_move_if_pending`
proven idempotency helper reused. But: prefix matcher has no path
boundary anchor, file-move loops miss `.gitkeep`/dotfile cases leaving
stale legacy directories, partial-migration mixed states unhandled, and
the insertion rule for the new `paths.research_issues` key is
underspecified.

**Strengths**:
- Step 0 explicitly precedes Step 2
- `_move_if_pending` reuse from 0003
- Scan corpus post-config-rewrite so overrides flow through
- Plan acknowledges partial-state hazard for new key
- Negative-test (byte-identity) pattern for non-matching references

**Findings**: 12 findings — 2 critical, 5 major, 5 minor (see above).

### Test Coverage

**Summary**: Plan is genuinely test-first with ~18 enumerated test
cases and good happy-path shape coverage, but several AC items map
only to grep-based or manual assertions, and the edge-case matrix is
thin: no fixture covers .gitkeep / .DS_Store / partial-state re-runs /
mixed legacy-key state / git-only VCS / config.local.md / refs
embedded inside files being moved. Prose rewriter is restricted to
fenced code blocks despite many out-of-fence shapes in the real corpus.

**Strengths**:
- Strict TDD (tests before implementation)
- Non-matching byte-identity tests per shape
- Three-level idempotency tests
- Scan-corpus-from-config test covers AC #4 novelty
- Reuses 0002/0003 fixture conventions
- Explicit conflict-at-destination test

**Findings**: 13 findings — 5 major, 8 minor.

### Code Quality

**Summary**: Plan is well-organised and TDD-driven, but "lift verbatim"
from 0001/0002/0003 is copy-paste-by-design that bakes in long-term
maintenance debt. Several phases under-specify non-trivial logic
(especially the 4-clause sed pipeline and the generalised inbound
rewriters) while over-specifying low-level shell scaffolding. The
DIR_KEYS/DIR_DEFAULTS shadow is patched in two places rather than
eliminated.

**Strengths**:
- TDD-first phase organisation
- Current State Analysis names specific file lines
- Step 0/Step 2 temporal coupling identified
- Idempotency/conflict-detection/VCS-rename as testable success criteria
- AC refinements + non-goals negotiated up-front

**Findings**: 8 findings — 3 major, 4 minor, 1 suggestion.

### Safety

**Summary**: The plan inherits a useful framework-level dirty-tree
guard but the guard's coverage is materially narrower than the
migration's blast radius. The plan is non-atomic across its three
steps, has no rollback if Step 2 or 3 fails partway, and Phase 7
applies the migration to the plugin's own repo without an isolated
rehearsal. Recovery depends entirely on the user having a clean VCS
state — but the guard does not enforce a clean tree outside the
meta/.accelerator perimeter that Step 3 rewrites.

**Strengths**:
- Recovery story explicit (jj op restore / git reset)
- Idempotency first-class with explicit second-run tests
- Collision detection planned and tested
- Plain mv preserves rename history
- Phase 7 enumerates concrete post-conditions
- Step 0 capture before Step 2 mutation

**Findings**: 9 findings — 2 critical, 5 major, 2 minor.

### Compatibility

**Summary**: Breaking config-schema change (three path renames, one
template rename, one new key) with `accelerator:migrate` as the bridge.
The migration design is sound, but there is a hard window between
plugin upgrade and migrate-run during which userspace skills resolve
unknown keys to empty paths. Several specific compatibility risks are
also under-addressed: silent override-loss for `templates.research`,
the bash↔Rust JSON contract for visualiser `doc_paths`, the
notification-line format being claimed as a contract, and the
once-only nature of inbound-link rewriting versus content imported
later.

**Strengths**:
- Migration is idempotent and inherits dirty-tree guard
- Custom override values preserved verbatim (correct call)
- Existing SessionStart hook warns about pending migrations
- Scan corpus config-driven (covers relocated dirs)
- Phase 7 as end-to-end integration test

**Findings**: 9 findings — 2 critical, 5 major, 2 minor.

### Documentation

**Summary**: Phase 8 narrative-surface coverage is thoughtful and
tracks the README's seven cited locations, but several documentation
surfaces are unaddressed: `configure/SKILL.md` line 724 hardcodes
`meta/research/`, four `templates/*.md` reference legacy paths in
their bodies, and the new `paths.research_issues` key has no
narrative introduction telling users what it is for. CHANGELOG
approach risks leaving historical entries with broken-looking path
references, and the documents-locator changes correctly model four
research groups but the legend bullet still calls out a single
`research` group in prose.

**Strengths**:
- Phase 8 enumerates README hot-spots with exact replacements
- configure/SKILL.md drift fix bundled in
- migrate/SKILL.md stale text fix bundled in
- documents-locator changes cover three layers (legend, template, intent)
- paths/SKILL.md legend update called out explicitly
- CHANGELOG entry includes user-actionable migrate guidance

**Findings**: 10 findings — 5 major, 5 minor.

---

## Re-Review (Pass 2) — 2026-05-12

**Verdict:** REVISE

The revision substantially raised the safety and design baseline:
all 8 critical findings from pass 1 are resolved, all 10
documentation findings are resolved, and the test-coverage matrix
nearly doubled. The four design decisions (D1–D4) are well-recorded
and propagate coherently through Phases 3–8. However, the
substantial new bash code introduced in the revision carries **2
new critical findings** (both correctness defects that would prevent
the migration from running correctly) and ~13 new major findings,
mostly concentrated in implementation-detail gaps. Verdict remains
REVISE: criticals must be addressed before implementation can begin.

### Previously Identified Issues

**Critical (8):**
- 🔴 **Correctness**: Prefix matching boundary anchor — **Resolved** (perl with `\Q…\E` + character-class `(?=[/"'\s)\]#]|$)`)
- 🔴 **Correctness**: Mixed-state runs corrupt placement — **Resolved** (mixed-state refusal in Phase 3 lines 748-756)
- 🔴 **Architecture**: Value-preserving rewrite under user overrides — **Resolved** (D1 + `_xform_append_codebase` vs `_xform_identity`)
- 🔴 **Architecture**: Design-inv/gaps overrides silently ignored — **Resolved** (`INV_HAD_OVERRIDE` / `GAPS_HAD_OVERRIDE` flags branch the planning logic; new pair in Step 3 excluded when no move)
- 🔴 **Safety**: Dirty-tree pre-flight scope — **Resolved** (Phase 5 `preflight_scan_corpus_clean`)
- 🔴 **Safety**: Atomicity/rollback story — **Partially resolved** (cross-step invariants documented; jj op-id capture deferred to manual operator step — see new finding)
- 🔴 **Compatibility**: Upgrade-without-migrate window — **Resolved** (D2 documentation-only with CHANGELOG explicit hazard description)
- 🔴 **Compatibility**: Bash→Rust JSON contract — **Resolved** (D3 + Phase 8.2 explicit file enumeration)

**Major (26):** All structurally addressed. Selected highlights:
- 🟡 `rmdir … || true` swallows non-empty errors — **Resolved** (`_cleanup_legacy_parent` diagnostic)
- 🟡 paths.research_issues insertion underspecified — **Resolved** (algorithm spelled out per form)
- 🟡 Duplicate-key handling — **Resolved** (mixed-state refusal)
- 🟡 Move loops don't handle .gitkeep/.DS_Store — **Resolved** (`shopt -s dotglob` + explicit .DS_Store skip)
- 🟡 4-clause sed pipeline elision — **Resolved** (awk-based per-form rewriters spelled out)
- 🟡 Conflict detection too late — **Resolved** (up-front `_check_collisions`)
- 🟡 No isolated rehearsal before Phase 7 — **Resolved** (`mktemp -d` clone rehearsal)
- 🟡 templates.research silently invalidates overrides — **Resolved** (D4 + `_rewrite_template_pair` in Phase 4)
- 🟡 Inbound rewriting once-only — **Partially resolved** (documented; env-var promise undefined — see new finding)
- 🟡 Forward-compatibility — **Partially resolved** (not yet documented in CHANGELOG)
- 🟡 configure/SKILL.md:724 — **Resolved**
- 🟡 Template body references — **Resolved**
- 🟡 Visualiser server fixtures — **Resolved** (Phase 8.2 enumerates `config.valid.json`, `config.optional-override-null.json`, `test-launch-server.sh`, `common/mod.rs`, `LifecycleClusterView.test.tsx`)

**Minor (28):** All addressed or downgraded. Phase 7 "three Renamed lines" assertion corrected to zero. README meta/ table now tree-style with prelude. documents-locator legend ordering aligned to PATH_KEYS. Final-verification grep broadened.

### New Issues Introduced

**Critical (2):**

- 🔴 **Correctness**: HAD_OVERRIDE flag written inside `$()` subshell never reaches caller
  **Location**: Phase 3 Step 0 (probe_legacy_path lines 694-721)
  `probe_legacy_path` writes the override-detection flag via
  `printf -v "$out_had_override_var" '1'`, but every call site
  captures stdout with `OLD_RESEARCH=$(probe_legacy_path …)`. The
  `$(…)` subshell discards the assignment. `RESEARCH_HAD_OVERRIDE`,
  `INV_HAD_OVERRIDE`, `GAPS_HAD_OVERRIDE` remain `0` regardless of
  whether keys are present. **D1 honor-overrides semantics collapse
  to the no-override branch for every user** — design-inv/gaps will
  always move, defeating the work of this revision.

- 🔴 **Correctness/Code-Quality**: awk references undefined
  `shellquote()` and uses gawk-only 3-arg `match()`
  **Location**: Phase 4 rewrite_one_key (lines 1004, 1006, 1017, 1019)
  The awk in `rewrite_one_key` calls `shellquote(oldval)` (never
  defined) and uses `match($0, regex, m)` 3-arg form (gawk-only;
  fails silently on macOS BSD `/usr/bin/awk`). Phase 4 rewriter
  cannot execute on macOS without gawk, and the function reference
  is a parse error in any awk. **The migration cannot rewrite user
  config at all on macOS without gawk installed.**

**Major (13):**

- 🟡 **Safety/Correctness**: Step 3 pre-flight runs AFTER Steps 1+2 already mutated the working tree — dirty scan corpus aborts mid-migration with files already moved and config already rewritten. Move `preflight_scan_corpus_clean` to before Step 1.
- 🟡 **Correctness**: Mixed-state refusal fires on any new key being present, including post-successful-prior-run state — breaks idempotency claim (second run will fatal).
- 🟡 **Correctness**: Negative-lookbehind regex requires Perl 5.30+; no pre-flight version check. Older Perl (macOS Mojave 5.18, Linux LTS variants) will error mid-Step-3 after Steps 1-2 already mutated.
- 🟡 **Correctness**: `_insert_research_issues_if_needed` injects default-value override into any config with a non-absent `paths:` block, even when the user never overrode research — pinning the user to the legacy-default-derived path.
- 🟡 **Correctness**: probe_legacy_path awk doesn't strip inline `# comments` from values; treats explicit empty value as absent.
- 🟡 **Code Quality**: Undefined helpers — `fatal`, `log_info`, `log_error` are not defined in `log-common.sh` or anywhere else. Three different conventions are mixed across Phase 3 and Phase 5.
- 🟡 **Code Quality / Compatibility**: `_rewrite_template_pair` function body is referenced (Phase 4 line 1089) but never defined — D4 implementation is underspecified, including templates-form-detection and mixed-state handling.
- 🟡 **Compatibility/Documentation**: `ACCELERATOR_MIGRATE_FORCE_RERUN=1` promised in CHANGELOG and Migration Notes but never implemented — contract/behaviour mismatch.
- 🟡 **Compatibility**: `ACCELERATOR_MIGRATE_FORCE` env var overloaded (dirty-tree bypass AND no-VCS bypass) — single flag controls two distinct safety bypasses with different recovery implications.
- 🟡 **Compatibility**: Notification line oscillates between "contract surface" (Phase 4 success criteria require exact-line match) and informational (interleaved with `log_info` and Step 3 banner). Pick one.
- 🟡 **Compatibility**: Mixed-state refusal can't distinguish intentional pre-migration hand-edits from leftover aborted-run state — both get blocked with the same opaque "Resolve manually and retry" message.
- 🟡 **Safety**: jj op-id capture is a manual operator step in Phase 7 instructions, but Migration Notes claims the op-id is "captured at Step 3 entry and printed to stderr". End-user repos get no breadcrumb from the script.
- 🟡 **Test Coverage**: Double-substitution guarantee (negative-lookbehind) has no automated test for partially-rewritten input. The single mutation-resistant test the regex needs.

**Minor (~10):** various — see per-lens output below. Highlights:
- 🔵 **Code Quality**: ~400-line migration script with 17+ helpers; deferred extraction now harder, not easier
- 🔵 **Code Quality**: `printf -v` indirect-assignment pattern with `_unused=""` placeholder is action-at-a-distance
- 🔵 **Test Coverage**: No-VCS bypass branch (`ACCELERATOR_MIGRATE_FORCE=1`) untested
- 🔵 **Test Coverage**: `_xform_*` and `detect_paths_form` helpers tested only via integration
- 🔵 **Documentation**: External file line-number anchors (~line N) will drift before merge
- 🔵 **Safety**: `dotglob` toggling is per-helper but not subshell-scoped — implicit state for callers
- 🔵 **Safety**: Expanded prose rewriter (narrative, bullet-bare, backtick-inline) increases over-rewrite risk on intentionally-quoted historical paths inside ADRs/plans
- 🔵 **Safety**: `config.local.md` (conventionally untracked) gets rewritten with no backup

### Assessment

The revision is a substantial improvement: every prior critical and
major has been structurally addressed, and the design decisions are
sound. However, the substantial new bash code carries **two
critical implementation defects** (HAD_OVERRIDE subshell bug, awk
portability) that would prevent the migration from running
correctly on the target platform. Both are easily fixable but
demand another revision pass.

The pattern of the new findings — undefined helpers, subshell
semantics, env-var promises without implementation, ordering
hazards — suggests the plan's bash specifications would benefit
from an executable dry-run pass (run each snippet against a real
fixture, in isolation, before declaring the plan finalised). The
plan's TDD ethos covers post-implementation tests but not the
pre-implementation viability of the spec itself.

Recommended next actions before re-review:
1. Fix HAD_OVERRIDE to use stdout return channel (not subshell-captured `printf -v`).
2. Replace gawk-only `match($0, re, m)` with `~` + `sub()` (mirror 0003's pattern); define or eliminate `shellquote()`.
3. Hoist `preflight_scan_corpus_clean` and `_check_collisions` to a Step 0 phase before Steps 1-2 mutate.
4. Either implement `ACCELERATOR_MIGRATE_FORCE_RERUN=1` or remove the promise from CHANGELOG/Migration Notes.
5. Define `_rewrite_template_pair` body explicitly OR parameterise `_rewrite_pair`.
6. Define `fatal`, `log_info`, `log_error` (or pick existing convention from 0001/0002/0003 — `log_warn` + `echo … >&2; exit 1`).
7. Add Perl version pre-flight; either require 5.30+ explicitly or use fixed-width lookbehind.
8. Fix mixed-state refusal: only fail when BOTH legacy AND renamed are present (current logic blocks idempotent re-runs and intentional pre-migration users).
9. Fix probe_legacy_path awk to strip inline `# comments` and distinguish absent vs empty.
10. Add automated tests for: double-substitution recovery, no-VCS bypass, `_xform_*` direct calls.
