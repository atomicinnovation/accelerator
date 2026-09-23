---
type: "work-item"
id: "0241"
title: "False Dirty-Tree Detection on jj Repositories"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "ready"
kind: "bug"
priority: "high"
parent: "work-item:0136"
relates_to: ["work-item:0116", "work-item:0119", "work-item:0286", "work-item:0198", "work-item:0124"]
tags: ["migration", "vcs", "jj"]
last_updated: "2026-09-23T16:28:26+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-771"
---

# 0241: False Dirty-Tree Detection on jj Repositories

**Kind**: Bug
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

On jj repositories the migrate run base is the working-copy commit `@`,
whose id changes on every snapshot, so the guarded resume never engages and
users are forced onto `ACCELERATOR_MIGRATE_FORCE=1`, which removes the
safety net the pre-flight exists to provide.

Users want the dirty-tree checks to agree with `jj status`, so that
`accelerator migrate` resumes partial runs and refuses only over genuinely
foreign dirt. Beyond the observed defect, the jj dirty-path computation
diverges from `jj status` in two further ways that would cause the same
false refusals: it ignores the git excludes and the new-file size limit.
Because the computation is shared, fixing those deliberately changes what
all three of its consumers report on jj: the migrate pre-flight,
`work sync`, and the VCS status renderer. The refusal also gains the list
of foreign paths, so the user can see what blocked the run.

## Context

Reported in the further-ideas backlog as "`accelerator migrate` detect dirty
tree when not dirty", and reproduced in GitHub issue #97: in a jj-colocated
repository, resuming a stalled 0007 migration with `--decisions-file` is
refused as a dirty working tree even though every dirty path is the
migration's own output and the parent of `@` is unchanged. The stall
message promises a resume that never happens.

The fix covers every jj repository. Requirements 1, 2, 4, 5 and 7 do not
depend on the backend. For requirement 3, a repository with a git backend,
colocated or not, reads the git excludes from its backing git repository:
the workspace's `.git` when colocated, `.jj/repo/store/git` otherwise. A
repository with another backend has no backing git repository, so only
`core.excludesFile` from the user's global git config (or git's default
global ignore file) applies, as in `jj status`.

Terms used below. Guarded resume, pre-flight, owned dirt, foreign dirt and
the per-run manifest come from 0119; stall comes from 0116; run base,
migrate scope and dirty-path computation are introduced here.

- **Pre-flight**: the check `accelerator migrate` runs before applying
  migrations, which refuses when the working copy has foreign dirt.
- **Stall**: a deliberate halt of an interactive migration that needs a
  human decision; the stall message names a decisions file to resume with.
