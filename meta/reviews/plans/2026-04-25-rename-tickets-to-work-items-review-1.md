---
date: "2026-04-25T21:07:33+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-25-rename-tickets-to-work-items.md"
review_number: 1
verdict: REVISE
lenses: [architecture, correctness, safety, test-coverage, compatibility, usability, code-quality]
review_pass: 1
status: complete
---

## Plan Review: Rename `tickets` → `work` and `ticket` → `work-item` (with Migration Skill)

**Verdict:** REVISE

The plan is methodical, well-scoped, and TDD-disciplined, with a sound
phase decomposition (ADRs → rename → framework → self-apply → release)
and a thoughtful migration skill that materially improves on the v1.8.0
manual-instruction precedent. However, two critical defects in the
migration script (BSD-only `sed -i ''` invocation that fails on Linux;
silent destructive rename of users' custom-pinned `paths.tickets`
directories) would corrupt user data on real-world execution, and a
cluster of major findings around partial-failure semantics, missing
working-tree pre-flight checks, deferred forward-references, and
under-specified config-rewrite logic warrant rework before
implementation begins. The framework's UX is also too thin
(no logging, no preview, no migrate-discoverability hint) for a
destructive default.

### Cross-Cutting Themes

- **Custom `paths.tickets` override is silently destroyed** (flagged by:
  architecture, correctness, safety, compatibility, usability,
  code-quality — 6 of 7 lenses) — Test case 7 in §4.1 and step 2 of
  §4.6 specify renaming a user's pinned `meta/custom-tix/` directory to
  the new default `meta/work/`. Every lens that examined this case
  flagged it as the wrong default. The plan itself flags it as needing
  user confirmation. Conservative semantics (rewrite the key only,
  preserve the pinned path) honours user intent and is also simpler.

- **`sed -i ''` is not portable** (flagged by: correctness, safety,
  compatibility) — The plan labels `sed -i '' 's/^ticket_id:/.../'`
  "BSD-portable", but on GNU sed (Linux, including most CI runners)
  this fails because GNU sed parses `''` as the script. The migration
  silently no-ops on Linux while the state file records success.

- **Destructive-by-default with no preview, no confirmation, no clean-
  tree check** (flagged by: architecture, safety, usability) — The plan
  reverses the source research's `DRY_RUN`/`--apply` recommendation
  without ADR-level justification. The driver does not check
  `jj status`/`git status` for unstaged changes before mutating, so a
  user mid-edit can lose unsaved work that VCS-as-safety-net cannot
  recover.

- **Partial-failure leaves filesystem inconsistent** (flagged by:
  safety, correctness, test-coverage) — The migration is six sequential
  shell ops (`mv` directories, `sed` frontmatter, rewrite config,
  append state file). Mid-script interruption between steps leaves a
  state where `meta/work/` contains files with stale `ticket_id:`
  frontmatter, no state-file entry, and no `meta/tickets/` to revert
  to. Per-script idempotency is asserted but not validated against this
  interleaving.

- **Migration framework duplicates config-parsing logic** (flagged by:
  architecture, code-quality) — Step 1 of §4.6 inlines its own
  `paths.tickets` reader because "the new code no longer recognises"
  the old key. As migrations accumulate, the registry becomes a
  graveyard of bespoke YAML/sed snippets, none of which use the
  project's existing extraction primitives.

- **`/accelerator:migrate` has no discoverability path** (flagged by:
  compatibility, usability) — Users who upgrade and run an old slash
  command (`/accelerator:create-ticket`) get a generic "skill not
  found" with no hint pointing at `/accelerator:migrate`. The
  CHANGELOG bullet is the only signal, and most users don't read
  release notes.

- **`find_repo_root` factoring undecided** (flagged by: architecture,
  code-quality) — §4.5 leaves "use test-helpers.sh OR replicate inline"
  unresolved. The right answer (already-existing
  `config_project_root` in `scripts/config-common.sh`) is overlooked
  and should be mandated.

