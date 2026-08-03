---
type: plan-validation
id: "2026-08-02-0187-generalise-sub-binary-registration-surface-validation"
title: "Validation Report: Generalise the Sub-Binary Registration Surface Implementation Plan"
date: "2026-08-03T13:14:54+00:00"
author: "Toby Clemson"
producer: validate-plan
status: complete
result: "pass"
parent: "work-item:0187"
target: "plan:2026-08-02-0187-generalise-sub-binary-registration-surface"
tags: [build-system, distribution, rust]
last_updated: "2026-08-03T13:14:54+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Generalise the Sub-Binary Registration Surface

### Implementation Status

All five phases are implemented and committed, one commit per phase, in the
plan's declared dependency order:

- ✓ **Phase 1: Extract the SKILL.md parsing into a dependency-free leaf** —
  `zynvyolrnsut` "Extract the SKILL.md parsing primitives into a shared leaf"
- ✓ **Phase 2: The generalised guard** — `ttktlnmsnzkv` "Generalise the
  dispatch-coherence guard over the dispatch token"
- ✓ **Phase 3: Generalise the debug-archive stage** — `zvpqmmkxnsnr`
  "Generalise the debug-archive stage over the dispatch token"
- ✓ **Phase 4: Injectable seams on the release-stage builders** — `vpztlkwrznrv`
  "Give the release-stage builders injected token collections"
- ✓ **Phase 5: The registration checklist** — `xqnrlonoupln` "Document the
  sub-binary registration surface as a checklist"

35 files changed, +7537/−239 (of which three are the meta artefacts). Every file
the plan names is present with the shape the plan specifies; no file outside the
plan's change set was touched. The working copy is clean at `d7f96db6`.

### Automated Verification Results

Every command in the plan's Success Criteria was re-run at validation time.
Exit codes captured directly (not through a pipe, which would mask them):

| Command | Result |
|---|---|
| `mise run check` (read-only CI mirror) | ✓ exit 0 |
| `mise run build-system:check` | ✓ exit 0 |
| `mise run test:unit:tasks` | ✓ exit 0 — 619 passed |
| `mise run test:integration:tasks` | ✓ exit 0 — 65 passed |
| `mise run lint:dispatch-coherence:check` | ✓ exit 0 |
| `mise run lint:skill-permissions:check` | ✓ exit 0 |
| `uv run python -c "import tasks, tasks.build, tasks.lint, tasks.manifest"` | ✓ imports cleanly |
| `uv run pytest tests/unit/tasks/shared/test_skill_parsing.py` | ✓ 13 tests |
| `uv run pytest tests/unit/tasks/shared/test_dispatch_coherence.py` | ✓ 52 tests |
| `uv run pytest tests/unit/tasks/shared/test_paths.py` | ✓ 5 tests |
| `uv run pytest tests/unit/tasks/test_dispatch_coherence.py` | ✓ 2 tests |
| `uv run pytest tests/unit/tasks/test_registration_docs.py` | ✓ 75 tests |
| `uv run pytest tests/unit/tasks/test_workflows.py` | ✓ 20 tests |
| `uv run pytest tests/unit/tasks/test_build.py` | ✓ 34 tests |
| `uv run pytest tests/integration/tasks/test_github.py` | ✓ (22-upload pin green) |

⚠️ `mise run test` (all suites in parallel) exited 1 on both attempts, each time
in a **different component 0187 does not touch**, and neither failure reproduces
in isolation:

- Run 1 — `test:unit:tasks` ::
  `TestVendorShimMarkerDigest::test_ignores_a_release_version_bump`, a
  pre-existing test (untouched by these commits) that `shutil.copytree`s the
  whole `cli/` tree including `cli/visualiser/frontend/node_modules`. It raised
  `shutil.Error` on dangling symlinks under `@tanstack/*` while concurrent
  frontend tasks were touching `node_modules`. `test_build.py` alone: 34 passed.
- Run 2 — `test:unit:frontend` ::
  `use-dev-activation.test.tsx > restores the LATEST non-/dev path`, 1 failure
  of 2536, alongside a `[vitest-worker]: Timeout calling "onTaskUpdate"`. That
  file alone: 10 passed. `test:unit:tasks` in that same run: 619 passed.

0187 changed zero frontend files and zero files in either failing test, so
neither is attributable to this work. Both are load flakes under the fully
parallel roll-up. Recorded below as follow-ups.

### Code Review Findings

#### Matches Plan

