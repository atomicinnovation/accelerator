---
type: "pr-description"
id: "37"
title: "Untrack the visualiser debug archives"
date: "2026-08-02T19:20:49+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
relates_to: ["work-item:0168"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/37"
pr_number: 37
tags: ["build", "release", "visualiser", "gitignore", "hygiene"]
revision: "acb88ff0c88b3fa5c5cf752300c9e050bfacf7a7"
repository: "accelerator"
last_updated: "2026-08-02T19:20:49+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Untrack the visualiser debug archives

## Summary

The four `skills/visualisation/visualise/bin/accelerator-visualiser-*.debug.tar.gz` archives are release upload assets, not source. No `.gitignore` rule covered them, and `git.commit_version` stages with a bare `git add .`, so every prerelease bump has been committing ~17 MB of fresh gzip blobs into history. This adds the missing ignore rule and evicts the four tracked files.

## Changes

- **`.gitignore`** — one new rule, `bin/*.debug.tar.gz`, placed with the other `bin/` build-output ignores, with a comment recording why the archives live under the committed skill tree at all (the SLSA provenance glob in `main.yml` is rooted there) and why nothing in the plugin package needs them.
- **The four archives are deleted.** An ignore rule only suppresses *auto-tracking of untracked paths* — these were already in the tree, so the rule alone would have changed nothing. Both halves are required and both are in this diff.

After this lands, `skills/visualisation/visualise/` holds exactly one tracked file: `SKILL.md`.

## Context

`build.create_debug_archives` writes the archives into `BIN_DIR` (`skills/visualisation/visualise/bin/`) during `prerelease:prepare` / `release:prepare`, and `github._release_uploads` attaches them to the GitHub release from that same on-disk location. They are never entries in `manifest.json` and the launcher never fetches them — they exist only so someone can symbolicate a crash from a stripped release binary.

They became tracked at `1.24.0-pre.17` and have been rewritten by every bump since:

| Bump | Effect on the four archives |
| --- | --- |
| `1.24.0-pre.17` | added — 17,385,122 bytes |
| `1.24.0-pre.18` | four new blobs |
| `1.24.0-pre.19` | four new blobs |
| `1.24.0-pre.20` | four new blobs |

The per-bump byte deltas are tiny (`+5,927`, `-2,163`, …), but that is the *size* difference, not the *pack* difference: each bump stores four fresh ~4.3 MB blobs, and gzipped tarballs do not delta-compress against their predecessors. Four prereleases have gone through, so the cost is roughly additive, not amortised.

This is the follow-up PR #32 named. That PR added `frontend/dist/` and `server/target/` ignores for the same stale tree and explicitly deferred this: *"Untracking those four archives is separate in-flight work … and is not touched here."* It also flagged the coupling that shaped the pattern chosen here — any rule broad enough to catch the bare `accelerator-visualiser-*` binary would also have caught these archives while they were tracked, silently blocking `git add`. With them untracked that constraint is gone, but this PR does not widen the pattern; `bin/*.debug.tar.gz` is suffix-anchored and matches nothing else.

## Testing

- [x] `mise run check` — exit 0 (the exact read-only set CI runs: format, lint and types across all four components).
- [x] `uv run pytest tests/unit/tasks tests/tasks tests/integration/tasks -q` — 460 passed. The release/github/build suites exercise `debug_archive_path`, `create_debug_archives` and `_release_uploads` against tmp fixtures, so none of them depended on the archives being tracked.
- [x] `./scripts/validate-corpus-frontmatter.sh meta` — exit 0, with `meta/prs/37-description.md` in place.
- [x] `mise run lint:scripts:exec-bits:check` — exit 0. The exec-bits guard walks the tree honouring `.gitignore`, so a new ignore rule is capable of moving it; it does not.
- [x] **Ignore rule verified against a fresh path.** Created `bin/probe.debug.tar.gz`; `jj status` did not report it. Also verified the rule is unanchored and therefore covers both `bin/` trees, while the suffix keeps it clear of the tracked vendored shims (`bin/accelerator-verify-<platform>`).
- [x] **Release path re-read for breakage.** `create_debug_archives` opens with `BIN_DIR.mkdir(parents=True, exist_ok=True)`, so a checkout that no longer carries the directory still works. The provenance step's `skills/visualisation/visualise/bin/accelerator-visualiser-*` glob and `_release_uploads` both read the on-disk files, which `prerelease:prepare` still writes before either runs. Neither depends on git.
- [ ] **An actual release cut.** That the next bump commit comes out clean can only be observed on a real prerelease, which is CI- and release-gated.

## Notes for Reviewers

**A comment earlier in `.gitignore` is now stale and this PR does not fix it.** The block above `skills/visualisation/visualise/frontend/dist/` — added by PR #32 — reads *"Only `SKILL.md` and the four `bin/*.debug.tar.gz` archives under this tree are tracked"*. After this change only `SKILL.md` is. It is a comment, so nothing breaks, but it is wrong the moment this merges. Left alone deliberately so the diff stays exactly the ignore rule plus the eviction; say the word and I will fold the one-line correction in.

**`_assert_no_leaked_artifacts` never had a chance of catching this.** Its markers are `.sec`, `dist/release/` and `dist/`, and its comment describes itself as a backstop for *"anything under the gitignored staging tree (present only if the .gitignore entry regressed)"*. The debug archives were under neither, and were not gitignored, so they walked straight into `git add .` four times. Once this merges they stop appearing in `git status --porcelain` at all and the guard stays quiet either way — which is the argument for adding `.debug.tar.gz` to `_ARTIFACT_MARKERS` as defence-in-depth against a future regression of this very rule. Not done here; happy to as a follow-up if you want the guard to cover it.

**History is not rewritten.** The ~70 MB already in the pack across pre.17–pre.20 stays there. This PR only stops the bleeding; reclaiming it would need a filter of `main`, which is not worth it for a repo this size.

**Local re-runs during the transition still show the archives as modified.** Both git and jj key "tracked" off the parent commit, so until this merges, anyone who runs `build:debug-archives` on a branch based below it will see the four paths reappear as changes despite the rule. That resolves itself once the deletion is on `main`.

**Stack position.** Fourth and last: #34 (0169 split) → #35 (jj pin bump) → #36 (work-item closeout) → **#37**. Content-independent of all three; merge order is free once the bases retarget.