- **Phase 1.9 `:53` forward-reference deferral leaves intermediate
  commits broken** (flagged by: architecture, correctness,
  code-quality) — Phase 1.14 updates the doc-comment listing
  recognised template keys to drop `ticket`, but Phase 1.9 defers the
  helper-script call site that passes `ticket` to Phase 3. Doc and
  call site contradict between phases.

- **15 default-literal duplications deferred** (flagged by:
  architecture, code-quality) — The rename touches every duplication
  anyway. Centralising path defaults now amortises work the plan
  acknowledges will need doing later.

### Tradeoff Analysis

- **Safety vs simplicity in dry-run policy**: Removing the dry-run
  flag simplifies the framework but eliminates the strongest
  safety affordance. Recommendation: keep the framework simple but
  add the cheap pre-flight (clean-tree check) and a one-line
  preview ("about to apply N migrations: …") rather than a full
  dry-run mode.

- **Terminology neutrality vs typing friction**: `work-item`
  removes service-desk slant but adds 5 characters to every slash
  command. Most users come from Jira/Linear/GitHub Issues where
  "ticket" is colloquially universal. The terminology ADR (Phase 0)
  should engage with typing/discoverability cost explicitly rather
  than treating the choice as semantically motivated only.

### Findings

#### Critical

- 🔴 **Correctness**: `sed -i ''` is not portable across BSD and GNU sed
  **Location**: Phase 4.6 step 4
  Plan asserts `sed -i '' 's/^ticket_id:/work_item_id:/'` is
  "BSD-portable". On GNU sed (Linux, default on most CI runners),
  this fails — `''` is parsed as the sed script, and the rewrite
  silently does not occur. State file records success, leaving
  Linux users with mixed-schema frontmatter the renamed plugin no
  longer recognises.

- 🔴 **Correctness**: Pinned-config override silently destroys user data
  **Location**: Phase 4.1 test 7 and Phase 4.6 step 1-2
  A user with `paths.tickets: meta/custom-tix` finds their
  custom directory renamed to `meta/work/` and their config key
  rewritten with an ambiguous value. Sharing/symlink/external-tool
  integrations break silently. Recovery requires manual rename and
  config edit. The plan flags this as "subtle — confirm with user"
  but proceeds with the destructive default.

#### Major

- 🟡 **Architecture**: State-file write semantics for empty/no-op
  migrations contradicts the ADR contract
  **Location**: Phase 4.1 test case 6 vs Phase 0 ADR contract
  Test 6 records a no-op migration as "applied" while the ADR says
  state file records "on success". Conflates "ran and exited 0"
  with "transformation occurred", leaving partial-failure recovery
  paths ambiguous.

- 🟡 **Architecture / Code Quality**: Migration script duplicates
  config-parsing logic rather than reusing the config layer
  **Location**: Phase 4.6 step 1
  Inlining a YAML-by-sed parser inside one-shot migrations becomes
  a maintenance graveyard as more migrations land. No reuse of
  existing `config_extract_frontmatter`/awk helpers.

- 🟡 **Architecture / Safety / Usability**: Plan silently drops the
  dry-run default specified in research without ADR justification
  **Location**: Phase 4.4–4.6, "What We're NOT Doing"
  Research recommended `DRY_RUN=1` and dry-run-by-default with
  `--apply`. Plan reverses both decisions without engaging the
  research's safety reasoning.

- 🟡 **Safety**: No working-tree cleanliness check before destructive
  migration
  **Location**: Phase 5.1 / Phase 4.4 SKILL.md
  Driver doesn't programmatically verify a clean working tree.
  A user mid-edit loses unstaged work that VCS-as-safety-net
  cannot recover.

- 🟡 **Safety / Correctness**: Partial-update window between mv and sed
  leaves repo in inconsistent state on crash
  **Location**: Phase 4.6 step 4
  Crash between `mv tickets work` and `sed` mid-pass leaves
  `meta/work/` with stale `ticket_id:` frontmatter, no state-file
  entry, no `meta/tickets/` to revert. Per-script idempotency is
  asserted but doesn't cover this interleaving.

