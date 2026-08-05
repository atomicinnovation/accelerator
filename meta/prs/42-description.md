---
type: pr-description
id: "42"
title: "Generalise the sub-binary registration surface"
date: "2026-08-03T13:46:38+00:00"
author: "Toby Clemson"
producer: describe-pr
status: complete
work_item_id: "0187"
parent: "work-item:0187"
relates_to: ["work-item:0136", "work-item:0168"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/42"
pr_number: 42
tags: [build-system, distribution, release, rust, docs]
revision: "ba749cf4d48dedd788af9251f7cded140686a0b9"
repository: "accelerator"
last_updated: "2026-08-03T13:46:38+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Generalise the sub-binary registration surface

## Summary

`accelerator-visualiser` is the only dispatched sub-binary today, and the producer side is shaped around that fact: the dispatch-coherence check reads one hardcoded SKILL.md path, the debug-archive stage is hardwired to one token, and the registration procedure itself exists only in research. Five epic-0136 stories each ship a sub-binary and would otherwise rediscover the surface one wrong guess at a time.

This makes the release path token-generic, replaces the visualiser-shaped coherence check with a real bidirectional guard that runs on every PR rather than only at release time, and writes the surface down as a thirteen-point checklist that a test keeps honest **in both directions** — a rename in code fails the test, not just a stale README.

Five phases, one commit each, each green on top of its predecessors.

## Changes

**A dependency-free parsing leaf** — `tasks/shared/skill_parsing.py`. The SKILL.md parsing core (eight functions, six regex constants) sat inside `tasks/lint/skill_permissions.py`, so consuming it meant dragging in `invoke`, `tasks.lint` and — via `vendor_shims` — `tasks.build`. A module-level import from the guard would have been a **hard circular import**, breaking `import tasks`, `tasks.build`, `tasks.lint` and `tasks.manifest`: every `mise run` entry point. The move is byte-for-byte pure; the only change is `_BARE_LAUNCHER` → `BARE_LAUNCHER`, now *derived* from a new `LAUNCHER` constant so the launcher path has one spelling. Adds `launcher_token`, so a rule's subcommand segment is extracted by exactly the code that extracts an invocation's. `tests/integration/support/skill_corpus.py` re-points in the same commit — `skill_permissions` is deliberately not left as a re-export façade.

**A generalised dispatch guard** — `tasks/shared/dispatch_coherence.py`, replacing `validate_dispatch_coherence` and `_VISUALISE_SKILL_RELATIVE` in `tasks/build.py`. It checks every non-exempt token in an injected collection in both directions and rejects the registry shapes that would make it vacuous or ship an undispatchable token: an empty collection, a stale exemption, an all-exempt collection, an exemption whose token *is* invoked, a reserved (`verify`/`launcher`) or built-in-shadowing token, and a token outside `^[a-z][a-z0-9-]*$`. Each failure names its cause and the offending SKILL.md. New `SKILL_EXEMPT_SUBBINARIES` registry, empty on landing.

**The guard now fails on a PR, not mid-release.** New `lint:dispatch-coherence:check` leaf wired into `build-system:check` (what CI actually runs) *and* `lint:check` (the only path a bare `mise run` reaches). `check` does **not** depend on `lint:check` and no CI job runs `lint:check`, so the latter alone would have reproduced `lint:skill-permissions:check`'s blind spot. The argument-free release call also moves to the first statement of `emit_manifest`.

**A token-generic debug-archive stage.** `DEBUG_ARCHIVE_DIRS` (a `MappingProxyType` registry) replaces the hardwired `debug_archive_path`, which now takes a **required** `bin_dir` — a `BIN_DIR` default is correct only for the visualiser. `create_debug_archives` splits into two pure helpers plus a thin argument-free `@task`, with guards that reject an undispatched registry key and a non-`bin/` directory. `_release_uploads` loops the registry instead of appending one archive per platform.

**`.gitignore`'s archive rule never worked, and now does.** A mid-string separator anchors a pattern to its own directory, so `bin/*.debug.tar.gz` matched only `<root>/bin/` — never `skills/visualisation/visualise/bin/`, which is where the archives are written. They were untracked *and* unignored, one `git add .` away from the pushed version-bump commit. Widened to `**/bin/*.debug.tar.gz`, with `.debug.tar.gz` added to `_ARTIFACT_MARKERS` as the backstop and `_assert_no_leaked_artifacts` switched to `-uall` (porcelain's default untracked mode collapses a wholly-untracked directory to one line, hiding exactly the case the marker is for).

**The signed manifest was shipping unattested.** All three `attest-build-provenance` blocks listed only `skills/…/accelerator-visualiser-*` and `dist/release/accelerator-*`, while `_release_uploads` also uploads `manifest.json` and `manifest.minisig` — the one document naming every sub-binary's sha256 and inline signature. Both are added to all three blocks, and the existing attest test is extended into a real coverage assertion **derived from `_release_uploads()`**, plus one-attestation-per-signing-step-per-job, sign→attest→finalise ordering, and subject-path set equality across blocks.

