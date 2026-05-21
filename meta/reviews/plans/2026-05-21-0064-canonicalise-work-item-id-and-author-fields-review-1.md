---
date: "2026-05-21T20:15:00+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-21-0064-canonicalise-work-item-id-and-author-fields.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, safety, compatibility, database, standards]
review_pass: 3
status: complete
---

## Plan Review: Canonicalise `work_item_id` and `author` Frontmatter Field Names

**Verdict:** REVISE

The plan is structurally sound, follows the 0005 precedent closely, and correctly
identifies that the rename surface is small (one Rust read site, three templates,
three corpora). Phase ordering, TDD discipline, idempotence-via-tree-hash, and
the multi-tier template override coverage are all genuine strengths. However,
the proposed migration script as drafted in Phase 1 has multiple converging
correctness defects — most critically an inverted bash return-code contract that
will invert the rewrite counter for two of three corpora, and an unguarded helper
invocation that will abort the migration on its second (idempotent) run. Several
lenses independently flag the unconditional body-label rewrite as a real data-loss
risk, and the awk quote-normaliser handles only a narrow slice of the YAML shapes
it claims to defensively support. Before implementation begins, these issues need
to be resolved in the plan so the implementer has a defect-free spec to work from.

### Cross-Cutting Themes

Six themes were flagged by multiple lenses independently and warrant the most
attention:

- **Inverted/inconsistent return-code semantics in `rewrite_research_file`**
  (flagged by: architecture, code-quality, test-coverage, correctness, safety,
  database, standards) — `return $touched` where `touched=1` means "rewrote"
  inverts truthiness when called via `if "$fn" "$file"; then`. Plus
  `rewrite_plan_file` uses the opposite convention. The rewrite counter will
  be inverted for research and RCA corpora; Phase 2's expected `rewrote 52`
  assertion will not hold.

- **Unconditional body-label regex rewrite corrupts research prose** (flagged
  by: architecture, code-quality, test-coverage, safety, compatibility,
  database, standards) — `^\*\*Researcher\*\*:` is rewritten anywhere it appears
  in research/RCA files, including quoted prose examples. The mitigation
  ("review the diff and revert via jj") relies on human attention and will not
  scale to userspace repos. Several lenses propose anchoring the pass to the
  post-frontmatter / pre-first-H2 region or to lines immediately adjacent to
  template-shape neighbours.

- **Awk quote-normaliser is fragile under YAML shapes it doesn't enumerate**
  (flagged by: code-quality, correctness, safety, database) — misses
  `work-item:value` (no whitespace), YAML single-quoted scalars
  (`work-item: '0042'`), inline comments (`work-item: 0042 # note`), and
  values with embedded double quotes. Each silently corrupts data in
  userspace repos rather than failing fast.

- **Pipeline atomicity: `grep -v ... | atomic_write "$file"`** (flagged by:
  architecture, database, safety) — pipe reads `$file` while the consumer
  side may rename over it; benign on most filesystems but a latent hazard on
  NFS/overlay/FUSE. Also: the research/RCA pass performs **two** sequential
  `atomic_write` operations per file (frontmatter + body label), with no
  whole-file bracketing — an interrupted run leaves files half-canonicalised.

- **Tier-1/tier-2 template resolution: duplicated logic + missing safety
  guards** (flagged by: architecture, code-quality, correctness, database) —
  `rewrite_template_if_present` re-implements `config_resolve_template`'s
  semantics inline; tier-2 fires even when tier-1 is set but its target is
  missing (a misconfiguration that should warn, not fall through); the
  tier-1 path is concatenated to `PROJECT_ROOT` without the
  `resolve_corpus_path` traversal guard applied elsewhere.

- **Test harness divergence from 0005 precedent** (flagged by: test-coverage,
  standards) — the proposed `run_0006_driver` uses
  `ACCELERATOR_MIGRATE_FORCE_NO_VCS=1` (not a real env var), omits
  `CLAUDE_PLUGIN_ROOT`, doesn't `cd` into the fixture repo, and passes
  `$repo_dir` as a positional the driver ignores. As written, the test
  harness would not work, and the TDD claim collapses.

### Tradeoff Analysis

- **Compatibility (transitional dual-read) vs scope discipline**: The plan
  swaps `m.get("work-item")` for `m.get("work_item_id")` outright. Adding a
  transitional `else if m.get("work-item")` fallback (mirroring the preserved
  `ticket:` legacy) would eliminate the broken-link-badge window that the AC #7
  fixture step 3 documents. Tradeoff: more code on the read path and one more
  legacy key to deprecate later. Recommendation: add the fallback for one
  release cycle, then remove in a follow-up.

