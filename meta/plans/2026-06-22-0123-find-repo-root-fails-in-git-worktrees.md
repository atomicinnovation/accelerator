---
type: plan
id: "2026-06-22-0123-find-repo-root-fails-in-git-worktrees"
title: "find_repo_root fails in git worktrees (-d → -e) Implementation Plan"
date: "2026-06-22T14:07:17+00:00"
author: Phil Helm
producer: create-plan
status: draft
work_item_id: "work-item:0123"
parent: "work-item:0123"
derived_from: ["codebase-research:2026-06-22-0123-find-repo-root-fails-in-git-worktrees"]
relates_to: ["work-item:0058", "work-item:0020", "work-item:0124"]
tags: [plan, scripts, vcs, git, worktree, conductor, bug]
revision: "de38f5413b80f247fa8104ebebca0ed43d9a234b"
repository: "accelerator"
last_updated: "2026-06-22T14:18:48+00:00"
last_updated_by: Phil Helm
schema_version: 1
---

# find_repo_root fails in git worktrees (-d → -e) Implementation Plan

## Overview

`find_repo_root()` and `vcs_mode()` in
[`scripts/vcs-common.sh`](../../scripts/vcs-common.sh) detect VCS markers with a
directory test (`[ -d "$dir/.git" ]`). In a git **linked worktree** `.git` is a
regular **file** (a `gitdir:` pointer), so the test never matches. The result:
`find_repo_root` walks to `/` and returns 1 (aborting `set -euo pipefail`
callers such as the visualiser launchers with empty stderr), and `vcs_mode`
returns `none` (routing `work-item-file-dirty.sh` into its "fail safe to dirty"
branch, so every work-item file reads as dirty inside a worktree).

The fix is the same one-character change in both functions — `-d` → `-e`
(existence rather than directory-ness) — mirroring the sibling helper
`_ancestor_has_marker`, which already uses `-e`. The work item (0123) scoped
only `find_repo_root`; this plan additionally fixes `vcs_mode` (per the
research's headline finding and the user's scope decision) so the work item's
acceptance criterion — "all ~30 downstream callers work unchanged in Conductor
workspaces" — is genuinely met.

> **Scope note**: This widens the work item's stated scope. Work item 0123's
> "Out of scope" and "Acceptance Criteria" sections have been amended to include
> `vcs_mode` and its regression test so the two artifacts agree.

## Current State Analysis