- 🟡 **Safety**: Merge-collision abort lacks specification and is
  untested
  **Location**: Phase 4.6 step 2
  No test case in §4.1 covers the both-dirs-exist abort. No
  recovery procedure documented. Frustrated user may
  `rm -rf meta/work` to "unblock", destroying real data.

- 🟡 **Correctness**: Idempotency guards specify two branches but not
  the third state
  **Location**: Phase 4.6 steps 2-3
  Of four (source × target) presence states, only two are
  specified. Tests 2 and 6 depend on the unspecified branches
  being silent no-ops; behaviour is implementation-dependent.

- 🟡 **Correctness**: Multi-key config rewrite has no canonical edit
  logic for nested vs flat YAML
  **Location**: Phase 4.6 step 5
  A naive sed substitution silently corrupts user config files
  (e.g. comments containing "tickets", future key collisions,
  flat dotted vs nested two-line forms).

- 🟡 **Correctness**: `mise run test:tickets` task wiring not
  addressed across the rename
  **Location**: Phase 1 §1.2 + §1.7
  Plan renames `test-ticket-scripts.sh` but does not mention
  updating the mise task definition that wires `test:tickets`.
  Phase 1 cannot turn green as written.

- 🟡 **Correctness**: Phase 1 RED commit leaves overall test suite
  red, not the targeted subsets only
  **Location**: Phase 1 §1.5
  Hierarchy-format test reads non-existent files after §1.1, so
  `test:format` errors out (not just fails). The RED commit
  becomes a poison-pill bisect revision.

- 🟡 **Test Coverage**: RED-state verification doesn't assert failures
  are due to the expected cause
  **Location**: §1.5, §2.3, §3.2, §4.3
  Each phase says "expected to fail" without recording which
  specific tests must fail with which messages. Conflates
  "red for the right reason" with "red".

- 🟡 **Test Coverage**: Migration test suite misses critical edge
  cases
  **Location**: Phase 4.1
  Missing: both dirs already exist (collision), malformed YAML
  in user config, corrupt state file, frontmatter with both
  `ticket_id:` and `work_item_id:`, paths with spaces/unicode,
  read-only files inside `meta/tickets/`.

- 🟡 **Test Coverage**: No content-preservation regression check for
  the 29 self-applied work-items
  **Location**: Phase 5.4
  Plan verifies file count and frontmatter keys but never asserts
  that work-item bodies survive the migration byte-for-byte.

- 🟡 **Test Coverage**: Eval suites use live LLM calls — no plan for
  cost/runtime/CI impact
  **Location**: Phase 3.2 / Phase 4 success criteria
  Running full evals at every RED→GREEN gate may be expensive,
  slow, and flaky. Distinguish format-check from execution.

- 🟡 **Test Coverage**: Phase 1 GREEN may not produce a fully-green
  suite — boundary is fuzzy
  **Location**: Phase 1 §1.9 + success criteria
  Plan defers `:53` to Phase 3 but Phase 1 success criteria do
  not enumerate exactly which `mise run test:*` subsets must be
  green at end-of-phase.

- 🟡 **Compatibility**: Plan asserts current version is `1.19.0-pre.2`
  but actual is `1.19.0-pre.4`
  **Location**: Phase 6.2
  Plan was written against stale state. Suggests other line-number
  references (research-cited `:556`, `:182`, etc.) may also have
  drifted.

- 🟡 **Compatibility**: Migration framework has no forward/backward
  compatibility model
  **Location**: Phase 4.4–4.6 ADR
  Downgrade scenarios (state file references unknown migration IDs)
  produce silently inconsistent state. Forward composition with
  future migrations is implicit and undocumented.

- 🟡 **Compatibility / Usability**: User running old slash command
  after upgrade but before migrate sees opaque errors
  **Location**: Phase 1.11/1.12 + Phase 2
  No SessionStart hint, no error-mapping. Users hit confusing
  dead-ends and must discover migration out-of-band.

- 🟡 **Usability**: Skill name `migrate` is overloaded — collides with
  database/branch/data-migration vocabulary
  **Location**: Phase 4.4
  Slash-command picker shows "migrate" without context; many users
  will assume it's about their app's database and skip it.