**Phase 1 is a genuine pure move.** Byte-comparing each moved definition against
`tasks/lint/skill_permissions.py` at the parent commit: all eight functions
(`_frontmatter_lines`, `frontmatter_bash_rules`, `has_bare_bash`,
`frontmatter_name`, `preprocessor_commands`, `is_plugin_invocation`,
`covered_by`, `has_metacharacter`) and all six retained constants are
**identical**. `_BARE_LAUNCHER` → `BARE_LAUNCHER` is the only change, and it is
now derived from `LAUNCHER` rather than spelled a second time.
`tests/integration/support/skill_corpus.py` was re-pointed in the same change,
as the plan required.

**Phase 2's guard matches the plan's source almost line for line.**
`_authorises` carries the keyword-only `token`/`command`, has no dead
`BARE_LAUNCHER` conjunct; `_is_over_broad` is the two-part veto; `_bindings`
returns the (bound, invoked) pair; `_registry_problems` covers all six registry
shapes; `violations` takes a required `root` with keyword defaults on the real
constants. The lint leaf is registered across all four required files
(`tasks/lint/dispatch_coherence.py`, `tasks/lint/__init__.py`,
`tasks/__init__.py`, `mise.toml`) and is wired into **both** `build-system:check`
(`mise.toml:354`) and `lint:check` (`:471`) — the dual placement the plan argued
for. `validate_dispatch_coherence()` is the **first** statement of
`emit_manifest`, argument-free, ahead of `atomic_write_text`.

**Phase 3.** `.gitignore:54` is `**/bin/*.debug.tar.gz` with the anchoring
rationale in a comment, `:28`'s stale "are tracked" claim corrected;
`_ARTIFACT_MARKERS` gained `.debug.tar.gz` and `_assert_no_leaked_artifacts`
gained `-uall`; `DEBUG_ARCHIVE_DIRS` is a `MappingProxyType` and
`_SUBBINARY_MANIFESTS` was converted with it (annotation re-typed to `Mapping`);
`debug_archive_path` takes a **required** `bin_dir`; `_debug_archive_targets` /
`_write_debug_archives` / thin argument-free `@task` split matches, with both
`RuntimeError` guards. All three `attest-build-provenance` blocks gained
`manifest.json` and `manifest.minisig`.

**Phase 4.** All four seams use one idiom — a direct default of the real
constant on a private helper, `Iterable[str]`-annotated; every `@task` stays
argument-free (`invoke --help github.upload-and-verify-release` lists only
`--version`); `RELEASE_MANIFEST`/`RELEASE_MANIFEST_SIG` stay module globals;
`upload_and_verify_release` resolves the collection once and threads it to both
`_release_uploads` and `_release_reverifies`; `if not names: return []` is
retained in `_subbinary_reverifies`; `collect_entries`' first parameter is
renamed `tokens`. `_assert_staged_manifest_is_current` lives in `_publish`
between `_assert_no_leaked_artifacts` and `commit_version`, and carries the
load-bearing **version** comparison as well as the token-set one.

**Phase 5.** The README section is at the specified insertion point with exactly
thirteen numbered points, the closed verb set, the `[PR]`/`[release]`/`[author]`
tags, and the lead-in defining "token". `tasks/CLAUDE.md` carries the third
bullet verbatim and root `CLAUDE.md:14-16` names both the task-tree shape and
the checklist. The anchor `tasks/README.md#registering-a-dispatched-sub-binary`
resolves and is already linked from 0168 and 0170–0173.

**The guards are non-vacuous against the real tree.** Verified by mutating a
copy of the real `skills/` tree and calling `violations(root, tokens=,
exempt=())` — both halves of the review-1 probe defect confirmed closed:

| Mutation of the visualiser's rule | Result |
|---|---|
| baseline (unmutated real tree) | GREEN |
| broadened to `Bash(…/accelerator *)` | raises, naming `skills/visualisation/visualise/SKILL.md` |
| narrowed to `Bash(…/accelerator visualiser --owner-pid *)` | **GREEN** — a correctly scoped rule is not a false positive |
| `Bash(…/accelerator v*)` | raises — wildcarded segment |

**The workflow and docs guards fail on mutation** (each mutation applied in
place and reverted; working copy verified clean afterwards):

| Mutation | Failing test |
|---|---|
| one `subject-path` typo'd in one block | `test_every_attest_block_declares_the_same_subjects` + `test_attest_globs_cover_every_published_asset` |
| one whole attest block deleted | `test_every_signing_step_is_attested_before_it_publishes` |
| `[lints.clippy]` renamed in `cli/visualiser/server/Cargo.toml` | `test_each_named_thing_still_resolves_in_its_source[…-[lints.clippy]]` |

Plus: `.gitignore`'s widened rule matches
`skills/visualisation/visualise/bin/accelerator-visualiser-linux-x64.debug.tar.gz`
under `pathspec.GitIgnoreSpec`; `assert len(uploads) == 22` is intact
(`test_github.py:327`); `test_includes_subbinary_assets_when_present` (`:404`)
is unmodified — no diff hunk covers it; and `skills/` is byte-unchanged across
all five commits, so the drafting-time permission assumption held.