- [`find_repo_root`](../../scripts/vcs-common.sh#L8-L18) seeds from `$PWD`,
  walks up while `dir != /`, and tests `[ -d "$dir/.jj" ] || [ -d "$dir/.git" ]`
  at line 11. In a git worktree this is false at every level → return 1.
- [`vcs_mode`](../../scripts/vcs-common.sh#L27-L36) tests the identical `-d` on
  `.jj` (line 29) and `.git` (line 31); in a worktree both are false → returns
  `none`. Its sole production caller,
  [`work-item-file-dirty.sh:45`](../../skills/work/scripts/work-item-file-dirty.sh#L45),
  treats `none` as indeterminate and
  [fails safe to dirty](../../skills/work/scripts/work-item-file-dirty.sh#L101-L104).
- The blast radius is ~26 production call sites across three sourcing tiers, but
  the `-d`→`-e` change is **monotonic**: `-e` matches a superset of `-d`, so no
  currently-working caller can regress; the change only *adds* the worktree
  `.git`-file match.
- The consistency precedent already exists in-file:
  [`_ancestor_has_marker:49`](../../scripts/vcs-common.sh#L49) uses `-e`.
- [`hooks/vcs-detect.sh:28-36`](../../hooks/vcs-detect.sh#L28-L36) also uses `-d`
  on the markers, but its `else` arm defaults to `git`, which is correct for a
  worktree — benign, no change needed.

### Key Discoveries

- The regression test home already exists and was *intentionally* left open for
  this fix: the `find_repo_root` block at
  [`hooks/test-vcs-detect.sh:424-446`](../../hooks/test-vcs-detect.sh#L424-L446)
  carries a comment stating it deliberately does *not* lock in worktree
  behaviour "to keep room for a future fix."
- The fixture is ready:
  [`make_git_linked_worktree`](../../hooks/test-vcs-detect.sh#L86-L94) builds a
  real worktree whose `.git` is a file and sets `FIXTURE_PARENT` /
  `FIXTURE_WORKTREE`. It is already used against `find_git_main_worktree_root`
  and `classify_checkout`, but never against `find_repo_root` or `vcs_mode`.
- `vcs-common.sh` is sourced at
  [`test-vcs-detect.sh:254`](../../hooks/test-vcs-detect.sh#L254), so both
  functions are in scope inside the regression block.
- The suite is a flat top-to-bottom script (no test framework); assertions come
  from `scripts/test-helpers.sh` (`assert_eq`), never `exit` on failure, and
  tally via `test_summary`. Run with `bash hooks/test-vcs-detect.sh` or
  `mise run test:integration:hooks`. Missing tooling exits 77 (skip).
- `-e` is bash-3.2-safe; the change must pass `scripts/lint-bashisms.sh` and
  `mise run check`.

## Desired End State

From inside a git linked worktree:

- `find_repo_root` returns the worktree path with exit 0 (no abort under
  `set -euo pipefail`), both from the worktree root and from a nested
  subdirectory.
- `vcs_mode "<worktree-root>"` returns `git`.
- `/accelerator:visualise` (and `stop`/`status`) start successfully.
- `work-item-file-dirty.sh` reports a clean work-item file as clean (not "always
  dirty").
- Both behaviours are locked in by regression tests that fail if reverted to
  `-d`; `mise run check` and `scripts/lint-bashisms.sh` pass.

## What We're NOT Doing

- No broader refactor of `find_repo_root` / `vcs_mode` to delegate to the 0058
  authoritative-probe layer (`classify_checkout`,
  `find_git_main_worktree_root`). The minimal `-d`→`-e` patch is the chosen
  direction; delegation is a separate, larger concern. **This deferral must be
  tracked, not left in prose** — 0123 exists precisely because 0058 carved
  `find_repo_root` out of scope and that deferral was never recorded as
  actionable, so the gap resurfaced as a production bug. Capture the convergence
  as a backlog work item ("migrate the legacy lexical helpers `find_repo_root` /
  `vcs_mode` to delegate to the 0058 probe layer") and link it from this plan's
  `relates_to`. See [Follow-up](#follow-up).
- No change to `hooks/vcs-detect.sh` (its `-d` use is benign — `else` defaults
  to `git`).
- No patching of the installed plugin-cache copy
  (`~/.claude/plugins/cache/…`); the durable fix is repo source + a release.
- No hardening of individual `set -e` callers; making `find_repo_root` succeed
  is the durable fix.

## Implementation Approach

Test-driven, two phases, each targeting one defect and one function:

- **Phase 1** fixes `find_repo_root` and the loud visualiser abort.
- **Phase 2** fixes `vcs_mode` and the silent work-item "always dirty"
  degradation.

The two *fixes* are independent (the functions do not interact), but the two
*phases are not freely reorderable*: both add assertions to the same
`find_repo_root` regression block (`hooks/test-vcs-detect.sh:424-446`), and
Phase 1 is the one that deletes-and-replaces the "room for a future fix" comment
there. If the phases land out of the presented order, that shared block must be
reconciled by hand. Treat Phase 1 → Phase 2 as the intended order; if they are
landed as a single change, merge the two `[0123]` stanzas into one.

Within each phase, follow red-green: add the failing worktree assertion to the
existing regression block first, confirm it fails against the current `-d`
code, then apply the one-line `-d`→`-e` fix and confirm green. Phase 1 lands
the headline bug fix; Phase 2 is additive (the two functions are independent),
but is presented — and should land — second because Phase 1 is the work item's
original scope and owns the shared-comment rewrite.

---

## Phase 1: Fix `find_repo_root` and add worktree regression test

### Overview

Make `find_repo_root` succeed inside a git linked worktree, and lock the
behaviour in with assertions from both the worktree root and a nested
subdirectory.

### Changes Required

#### 1. Regression test (RED first)

**File**: [`hooks/test-vcs-detect.sh`](../../hooks/test-vcs-detect.sh#L424-L446)
**Changes**: Append a worktree stanza to the existing `find_repo_root`
regression block (lines 424-446) and update the closing comment, which currently
documents the deliberate omission, to state the behaviour is now asserted.

```bash
echo "Test [0123]: find_repo_root returns worktree root in a git linked worktree"
make_git_linked_worktree
RESULT=$( (cd "$FIXTURE_WORKTREE" && find_repo_root))
assert_eq "git linked worktree root" "$FIXTURE_WORKTREE" "$RESULT"

echo "Test [0123]: find_repo_root walks up from a nested subdir in a worktree"
mkdir -p "$FIXTURE_WORKTREE/nested/deeper"
RESULT=$( (cd "$FIXTURE_WORKTREE/nested/deeper" && find_repo_root))
assert_eq "git linked worktree nested subdir" "$FIXTURE_WORKTREE" "$RESULT"

echo "Test [0123]: find_repo_root exits 0 under set -e in a worktree"
RC=0
( set -e; cd "$FIXTURE_WORKTREE" && find_repo_root >/dev/null ) || RC=$?
assert_eq "git linked worktree exit code" "0" "$RC"
```

The exit-code assertion guards the *actual* production failure mode (a non-zero
return aborting a `set -euo pipefail` caller with empty stderr), which the
stdout-value assertions above do not — mirroring the existing `|| RC=$?` capture
pattern at [`hooks/test-vcs-detect.sh:286-288`](../../hooks/test-vcs-detect.sh#L286-L288).

**Delete the entire lines 443-446 comment block** (`(We deliberately do NOT lock
in … keeps room for a future fix.)`) and replace it wholesale with a one-line
note that 0123 now asserts the worktree case. Every clause of the old comment —
including ".git is a file there, find_repo_root's -d test skips it" — becomes
false once the fix lands, so none of it may survive; do not append to it.

#### 2. The fix (GREEN)

**File**: [`scripts/vcs-common.sh`](../../scripts/vcs-common.sh#L11)
**Changes**: Line 11, `-d` → `-e` on both markers.

```bash
    if [ -e "$dir/.jj" ] || [ -e "$dir/.git" ]; then
```

### Success Criteria

#### Automated Verification

- [x] Before the fix, the new worktree assertions FAIL: `bash hooks/test-vcs-detect.sh`
      reports FAIL for the two `[0123]` `find_repo_root` tests. (RED confirmed —
      under `set -e` the suite aborts at the first `[0123]` value assertion,
      reproducing the production failure mode exactly.)
- [x] After the fix, the hooks suite passes: `mise run test:integration:hooks`
      (129 passed, 0 failed).
- [x] Reverting line 11 to `-d` makes the new tests fail again (red-green proof).
- [x] Bashisms/bash-3.2 floor passes: `bash scripts/lint-bashisms.sh`
- [ ] Full read-only CI mirror passes: `mise run check` (run once at end of
      Phase 2 — both phases touch only shell files).

#### Manual Verification

- [ ] From inside a git linked worktree (e.g. a Conductor workspace),
      `/accelerator:visualise` starts successfully (no empty-stderr abort).
- [ ] `/accelerator:visualise stop` and `status` also work from the worktree.

---

## Phase 2: Fix `vcs_mode` and add worktree regression test

### Overview

Make `vcs_mode` return `git` (not `none`) for a worktree root, restoring correct
work-item dirty-detection inside worktrees. Independent of Phase 1.

### Changes Required

#### 1. Regression test (RED first)

**File**: [`hooks/test-vcs-detect.sh`](../../hooks/test-vcs-detect.sh#L424-L446)
**Changes**: Add `vcs_mode` assertions alongside the Phase 1 stanza. Reuse the
`$FIXTURE_WORKTREE` already built by the Phase 1 stanza — do **not** call
`make_git_linked_worktree` again (it is already in scope, and rebuilding is
wasted work that reads as if a fresh fixture were required). Also lock in the
`.jj`-WINS ordering against a colocated checkout using the existing
[`make_colocated_secondary`](../../hooks/test-vcs-detect.sh#L96-L157) fixture
(`FIXTURE_TARGET`), where `.jj` is a directory and `.git` is a file — the case
where the `-e "$root/.jj"` precedence must still win.

```bash
echo "Test [0123]: vcs_mode returns git for a git linked worktree root"
# pre-fix: .git is a file, the -d test fails → vcs_mode returns 'none'.
assert_eq "vcs_mode worktree" "git" "$(vcs_mode "$FIXTURE_WORKTREE")"

echo "Test [0123]: vcs_mode preserves .jj-WINS for a colocated checkout"
make_colocated_secondary
assert_eq "vcs_mode colocated" "jj" "$(vcs_mode "$FIXTURE_TARGET")"
```

The colocated assertion is a non-regression guard: it returns `jj` under both
`-d` and `-e` (`.jj` is a directory there), so it does not go RED before the fix,
but it locks the WINS ordering the `-d`→`-e` change must not disturb.

#### 3. End-to-end consumer regression (the protected behaviour)

**File**: [`skills/work/scripts/test-work-item-scripts.sh`](../../skills/work/scripts/test-work-item-scripts.sh)
**Changes**: The existing `work-item-file-dirty.sh` tests drive the guard through
`WORK_DIRTY_MODE_OVERRIDE`, which short-circuits the real `vcs_mode()` call
([`work-item-file-dirty.sh:41-46`](../../skills/work/scripts/work-item-file-dirty.sh#L41-L46)),
so they never exercise the worktree path this plan fixes. Add a test that runs
the guard **without** the override, from inside a real git linked worktree, so
the `find_repo_root` → `vcs_mode` → dispatch chain is exercised end-to-end:

```bash
# In a real git linked worktree (no WORK_DIRTY_MODE_OVERRIDE), a committed
# work-item file reads CLEAN (exit 1) and a modified one reads DIRTY (exit 0).
# Pre-fix, vcs_mode returns 'none' → fail-safe-to-dirty → the clean case
# wrongly returns exit 0.
( cd "$WT" && git add item.md && git commit -q -m "add" )
( cd "$WT" && "$DIRTY_SH" "$WT/item.md" ); assert_eq "wt clean" "1" "$?"
printf 'changed\n' >>"$WT/item.md"
( cd "$WT" && "$DIRTY_SH" "$WT/item.md" ); assert_eq "wt dirty" "0" "$?"
```

This suite does not currently have the `make_git_linked_worktree` helper (it
lives in `hooks/test-vcs-detect.sh`). During implementation, either (a) build a
minimal inline worktree in this test (`git init` a parent, commit, `git worktree
add`), or (b) factor `make_git_linked_worktree` into a shared test helper that
both suites source. Prefer (a) for a single call site to avoid over-abstraction;
choose (b) only if a second consumer needs it. Note the `$?`-capture caveat: the
guard runs under the suite's own shell, so capture the exit code immediately
(`rc=$?`) rather than relying on `set -e`, consistent with the suite's style.

#### 2. The fix (GREEN)

**File**: [`scripts/vcs-common.sh`](../../scripts/vcs-common.sh#L27-L36)
**Changes**: Lines 29 and 31, `-d` → `-e`. The `.jj`-WINS ordering is preserved:
`-e "$root/.jj"` (false in a git worktree) falls through to `-e "$root/.git"`
(true, the pointer file) → returns `git`.

```bash
  if [ -e "$root/.jj" ]; then
    printf 'jj\n'
  elif [ -e "$root/.git" ]; then
    printf 'git\n'
```

Also add a one-line note to the `vcs_mode` header comment
([`scripts/vcs-common.sh:20-26`](../../scripts/vcs-common.sh#L20-L26)) — and to
the `find_repo_root` comment
([`scripts/vcs-common.sh:6-7`](../../scripts/vcs-common.sh#L6-L7)) — recording
that `-e` (existence, not directory-ness) is deliberate because `.git` is a
regular *file* in a git linked worktree. Keep the file's terse comment style.
This stops a future reader "tightening" `-e` back to `-d` believing the file case
is a bug.

### Success Criteria

#### Automated Verification

- [ ] Before the fix, the new `vcs_mode` worktree assertion FAILS (returns
      `none`): `bash hooks/test-vcs-detect.sh`
- [ ] Before the fix, the new end-to-end consumer test FAILS (the clean
      work-item file wrongly reads dirty/exit 0):
      `bash skills/work/scripts/test-work-item-scripts.sh`
- [ ] After the fix, both suites pass: `mise run test:integration:hooks` and
      `bash skills/work/scripts/test-work-item-scripts.sh`
      (`work-item-file-dirty.sh` is shell-tested there — there is no pytest
      target for it).
- [ ] The colocated non-regression assertion (`vcs_mode` → `jj`) passes both
      before and after the fix (WINS ordering undisturbed).
- [ ] Reverting lines 29/31 to `-d` makes the worktree `vcs_mode` test and the
      end-to-end consumer test fail again.
- [ ] `bash scripts/lint-bashisms.sh` and `mise run check` pass.

#### Manual Verification

- [ ] From inside a git linked worktree, a clean work-item file is reported as
      clean by `work-item-file-dirty.sh <path>` (exit 1), not "always dirty"
      (exit 0).
- [ ] A genuinely modified work-item file is still reported dirty (exit 0).

---

## Testing Strategy

### Unit / Integration Tests

- Extend the `find_repo_root` regression block in
  `hooks/test-vcs-detect.sh:424-446` with the worktree-root, nested-subdir,
  exit-code-under-`set -e`, and `vcs_mode` (worktree + colocated) assertions —
  the worktree cases driven by the existing `make_git_linked_worktree` fixture,
  the WINS guard by `make_colocated_secondary`.
- Add an **end-to-end consumer** regression in
  `skills/work/scripts/test-work-item-scripts.sh` that runs
  `work-item-file-dirty.sh` from inside a real git linked worktree *without* the
  `WORK_DIRTY_MODE_OVERRIDE` seam, asserting clean→exit 1 / modified→exit 0.
  This is the assertion that actually protects Phase 2's stated benefit; the
  existing override-based tests do not exercise the `vcs_mode` path.
- Each RED-capable assertion (worktree `find_repo_root`, its exit code, worktree
  `vcs_mode`, and the end-to-end consumer test) is written and confirmed RED
  against the current `-d` code before the corresponding `-d`→`-e` fix turns it
  GREEN. The colocated `vcs_mode` assertion is a non-regression guard (green
  both before and after).

### Key Edge Cases

- Nested subdirectory inside the worktree (walk-up correctness, not just the
  immediate directory).
- `.jj`-WINS ordering preserved in `vcs_mode` for colocated checkouts (covered
  by existing colocated tests — verify they still pass).
- `-e` matching a `.git`/`.jj` of any file type (regular file, symlink-to-
  existing, directory) still yields a correct repo-root/mode: the marker's
  *presence* is the only signal `find_repo_root` needs, so the edge is correct-
  by-construction, not merely unrealistic. `-e` follows symlinks and is true for
  any existing entry on both macOS and Linux; the only case where `-e` differs
  from `-d` in the *false* direction is a broken-symlink `.git`, which is not a
  supported repo layout. The authoritative-probe helpers are unaffected either
  way.

### Manual Testing Steps

1. In a git linked worktree, run `/accelerator:visualise` and confirm it starts.
2. In the same worktree, run `work-item-file-dirty.sh` against a clean and a
   modified work-item file and confirm correct clean/dirty results.

## Migration Notes

None. The change is backward-compatible (monotonic): `-e` matches everything
`-d` matched plus worktree pointer files. No data or schema migration.

## Follow-up

This plan deliberately patches the legacy lexical detection strategy rather than
converging it with the 0058 authoritative-probe layer. To stop that divergence
becoming permanent-by-default (the exact failure mode that produced 0123):

1. **Tracked as work item [`0124`](../work/0124-converge-vcs-detection-on-probe-layer.md)**
   — "converge the legacy lexical helpers (`find_repo_root`, `vcs_mode`) on the
   0058 probe layer" — linked from this plan's `relates_to`. This is the tracked
   follow-up that 0058 failed to produce; it is what stops the divergence
   becoming a third production bug.
2. **Consider a cross-strategy characterisation assertion** (cheap, can land
   with this work or the follow-up). Note the two strategies answer *different
   questions* — `find_repo_root` returns the current workspace ("where am I"),
   while `find_git_main_worktree_root` returns the canonical main checkout — so
   they correctly *agree* only for a plain main checkout (`make_main_git_checkout`)
   and correctly *differ* for a linked worktree (`make_git_linked_worktree`:
   worktree path vs main path). A test that pins both relationships (agree-on-main,
   documented-differ-on-worktree) catches accidental drift without falsely
   asserting equality where divergence is intended. Likewise `vcs_mode` is a
   command-set selector with `.jj`-WINS, not topology, so any future delegation
   to `classify_checkout` must preserve that mapping rather than pass the
   `KIND=` verdict through.

## References

- Original work item: [`meta/work/0123-find-repo-root-fails-in-git-worktrees.md`](../work/0123-find-repo-root-fails-in-git-worktrees.md)
- Research: [`meta/research/codebase/2026-06-22-0123-find-repo-root-fails-in-git-worktrees.md`](../research/codebase/2026-06-22-0123-find-repo-root-fails-in-git-worktrees.md)
- `find_repo_root`: [`scripts/vcs-common.sh:8-18`](../../scripts/vcs-common.sh#L8-L18)
- `vcs_mode`: [`scripts/vcs-common.sh:27-36`](../../scripts/vcs-common.sh#L27-L36)
- Consistency precedent (`-e`): [`scripts/vcs-common.sh:45-62`](../../scripts/vcs-common.sh#L45-L62)
- `vcs_mode` consumer: [`skills/work/scripts/work-item-file-dirty.sh:45,101-104`](../../skills/work/scripts/work-item-file-dirty.sh#L45)
- Test fixture + regression block: [`hooks/test-vcs-detect.sh:86-94`](../../hooks/test-vcs-detect.sh#L86-L94), [`hooks/test-vcs-detect.sh:424-446`](../../hooks/test-vcs-detect.sh#L424-L446)
- Predecessor work item: [`meta/work/0058-workspace-worktree-boundary-detection.md`](../work/0058-workspace-worktree-boundary-detection.md)
</content>
</invoke>