**Injected token collections on the release-stage builders**, under one idiom five sibling stories can copy: a direct default of the real constant on a *private helper*, annotated `Iterable[str]`. Every `@task` stays argument-free, because an invoke parameter becomes an operator-facing CLI flag on a release-path leaf. New `_subbinary_signing_targets` and `_subbinary_uploads`; `upload_and_verify_release` resolves the collection once and threads it, so the "every asset uploaded" and "every asset re-verified before `--draft=false`" lists cannot derive from different values. `collect_entries`' first parameter is renamed `subbinaries` → `tokens`.

**A manifest/token agreement check in `_publish`** (`_assert_staged_manifest_is_current`), between the leak guard and `commit_version`. `*:finalise` is separately invocable and `dist/release/` is never cleaned, so a manifest from an earlier cut is reachable — and since the registry changes once per sub-binary story, a stale manifest has the *same* token set. The **version** comparison is what catches it. Placed here rather than in `emit_manifest`, where both operands derive from one constant and the comparison would be a tautology.

**The thirteen-point checklist** — `## Registering a dispatched sub-binary` in `tasks/README.md`, each point tagged with where a mistake surfaces (**[PR]** a test or CI gate, **[release]** the release job, **[author]** nothing at all). Pointers added from `tasks/CLAUDE.md` and the root `CLAUDE.md`, since discovery otherwise depends on already holding the anchor link.

## Context

- Work item: `meta/work/0187-generalise-sub-binary-registration-surface.md`
- Plan: `meta/plans/2026-08-02-0187-generalise-sub-binary-registration-surface.md` (now `status: done`)
- Research: `meta/research/codebase/2026-08-02-0187-generalise-sub-binary-registration-surface.md`
- Reviews: `meta/reviews/plans/2026-08-03-0187-…-review-1.md`
- Validation: `meta/validations/2026-08-02-0187-…-validation.md`
- Parent epic: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Governing decision: ADR-0054 (git-style modular CLI of on-demand static binaries)
- Blocker **discharged**: 0168 (fold visualiser into the cli workspace) is `done`, so `_SUBBINARY_MANIFESTS`' visualiser entry is stable.
- Unblocks 0169–0173, the five stories that each ship a sub-binary.

This PR also closes both follow-ups PR #37 explicitly left open: the stale `.gitignore` comment it flagged (*"Only `SKILL.md` and the four `bin/*.debug.tar.gz` archives under this tree are tracked"*) is corrected, and `.debug.tar.gz` is added to `_ARTIFACT_MARKERS` as the defence-in-depth that description offered.

## Testing

- [x] `mise run check` — exit 0. The exact read-only set CI runs, across all four components.
- [x] `mise run build-system:check` — exit 0, including the new `lint:dispatch-coherence:check` member.
- [x] `mise run test:unit:tasks` — exit 0, **619 passed**. `mise run test:integration:tasks` — exit 0, **65 passed**.
- [x] `mise run lint:dispatch-coherence:check` and `mise run lint:skill-permissions:check` — both exit 0.
- [x] `uv run python -c "import tasks, tasks.build, tasks.lint, tasks.manifest"` — the circular import the guard would otherwise have introduced is gone at its cause.
- [x] `./scripts/validate-corpus-frontmatter.sh meta` — exit 0.
- [x] **Phase 1 is a verified pure move.** Byte-comparing each moved definition against `skill_permissions.py` at the parent commit: all eight functions and all six retained constants are identical.
- [x] **The guard is non-vacuous against the real skills tree**, not only against fixtures. Mutating a copy of `skills/`: broadening the visualiser's rule to `Bash(…/accelerator *)` raises naming that SKILL.md; `Bash(…/accelerator v*)` raises on the wildcarded segment; and — the other half of the defect below — narrowing to `Bash(…/accelerator visualiser --owner-pid *)` stays **green**.
- [x] **The workflow and docs guards fail on mutation.** A typo'd `subject-path` reddens two tests; a deleted attest block reddens the count test; renaming `[lints.clippy]` in `cli/visualiser/server/Cargo.toml` reddens the checklist's source-existence test. Each mutation reverted, working copy verified clean.
- [x] **`.gitignore`'s widened rule matches** an archive under the nested `bin/` tree, asserted via `pathspec.GitIgnoreSpec` in `tests/integration/tasks/test_release.py` and re-checked by hand.
- [x] **No behavioural drift in the release staging set.** `assert len(uploads) == 22` is intact, and `test_includes_subbinary_assets_when_present` — the suite's only end-to-end evidence that a registered token reaches a real `gh release upload` invocation — is untouched by this diff.
- [x] `invoke --help github.upload-and-verify-release` lists only `--version` — no seam leaked onto a publish task.
- [x] `skills/` is byte-unchanged across all five commits: the stricter guard passes the shipped tree as-is.
- [ ] **A real release cut.** `_assert_staged_manifest_is_current` and the two new `subject-path` entries are covered by test but first exercised for real on the next prerelease, which is CI- and secret-gated.