- 🟡 **Usability**: Terminology choice trades typing friction and
  colloquial familiarity for neutrality
  **Location**: Phase 0 ADR 1
  `/accelerator:create-work-item` is 5 characters longer than
  `/accelerator:create-ticket` and adds a hyphen. Most developers
  colloquially say "ticket" regardless of tool.

- 🟡 **Code Quality**: Driver script's `find_repo_root` factoring is
  left as an unresolved either/or
  **Location**: Phase 4.5
  Production driver depending on `test-helpers.sh` is a layering
  violation; replicating inline duplicates logic. Existing
  `config_project_root` is the canonical resolver and should
  be mandated.

- 🟡 **Code Quality**: Driver script lacks logging, error context,
  and run summary
  **Location**: Phase 4.5
  Per-step output, run-id tagging, end-of-run summary all absent.
  3am-pager scenario requires manual `jj diff` triage.

- 🟡 **Code Quality**: Hyphenation rule is enforced only by reviewer
  attention
  **Location**: Phase 3.6
  "Prose vs literal" rule (work item with space, work-item with
  hyphen) has no automated guard. Future edits will silently drift.

- 🟡 **Code Quality**: 15 default-literal duplications deferred to
  separate cleanup
  **Location**: "What We're NOT Doing"
  Rename touches every duplicate anyway; deferring centralisation
  means paying the same coordination cost on every future migration.

#### Minor

- 🔵 **Architecture**: Phase boundary leaks — `:53` of
  `work-item-template-field-hints.sh` straddles Phase 1 and Phase 3.
- 🔵 **Architecture**: Migration framework presented as generic but
  coupled to the rename — no second-migration smoke test.
- 🔵 **Architecture**: Review subsystem string-discriminator coupling
  not consolidated.
- 🔵 **Correctness**: Plan's line range for `refine-work-item` fence
  (353-365) is wider than actual fence (360-365).
- 🔵 **Correctness**: Test 4 read-only-parent-dir setup is
  platform-dependent (root in CI bypasses permissions).
- 🔵 **Correctness**: Phase 1.14 updates only the doc-comment of
  `config-read-template.sh:6`, not the recognition logic.
- 🔵 **Safety**: `sed -i` on user paths unsafe for paths with spaces
  or special characters.
- 🔵 **Safety**: Phase 3.5 hand-rolled bulk `jj mv` loop has no
  atomicity or partial-failure recovery.
- 🔵 **Safety**: Phase 5 self-apply lacks an explicit pre-migration
  bookmark/snapshot beyond ambient jj state.
- 🔵 **Test Coverage**: Test case 6 (empty repo) asserts behaviour
  whose contract is implicit.
- 🔵 **Test Coverage**: Driver-only tests (two pending, ordering,
  unknown-ID handling) absent.
- 🔵 **Test Coverage**: Mise task wiring for `test:migrate` uncertain
  ("verify by reading existing files like test-tickets.toml").
- 🔵 **Test Coverage**: Golden-fixture regenerate-by-hand-then-fix
  pattern invites self-confirming tests.
- 🔵 **Compatibility**: YAML rewrite via sed is brittle to
  formatting variations (nested vs flat).
- 🔵 **Compatibility**: Minor bump for breaking change is acceptable
  under existing convention but versioning policy is not written down.
- 🔵 **Usability**: ADR alternatives section omits abbreviations and
  shorter names (e.g. `item`, `card`).
- 🔵 **Usability**: Silent rewrite of `.claude/accelerator.md` will
  surprise users; per-file diff summary missing.
- 🔵 **Usability**: CHANGELOG instruction "Run `/accelerator:migrate`"
  too terse for the upgrade impact.
- 🔵 **Code Quality**: Byte-exact hierarchy fences across two files
  fragile (no canonical source).
- 🔵 **Code Quality**: "No migration metadata beyond filename ID"
  drops a one-line description per migration that costs almost
  nothing to add.
- 🔵 **Code Quality**: `rg '1\.19\.0-pre'` is a sanity check, not a
  contract; prior version-bump commit is the source of truth.