- **Run base**: the identifier, derived from the VCS revision, that a
  migration run records in its per-run manifest (as the manifest's run id)
  when it starts. Under git it is `HEAD`'s commit id.
- **Guarded resume**: the pre-flight proceeding over a dirty tree because
  every dirty path is owned dirt. It is observed as stderr beginning
  "Resuming over this run's own partial migration output:" and listing the
  owned paths, with the manifest's run base unchanged. A resumed 0007 run
  completes when 0007 appears in the applied ledger.
- **Applied ledger**: `.accelerator/state/migrations-applied`, one applied
  migration id per line.
- **Stale manifest**: a manifest whose recorded run base differs from the
  current run base.
- **Owned dirt**: dirty paths recorded in a manifest that is not stale. A
  stale manifest owns nothing, per 0119's fail-closed rule.
- **Foreign dirt**: any other dirty path under a migrate scope, including
  paths recorded in a stale manifest.
- **Migrate scope**: the paths the pre-flight checks for dirt, `meta/`,
  `.accelerator/` and `.claude/accelerator`.
- **Dirty-path computation**: the shared adapter code that lists changed
  and untracked paths in the working copy, reached through `dirty_paths`
  and `working_copy_diff`. Its only callers are the three consumers named
  in the Summary. It returns repo-relative paths.
- **Snapshot**: jj's own act of recording the working copy into `@`,
  triggered by a jj CLI command such as `jj status`. The dirty-path
  computation is not a snapshot in this sense, and the criteria's "the
  working copy is snapshotted" means running `jj status`.

Under git the run base is stable across edits. Under jj it is `@`'s id,
which changes on every snapshot, including the `jj status` the user runs to
inspect the dirt.

The governing principle: the current jj change is treated exactly as git
treats its working copy. Its parents are the base; its content is the dirt.

## Reproduction

Environment: a jj-colocated git repository. #97 does not record the jj
version; the defect has not yet been reproduced on the pinned jj 0.43.0,
and the first failing test does so.

The corpus must hold a structurally ambiguous reference: one that migration
0007 (`0007-unify-meta-corpus-frontmatter`) cannot rewrite without a human
choosing its target. The stall fixture in
`cli/migrate-cli/tests/list_and_decisions_file.rs` builds one.

1. Run `accelerator migrate`. Migration 0007 stalls; the stall message
   names a decisions file and says a re-run with `--decisions-file`
   resumes.
2. Write the named decisions file.
3. Run `jj status` to inspect the working copy.
4. Run `accelerator migrate --decisions-file <path>`.

Expected: the guarded resume engages and 0007 completes.

Actual: the run is refused with the dirty-working-tree refusal
(`DIRTY_TREE_REFUSAL`), although every dirty path is owned dirt. The only
way forward is `ACCELERATOR_MIGRATE_FORCE=1`.

Not yet observed, inferred from the dirty-path computation:

- An untracked file under `meta/` matching `core.excludesFile` or
  `.git/info/exclude`: `jj status` omits it; migrate reports it as foreign
  dirt and refuses.
- An untracked file under `meta/` larger than `snapshot.max-new-file-size`:
  `jj status` omits it; migrate reports it as foreign dirt and refuses.

## Requirements

1. On jj, the run base is derived from the parents of `@`, the analogue of
   git's `HEAD`. For a single parent it is that parent's commit id; for a
   merge it is the parents' commit ids sorted lexicographically and joined
   by `+`. It follows the parents' commit ids, so it changes whenever any of
   them changes, whether `@` moves (`jj new`, `jj commit`, rebasing `@`,
   `jj edit` of a change with other parents) or a parent is rewritten in
   place (`jj describe @-`). It does not change when only `@`'s content or
   description changes.
2. The revision stamped into corpus document metadata on jj remains `@`'s
   commit id; only the run base moves to the parents.
3. The jj dirty-path computation honours the git excludes as `jj status`
   does, for all three consumers. The excludes are `core.excludesFile`
   (`~`-expanded, and resolved against the workspace root when relative),
   git's default global ignore file when `core.excludesFile` is unset, and
   the backing git repository's `info/exclude`. A tracked file matching an
   exclude is still reported when changed.
4. The jj dirty-path computation does not report untracked files larger than
   `snapshot.max-new-file-size`, for all three consumers. The limit is
   resolved as jj 0.43.0 resolves it for `jj status` in that workspace:
   across the system, user, repo and workspace config layers (the user
   layer being the paths in `JJ_CONFIG` when it is set, used exclusively,
   else jj's platform `config.toml` and `conf.d` and the legacy
   `~/.jjconfig.toml`), later layers overriding earlier ones, in every value form
   jj accepts (a byte count such as `1024`, or a string such as `"1024"` or
   `"1KiB"`), falling back to jj's 1 MiB default when no layer sets it. A
   limit of `0` means no limit. A file exactly at the limit is reported. A
   value jj rejects is a dirty-path computation error whose message names
   `snapshot.max-new-file-size`; each consumer handles it as it handles any
   such error today, logging a warning that carries that message. For the
   migrate pre-flight that means proceeding as if the tree were clean, so
   an invalid limit disables the dirty-tree refusal; changing that is
   outside this item. Resolving the limit writes nothing to the repository
   or to the user's jj config directory.
5. The migrate dirty-tree refusal names each foreign dirty path, and no owned
   path, on both git and jj.
6. Git behaviour is unchanged apart from requirement 5: the existing git
   tests for `migrate`, `work sync` and the VCS status renderer pass
   unmodified, apart from assertions on the refusal text.
7. On jj, a run that stalled before this change, whose manifest records an
   `@`-based run base, is treated as stale: every attempt to resume it is
   refused, listing its outputs as foreign dirt. The user recovers by
   running once with `ACCELERATOR_MIGRATE_FORCE=1`, which starts a new run
   recording a parent-derived run base; if that run stalls, resuming it
   needs no force. `CHANGELOG.md` tells users so.

## Acceptance Criteria

Each jj criterion holds in both a colocated and a non-colocated repository.

- [ ] Given a jj repository where migration 0007 has stalled, when the user
  writes the decisions file named in the stall message, runs `jj status`,
  then runs `accelerator migrate --decisions-file <path>`, then the guarded
  resume engages and 0007 completes without `ACCELERATOR_MIGRATE_FORCE`.
- [ ] Given a jj repository where 0007 has stalled and only its owned dirt
  is present, when a file the run already wrote is edited and `jj status`
  is run, or `@` is described with `jj describe`, then
  `accelerator migrate --decisions-file <path>` engages the guarded resume;
  and when instead `@` is rebased onto a different parent or `@`'s parent
  is rewritten with `jj describe @-`, then the manifest is stale and the
  run is refused, naming the run's recorded paths as foreign dirt.
- [ ] Given a jj repository where 0007 has stalled, when `jj new` or
  `jj commit` starts a new change, or `jj edit` moves to a change with a
  different parent, leaving no dirt under a migrate scope, then
  `accelerator migrate --decisions-file <path>` runs as a fresh run: stderr
  carries no resume message, and the new manifest's run base is the new
  parent's commit id.
- [ ] Given a clean jj repository, when a migration run starts, then the
  run base in its manifest is `@`'s parent's commit id; for a merge of two
  or of three parents it is their commit ids sorted lexicographically and
  joined by `+`, whichever order the parents are recorded in; and when
  `@`'s parent is the root commit it is the root commit's id.
- [ ] Given a jj repository where 0007 has stalled, when `jj edit` moves to
  a sibling change with the same parent whose content under `meta/` the
  run did not write, then the run base is unchanged and the run is refused,
  naming that content as foreign dirt.
- [ ] Given a jj repository whose working-copy commit is a merge on which
  0007 has stalled, when an owned file is edited and `jj status` is run,
  then the guarded resume still engages; and when one parent of the merge
  is rewritten with `jj describe`, then the manifest is stale and the run
  is refused.
- [ ] Given a jj repository, when a corpus document is written, then its
  stamped revision is `@`'s commit id, as today.
- [ ] Given a jj repository whose only untracked file under `meta/` matches
  a pattern in `core.excludesFile`, when each consumer runs, then
  `accelerator migrate` proceeds, `work sync` reports the tree clean, and
  the VCS status renderer does not list the file. Likewise for a pattern in
  the backing git repository's `info/exclude`; for a pattern in
  `$XDG_CONFIG_HOME/git/ignore` when `core.excludesFile` is unset and
  `XDG_CONFIG_HOME` is non-empty; for a pattern in `~/.config/git/ignore`
  when both are unset, and again when `XDG_CONFIG_HOME` is empty; for
  `core.excludesFile` given as a relative path, which resolves against the
  workspace root; and for `core.excludesFile` given as `~/ignore-file`,
  which resolves against the home directory.
- [ ] Given a jj repository with `core.excludesFile` set to a file without
  the pattern and the pattern only in git's default global ignore file,
  then the untracked file is listed by the migrate refusal and the VCS
  status renderer and `work sync` reports the tree dirty; and a changed
  tracked file matching an exclude pattern is likewise listed by the
  refusal and the renderer and makes `work sync` report the tree dirty.
- [ ] Given a jj repository whose only untracked files under `meta/` are
  2 KiB, 1024 B and 512 B, and a 1 KiB limit, then the migrate refusal and
  the VCS status renderer list the 1024 B and 512 B files and not the
  2 KiB file, for each of these layer setups:
  - repo 1 KiB, user 1 MiB;
  - workspace 1 KiB, repo 1 MiB, user unset;
  - user 1 KiB set through `JJ_CONFIG`, every other layer unset;
  - user 1 KiB set through `JJ_CONFIG`, system 1 MiB;
  - system 1 KiB, every other layer unset;
  - repo 1 KiB written as `1024`, as `"1024"` and as `"1KiB"`.
- [ ] Given the same files, then the migrate refusal and the VCS status
  renderer list all three files, including the 2 KiB one, for each of
  these setups, where a later layer raises the limit set by an earlier
  one:
  - user 1 KiB, repo 1 MiB;
  - repo 1 KiB, workspace 1 MiB;
  - system 1 KiB, user 1 MiB set through `JJ_CONFIG`.
- [ ] Given each 1 KiB-winning layer setup above and a jj repository whose
  only untracked file under `meta/` is 2 KiB, then `work sync` reports the
  tree clean; with only the 1024 B file present, it reports the tree
  dirty; and for each 1 MiB-winning setup with only the 2 KiB file
  present, it reports the tree dirty.
- [ ] Given a jj repository with the limit unset in every layer, then an
  untracked file of exactly 1,048,576 B under `meta/` is reported and a
  2 MiB one is not, and with only the 2 MiB file present
  `accelerator migrate` proceeds; with the limit set to `0`, the 2 MiB file
  is reported, and with only it present `work sync` reports the tree
  dirty.
- [ ] Given a jj repository with the limit set to `"abc"` in repo config and
  foreign dirt at `meta/a.md`, when each consumer runs, then each emits a
  `warn`-level log event whose error names `snapshot.max-new-file-size`;
  `accelerator migrate` proceeds as if the tree were clean; `work sync`
  reports every item's dirtiness as unknown; and the VCS status renderer
  prints `(status unavailable)`.
- [ ] Given a jj repository with a 2 KiB and a 1024 B untracked file under
  `meta/` and a user limit of 1 MiB, and a 1 KiB limit set in one of: a
  legacy `.jj/repo/config.toml`; a legacy `.jj/workspace-config.toml`; or
  the repo config of a repository on which `jj status` was run at path A
  before it was copied to path B, with the dirty paths computed at B; when
  `dirty_paths` is called once, then its result omits the 2 KiB file and
  includes the 1024 B file; and the paths and content hashes of the user's
  jj config directory and of `.jj/repo/config.toml`,
  `.jj/workspace-config.toml`, `.jj/repo/config-id` and
  `.jj/workspace-config-id` are identical immediately before and
  immediately after that call.
- [ ] Given owned dirt at a path in the run manifest and foreign dirt at
  `meta/a.md` and `.accelerator/b`, when `accelerator migrate` refuses, then
  stderr carries the existing refusal text and names `meta/a.md` and
  `.accelerator/b` but not the owned path; and with no manifest, a first
  run over the same foreign dirt names both paths; on both git and jj.
- [ ] Given a git repository: when migration 0007 has stalled, the decisions
  file is written and `git status` is run, then the guarded resume engages
  without `ACCELERATOR_MIGRATE_FORCE`; when the only untracked file under
  `meta/` matches `core.excludesFile` or `.git/info/exclude`, then
  `accelerator migrate` proceeds; the run base and the stamped corpus
  revision are both `HEAD`'s commit id; and the existing git tests for
  `migrate`, `work sync` and the VCS status renderer pass unmodified apart
  from assertions on the refusal text.
- [ ] Given a jj repository whose corpus holds two ambiguous references and
  whose manifest records a stalled run with an `@`-based run base from
  before this change, and only that run's dirt, when
  `accelerator migrate --decisions-file <path>` runs, then it exits with
  status 1, prints the dirty-tree refusal naming that run's dirty paths as
  foreign, prints no panic output, and leaves the working copy and manifest
  unmodified; a second attempt without force is refused the same way; with
  `ACCELERATOR_MIGRATE_FORCE=1` and a decisions file covering only the
  first reference, a new run starts and stalls on the second, naming a new
  decisions file; and a re-run with that file and without
  `ACCELERATOR_MIGRATE_FORCE` engages the guarded resume.
- [ ] Given each jj repository in the run-base, exclude and size criteria
  above except the `"abc"` and copied-repository ones, when `dirty_paths`
  is called over the whole workspace and its result recorded, and
  `jj status` (jj 0.43.0) is then run on the same repository, then the
  recorded repo-relative paths equal the repo-relative paths `jj status`
  lists, counting a rename as both its source and destination path.
- [ ] `CHANGELOG.md` carries an entry for this fix stating that a jj
  migration run which stalled before upgrading cannot be resumed; the user
  runs once with `ACCELERATOR_MIGRATE_FORCE=1`, which starts a new run whose
  later stalls resume without it.

## Open Questions

- Can requirement 4 resolve the limit faithfully through a hand-built
  `StackedConfig`, without `UserSettings`? The first implementation step
  settles this. If it cannot, a work item for a `lint:vcs-settings:check`
  exception is created at once and added to `blocked_by`, and Toby Clemson,
  as owner of the build-system lints, decides it.

## Dependencies

- Blocked by: none, unless requirement 4 proves to need a
  `lint:vcs-settings:check` exception (see Open Questions)
- Blocks: none
- Related: 0119 (built the guarded resume this repairs, and defines owned
  and foreign dirt and the per-run manifest); 0116 (defines the stall);
  0286 (rewrites the migrate skill's dirty-path guidance, which should
  point at the paths the refusal now lists; whichever lands second
  reconciles the two); 0198 (the VCS status renderer, whose jj output
  changes under requirements 3 and 4); 0124 (a separate false-dirty bug in
  git-worktree root detection)
- Consumers whose jj output changes: the migrate pre-flight, `work sync`
  dirtiness and the VCS status renderer (requirements 3 and 4)
- External: parity is defined against the jj version pinned in `mise.toml`
  (0.43.0), which is kept in lockstep with the `jj-lib` 0.43 crate pin. A
  jj upgrade must re-check exclude and `snapshot.max-new-file-size`
  behaviour, and the repo and workspace config discovery, which reproduces
  jj-lib's `secure_config` layout (config-id files, per-id directories,
  legacy paths). The `jj status` parity tests are the tripwire: they fail
  when an upgrade changes that behaviour.