⚠️ `mise run test` (all suites, fully parallel) exited 1 on both attempts — **each time in a different component this PR does not touch**, and neither reproduces in isolation. Run 1: `TestVendorShimMarkerDigest::test_ignores_a_release_version_bump`, a pre-existing test that `shutil.copytree`s `cli/` *including* `cli/visualiser/frontend/node_modules`, raising on dangling `@tanstack/*` symlinks while concurrent frontend tasks touch that tree (`test_build.py` alone: 34 passed). Run 2: one frontend timing test of 2536, `use-dev-activation.test.tsx`, alongside a `[vitest-worker]` timeout (that file alone: 10 passed; `test:unit:tasks` in the same run: 619 passed). Both are load flakes in untouched code — see Notes.

## Notes for Reviewers

**The work item's specified permission probe does not work, in either direction.** It asked the guard to probe `covered_by("${CLAUDE_PLUGIN_ROOT}/bin/accelerator <token>", rule)`. `covered_by` appends `*` to a rule that lacks one, so the space before the `*` is a literal that must match — the probe returns `False` against the visualiser's real `Bash(… visualiser *)` rule and would fail the release for a correctly bound skill. The synthetic-sentinel variant drafted in its place is wrong *both* ways: it reports `Bash(… visualiser start)` and `Bash(… visualiser --owner-pid *)` as unbound, while certifying `Bash(… accelerator v*)` and `Bash(… accelerator [a-y]*)` as properly scoped — the latter pre-authorising every token not starting with `z`. The implemented check probes with the **actual invoked command** and adds a structural assertion that the rule's token segment equals the token exactly. `BARE_LAUNCHER` moves out of the per-rule matcher, where it would be dead code, into a skill-level veto where it catches globs at or above the binary segment that the charset check cannot see. Both halves are pinned by test and demonstrated against the real tree above.

**Two directions, deliberately different strictness.** *Binding* is strict — prefix-anchored, metacharacter-free, one token — because `skill_permissions` refuses to coverage-check a chained command and the two guards must not disagree about what a valid invocation is. *Invocation→registration* is deliberately permissive, scanning every launcher occurrence anywhere in the command text, because it is the fail-**closed** half: a token it cannot see is a token that ships unregistered.

**Three intended behavioural changes.** An empty `DISPATCHED_SUBBINARIES` previously *passed* the guard and now fails it (`test_both_absent_is_coherent` is deleted, deliberately reversing a committed assertion). The guard is stricter about what binds, so a skill edit landing between drafting and pickup that broadens a launcher rule surfaces as a red PR gate. And the `.gitignore` widening ignores files that were previously untracked-and-unignored — no tracked file becomes ignored.

**Known limitation, retained on purpose.** The checklist test's "imperative action line" predicate is a closed verb whitelist (`Add`, `Register`, `Update`, `Document`, `Extend`, or the literal `No action when`) coupled to English prose, so an innocuous rewording can fail the build for a reason that is not a defect. It was an accepted major in plan review 1; it is kept because an acceptance criterion requires it, and mitigated by naming the set in both the plan and the test with all thirteen items drafted against it.

**Recorded, not closed.** `covered_by`'s path-alias and quoting forms (`…/bin/../bin/accelerator *`, `…//bin/accelerator *`, `"…/bin/accelerator" *`) evade *both* guards today — they match no bare-launcher probe and yield no token. The new matcher-contract table records each with its actual outcome so the gap is legible now that a release gate depends on the model, but narrowing `covered_by` is explicitly out of scope: it models Claude Code's own matcher and is untested against it.

**Two small gaps I would take as a follow-up, not a blocker.** The validation found that two absence source scans — `_VISUALISE_SKILL_RELATIVE` and `_BARE_LAUNCHER` under `tasks/` — lack the positive control the plan mandates for every other scan in this change. Both are plain substring predicates rather than regexes, so there is no pattern to get subtly wrong, but a broken root or glob would let each pass vacuously. One line each fixes it.

**The full-suite flakes are worth their own items.** `test_ignores_a_release_version_bump` copytrees `node_modules` and follows symlinks, which makes it the most likely cause of a red `mise run` on a developer machine; adding `"node_modules"` to its ignore patterns fixes it and speeds it up. The frontend `use-dev-activation` timing test is unrelated to anything here. Neither is caused by this PR — it changes zero frontend files and zero files in either failing test.

**Scope note.** The debug-archive generalisation was absorbed here rather than raised as a sibling, overriding the work item's "local to the token loop" boundary rule, because the `bin/` constraint the new registry enforces is meaningless until the ignore rule actually covers a nested `bin/` tree. The checklist is thirteen points rather than the item's ten: an eleventh for user-facing docs, a twelfth for `DEBUG_ARCHIVE_DIRS` (which this PR creates as a registration point), a thirteenth for `cli/deny.toml`. All deviations are enumerated in the plan's *Deviations from the Work Item* section.

**Size.** 32 files of code and config (+2151/−239), plus four meta artefacts. The meta half is the research, plan, review and validation; reviewing the code half alone is reasonable.