- 🔵 **Code Quality**: Bulk fixture-rename loop in Phase 3.5 encoded
  only as a code-comment, risk of partial rename.

### Strengths

- ✅ Clean phase decomposition with explicit jj-change atomicity
  (Phase 5 self-apply separated from Phase 4 framework build)
- ✅ Strict TDD red→green sequencing per phase makes intent
  verifiable from the test suite
- ✅ "What We're NOT Doing" section explicitly names deferrals and
  rationales
- ✅ Migration framework deliberately small (sorted glob registry,
  newline-delimited state file, no metadata) — YAGNI-respecting
- ✅ Belt-and-suspenders idempotency contract (state file +
  per-script idempotency) explicit and well-justified
- ✅ Plan correctly identifies that lens directory names
  (`clarity-lens`, etc.) do NOT need renaming
- ✅ CHANGELOG verification confirms the surface is wholly Unreleased,
  justifying the no-shim clean break
- ✅ Pluralisation rule (singular per-item, plural bulk) preserved
  consistently
- ✅ Phase 5 includes byte-level verification commands before commit
- ✅ Eval JSON binding awareness — plan correctly identifies and
  addresses fixture-string coupling

### Recommended Changes

Ordered by impact. Address the critical findings before any
implementation work begins.

1. **Reverse the pinned-config-override default** (addresses: 6
   cross-cutting findings) — In §4.6 steps 1-2, change the migration
   to preserve the user's pinned directory location and rewrite only
   the config key (`paths.tickets: <custom>` → `paths.work: <custom>`).
   Rename the directory only when the resolved path is the default.
   Update test 7 to assert this. Add a companion test for the
   default-path branch.

2. **Replace `sed -i ''` with a portable form** (addresses:
   correctness, safety, compatibility) — Use
   `sed 's/.../.../' file > file.tmp && mv file.tmp file` (which is
   also safer for partial-write protection) or `perl -i -pe`. Add a
   Linux test case (or a CI run on Linux) so this defect cannot be
   hidden by macOS-only testing.

3. **Add a clean-tree pre-flight check to the driver** (addresses:
   safety, usability "destructive-by-default", architecture
   "dry-run dropped without justification") — `run-migrations.sh`
   refuses to run if `jj status` (or `git status --porcelain`)
   reports uncommitted changes in `meta/` or `.claude/accelerator*.md`,
   with a `--force` env-var override. Document this as the primary
   safety mechanism in the SKILL.md.

