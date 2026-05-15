---
work_item_id: "0058"
title: "Workspace and Worktree Boundary Detection at Session Start"
date: "2026-05-15T09:53:35+00:00"
author: Toby Clemson
type: story
status: ready
priority: medium
parent: ""
tags: [hooks, vcs, jj, git, session-start]
---

# 0058: Workspace and Worktree Boundary Detection at Session Start

**Type**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As an Accelerator user working inside a jj secondary workspace or a git linked worktree nested under a parent repository, I want Accelerator to detect the workspace/worktree boundary at session start and inject context describing it, so that the model honours the boundary — never editing, running VCS commands against, or researching files in the parent repository instead of the active workspace.

Today, when a workspace or worktree directory is nested inside an ignored directory of the parent repo, Accelerator frequently treats the parent repo as the working surface, sometimes making edits in it. Because the user clears context often, any in-session correction is lost on the next clear; the detection must therefore be delivered through the SessionStart `additionalContext` channel so it is restored automatically on every fresh session.

## Context

Accelerator already ships a `hooks/vcs-detect.sh` SessionStart hook that probes for `.jj` and `.git` at the repo root and injects an `additionalContext` block telling the model which VCS to use. It does not currently distinguish between the *main* checkout of a repository and a *secondary* checkout (a jj workspace or a git linked worktree), so the model receives no signal that it should keep its operations inside a narrower boundary than the surrounding filesystem.

The accelerator convention places workspaces inside a `workspaces/` directory that the parent repo ignores. Because the parent's `.git`/`.jj` are still discoverable by walking up from inside the workspace, ambient tooling (and the model itself, via grep/find) routinely strays into the parent — sometimes silently, sometimes destructively.

Detection signals discovered during research:

- **jj secondary workspace**: `$JJ_ROOT/.jj/repo` is a *file* whose contents are a relative path to the main repo's `.jj/repo` directory. In the main workspace, the same path is a *directory*. (Anchor on `jj workspace root`, not path-walking.)
- **git linked worktree**: `git rev-parse --git-dir` differs from `git rev-parse --git-common-dir`; the worktree's `.git` is a file with a `gitdir: …/worktrees/<name>` pointer. The parent repo root is `dirname $(realpath $(git rev-parse --git-common-dir))`.
- **Cross-VCS nesting**: jj workspaces nested inside a git parent (or vice versa) require running both probes independently and comparing results — neither probe alone is sufficient.

Claude Code 2.1.0+ delivers SessionStart `additionalContext` silently (no user-visible message), capped at 10,000 characters, surviving the same lifecycle as the existing VCS-mode context that `vcs-detect.sh` already produces.

**Terminology**: *Main checkout* — a workspace/worktree whose VCS metadata is canonical (jj: `.jj/repo` is a directory; git: `.git` is a directory equal to `git rev-parse --git-common-dir`). *Secondary workspace / linked worktree* — a checkout whose VCS metadata points at a parent repo (jj: `.jj/repo` is a file; git: `.git` is a file). *Colocated* — a single directory that is simultaneously a jj workspace and a git worktree (same path, two independent parent repos). *Cross-VCS nesting* — a jj secondary workspace inside a pure-git parent, or a git worktree inside a pure-jj parent (different inner and outer VCS, not colocation). The work item's acceptance criteria treat colocation and cross-VCS nesting as distinct configurations.

## Requirements

- Detect, on SessionStart, whether the current working directory is inside (a) a jj secondary workspace, (b) a git linked worktree, or (c) both (the colocated cross-VCS case).
- When detection is positive, inject an `additionalContext` block stating: the fact (you are inside a workspace/worktree), the workspace/worktree absolute path, the parent repository absolute path, and the constraint (do not edit, run VCS commands against, or research files outside the workspace/worktree boundary).
- When detection is negative (main workspace, main worktree, or non-repo), emit nothing workspace/worktree-related — leave the existing VCS-mode context unchanged.
- Use authoritative VCS probes for detection: `jj workspace root` plus the `.jj/repo` file-vs-directory test for jj; `git rev-parse --git-dir` vs `--git-common-dir` for git. Do not rely on path-walking up the directory tree.
- Extend `hooks/vcs-detect.sh` rather than adding a sibling SessionStart hook. The boundary context is a natural continuation of the existing VCS-mode block and reuses its `REPO_ROOT` and `VCS_MODE` computation; producing one coherent message reads better than stacking two `additionalContext` blocks. Shared detection helpers belong in `scripts/vcs-common.sh` alongside the existing `find_repo_root`.
- Emit all paths normalised via `realpath` so that macOS `/private/var` vs `/var` (and equivalent symlink-resolution quirks) compare equal across every acceptance criterion.
- Cover the colocated jj+git case: a directory that is *both* a jj secondary workspace *and* a git linked worktree should produce a single coherent message rather than two stacked blocks.
- Cover cross-VCS nesting: a jj workspace inside a pure-git parent, and a git worktree inside a pure-jj parent, must both be detected and described correctly.