- Build system: `lint:vcs-settings:check` forbids constructing jj
  `UserSettings` in `cli/vcs-adapters` beyond two named exceptions, so
  requirement 4 must resolve the limit through a `StackedConfig` built
  without `UserSettings`. Granting a new exception is not part of this item;
  if one is needed it is decided in its own work item (see Open Questions).
- Test environment: the parity tests use the existing hermetic jj harness
  in `vcs-test-support` (`hermetic::assert_jj_matches("0.43.0")`,
  `Hermetic::jj`), which `cli/vcs-adapters/tests/dirty_paths.rs` already
  uses to spawn the pinned jj CLI.

## Assumptions

- Issue #97 and the backlog entry describe the same defect. The exclude and
  size parity requirements are inferred from the code rather than observed;
  they stay in this item because each is the same principle applied to the
  same dirty-path computation, and each produces the same false refusal.
- Anchoring to the parents of `@` means content already snapshotted into
  `@`, described or not, counts as dirty until a new change is started,
  matching git's uncommitted changes.
- A run that stalled before this change recorded `@` as its run base, which
  will never equal the new parent-derived run base; requirement 7 covers
  its recovery.

## Technical Notes

- Run-base check: `cli/migrate/src/preflight.rs` compares the manifest's run
  id with `revision`, sourced via `FileMigrationContext::revision` in
  `cli/migrate-adapters/src/context.rs`. Corpus metadata reads the same
  revision today, so the run base is read separately rather than by
  changing `jj_revision`.