**Manual greps** all return what the plan predicted: no `_BARE_LAUNCHER` or
`_VISUALISE_SKILL_RELATIVE` anywhere under `tasks/`; `skill_parsing.py` imports
only `fnmatch` and `re`; no literal `visualiser` in `dispatch_coherence.py`,
`skill_parsing.py` or `github.py`; no `is not None` sentinel in `github.py` or
`signing.py`; no line over 80 columns in the new README section (the file's only
over-80 lines are pre-existing table rows outside it).

#### Deviations from Plan

- **The SLSA test was split, not extended.** The plan said to extend
  `test_attest_globs_include_the_launcher_binaries` with four assertions; the
  implementation replaced it with three named tests —
  `test_every_signing_step_is_attested_before_it_publishes` (count + ordering),
  `test_every_attest_block_declares_the_same_subjects` (symmetry), and
  `test_attest_globs_cover_every_published_asset` (coverage derived from
  `_release_uploads()` with the non-empty guard and `@actions/glob` semantics,
  `*` not crossing `/`). All four required assertions are present; the split
  gives sharper failure attribution, as the mutation table above shows. An
  improvement, not drift.
- **One assertion beyond the plan**: the count test also asserts
  `len(finalises) == len(signs)` per job, which is what lets the
  `sign < attest < finalise` zip be `strict=True`.

No other deviation found. Every deviation the plan itself recorded under
"Deviations from the Work Item" is implemented as written.

#### Potential Issues

- **Two absence source scans lack the positive control the plan's Testing
  Strategy mandates** ("each is an absence assertion, so each is paired with a
  positive control"):
  `TestSourceScans::test_no_hardcoded_visualiser_skill_path_under_tasks`
  (`tests/unit/tasks/shared/test_dispatch_coherence.py:450`) and
  `test_the_private_bare_launcher_alias_is_gone`
  (`tests/unit/tasks/test_skill_permissions.py:132`). Both assert `offenders ==
  []` over `(REPO_ROOT / "tasks").rglob("*.py")` with no companion assertion
  that the file list is non-empty. The exposure is smaller than the plan's
  stated worry — both predicates are plain substring tests, not regexes, so
  there is no pattern to get subtly wrong — but a broken root or glob would make
  each pass vacuously. The sibling `test_the_guard_names_no_token` does carry
  its control, and `test_the_parsing_leaf_imports_nothing_but_re_and_fnmatch`
  uses set equality, which cannot go vacuous.
- **`test_ignores_a_release_version_bump` copytrees `node_modules`.**
  Pre-existing and outside this plan's scope, but it is now the most likely
  cause of a red `mise run` on a developer machine: `shutil.copytree(cli_src,
  cli_dst, ignore=shutil.ignore_patterns("target"))` walks
  `cli/visualiser/frontend/node_modules` and follows symlinks, so any concurrent
  frontend task makes it raise. Adding `"node_modules"` to the ignore patterns
  would fix it and speed the test up.

### Manual Testing Required

Everything in the plan's Manual Verification lists was discharged mechanically
above, with two exceptions that need a human or a release environment:

1. Documentation reading:
   - [ ] Confirm the new `## Registering a dispatched sub-binary` section reads
     as a checklist an author can follow top-to-bottom without consulting the
     research or the plan. (It was read end-to-end during validation and does;
     this is a taste call worth a second pair of eyes before the first sibling
     story picks it up.)
2. Release path:
   - [ ] The next real release cut is the first end-to-end exercise of
     `_assert_staged_manifest_is_current` and of the two new `subject-path`
     entries. Both are covered by test, and the plan deliberately does not
     require a local cut (it needs the signing secret), but the first
     prerelease after this lands is the confirmation.

### Recommendations

- **Land as is.** Every acceptance-relevant gate is green and the guards are
  demonstrably live against the real tree, not just against fixtures.
- **Add the two missing positive controls** to the `_VISUALISE_SKILL_RELATIVE`
  and `_BARE_LAUNCHER` scans — a one-line `assert offenders_scanned` or a
  known-positive companion string. Cheap, and it completes the pattern the plan
  set for every other scan in this change.
- **Raise a follow-up for `test_ignores_a_release_version_bump`** to exclude
  `node_modules` from its copytree. It is pre-existing, unrelated to 0187, and
  the reason a full `mise run test` can go red on an otherwise clean tree.
- **Treat the frontend `use-dev-activation` timing flake as its own item** —
  1 failure in 2536 under parallel load, green in isolation, and unrelated to
  this work.
- The plan's own follow-up note stands: the first sibling token to land should
  convert `assert len(uploads) == 22` to a derived expression, as checklist
  point 1 already instructs.