## Acceptance Criteria

- [ ] **AC1**: Given a session started inside a jj secondary workspace, when SessionStart hooks fire, then the injected `additionalContext` contains: (a) the workspace's absolute path; (b) the parent repo's absolute path; and (c) three prohibitions phrased verbatim as `do not edit files in <parent>`, `do not run VCS commands against <parent>`, and `do not grep, find, or research files in <parent>`, where `<parent>` is substituted with the resolved parent-repo absolute path. The prohibitions are verifiable by asserting each of the three exact phrases (with `<parent>` substituted) appears as a contiguous substring of the block.
- [ ] **AC2**: Given a session started inside a git linked worktree, when SessionStart hooks fire, then the injected `additionalContext` contains: the worktree's absolute path, the parent repo's absolute path (resolved from `git rev-parse --git-common-dir`), and the same three prohibitions enumerated in AC1.
- [ ] **AC3**: Given a session started inside a directory that is both a jj secondary workspace and a git linked worktree (same-path colocation), when SessionStart hooks fire, then exactly one `additionalContext` block is emitted, structured as: (a) the shared boundary path emitted once as a single labelled field, (b) the jj parent repo path emitted as a separate labelled field, (c) the git parent repo path emitted as a separate labelled field, and (d) the same three prohibitions enumerated in AC1 applied to both parent repos.
- [ ] **AC4**: Given a session started inside a jj secondary workspace nested under a pure-git parent (cross-VCS nesting), when SessionStart hooks fire, then the inner-boundary path equals the output of `jj workspace root` and the outer-parent path equals the directory containing the parent's `.git` (resolved via `git rev-parse --git-common-dir`). (Both paths are `realpath`-normalised per the global Requirements note.)
- [ ] **AC5**: Given a session started in the main jj workspace or main git worktree of any repository, when SessionStart hooks fire, then no workspace/worktree warning is emitted and the `additionalContext` produced by `vcs-detect.sh` is byte-identical to the corresponding golden snapshot in `tests/fixtures/vcs-detect-pre-0058/`. Snapshots for a main jj workspace and a main git checkout MUST be captured into that fixture directory before any implementation work on `vcs-detect.sh` or `vcs-common.sh` begins.
- [ ] **AC6**: Given a session started in a directory that is neither a workspace nor a worktree (plain repo or non-repo), when SessionStart hooks fire, then the hook produces no workspace/worktree output and does not error.
- [ ] **AC7**: Detection helpers reside in `scripts/vcs-common.sh` (or a sibling sourced module) and expose three independently-callable shell functions: `find_jj_main_workspace_root <dir>`, `find_git_main_worktree_root <dir>`, and `classify_checkout <dir>` (the latter printing exactly one of `main`, `jj-secondary`, `git-worktree`, `colocated`, or `none` on stdout). After `source scripts/vcs-common.sh`, each function is invocable from a shell with no other setup, prints its result on stdout, and exits with status 0 on success.
- [ ] **AC8**: The existing `SessionStart` entry for `vcs-detect.sh` in `hooks/hooks.json` is retained unchanged (empty `matcher`, `command` rooted at `${CLAUDE_PLUGIN_ROOT}/hooks/vcs-detect.sh`), so the extended hook fires on every session start including after `/clear`.
- [ ] **AC9**: The hook-placement decision (extend `vcs-detect.sh` rather than add a sibling `workspace-detect.sh`) is recorded as a top-of-file comment in `hooks/vcs-detect.sh` naming the alternative considered and the rationale (one coherent VCS-environment message; shared `REPO_ROOT` / `VCS_MODE` computation).

## Open Questions