- jj revision: `jj_revision` in `cli/vcs-adapters/src/library.rs` returns the
  view's working-copy commit id; git's `git_revision` returns `HEAD`.
- jj dirty-path computation: `snapshot::working_copy_diff` in
  `cli/vcs-adapters/src/library/snapshot.rs` passes
  `base_ignores: GitIgnoreFile::empty()` and `max_new_file_size: u64::MAX`;
  its diff baseline is already the parent tree of `@`.
- Consumers and their error handling: the migrate pre-flight reaches the
  computation through `cli/migrate-adapters/src/dirty_path_scanner.rs`,
  which logs a warning and treats the tree as clean on error; `work sync`
  through `cli/work-adapters/src/sync/working_copy_status.rs`, which logs a
  warning and marks dirtiness unknown; the VCS status renderer through
  `library/status_log.rs`, whose caller in `cli/vcs-cli/src/report.rs`
  logs a warning and prints `(status unavailable)`. All three warnings are
  `warn`-level tracing events carrying the error.
- `lint:vcs-settings:check` keeps jj `UserSettings` out of
  `cli/vcs-adapters`, so `snapshot.max-new-file-size` must be read without
  them. `library.rs` already builds a `StackedConfig` from the system, user
  and `JJ_CONFIG` layers to resolve `user.name`; it has no repo or workspace
  layer, which requirement 4 needs.