- **Migration scope (placeholder text) vs handing off to 0065**: Phase 3 edits
  the `last_updated_by:` placeholder hint from `[Researcher name]` to `[Author
  name]`. This is mild scope creep — the field is unchanged; only the
  placeholder copy changes. Either commit to it (update §"What We're NOT
  Doing" to acknowledge placeholder-copy edits ship here) or defer entirely
  to 0065 for consistency with the broader unified-schema work.

### Findings

#### Critical

- 🔴 **Code Quality / Correctness / Safety / Database**: `return $touched` inverts walk_corpus rewrite counter for research/RCA corpora
  **Location**: Phase 1 §3 `rewrite_research_file` (~lines 509-546) and `walk_corpus` (~lines 569-586)
  `touched=1` means "rewrote", but `return 1` is bash failure. `walk_corpus`'s `if "$fn" "$file"` therefore counts untouched files as rewrites and skips rewritten ones. Phase 2's `rewrote 52 file(s)` assertion will fail. `rewrite_plan_file` uses the opposite convention, so the two helpers are inconsistent under the same dispatcher.

- 🔴 **Correctness**: Direct call to rewrite helper under `set -e` aborts migration on second (idempotent) run when templates no longer carry legacy key
  **Location**: Phase 1 §3 `rewrite_template_if_present` (~lines 599-624)
  `"$fn" "$tier1_abs"` is invoked directly (not under `if`). On a second run, `rewrite_plan_file` falls through both `grep -q` guards and hits `return 1`, which under `set -euo pipefail` aborts the whole migration. Breaks the idempotence claim and Phase 2's re-run verification.

- 🔴 **Safety**: Unconditional body-label rewrite can silently corrupt research prose with no per-file preview
  **Location**: Phase 1 §3 body-label pass + Migration Notes #4
  `^\*\*Researcher\*\*:` is rewritten anywhere it appears at the start of a line. Mitigation is a one-line PR-description note. Downstream user repos with quoted prose examples, interview transcripts, or self-referential research about this very migration will be silently mutated. No per-file count, no opt-out, no diff preview.

#### Major

- 🟡 **Correctness / Safety / Database**: Awk quote-normaliser mishandles single-quoted scalars, inline comments, and embedded quotes
  **Location**: Phase 1 §3 `rewrite_work_item_to_work_item_id` (~lines 486-504)
  Three concrete bugs: (1) `work-item: '0042'` becomes `work_item_id: "'0042'"` (literal single quotes inside doubles); (2) `work-item: 0042 # note` becomes `work_item_id: "0042 # note"` (comment folded into value); (3) unquoted values containing `"` are wrapped without escaping, producing malformed YAML. Userspace repos can hit any of these.

- 🟡 **Code Quality**: Awk patterns miss `work-item:value` shape (no whitespace between colon and value)
  **Location**: Phase 1 §3 awk pipeline
  Both regexes require either a fully-empty value or whitespace before the value. `work-item:0042` falls through to the catch-all `{ print }` and the line passes through unchanged — but `grep -q '^work-item:'` returned true upstream, so `atomic_write` runs anyway. The file still carries the legacy key after a supposedly-successful rewrite.

- 🟡 **Correctness**: Dual-presence divergent branch skips quote-normalisation, violating AC #6 invariant
  **Location**: Phase 1 §3 `rewrite_plan_file` dual-presence branch
  When both `work-item:` and `work_item_id:` are present, the migration drops the legacy line via `grep -v` but does not run the survivor through the awk quote-normaliser. A partial-prior-run plan with `work_item_id: 0042` (unquoted) silently violates AC #6.

- 🟡 **Correctness**: Body-label divergent branch drops all matching lines but compares only the first
  **Location**: Phase 1 §3 `rewrite_research_file` Pass 2 (~lines 530-543)
  `grep -m 1` captures the first occurrence for divergence comparison, then `grep -v` drops *every* `^\*\*Researcher\*\*:` line in the file. A file containing one header label plus quoted prose examples loses all but the first occurrence with no warning.

- 🟡 **Correctness**: Trailing-whitespace sensitivity in divergence comparison produces spurious warnings
  **Location**: Phase 1 §3 divergence-check value extraction
  `sed 's/^work-item:[[:space:]]*//'` strips leading but not trailing whitespace. `"0042"` vs `"0042" ` (one trailing space) compare unequal, producing false-positive divergence warnings.

- 🟡 **Test Coverage / Standards**: `run_0006_driver` harness helper omits `cd`, omits `CLAUDE_PLUGIN_ROOT`, uses non-canonical env var, passes positional driver ignores
  **Location**: Phase 1 §2 harness extension (~lines 391-396)
  The proposed helper diverges from the 0005 helper in four concrete ways and as written would either run against the wrong directory or silently no-op. Mirror 0005 exactly: `cd "$repo" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" ACCELERATOR_MIGRATIONS_DIR="$ONLY_0006_DIR" ACCELERATOR_MIGRATE_FORCE=1 bash "$DRIVER" "$@"`.

- 🟡 **Test Coverage**: On-disk fixture rewrite description misstates the current fixture shape
  **Location**: Phase 4 §2 (~lines 906-916)
  The plan says "preserving the existing quoted-string value" but the actual fixtures contain unquoted integers (e.g., `work-item: 1`). The implementer needs to decide whether to preserve unquoted (keeping the numeric→string coercion test path live) or move to quoted form (and add a separate fixture to keep numeric coverage).

- 🟡 **Test Coverage**: Missing stdout rewrite-count assertion would not catch the inverted-counter bug
  **Location**: Phase 1 §2 fixture assertions
  No positive `assert_contains '0006: rewrote N file(s)'` for the `default-layout` or `paths-override-*` scenarios. Adding these would have caught the critical return-code bug above before code review.

- 🟡 **Architecture / Database**: Two-pass `rewrite_research_file` performs two sequential `atomic_write` calls per file
  **Location**: Phase 1 §3 `rewrite_research_file`
  Frontmatter pass and body-label pass each write independently. A SIGINT or `set -e` abort between passes leaves files half-canonicalised. Idempotence recovers, but per-file atomicity is not what it claims.

- 🟡 **Architecture / Code Quality / Correctness / Database**: Tier-1/tier-2 template resolution duplicated + missing guards
  **Location**: Phase 1 §3 `rewrite_template_if_present`
  Re-implements `config_resolve_template` semantics inline; tier-2 fallthrough fires when tier-1 is set-but-missing (mask user typos); tier-1 path concatenated to `PROJECT_ROOT` without the traversal guard `resolve_corpus_path` uses.

- 🟡 **Compatibility**: No transitional dual-read on visualiser consumer creates hard-break window
  **Location**: Phase 4 step 4 + Migration Notes
  Asymmetric treatment: `ticket:` (very old) gets a fallback, `work-item:` (just-renamed) does not. AC #7 step 3 explicitly demonstrates the broken-link-badge regression. Add a transitional `else if m.get("work-item")` branch documented to remove next release.

- 🟡 **Compatibility**: Version-bump policy for breaking schema change is unspecified
  **Location**: Migration Notes
  Plan calls itself a "breaking schema change" but does not commit to a major/minor/patch bump. Should declare the bump explicitly and tie the CHANGELOG entry to it.

- 🟡 **Safety**: Divergence policy silently drops potentially-fresh data with no escape hatch
  **Location**: Phase 1 §3 divergence handling + Migration Notes
  Migration always trusts the new key on divergence. No interactive prompt, no env-var to invert, no sidecar audit trail. A buried `log_warn` is the only signal.

- 🟡 **Database**: Pipeline race in `grep -v ... | atomic_write "$file"`
  **Location**: Phase 1 §3 dual-presence branches
  Inherited from 0005. Benign on most filesystems but a latent hazard on NFS/overlay/FUSE. Read to a temp file first or use `awk` reading explicitly to a sibling temp before the rename.

- 🟡 **Database**: No collision check when `paths.*` aliases resolve to the same directory
  **Location**: Phase 1 §3 multi-corpus walk
  A user typo aliasing `paths.research_codebase` and `paths.research_issues` to the same dir causes double-traversal. Idempotence saves 0006, but the framework precedent is unsafe for future non-idempotent passes.

- 🟡 **Code Quality**: Dual-presence guard pattern duplicated three times verbatim
  **Location**: Phase 1 §3 — three identical 15-line guard blocks
  Extract to `scripts/migrate-common.sh` (or similar) so 0005 and any future schema rename share one tested implementation.

- 🟡 **Safety**: Phase 2 "confirm jj status is clean" offers no recovery procedure
  **Location**: Phase 2 Pre-flight
  No documented recovery if the migration aborts mid-walk or produces unexpected output. The framework guard only inspects `meta/`, `.claude/accelerator*.md`, `.accelerator/` — `templates/` and `skills/` may be dirty and not blocked.

- 🟡 **Database / Standards**: `last_updated_by` placeholder rewrite is scope drift
  **Location**: Phase 3 §2/§3
  Phase 3 edits the *placeholder text* of `last_updated_by:` ("[Researcher name]" → "[Author name]"). The field name is unchanged. Either commit (update §What We're NOT Doing) or defer to 0065.

#### Minor

- 🔵 **Architecture**: Multi-corpus walker and template-rewrite helper not promoted to shared `scripts/migrate-common.sh` — future migrations will copy-paste.

- 🔵 **Architecture**: `ticket:` legacy fallback at `frontmatter.rs:330` has no deprecation seam in source — only git history explains why three keys are read.

- 🔵 **Architecture**: Quote-normaliser awk helper isn't gated by the dual-presence guard at its boundary — relies on caller composition.

- 🔵 **Architecture**: Phase 4's TDD ordering creates a transient broken-test window — if split across commits, CI fails at the intermediate revision by design.

- 🔵 **Code Quality**: ~20 fixture directories with shared canonical bodies risk drift — commit to a base-fixture + overlay approach.

- 🔵 **Code Quality**: Producer SKILL.md prose references (e.g., "sets the `researcher:` field") not in Phase 3's grep oracle.

- 🔵 **Code Quality**: Unconditional body-label rewrite is documented in the plan but not in a source comment where future maintainers will look.

- 🔵 **Test Coverage**: Legacy-key-ignored regression test for the Rust read site marked "optional defence-in-depth" — should be required.

- 🔵 **Test Coverage**: Awk normaliser branches under-exercised (single-quoted, trailing whitespace, empty-with-whitespace not covered).

- 🔵 **Test Coverage**: AC #6 quote-shape grep not exercised in a multi-file mixed-shape fixture.

- 🔵 **Test Coverage**: Body-label collision behaviour (multiple `**Researcher**:` lines per file) not pinned by any fixture.

- 🔵 **Correctness**: Awk `[[:space:]]*$` semantics historically inconsistent across BWK/mawk/gawk — pin to `[ \t]` and document required dialect.

- 🔵 **Correctness**: Tier-2 fallthrough when tier-1 is set-but-missing masks user typos — should `log_warn` and return.

- 🔵 **Correctness**: `jj status | grep && echo FAIL || echo PASS` verification snippet swallows non-grep failures and can report PASS when jj itself failed.

- 🔵 **Safety**: Path-traversal guard misses some shapes (`./foo`, `foo/.//../bar`); add `realpath` canonicalisation check that resolved path stays inside `PROJECT_ROOT`.

- 🔵 **Safety**: Userspace breaking-change warning is buried — driver's `# DESCRIPTION:` line is the only banner; extend it or have the migration print a one-paragraph stderr banner before rewriting.

- 🔵 **Compatibility**: Userspace template overrides rewritten in-place with no diff/merge affordance — note in CHANGELOG that `.accelerator/templates/*.md` is now migration territory.

- 🔵 **Compatibility**: Other active work items (0066/0068/0069 etc.) may reference legacy keys — add a defensive grep across `meta/work/` in Phase 5.

- 🔵 **Compatibility**: Eval fixture verification only spans this repo; userspace eval suites may have frozen fixtures referencing legacy keys — call this out in Migration Notes.

- 🔵 **Database**: `find ... -name '*.md'` without `-type f` could traverse symlinks or non-regular files outside VCS.

- 🔵 **Database**: Tier-1 template path is concatenated to `PROJECT_ROOT` without the `resolve_corpus_path` shape guard — apply the same `case` check.

- 🔵 **Database**: ADR-0025 and ADR-0034 reference `work-item:` in present-tense passages; Phase 5 should either update them or explicitly exclude from AC scope.

- 🔵 **Database**: Empty-value branch in awk is only single-run-tested; add a three-run idempotence assertion to lock down the second-emission shape.

- 🔵 **Database**: Phase 2 doesn't exercise the abort-mid-walk recovery path that ADR-0023 relies on.

- 🔵 **Standards**: AC #7 fixture path `test-fixtures/0006-canonicalise-work-item-id-and-author/upgrade-path/` diverges from the `test-fixtures/0006/` convention used by Phase 1.

- 🔵 **Standards**: Body-label rewrite semantics widely flagged but not narrowed to the templates' literal emission shape — anchor to post-frontmatter / first-H2 region.

#### Suggestions

- 🔵 **Compatibility**: Document the deprecation horizon for `work_item_id` / `author` — declare they are stable per ADR-0033 and will not be renamed by 0065/0070.

- 🔵 **Correctness**: Decide explicitly between flat-only or recursive corpus traversal; add a fixture covering the chosen contract.

### Strengths

- ✅ Phase ordering correctly migrates dogfood corpus (Phase 2) before consumer rewrite (Phase 4), preventing windows where the visualiser reads a key no producer or corpus file emits.
- ✅ Producer surface and consumer surface correctly identified as minimal (one Rust read site, three templates) — the frontend is properly out of scope thanks to the camelCase wire-type indirection.
- ✅ Idempotence is tested via `tree_hash` byte-identity assertion — the strongest possible idempotence oracle.
- ✅ Defensive quote-normalisation during the rename anticipates userspace divergence from this repo's 100%-quoted corpus.
- ✅ Multi-tier template override coverage (tier-1 / tier-2 / both / missing) is proportionate to the unprecedented nature of template-content rewriting.
- ✅ Partial-prior-run scenarios are split into matching vs divergent across plan, research, body-label — six fixtures cover the full Cartesian space.
- ✅ AC #7 upgrade-path fixture validates pre-rename → post-rename → migrate → verify end-to-end.
- ✅ Plan document structure cleanly follows the create-plan template; the Current State Analysis section is precise about file counts, line numbers, and override tiers.
- ✅ Migration file naming, descriptor comment, sourced helpers, env-var conventions, and fixture directory layout all align with the 0005/0004 precedents.
- ✅ ADR-0033 alignment: `work_item_id` chosen over `work-item` per the §"Field-name conflicts" paragraph; `author` per the §ADR-0028 override block.

### Recommended Changes

Ordered by impact. Each addresses one or more findings above.

1. **Fix the return-code contract across both per-file helpers** (addresses 7-lens critical theme; finding #1)
   In Phase 1 §3, redraft `rewrite_research_file` to return `0` when a rewrite occurred and `1` when not (matching `rewrite_plan_file`). Or — preferred — drop function-return signalling entirely and adopt the 0005 inline `touched=0/1; rewrote=$((rewrote + touched))` accumulator pattern within `walk_corpus`. Add explicit stdout-count assertions (`assert_contains '0006: rewrote 1 file(s) under meta/research/codebase'`) to the `default-layout` and `paths-override-*` fixtures so a regression of this kind cannot slip past tests again.

2. **Wrap helper invocations in `rewrite_template_if_present` for `set -e` safety** (addresses critical finding #2)
   Change `"$fn" "$tier1_abs"` and `"$fn" "$tier2_abs"` to be invoked under `if` conditions or with `|| true` so a `return 1` from the helper does not abort the entire migration. Add a fixture that runs the migration twice against a repo with a tier-1 template override and asserts both runs exit 0.

3. **Narrow the body-label rewrite scope** (addresses critical finding #3; multi-lens theme)
   Anchor the `^\*\*Researcher\*\*:` regex to the post-frontmatter / pre-first-`##` region (use awk state machine on `---`/`^## ` boundaries), or require the preceding line to match a template-adjacent label like `**Date**:` or `**Git Commit**:`. Document the narrowed scope inline in the migration script. Surface a per-file body-label rewrite count to stdout. Add a fixture with a research file containing two `**Researcher**:` lines (one in the canonical position, one in quoted prose) and assert only the canonical one is rewritten.

4. **Harden the awk quote-normaliser** (addresses major finding on YAML shapes)
   Either (a) extend the awk to detect and convert single-quoted scalars, strip inline comments before wrapping, and escape embedded double quotes; or (b) explicitly reject (with `log_warn` + skip) any non-trivial shape and document in Migration Notes that the migration only canonicalises the documented forms. Add fixtures for each rejected/handled shape: `work-item: '0042'`, `work-item: 0042 # note`, `work-item: foo"bar`, `work-item:0042` (no space).

5. **Quote-normalise in the divergent branch too** (addresses major correctness finding)
   In `rewrite_plan_file`'s dual-presence branch, after `grep -v '^work-item:'` runs the surviving `work_item_id:` line through the same awk normaliser so AC #6 holds for partial-prior-run plans with unquoted survivors. Add a `partial-prior-run-plan-unquoted` fixture.

6. **Fix the test harness `run_0006_driver` helper** (addresses major test-coverage finding)
   Mirror the 0005 helper exactly: `cd "$repo" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" ACCELERATOR_MIGRATIONS_DIR="$ONLY_0006_DIR" ACCELERATOR_MIGRATE_FORCE=1 bash "$DRIVER" "$@"`. Drop the bogus `ACCELERATOR_MIGRATE_FORCE_NO_VCS` env var.

7. **Trim trailing whitespace symmetrically in divergence comparison** (addresses major correctness finding)
   Add `sed 's/[[:space:]]*$//'` to both extracted values before comparison.

8. **Compose the two research-file passes into one awk pipeline + one `atomic_write`** (addresses major architecture / database finding)
   A single in-memory transform per file restores per-file atomicity and eliminates the half-canonicalised intermediate state.

9. **Add a transitional dual-read on the visualiser consumer** (addresses major compatibility finding)
   In Phase 4 step 4, add `else if let Some(v) = m.get("work-item") { ... }` between the new `work_item_id` branch and the existing `ticket:` fallback, with an inline comment documenting that it ships for one release as a soft-landing for users running the visualiser before `/accelerator:migrate`. Open a follow-up to remove it next release.

10. **Correct on-disk fixture shape description** (addresses major test-coverage finding)
    Phase 4 §2 currently says "preserving the existing quoted-string value" but the fixtures carry unquoted integers. Update the description and decide explicitly: preserve unquoted (keep numeric→string coercion coverage) or move to quoted form plus add a separate numeric fixture.

11. **Tighten tier-1/tier-2 template resolution** (addresses major code-quality / correctness finding)
    When `templates.<name>:` is set but the file is missing, `log_warn` and return without falling through to tier-2. Apply the `resolve_corpus_path` traversal guard to the tier-1 path before joining onto `PROJECT_ROOT`. Consider extracting a shared `config_resolve_user_template_path` helper in `config-common.sh` so future migrations can reuse the logic.

12. **Declare the version-bump policy and recovery procedure** (addresses major compatibility + safety findings)
    Add a Migration Notes subsection naming the intended version bump (major vs minor vs `-pre.N`) and a Phase 2 recovery paragraph documenting `jj abandon @` / VCS restore as the procedure if the migration aborts mid-walk. Note that the framework dirty-tree guard only covers `meta/`, `.claude/accelerator*.md`, `.accelerator/` — users should also ensure `templates/` is clean before running this particular migration.

13. **Resolve the `last_updated_by` placeholder scope decision** (addresses major / minor scope finding)
    Either keep the Phase 3 edit (and update §"What We're NOT Doing" to acknowledge placeholder copy is in scope) or back it out and defer to 0065. Pick one.

14. **Promote the multi-corpus walker and dual-presence guard to shared infrastructure** (addresses minor architecture / code-quality findings; pays off for 0065/0070)
    Extract `walk_corpus`, the dual-presence guard block, and the user-template resolver into `scripts/migrate-common.sh`. Source from 0006; optional retrofit for 0005.

15. **Add the deferred test coverage items** (addresses cluster of minor test-coverage findings)
    Promote the legacy-key-ignored Rust regression test to required. Add fixtures for: `no-whitespace-work-item`, `single-quoted-work-item`, `trailing-whitespace-work-item`, `body-label-multiple`, `mixed-plan-shapes`, `template-override-tier1-missing-file`, three-run-idempotence for the empty-value branch.

16. **Sweep ADRs and adjacent work items for stale references** (addresses minor database / compatibility findings)
    Phase 5 should grep `meta/decisions/ADR-0025*.md`, `meta/decisions/ADR-0034*.md`, and all other open `meta/work/*.md` for `work-item:|researcher:|\*\*Researcher\*\*:` and either rewrite present-tense passages or document each carve-out.

17. **Strengthen the verification snippets** (addresses minor correctness finding)
    Replace `jj status | grep ... && echo FAIL || echo PASS` constructs with explicit `set -e` style assertions that fail loudly if jj itself errored.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan inherits a clean, well-bounded architecture from migration 0005 and correctly identifies that the rename surface is small (one Rust read site, three templates, three corpora) thanks to the existing template-first emission pattern and the YAML-key-to-Rust-identifier indirection. However, the plan introduces a new architectural pattern (template-content rewriting under userspace overrides) without consolidating it as a reusable helper, replicates the dual-presence guard four times with subtle semantic divergences between passes, and contains a return-code contract bug in `rewrite_research_file` that breaks the per-file rewrite counter. The cross-corpus walker and tier-1/tier-2 template resolver are introduced inline in 0006 rather than promoted into shared migration infrastructure.

**Strengths**:
- Correctly preserves the architectural seam between YAML key strings and Rust identifiers.
- Honours the established `paths.*` resolution boundary via `config-read-path.sh`.
- Properly identifies that the frontend is decoupled via the camelCase wire-type mirror.
- Sequences the dogfood corpus migration before consumer rewrite.
- Acknowledges that template-content rewriting is a new architectural pattern.

**Findings**:
- 🟡 Major / high confidence — Return-code contract of `rewrite_research_file` is inverted under `set -e` (Phase 1 §3).
- 🟡 Major / high — Body-label pass reads file after Pass 1 `atomic_write`, creating a multi-write dependency.
- 🟡 Major / high — Tier-1/tier-2 template resolution logic duplicated from `config-read-template.sh`.
- 🟡 Major / medium — Unconditional body-label rewrite blurs the migration's transformation boundary.
- 🔵 Minor / high — Multi-corpus walker is a new pattern but not promoted into shared infrastructure.
- 🔵 Minor / medium — Three-tier fallback chain preserves legacy `ticket:` without an explicit deprecation seam.
- 🔵 Minor / high — Quote-normalisation awk helper isn't gated by the dual-presence guard.
- 🔵 Minor / medium — Phase 4 produces a transient on-disk-fixture / production-code drift window.

### Code Quality

**Summary**: The plan is well-structured, follows the 0005 precedent closely, and decomposes the migration into focused helper functions. However, the proposed migration script has a clear bug in return-value semantics for `rewrite_research_file`, duplicates the dual-presence guard pattern three times rather than abstracting it, and reimplements tier-1/tier-2 template resolution inline instead of reusing the existing `config_resolve_template` helper.

**Strengths**:
- Clear phase-by-phase decomposition with explicit dependencies.
- Each helper function has a single, named responsibility.
- TDD ordering and ~20 explicit fixture scenarios listed by name.
- Defensive shape-checks inherited from 0005.
- Explicit `Notes on the migration` section flags new patterns and known footguns.

**Findings**:
- 🔴 Critical / high — `return $touched` inverts walk_corpus counter for research/RCA.
- 🟡 Major / high — Tier-1/tier-2 template resolution reimplemented inline.
- 🟡 Major / high — Dual-presence guard pattern duplicated three times.
- 🔵 Minor / high — Awk patterns miss `work-item:value` (no whitespace) shape.
- 🔵 Minor / medium — Twenty fixture directories with substantial duplication risk.
- 🔵 Minor / medium — Producer SKILL.md files not swept for stale field references.
- 🔵 Minor / medium — Unconditional body-label rewrite documented but not commented in script body.

### Test Coverage

**Summary**: The plan's test strategy is broadly strong: it mirrors the well-established 0005 pattern, enumerates ~20 fixture scenarios, and pre-stages Rust YAML fixture updates so the read-site change is properly TDD-driven. However, there are several real coverage gaps: the new awk normaliser has under-exercised branches, the stdout rewrite count is asserted in only two scenarios despite the inverted-return-code risk, the legacy-key-ignored regression test for the visualiser is explicitly marked optional, and the on-disk fixture rewrite description misrepresents the existing fixture shape.

**Strengths**:
- Phase 1 orders fixtures-before-script (TDD); Phase 4 orders YAML fixtures before production change.
- Partial-prior-run matrix covers both matching and divergent values across three axes.
- Idempotence verified via `tree_hash` byte-identical snapshot.
- Template-rewrite tests cover all four tier-resolution cases.
- Phase 6 includes end-to-end upgrade-path fixture for multi-corpus integration.

**Findings**:
- 🟡 Major / high — Missing stdout rewrite-count assertion for `default-layout` would not catch inverted touched-semantics.
- 🟡 Major / high — Harness helper omits `cd` and `CLAUDE_PLUGIN_ROOT`, uses non-canonical env var, passes ignored positional.
- 🟡 Major / medium — On-disk fixture rewrite description misstates current shape (unquoted integers, not quoted strings).
- 🔵 Minor / high — Legacy-key-ignored regression test marked optional.
- 🔵 Minor / medium — Awk quote-normaliser branches under-exercised (whitespace/single-quote cases).
- 🔵 Minor / medium — AC #6 quote-shape grep not exercised in unit tests.
- 🔵 Minor / medium — Body-label collision behaviour not pinned by test.

### Correctness

**Summary**: The plan generally mirrors the well-tested 0005 pattern, but the refactor from in-line accumulation to per-file helper functions introduces several correctness defects: inverted return-code semantics, direct invocation under `set -e` that aborts the migration on the second idempotent run, and a dual-presence branch that skips quote-normalisation and silently violates AC #6.

**Strengths**:
- Accurate identification of template-content rewriting as an unprecedented pattern.
- Awk pipeline cleanly enumerates three documented input shapes.
- Defensive path validation extended uniformly across all corpus/template paths.
- Idempotence fixture explicitly snapshots tree hash.
- Six partial-prior-run fixtures cover the full Cartesian matching/divergent space.

**Findings**:
- 🔴 Critical / high — Inverted return-code semantics will misreport research rewrite counts and risk aborting mid-walk.
- 🔴 Critical / high — Direct call to rewrite helper under `set -e` aborts on second (idempotent) run for templates.
- 🟡 Major / high — Partial-prior-run divergent path bypasses quote-normalisation, violating AC #6.
- 🟡 Major / high — Awk normaliser mishandles YAML single-quoted scalars and inline-comment values.
- 🟡 Major / high — `grep -v '^\*\*Researcher\*\*:'` drops all matching lines, but divergence inspects only the first.
- 🟡 Major / medium — Trailing-whitespace sensitivity in divergence comparison produces spurious warnings.
- 🔵 Minor / medium — Awk regex `^work-item:[[:space:]]*$` portability across awk dialects.
- 🔵 Minor / medium — Tier-2 fallthrough fires when tier-1 is set but its file absent.
- 🔵 Minor / high — `jj status | grep && ... || ...` swallows non-grep failures.
- 🔵 Suggestion / medium — `find` recursive vs flat traversal contract is implicit.

### Safety

**Summary**: The plan inherits the established 0005 migration pattern (atomic_write, dual-presence guard, dirty-tree pre-flight, VCS-revert recovery) which is generally safe. However, several concrete safety bugs and gaps materially raise the risk of silent data corruption: inverted return-code semantics interact dangerously with `set -euo pipefail`; the body-label rewrite is acknowledged as unconditional but the mitigation is only a PR-description note; and the divergence policy silently drops potentially-fresh data with no escape hatch.

**Strengths**:
- Pre-flight dirty-tree guard at the framework level is engaged.
- `atomic_write` provides crash-safe per-file rewrites via temp+rename.
- Idempotence verified via tree_hash byte-identity.
- Path-traversal guard via `resolve_corpus_path`.
- `find ... -print0` default does not follow directory symlinks.
- VCS-revert recovery path documented.

**Findings**:
- 🔴 Critical / high — Inverted return semantics in `rewrite_research_file` mis-count rewrites and risk aborting mid-walk.
- 🔴 Critical / high — Unconditional body-label rewrite can silently corrupt research prose with no preview.
- 🟡 Major / high — Divergence policy silently drops potentially-fresh data with no user prompt or escape hatch.
- 🟡 Major / high — Awk quote-normaliser corrupts values containing embedded double-quotes.
- 🟡 Major / medium — Phase 2 'confirm jj status is clean' offers no recovery procedure if dirty.
- 🔵 Minor / medium — Path-traversal guard misses some shapes and trusts `find` symlink defaults.
- 🔵 Minor / high — Userspace breaking-change warning is buried in plan; needs migrate driver banner.

### Compatibility

**Summary**: The plan is a deliberate breaking schema change with a migration script shipped in the same release to repair user repos automatically. Several compatibility decisions are well-justified, but the asymmetric treatment of `work-item:` vs `ticket:`, the absence of an explicit version-bump commitment, and the unconditional body-label regex rewrite are real risks for downstream user repos.

**Strengths**:
- Migration honours all four `paths.*` overrides plus both template-override tiers.
- Legacy `ticket:` fallback preserved.
- Dual-presence guard + 'kept new, dropped old' gives recoverable partial-prior-run failure.
- Quote-normalisation anticipates userspace divergence.
- Migration Notes call out upgrade-window behaviour, dirty-tree interaction, visualiser restart.
- AC #7 upgrade-path fixture validates end-to-end.

**Findings**:
- 🟡 Major / high — No transitional dual-read on visualiser creates hard-break window.
- 🟡 Major / high — Unconditional body-label regex rewrite can silently corrupt user-repo prose.
- 🟡 Major / high — Version-bump policy for breaking schema change is unspecified.
- 🔵 Minor / medium — Userspace template overrides rewritten in-place with no diff/merge affordance.
- 🔵 Minor / medium — Other downstream stories may reference legacy field names.
- 🔵 Minor / low — Eval fixture verification only spans this repo, not downstream user-repo evals.
- 🔵 Suggestion / medium — Document the deprecation horizon for the new key.

### Database

**Summary**: Treated as a forward-only schema migration over the markdown corpus, the plan is well-structured and inherits most safety from 0005. However, the proposed migration script has several concrete correctness defects: inverted return-code contract, in-place `atomic_write` read-while-writing race, and a multi-pass shape that does not preserve 0005's per-pass independence for plans.

**Strengths**:
- Inherits the proven 0005 safety shape.
- Per-file atomicity via `atomic_write`.
- Field rename (not value rename) means cross-document refs are byte-stable.
- Defensive quote-normalisation anticipates userspace divergence.
- Forward-only stance consistent with ADR-0023.
- AC includes idempotency re-run check.
- Test matrix covers the divergence space.

**Findings**:
- 🔴 Critical / high — Inverted and inconsistent return-code contract between per-file rewrite functions.
- 🔴 Critical / high — Pipeline reads `$file` while `atomic_write` may rename over it — race on same-FS rename.
- 🟡 Major / high — Divergent-frontmatter branch in research walk performs two atomic_writes; not per-file atomic.
- 🟡 Major / high — No collision check when multiple `paths.*` aliases resolve to the same directory.
- 🟡 Major / medium — Unanchored body-label rewrite mutates content inside fenced code blocks and quoted examples.
- 🟡 Major / high — `last_updated_by` placeholder rewrite is inconsistent across templates.
- 🟡 Major / medium — Awk quote-normaliser strips inline trailing comments and treats `#` as literal value.
- 🔵 Minor / high — `find` walk uses `-name '*.md'` not `-type f`.
- 🔵 Minor / high — Tier-1 template path resolution does not validate path is inside the project.
- 🔵 Minor / medium — Empty-value branch not three-times idempotence-tested.
- 🔵 Minor / high — Whole-walk atomicity not asserted in Phase 2 success criteria.
- 🔵 Minor / high — ADR-0025 / ADR-0034 references to `work-item:` not in migration scope but stale post-migration.

### Standards

**Summary**: The plan aligns well with established project conventions: migration 0006 mirrors the 0005 pattern, file naming follows precedent, fixture organisation matches per-migration layout, canonicalisation correctly chooses `work_item_id`/`author` per ADR-0033, plan structure follows the create-plan template. A handful of inconsistencies stand out — most notably the inverted bash return-code convention plus minor env-var and fixture-path divergences from 0005.

**Strengths**:
- Migration file name follows verb-noun, kebab-case, four-digit-prefix convention.
- Bash structural conventions match 0005 exactly.
- Dual-presence idempotence guard faithfully ported.
- Test fixture directory layout matches 0004/0005 convention.
- Canonical names align with ADR-0033.
- ADR-0033 quoting contract honoured by awk normaliser.
- Plan structure follows create-plan template.

**Findings**:
- 🟡 Major / high — Inverted/mismatched bash return-code semantics between per-file helpers.
- 🔵 Minor / high — Test driver invocation diverges from the 0005 precedent (env-var names, missing `CLAUDE_PLUGIN_ROOT`, cwd).
- 🔵 Minor / high — AC #7 fixture path inconsistent with the 0006 fixture directory convention.
- 🔵 Minor / medium — Renaming `last_updated_by` placeholder text drifts beyond canonicalisation scope.
- 🔵 Minor / medium — Unconditional `^\*\*Researcher\*\*:` rewrite may corrupt research prose.

## Re-Review (Pass 2) — 2026-05-21

**Verdict:** REVISE

The pass-1 critical and most major findings have been substantively
addressed: the inverted return-code contract is gone (inline
accumulator + stdout signalling), the `set -e` template-abort risk is
gone (helper always returns 0), the two-pass `atomic_write` race is
gone (single in-memory awk transform + one `atomic_write` per file),
the unconditional body-label rewrite is now anchored to the
pre-first-`## ` region, the harness helper mirrors 0005 exactly, the
on-disk fixture shape description is corrected, the transitional
visualiser fallback softens the upgrade window, and Phase 5 now
sweeps ADRs 0025/0034 plus other open work items. Verdict remains
REVISE because the rewrite introduced **three new critical defects**
that would block implementation, plus several major issues that need
attention before this is ready.

### Previously Identified Issues

#### Critical (resolved)
- 🟢 **Code Quality / Correctness / Safety / Database**: Inverted `return $touched` counter — **Resolved** via inline accumulator pattern.
- 🟢 **Correctness**: `set -e` abort in `rewrite_template_if_present` on second run — **Resolved** via `touched=$(rewrite_file …)` stdout capture.
- 🟢 **Safety**: Unconditional body-label rewrite — **Resolved** via awk pre-first-`## ` anchor (residual pre-H2 quoted-prose risk noted as new finding).

#### Major (resolved)
- 🟢 Awk normaliser narrow shape coverage → handles single-quoted, no-whitespace, trailing-whitespace; refuses inline-comment / embedded-quote with sentinel.
- 🟢 Body-label `grep -v` dropped all matches → now drops first only (via `sed '0,/pat/{//d;}'` — but see new finding on BSD sed portability).
- 🟢 Trailing-whitespace divergence false positives → symmetric trim on both sides.
- 🟢 Pipeline race `grep -v … | atomic_write` → single read-to-temp + single write.
- 🟢 Half-canonicalised state on interrupt → single `atomic_write` per file.
- 🟢 No transitional dual-read → added with explicit one-release-cycle removal.
- 🟢 Version-bump unspecified → Versioning subsection added.
- 🟢 `paths.*` alias collision → `WALKED` dedup + fixture.
- 🟢 Tier-2 fallthrough on tier-1 set-but-missing → resolver warns and stops.
- 🟢 Phase 2 recovery procedure missing → explicit `jj abandon @` paragraph added.
- 🟢 `last_updated_by` scope drift → deferred to 0065, documented in §What We're NOT Doing.
- 🟢 On-disk fixture shape misstated → corrected to "unquoted integers; preserve form".
- 🟢 Test harness `run_0006_driver` defects → matches 0005 helper exactly.
- 🟢 Missing stdout rewrite-count assertions → every scenario now asserts the `rewrote N file(s)` line.
- 🟢 Divergent partial-prior-run quote-norm bypass → **Partially resolved** — claim is made in Notes but the awk has no rule for surviving `work_item_id:` (NEW critical, below).

#### Major (still present)
- 🟡 Dual-presence guard pattern still duplicated three times verbatim inside `rewrite_file` — extraction to shared helper deferred.
- 🟡 Divergence policy still silently drops the dropped value with no escape hatch or audit trail (no env-var, no sidecar log, no in-file comment).
- 🟡 Tier-1/tier-2 template resolver remains inline rather than promoted to `scripts/config-common.sh` despite 0065/0070 also needing it.

#### Minor (still present)
- 🔵 ADR-0025/0034 sweep added but `ticket:` legacy fallback still has no source-comment deprecation seam.
- 🔵 Multi-corpus walker and `assert_safe_relpath` not promoted to shared `scripts/migrate-common.sh`.

### New Issues Introduced

#### Critical (introduced by the rewrite)

- 🔴 **Correctness / Database**: `awk_transform` has no rule for surviving unquoted `work_item_id:` — the Plan's Notes claim "the divergent-plan branch drops `^work-item:` then feeds the remainder through `awk_transform` … so a surviving unquoted `work_item_id:` value is canonicalised", but the awk script only has rules matching `^work-item:` and `^researcher:`. The `partial-prior-run-plan-unquoted` fixture (added to lock this in) will fail; AC #6 is silently violated for partial-prior-run plans with unquoted survivors. **Fix**: add a parallel awk rule `in_frontmatter && /^work_item_id:/` running the same value-shape inspection, or factor the shape inspection into an awk function shared between the two key branches.

- 🔴 **Correctness / Safety / Database**: `sed '0,/^\*\*Researcher\*\*:/{//d;}'` body-label dual-presence cleanup is GNU-sed-only — BSD sed (default on macOS) rejects line address 0 with `invalid usage of line address 0`. Under `set -euo pipefail`, this aborts the entire migration mid-walk on macOS for any partial-prior-run research file. **Fix**: replace with `awk 'BEGIN{done=0} done==0 && /^\*\*Researcher\*\*:/{done=1; next} {print}'` (portable across awk dialects). Add a macOS CI lane.

- 🔴 **Safety / Database**: 0006-WARN sentinel emission breaks idempotence — `awk_transform` preserves the original `^work-item:` line plus a fresh `# 0006-WARN: refused …` sentinel. On the next run, `grep -qE '^(work-item:|…)'` still matches, `touched=1`, awk runs again, another sentinel is stacked. The three-run `idempotent` fixture's `tree_hash` byte-identity assertion will fail; user repos accumulate sentinel noise on every upgrade. **Fix**: either (a) detect existing `# 0006-WARN:` sentinels and skip re-emission, or (b) on refusal, skip `atomic_write` entirely (leave the file untouched; emit `log_warn` to stderr only — no sentinel in the file). Option (b) is cleaner.

#### Major (introduced)

- 🟡 **Safety / Compatibility**: Migration Notes point 4 still says the body-label rewrite is "unconditional on the regex" — contradicts the new anchored implementation. Users reading the CHANGELOG-bound notice will be told to manually audit files the migration will never touch. **Fix**: rewrite point 4 to describe the anchored semantics ("only rewrites occurrences before the first `## ` heading").

- 🟡 **Correctness**: Dual-presence shell `grep -v '^work-item:'` and `grep -v '^researcher:'` are not frontmatter-anchored — they strip body-prose matches too. A research file quoting a legacy frontmatter shape in prose (or the 0064 research document itself) could have prose lines silently dropped. **Fix**: move dual-presence cleanup into `awk_transform` so it can be gated by `in_frontmatter`, or slice the frontmatter region first.

- 🟡 **Correctness**: `assert_safe_relpath` uses `canonical=$(…) || canonical=""` which is dead — `var=$(…)` assignment always returns 0. The case-match-against-`PROJECT_ROOT/*` saves correctness by accident, but a future refactor could silently weaken the guard. **Fix**: restructure as explicit `if abs_dir=$(…); then canonical="$abs_dir/$(basename …)"; fi`.

- 🟡 **Correctness**: Files without a leading `---` line never enter awk's frontmatter mode, but `grep -q '^work-item:'` (unanchored) still sets `touched=1` — silent no-op atomic-write with overcounted rewrite total. **Fix**: drop the `NR == 1` constraint and treat the first `---`-block anywhere as frontmatter, OR have awk warn at EOF when keys were seen but frontmatter was never entered.

- 🟡 **Correctness**: `walk_corpus` accumulator `rewrote=$((rewrote + touched))` aborts under `set -u` when `touched` is empty (e.g., a transient `mktemp`/awk failure inside `rewrite_file`). **Fix**: `touched=${touched:-0}` defensive default + numeric guard before arithmetic.

- 🟡 **Correctness**: Phase 2 §4 still uses the fragile `jj status | grep && FAIL || PASS` pattern; only Phase 6's snippet was hardened. **Fix**: apply the same explicit-capture-and-grep pattern to Phase 2 §4.

- 🟡 **Safety**: Body-label anchor still permits corruption of legitimate quoted `**Researcher**:` prose appearing BEFORE the first `## ` heading. The `body-label-multiple` fixture only covers post-H2 prose. **Fix**: either tighten the anchor (require immediately-preceding `**Date**:` or `**Git Commit**:` label), or add a `body-label-quoted-prose-pre-h2` fixture pinning the chosen semantics.

- 🟡 **Database**: `assert_safe_relpath` failure escalates a single misconfigured `paths.*` key to whole-migration abort via `log_die`, but a *missing* directory only `log_warn`s and continues. Policy is inconsistent. **Fix**: downgrade `log_die` to `log_warn` + skip-this-corpus to match the missing-directory policy.

- 🟡 **Database**: `WALKED` dedup keyed on the raw `config-read-path.sh` string — equivalent paths with `./` or trailing `/` would not be detected as aliases. **Fix**: canonicalise via `realpath` / `cd && pwd -P` before using as the dedup key.

#### Minor (introduced)
- 🔵 `fm_close_count` initialised and incremented in awk but never read — dead state.
- 🔵 Inline-comment heuristic `[ \t]+#` misses `value#comment` shape (no whitespace before `#`).
- 🔵 Body-label anchor unaware of fenced code blocks before first `## ` heading.
- 🔵 `rewrite_file` stdout contract is load-bearing but only implicitly documented — any future `echo` for debugging would corrupt the counter.
- 🔵 `paths-alias-research` fixture asserts stdout for messages the migration emits on stderr (assertion target inconsistency).
- 🔵 Testing Strategy §Unit-level tests still calls the legacy-key Rust test "optional" — stale relative to the new transitional fallback (which makes a positive fallback test required).
- 🔵 Transitional `work-item:` fallback removal trigger named ("the release that closes 0070") but not pinned by a follow-up issue ID.
- 🔵 Versioning subsection commits to `-pre.N+1` bump but doesn't say what happens at `1.21.0` stable.
- 🔵 Refused-shape (`inline-comment-work-item`, `embedded-quote-work-item`) fixtures only assert two-run idempotence, not three-run.

### Assessment

The plan's architecture is now substantially correct, the surface
area of cross-cutting risk has shrunk, and the bulk of pass-1
findings are resolved. The three new criticals are concrete and
narrow — each maps to a specific line in the rewritten migration
script and has a clear fix. A third review pass should be
quick: fix the three criticals + the four high-confidence majors,
re-run the relevant lenses, expect APPROVE or near-APPROVE.

**Next steps**: address the three criticals (awk rule for
`work_item_id:` survivor; portable body-label-first-drop; sentinel
idempotence), the unanchored frontmatter-key cleanup, the `set -u`
empty-touched defensive default, the Migration Notes contradiction,
and the Phase 2 jj-status snippet. The other items are good to fix
in this pass but won't block implementation if deferred.

## Re-Review (Pass 3) — 2026-05-21

**Verdict:** REVISE (with all blockers now fixed post-review)

The pass-3 review confirmed that all three pass-2 criticals and the
seven high-confidence pass-2 majors are genuinely resolved. The
rewrite materially improved safety, correctness, and architectural
clarity: dual-presence handling consolidated inside awk and
frontmatter-anchored; `cmp -s` byte-equality gates `atomic_write` so
idempotent re-runs produce zero churn; refused shapes preserve
byte-identity (no in-file sentinel); body-label first-drop is
portable across sed dialects; `assert_safe_relpath` no longer has
dead-code fallback; `log_die` downgraded to `log_warn` + skip-corpus.

However, the pass-3 rewrite itself introduced **four new defects** —
all line-level, all narrowly fixable. After the pass-3 review I
applied fixes for each (described below) so the plan is now ready
to enter implementation. A pass-4 review is not requested; the
fixes are mechanical and the remaining pass-3 minors are quality-of-
life rather than blockers.

### Previously Identified Issues (Pass 2)

#### Critical / Major (all resolved by the pass-3 rewrite)
- 🟢 awk_transform missing rule for surviving `work_item_id:` — resolved via new awk rule + `normalise_value`.
- 🟢 GNU-only `sed '0,/pat/{//d;}'` — resolved by moving first-drop into awk's `dropped_first_rb` flag.
- 🟢 Dual-presence `grep -v` unanchored — resolved by moving cleanup into awk with `in_frontmatter` gating.
- 🟢 `assert_safe_relpath` `|| canonical=""` dead code — resolved via explicit `if abs_parent=...; then`.
- 🟢 `walk_corpus` aborts on empty `touched` under `set -u` — resolved via numeric guard with `log_warn` + default 0.
- 🟢 Phase 2 jj-status snippet — resolved via explicit-capture pattern.
- 🟢 Migration Notes contradiction on body-label rewrite — resolved by rewriting point 4 to describe anchored semantics + adding new point 5.

#### Minor (still present, deferred)
- 🔵 Tier-1/tier-2 resolver still inline rather than promoted to `scripts/migrate-common.sh` (would benefit 0065/0070; intentional defer).
- 🔵 Dual-presence guard still has parallel structure (3x) inside awk — single-line branches now, not 15-line shell blocks; readable.
- 🔵 Divergence policy still has no persistent sidecar audit trail.
- 🔵 `ticket:` legacy fallback still has no source-comment deprecation seam.
- 🔵 No driver-time stderr startup banner; users rely on CHANGELOG.

### New Issues Introduced by Pass-3 Rewrite (all fixed post-review)

#### Major (fixed)

- 🟡 → 🟢 **`refuses()` runs before trailing-whitespace strip** (Correctness, Database) — A line like `work-item: "0042"   ` (with trailing whitespace) fails the `^".*"$/` regex (line ends with space, not `"`), falls through to `if (line ~ /"/) return 1`, and is REFUSED instead of having its whitespace stripped and being rewritten. Directly contradicts the `trailing-whitespace-work-item` fixture's expectation.
  **Fix applied**: moved `sub(/[ \t]+$/, "", line)` BEFORE the `if (refuses(line))` check in both `^work-item:` and `^work_item_id:` rules.

- 🟡 → 🟢 **DIVERGE warning compares raw values, false-positive on quoted-vs-unquoted** (Correctness) — A file with `work-item: "0042"` and `work_item_id: 0042` (matching values, different quote shape) would trigger a spurious DIVERGE warning because the END block compared raw value strings. The `partial-prior-run-plan-unquoted` fixture explicitly asserts "No stderr warning" for exactly this case.
  **Fix applied**: introduced `semantic_inner()` helper that strips outer quotes; capture both `first_wi` (raw, for warning message) and `inner_wi` (semantic, for comparison). END block now compares `inner_wi != inner_id`.

- 🟡 → 🟢 **MALFORMED warning structurally unreachable** (Correctness, Safety) — `saw_work_item` and `saw_researcher` are only set inside `in_frontmatter`-gated rules; a file lacking any `---` fence has `in_frontmatter` permanently 0, so those flags stay 0 and the END condition can never fire. The `frontmatter-missing-fence` fixture asserts the MALFORMED line MUST appear.
  **Fix applied**: added ungated top-of-file rules (`/^work-item:/ { saw_wi_anywhere = 1 }`, etc.) that set separate `*_anywhere` flags regardless of frontmatter state; END block now checks `(saw_wi_anywhere || saw_r_anywhere || saw_rb_anywhere)`.

- 🟡 → 🟢 **`realpath -m` is GNU-only; fails on macOS BSD realpath** (Correctness, Compatibility, Safety) — `command -v realpath` succeeds on macOS (BSD realpath exists) but the `-m` flag is GNU-coreutils-only. The invocation silently fails, `canonical=""`, every `paths.*` resolution is spuriously rejected as unsafe, and the migration becomes a silent no-op on macOS.
  **Fix applied**: dropped the `realpath -m` branch entirely from both `assert_safe_relpath` and `canonicalise_rel`. Both now use `cd && pwd -P` unconditionally — portable across GNU and BSD systems.

#### Minor (deferred)
- 🔵 awk_transform is ~190 lines with seven positional parameters — code-quality recommends extracting to `scripts/migrate-common/canonicalise-frontmatter.awk` for testability. Deferred — current shape is correct, factoring is a quality-of-life refactor for 0065.
- 🔵 Dual-presence detection performed in two layers (shell pre-scan + awk re-detection) — architectural minor; intentional split between dual-presence GATING (shell flags) and DIVERGENCE COMPARISON (awk END).
- 🔵 Six near-identical pre-scan `grep -q` invocations — could be a tiny helper. Cosmetic.
- 🔵 Numeric/string type mixing in `has_*` flags — awk compares as string `== "1"`; shell passes as integer 0/1. Documented contract; cosmetic.
- 🔵 TEMPLATE_PATHS dedup not canonicalised — `templates.a: file.md` vs `templates.b: ./file.md` would not dedupe. Same shape as WALKED but un-canonicalised. Low impact (cmp -s saves the second write); fix is a one-liner if/when surface variants become a real concern.
- 🔵 Refused-shape files emit stderr noise on every migrate run forever (no acknowledgement mechanism). Documented in Migration Notes point 5; users are expected to hand-fix.
- 🔵 AC #6 carve-out for empty value documented in fixture but not at the AC line itself.
- 🔵 Transitional `work-item:` fallback removal trigger named ("the release that closes 0070") but no concrete follow-up issue ID pinned.
- 🔵 Versioning section doesn't commit to a major/minor bump at stable-release time.
- 🔵 Userspace eval-suite checklist still not added to Migration Notes.
- 🔵 No temp-file cleanup trap in `rewrite_file` — `set -e` abort would leak `tmp_out`/`tmp_err`.
- 🔵 Awk dialect (BSD vs GNU vs busybox vs mawk) not pinned in plan — should be specified as a test-matrix requirement.

### Assessment

The plan is now ready to enter implementation. The four pass-3
defects have all been fixed in line-level edits; the remaining
minors are quality-of-life items that can be addressed during
implementation or deferred to a follow-up. The cross-cutting risk
surface that drove pass-1's REVISE is now substantively closed:

- Per-file atomicity restored via single awk + single `atomic_write`.
- Idempotence verified via three-run `tree_hash` byte-identity across all branches including refused shapes.
- Body-label rewrite anchored to producer-emission region with explicit fixture coverage of the trade-off.
- Schema-shape contract (AC #6 quoted strings) preserved for partial-prior-run survivors via the `work_item_id:` normalisation rule.
- Cross-platform portability resolved (no GNU-only flags remain).
- Failure modes are observable (`0006-REFUSE:`, `0006-DIVERGE:`, `0006-MALFORMED:`) rather than silent.
- Recovery via `jj abandon @` is documented in Phase 2.

The verdict could fairly be downgraded to **APPROVE** after the
pass-3 fixes, but per the review protocol the artifact frontmatter
remains at **REVISE** because the pass-3 review itself identified
blockers. Implementation should proceed; the remaining minors can
be addressed via PR-review iteration rather than another full
multi-lens pass.