- Should the colocated jj+git case (workspace + worktree at the same path) ever produce two messages, or always one combined? AC3 mandates one combined; this question is recorded only in case implementation surfaces a reason to revisit.

## Dependencies

- Blocked by: none
- Blocks: any future PreToolUse enforcement that hard-blocks tool calls escaping the workspace boundary (out of scope here).
- Related: 0020 (original VCS-detection ADR / SessionStart context-injection groundwork)
- Requires: Claude Code 2.1.0+ — silent SessionStart `additionalContext` delivery (capped at 10,000 characters) is the chosen delivery mechanism; older Claude Code versions surface the context as a user-visible message and would degrade the UX.
- Requires (existing artefacts): `hooks/vcs-detect.sh`, `scripts/vcs-common.sh` (with `find_repo_root`), and `hooks/hooks.json` in their current shape — this work extends rather than replaces them. Concurrent refactoring of these files must coordinate with this work item.
- External: jj-vcs/jj — the `.jj/repo` file-vs-directory marker is officially an internal detail (tracking `jj workspace repo-root` upstream at `jj-vcs/jj#8758`); if jj changes the marker before that lands, detection breaks.

## Assumptions

- Context injection via SessionStart `additionalContext` is sufficient to change model behaviour. If the model is observed to defy strongly-worded SessionStart context (as it has done with some existing VCS-mode warnings), the fix would need to escalate from context-injection to PreToolUse enforcement — that would be a separate, larger work item.
- The new hook extends or sits alongside `vcs-detect.sh` and reuses the same `additionalContext` delivery mechanism — not a new top-level hook category.
- The accelerator convention of workspaces under an ignored `workspaces/` directory is the dominant failure case; detection that handles it correctly will handle simpler cases (worktrees beside the main repo) by construction.

## Technical Notes

- `scripts/vcs-common.sh` already exposes `find_repo_root()` (walks up looking for `.jj` or `.git`). New helpers should be added there. Note that `find_repo_root` does *not* distinguish secondary checkouts and may need either extension or supplementary helpers.
- `hooks/vcs-detect.sh` produces SessionStart `additionalContext` JSON; the same shape can carry the workspace/worktree fields.
- jj API caveat: the `.jj/repo` file-vs-directory marker is officially an internal detail per `jj-vcs/jj#8758`; an `jj workspace repo-root` command is requested upstream. Until then, the file-vs-directory test is the most reliable signal.
- macOS/Linux `realpath` differences (`/private/var` vs `/var`) need normalising when comparing paths.
- `GIT_CEILING_DIRECTORIES` may be useful to scope git's discovery during detection.

## Drafting Notes

- Treated as a story rather than a bug because the current behaviour is "feature absent", not "feature regressed". A reviewer may reasonably re-classify as bug given the destructive failure mode.
- Scoped to context-injection only, not PreToolUse enforcement, even though the user named editing, VCS commands, and research as risks — a single SessionStart context block addresses all three uniformly. Hard blocking would be a separate larger story.
- Excluded emitting any context for the main-workspace/main-worktree case to keep the SessionStart payload quiet by default. A reviewer might prefer symmetric "you are in the main repo" messaging.
- Linked 0020 as Related, not as parent — 0020 is an ADR-creation task scoped to the original `vcs-detect.sh` design, not an umbrella for ongoing VCS-detection enhancements.
- The "ignored directory inside parent repo" detail in the user's request is treated as a *diagnostic clue* about why misdirection happens (the model sees the parent repo via grep/find), not as a detection input — the fix uses VCS probes, not gitignore introspection.
- Priority left at medium per default; given the user reported the issue happens "pretty frequently" and has caused parent-repo edits, a reviewer may justifiably raise to high.

## References

- Related: 0020 (VCS abstraction layer — original ADR-creation ticket for `vcs-detect.sh` and SessionStart context injection)
- `hooks/vcs-detect.sh`, `hooks/vcs-guard.sh`, `scripts/vcs-common.sh`, `hooks/hooks.json`
- Jujutsu workspace docs: https://docs.jj-vcs.dev/latest/working-copy/
- `jj workspace repo-root` upstream request: https://github.com/jj-vcs/jj/issues/8758
- git-worktree(1): https://git-scm.com/docs/git-worktree
- Claude Code hooks reference: https://code.claude.com/docs/en/hooks