- jj 0.43.0 stores repo and workspace config outside the repository, in the
  user's jj config directory under `repos/<id>/config.toml` and
  `workspaces/<id>/config.toml`, the id read from `.jj/repo/config-id` and
  `.jj/workspace-config-id` (`jj_lib::secure_config`). jj-lib's
  `SecureConfig` loaders are not read-only: they migrate a legacy
  `.jj/repo/config.toml` or `.jj/workspace-config.toml` in place and
  rewrite `metadata.binpb` when the repository has moved or been copied. The
  adapter locates these files without calling those loaders.
- jj parses the limit as `HumanByteSize` (`jj_lib::settings`), treats `0` as
  unlimited, and skips a new file only when its size is strictly greater
  than the limit (`local_working_copy.rs`).
- `core.excludesFile` is read from the backing git repository's config
  snapshot, `~`-expanded, and joined to the workspace root when relative;
  unset, jj falls back to the XDG default. `info/exclude` is read from the
  backing git repository's directory. Without a git backend, only the
  global git config's `core.excludesFile` or the XDG default applies
  (jj-cli `WorkspaceCommandHelper::base_ignores`).
- Refusal text: `DIRTY_TREE_REFUSAL` in `cli/migrate-cli/src/render.rs`;
  `PreflightError::ForeignDirt` carries no paths today and maps to
  `kernel::Error::Failed`, exit status 1, in `cli/migrate-cli/src/main.rs`.
  The resume message is `render::resume_affordance`.
- The git dirty path via gix already drops stat-only `NeedsUpdate` entries,
  so git is not implicated.

## Drafting Notes

- The three consumers are all the callers of `dirty_paths` and
  `working_copy_diff` today.
- Corpus metadata keeps stamping `@`: a document's stamp records the state it
  was written from, which on jj is `@`, whereas the run base must be stable
  across the edits a run makes.
- Both parity gaps were found by reading the dirty-path computation
  (`GitIgnoreFile::empty()`, `u64::MAX`) against the principle of treating
  `@` like git's working copy, not from an observed failure.
- The invalid-limit behaviour follows each consumer's existing error
  handling rather than changing it. The pre-flight treating an erroring
  computation as clean is existing behaviour outside this item, and the
  item states it so the gap stays visible.
- Priority raised to high because #97 makes the guarded resume unusable on
  jj and forces the force-bypass.
- Title broadened from `migrate` because the defect lives in the shared VCS
  adapter, and from jj-colocated because the fix covers every jj repository
  with a git backend.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
- Reproduction: https://github.com/atomicinnovation/accelerator/issues/97
- Related: 0116, 0119, 0286, 0198, 0124