4. **Specify config-rewrite logic concretely** (addresses:
   correctness "multi-key rewrite", compatibility "YAML brittle to
   formatting") — Decide between (a) anchored regexes with a
   documented list, (b) requiring `yq`, or (c) reusing
   `config_extract_frontmatter`. Add fixtures covering nested,
   flat-dotted, comment-with-"tickets", and unsupported-form cases.
   Use temp-file-then-rename for atomic write.

5. **Atomic-resequence the migration steps** (addresses: safety
   "partial-update window", correctness "idempotency interleaving")
   — Either order frontmatter rewrites BEFORE directory rename
   (so directory rename is the atomic commit point), or strengthen
   per-step idempotency to detect leftover `ticket_id:` in
   `meta/work/*.md` on retry. Add a test for crash-mid-script
   recovery.

6. **Mandate `config_project_root` in the driver, drop the either/or**
   (addresses: architecture, code-quality) — In §4.5, remove the
   "OR replicate inline" option; require the driver to `source`
   `scripts/config-common.sh` and call `config_project_root`.

7. **Resolve Phase 1.9 forward-reference cleanly** (addresses:
   architecture, correctness, code-quality) — Choose one of:
   (a) move both `config-read-template.sh` doc-comment update
   (§1.14) and the helper-script call site (§1.9 :53) and the
   template rename to a single phase, or
   (b) keep `config-read-template.sh` recognising both keys until
   Phase 3 with a tiny scoped shim. Eliminate the ambiguity.

8. **Add `/accelerator:migrate` discoverability hint** (addresses:
   compatibility, usability) — Either a SessionStart detection that
   spots `meta/tickets/` and emits a one-line hint, or extend
   `config-read-review.sh`'s "unknown mode" error to suggest
   `/accelerator:migrate`. Mirror the v1.8.0 init-detection
   precedent.

9. **Specify per-phase test-suite expectations explicitly**
   (addresses: test-coverage "RED-state verification", "fuzzy phase
   boundary") — For each phase, list the exact `mise run test:*`
   subsets that MUST be green and which MAY remain red, with the
   expected failing test names/messages at RED checkpoints.

10. **Expand the migration test suite** (addresses: test-coverage
    "missing edge cases", safety "merge-collision untested") — Add
    test cases for: both dirs already exist, malformed YAML config,
    corrupt state file, frontmatter with both keys, two-pending-
    migration ordering. Replace test 4's read-only-parent-dir setup
    with a deterministic forced-failure (e.g. pre-create
    `meta/work` as a regular file).

11. **Add content-preservation hash check to Phase 5** (addresses:
    test-coverage "no body regression check") — Before §5.3, capture
    sha256 of each file body; after migration, assert byte-equal
    bodies under `meta/work/`.

12. **Add migration metadata and per-step logging to driver**
    (addresses: code-quality "lacks logging", "no metadata") — Add
    a `# DESCRIPTION:` comment convention to migration scripts,
    have the driver echo a tagged status line per migration, and
    print an end-of-run summary table. Cheap, large UX gain.

13. **Refresh stale references against current state** (addresses:
    compatibility "version drift", correctness "fence line ranges",
    "mise task wiring") — Before execution, re-verify every
    line-number reference in the plan against current files;
    update Phase 6.2's source value to `1.19.0-pre.x` (current is
    `pre.4`); update §3.7 to `360-365` for refine; verify and
    list the actual mise task file paths for `test:tickets` and
    the new `test:migrate`.

14. **Engage the typing-friction tradeoff in the terminology ADR**
    (addresses: usability) — Phase 0 ADR 1 should explicitly
    weigh the ~20% per-command keystroke cost against the
    neutrality gain, and consider at least one short-name
    alternative (e.g. `item`, `card`).

15. **Expand CHANGELOG bullet to two sentences with preconditions**
    (addresses: usability "too terse") — Mention "commit pending
    changes first; review the resulting diff before committing".
    Match the v1.8.0 precedent of being explicit about destructive
    operations.

16. **Add follow-up tracker for default-literal centralisation**
    (addresses: architecture, code-quality "deferred 15 literals")
    — File a follow-up work-item to extract `PATH_DEFAULTS` to a
    shared file. The plan can stay scoped; the debt should be
    visible after merge.

---

## Per-Lens Results

### Architecture

**Summary**: The plan is structurally sound for a mechanical rename and
proposes a sensibly-scoped migration framework that follows existing
shell-and-state-file conventions. However, the migration framework's
design has several architectural concerns: ambiguous scope boundaries
(generic registry but coupled to this single rename), inconsistent
abort/state semantics between plan body and Phase 4 tests (test case 6
violates the ADR contract), and a brittle migration script that
re-implements config-key parsing with raw sed rather than reusing the
config layer.

**Strengths**:
- Clear separation of concerns across phases (ADRs first, surface
  rename in 1-3, framework in 4, self-apply in 5, release in 6) with
  explicit failure-mode isolation
- Migration framework follows existing architectural idioms
- Belt-and-suspenders idempotency contract well-justified
- Test-first construction of the migration skill against isolated
  fixtures
- Phase 5 separation from Phase 4 as a distinct jj change
- Lens directory names correctly identified as not needing rename

**Findings**: state-file write semantics for empty/no-op contradicts
ADR contract (major); migration script duplicates config-parsing
logic (major); plan silently drops dry-run default without ADR
justification (major); production driver depends on a test-helper
function or duplicates it (minor); phase boundary leaks for `:53`
(minor); migration framework presented as generic but coupled to the
rename (minor); 15 default literals deferred increases rebase pain
(minor); pinned-path migration semantics undecided and load-bearing
(minor); string-based mode discriminators have no architectural
consolidation (minor).

### Correctness

**Summary**: The plan's TDD sequencing, idempotency story, and
self-apply approach are mostly internally consistent, but several
concrete correctness defects would surface during execution. Most
serious are `sed -i ''` portability (BSD-only), under-specified
else-branches in rename idempotency guards, and the design decision
in test 7 to silently migrate a user's pinned-custom-path data.

**Strengths**:
- Phase 4/Phase 5 atomicity via separate jj changes
- Idempotency-by-construction for the rename migration is sound
- State-file design correctly avoids partial entries on failure
- Hierarchy fence content is byte-identical today (lockstep updates
  preserve byte-equality)
- Eval JSON files binding fixture paths as strings — plan addresses
  this correctly with commit-coupled JSON+filesystem rename

**Findings**: `sed -i ''` not portable (critical); pinned-config
override is destructive (critical); idempotency guards specify two
branches but not the third state (major); state file atomicity
preserved but filesystem state is not (major); `mise run test:tickets`
wiring not addressed (major); multi-key rewrite spans nested and flat
YAML with no canonical edit logic (major); Phase 1 RED commit leaves
overall test suite red (major); plan's line range for refine fence
wider than actual (minor); test 4 read-only-parent setup non-portable
(minor); Phase 1.14 updates only doc-comment, not recognition logic
(minor).

### Safety

**Summary**: The plan ships a destructive filesystem migration tool
with explicit "no dry-run, no rollback" posture and VCS as the sole
safety net. While VCS-as-recovery is reasonable for a developer
plugin operating on tracked repos, the plan has multiple concrete
data-safety gaps: no working-tree cleanliness pre-flight, a
partial-update window between mv and sed -i with no transactional
or recovery semantics, an under-specified merge-collision handler,
and a "pinned config override" case that the plan flags as needing
user confirmation but proceeds with the more destructive choice.

**Strengths**:
- Phase 5 separated from Phase 4 enables independent revert
- ADR specifies abort-without-state-write contract preventing one
  class of partial-state corruption
- Per-migration idempotency required as "suspenders" to state-file "belt"
- Test case 4 verifies abort-without-state-update behaviour
- Phase 5 includes post-migration verification commands

**Findings**: no working-tree cleanliness check (major); partial-update
window between mv and sed (major); merge-collision abort lacks
specification and is untested (major); pinned-config rename to
default is destructive (major); sed on paths with spaces unsafe
(minor); Phase 3.5 bulk `jj mv` loop has no atomicity (minor); Phase 5
self-apply lacks pre-migration bookmark (minor); no dry-run + no
rollback + no warning UX (minor).

### Test Coverage

**Summary**: The plan demonstrates strong TDD discipline by sequencing
red-then-green steps for each rename phase and authoring a 7-case
test suite for the new migration framework before implementation.
However, RED-state verification is loose, several high-risk migration
edge cases are uncovered, and the self-apply in Phase 5 lacks a
content-preservation regression check.

**Strengths**:
- TDD red→green discipline explicitly built into Phases 1-4
- Migration test suite enumerates 7 distinct scenarios
- Reuses shared assertion helpers and setup_test_repo pattern
- Per-migration idempotency treated as suspenders alongside state-file belt
- Phase 5 byte-level verification commands before commit
- Plan recognises eval JSON files bind fixture filenames as strings

**Findings**: RED-state verification doesn't assert failure cause
(major); migration test suite misses critical edge cases (major); no
content-preservation regression check for 29 self-applied work-items
(major); test 7 design unsettled (major); eval suites use live LLM
calls — no plan for cost/runtime (major); Phase 1 GREEN may not
produce fully-green suite — boundary fuzzy (major); test 4 read-only
setup platform-dependent (minor); test 6 empty-repo contract
ambiguous (minor); test isolation between tests not asserted (minor);
golden fixture authored-by-hand pattern invites self-confirmation
(minor); driver script logic not tested in isolation (minor); mise
task wiring for test:migrate uncertain (minor).

### Compatibility

**Summary**: The plan correctly identifies that a clean break is
appropriate because the entire ticket surface lives in the CHANGELOG
Unreleased section, and provides a thoughtful migration skill. However,
the version bump source value is wrong (current is `1.19.0-pre.4`,
not `1.19.0-pre.2`), the migration's `sed -i ''` breaks on Linux,
the framework lacks any forward/backward compatibility design, and
old slash-command/config-key invocations after upgrade-before-migrate
produce confusing errors with no diagnostic guidance.

**Strengths**:
- Correctly verifies via CHANGELOG that the ticket surface is fully
  Unreleased
- Migration skill reduces user-visible breakage vs v1.8.0 precedent
- Idempotency treated as first-class contract
- Removes old `ticket` mode literal cleanly with non-zero exit
- Plan acknowledges pre-release adopters with concrete migration path

**Findings**: `sed -i ''` not portable across BSD/GNU (major); plan
asserts current version `1.19.0-pre.2` but actual is `pre.4` (major);
migration framework has no forward/backward compatibility model
(major); user running old slash command before migrate sees opaque
errors (major); minor bump for breaking change worth flagging policy
(minor); YAML rewrite via sed brittle to format variations (minor);
custom-path migration policy destroys user intent (minor).

### Usability

**Summary**: The plan executes a thorough mechanical rename and
introduces a sensible migration framework, but several
developer-experience concerns are underweighted: 'work-item' adds
typing friction relative to 'ticket' and conflicts with users'
colloquial vocabulary; the migrate skill's destructive-by-default
design with no dry-run, no preview output, and no error-message
bridge from old slash commands risks user surprise; and the
alternatives ADR omits short/abbreviated names.

**Strengths**:
- Pluralisation rule preserved consistently
- Migration skill is idempotent and re-runnable with audit-trail
  state file
- Explicit prose-vs-literal hyphenation rule keeps reading natural
- Phase 5 self-apply gives a worked example
- CHANGELOG bullet folded into release section with command surfaced

**Findings**: terminology trades typing friction and colloquial
familiarity for neutrality (major); `/accelerator:migrate`
discoverability fragile (major); destructive-by-default with no
preview violates least-surprise (major); skill name `migrate`
overloaded with database/branch vocabulary (major); ADR alternatives
omits abbreviations and shorter names (minor); silent rewrite of
`.claude/accelerator.md` will surprise users (minor); custom
`paths.tickets` override silently rerouted (minor); CHANGELOG
instruction too terse for upgrade impact (minor); plan diverges from
research on `DRY_RUN` without explaining why (minor).

### Code Quality

**Summary**: The plan is methodical, TDD-driven, and well-scoped,
with clear phase boundaries and explicit deferrals. Principal
concerns are duplication that the plan locks in (15 default literals,
inline reimplementation of the path reader inside the migration
script, find_repo_root choice undecided), and several places where
rules expressed only in plan prose are vulnerable to silent human
drift because no automated guard enforces them. The migration
framework is pleasingly minimal but its driver script lacks small
affordances that would make a destructive shell tool comfortable to
maintain.

**Strengths**:
- Strict TDD red→green sequencing makes intent verifiable
- Explicit "What We're NOT Doing" section names deferrals
- Phase 4 migration framework deliberately small and YAGNI-respecting
- Phase 5 self-apply decoupled into separate jj change
- Prose-vs-literal hyphenation rule is the right rule
- Step 1.9 explicit about forward-reference

**Findings**: `find_repo_root` factoring left as unresolved either/or
(major); migration scripts reimplementing removed config readers will
compound (major); driver script lacks logging, error context, run
summary (major); deferred forward-reference leaves intermediate
commit with broken script path (major); hyphenation rule enforced only
by reviewer attention (major); deferring centralisation of path
defaults should be tracked (major); byte-exact hierarchy fences
fragile (minor); "no migration metadata beyond filename ID" drops
cheap maintainability win (minor); `rg '1\.19\.0-pre'` is sanity check
not contract (minor); test 7 pinned-config override flagged "confirm
with user" (minor); fixture rename loop encoded only as code-comment
(minor).

---
*Review generated by /review-plan*
