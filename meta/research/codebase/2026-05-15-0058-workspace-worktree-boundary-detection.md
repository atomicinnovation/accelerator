---
date: 2026-05-15T12:35:58+01:00
researcher: Toby Clemson
git_commit: 08a7f5e3cdca3fb84bae5b5ce3a98c909ad2cbb7
branch: (detached / jj change xrmuuuzntsly)
repository: accelerator
topic: "Workspace and worktree boundary detection at session start (0058)"
tags: [research, codebase, hooks, vcs, jj, git, session-start, vcs-detect, vcs-common, worktree, workspace]
status: complete
last_updated: 2026-05-15
last_updated_by: Toby Clemson
---

# Research: Workspace and worktree boundary detection at session start (work item 0058)

**Date**: 2026-05-15T12:35:58+01:00
**Researcher**: Toby Clemson
**Git Commit**: 08a7f5e3cdca3fb84bae5b5ce3a98c909ad2cbb7
**Branch**: (detached, jj change `xrmuuuzntsly`)
**Repository**: accelerator

## Research Question

Gather all the codebase context needed to implement work item 0058
(`meta/work/0058-workspace-worktree-boundary-detection.md`): extend the
existing `SessionStart` VCS-detection hook so that, when the session is
started inside a jj secondary workspace or a git linked worktree, the model
receives an `additionalContext` block describing the boundary and prohibiting
edits / VCS commands / research against the parent repository. Identify the
files to be changed, the helpers/conventions already in place, the available
test harness, prior decisions that bind the design, and the constraints called
out by the pre-existing review of 0058.

## Summary

The implementation surface is small and well-scoped:

- **Hook to extend**: `hooks/vcs-detect.sh` — a 83-line `SessionStart` hook
  that today emits a single `hookSpecificOutput.additionalContext` JSON
  envelope describing whether the repo is `jj`, `jj-colocated`, or `git`.
  AC8/AC9 mandate keeping the hook in place and recording the placement
  rationale as a top-of-file comment.
- **Library to extend**: `scripts/vcs-common.sh` — currently a 19-line module
  exposing only `find_repo_root` (directory-walk that tests `-d $dir/.jj` or
  `-d $dir/.git`). AC7 adds three new functions:
  `find_jj_main_workspace_root`, `find_git_main_worktree_root`, and
  `classify_checkout`.
- **Hook registration**: `hooks/hooks.json` — the existing `SessionStart`
  entry for `vcs-detect.sh` must be retained unchanged (AC8).
- **Test harness**: bash `test-*.sh` scripts using `scripts/test-helpers.sh`
  assertions, discovered by `tasks/test/helpers.py::run_shell_suites` and
  invoked through `mise run test`. No existing tests cover `vcs-common.sh`
  or `vcs-detect.sh` — the nearest analogue is
  `hooks/test-migrate-discoverability.sh`. Crucially, `hooks/` is **not**
  currently wired into any `run_shell_suites` task, so a new
  `tasks/test/integration.py` task plus `mise.toml` entry is required to
  make new hook tests run in CI.
- **Critical gap in current detection**:
  - `find_repo_root` uses **directory-only** marker tests (`-d`), so a git
    linked worktree (where `.git` is a *file*) is silently skipped and the
    walk continues into the parent repo. This is the exact failure mode
    that 0058 fixes.
  - jj secondary workspaces carry a `.jj` *directory* whose `repo`
    sub-entry is a *file* pointing back to the main repo. `find_repo_root`
    finds the workspace correctly but `vcs-detect.sh` cannot tell it apart
    from a main workspace, so no boundary warning is emitted.
- **Review of 0058** (already on disk at
  `meta/reviews/work/0058-workspace-worktree-boundary-detection-review-1.md`)
  has two pass-2 majors the implementer must respect: the AC1 prohibition
  phrases must be **canonically pinned** (currently the work item has them
  as verbatim substrings — confirm they survive), and the AC5 golden
  snapshots must be **captured before** any change to `vcs-detect.sh` /
  `vcs-common.sh`. Plus a global rule: every emitted path must be
  `realpath`-normalised so macOS `/private/var` vs `/var` compares equal.
- **No dedicated ADR** exists for VCS detection, SessionStart hooks, or
  session context injection. The foundational design lives in research
  doc `2026-03-16-jujutsu-integration-and-vcs-autodetection.md` and plan
  `2026-03-18-vcs-skill-improvements.md`; ticket 0020 is the ADR-creation
  task for that design and has not yet produced an ADR.

## Detailed Findings

### `hooks/vcs-detect.sh` (the hook to extend)

**File**: `hooks/vcs-detect.sh:1-83`

- Shebang only — **no `set -e` / `-u` / `-o pipefail`** and no `trap`
  (`hooks/vcs-detect.sh:1`). Any new code added must follow the same
  convention or carefully introduce stricter modes without breaking the
  existing fall-through paths.
- `jq` dependency check at `hooks/vcs-detect.sh:4-7` — if absent, emits
  a top-level `{"systemMessage": …}` envelope and exits 0. The new code
  must continue to behave gracefully when `jq` is missing.
- Sources `scripts/vcs-common.sh` at `hooks/vcs-detect.sh:10-11`; calls
  `find_repo_root` at `hooks/vcs-detect.sh:13`.
- `VCS_MODE` derivation at `hooks/vcs-detect.sh:14-28`:
  - Empty `REPO_ROOT` → `git` (silent default).
  - `[ -d "$REPO_ROOT/.jj" ]` + `[ -d "$REPO_ROOT/.git" ]` → `jj-colocated`.
  - `[ -d "$REPO_ROOT/.jj" ]` only → `jj`.
  - Otherwise → `git`.
- `additionalContext` strings:
  - jj/jj-colocated at `hooks/vcs-detect.sh:32-55` (single double-quoted
    bash string with `${VCS_MODE}` interpolated at line 33).
  - git at `hooks/vcs-detect.sh:57-72`.
  - No separate branch for `jj-colocated` — it reuses the jj block with the
    mode token substituted.
- JSON envelope built via `jq -n --arg ctx "$CONTEXT" '{…}'` at
  `hooks/vcs-detect.sh:77-82`:
  ```json
  {"hookSpecificOutput":
    {"hookEventName": "SessionStart",
     "additionalContext": "<CONTEXT string>"}}
  ```
  This is the shape AC5's byte-identity assertion has to compare against,
  and is the shape the new boundary fields/sentences must integrate with.
- Reads **no** environment variables (no `CLAUDE_*` reads in
  `vcs-detect.sh`; relies entirely on `$PWD` via `find_repo_root` and
  `${BASH_SOURCE[0]}` at line 10).
- **No top-of-file docstring** — the file goes from shebang straight into
  the `jq` check. AC9 requires the implementer to add a comment naming the
  alternative placement (a sibling `workspace-detect.sh`) and the rationale
  ("one coherent VCS-environment message; shared `REPO_ROOT` / `VCS_MODE`
  computation").
- Gap relevant to 0058: when the cwd is a git linked worktree, `.git` is a
  *file* (gitlink), so `[ -d "$REPO_ROOT/.git" ]` at line 21 is false. If a
  `.jj` happens to sit at the same level (colocated cross-VCS), classification
  becomes `jj` rather than `jj-colocated`. And `find_repo_root` itself walks
  *past* worktree roots that have `.git` files only, finding the outer parent
  repo (or returning empty if walking ends at `/`).

### `scripts/vcs-common.sh` (the library to extend)

**File**: `scripts/vcs-common.sh:1-19`

Complete current contents are 19 lines, one function (`find_repo_root`,
`scripts/vcs-common.sh:8-18`):

```bash
find_repo_root() {
  local dir="$PWD"
  while [ "$dir" != "/" ]; do
    if [ -d "$dir/.jj" ] || [ -d "$dir/.git" ]; then
      echo "$dir"
      return 0
    fi
    dir="$(dirname "$dir")"
  done
  return 1
}
```

Conventions to follow when adding `find_jj_main_workspace_root`,
`find_git_main_worktree_root`, `classify_checkout`:

- **Sourced, not executed** — no `main`, no `BASH_SOURCE` guard, no
  executable bit semantics rely on the shebang. All call sites use
  `source "$.../scripts/vcs-common.sh"`.
- **No `set` flags** — header comment in the sister file
  `scripts/config-common.sh:5` documents this as deliberate ("matching the
  `vcs-common.sh` convention") so shared libraries inherit the caller's
  shell options.
- **Function naming**: lowercase snake_case, **unprefixed** (e.g.,
  `find_repo_root`, not `vcs_find_repo_root`). Confirmed as the repo
  convention in
  `meta/reviews/plans/2026-04-29-jira-integration-phase-1-foundation-review-1.md:608`.
- **Result-on-stdout contract**: function prints result on stdout, returns 0
  on success / non-zero with empty stdout on failure. Callers use
  `VAR=$(func)` (sometimes with `|| VAR="$PWD"`).
- **No realpath / no symlink resolution** anywhere today. 0058 *adds* a new
  rule (`realpath` on every emitted path) — both the new helpers and the
  hook must normalise; existing `find_repo_root` callers that compose with
  the new helpers may also need a normalisation pass.
- **No tests today**. `find_repo_root` is exercised indirectly by
  `skills/integrations/jira/scripts/test-jira-common.sh:26-32` only.

Live call sites of `find_repo_root` (to consider when changing semantics —
none should change behaviour because we are *adding* helpers, not modifying
the existing one):

- `scripts/vcs-status.sh:8`, `scripts/vcs-log.sh:8`,
  `scripts/config-common.sh:17`, `hooks/vcs-detect.sh:13`,
  `hooks/vcs-guard.sh:19`, `skills/decisions/scripts/adr-next-number.sh:31`,
  `skills/work/scripts/work-item-next-number.sh:49`,
  `skills/work/scripts/work-item-resolve-id.sh:36`,
  `skills/config/init/scripts/init.sh:14`,
  `skills/visualisation/visualise/scripts/{launch,status,stop}-server.sh:13/10/10`,
  `skills/design/inventory-design/scripts/playwright/run.sh:15`,
  `skills/integrations/jira/scripts/jira-common.sh:70`,
  `skills/integrations/jira/scripts/jira-auth.sh:92,133`,
  `skills/integrations/jira/scripts/jira-init-flow.sh:64,165`,
  `skills/integrations/jira/scripts/jira-create-flow.sh:174`,
  `skills/integrations/jira/scripts/jira-search-flow.sh:202`.

### `hooks/hooks.json` (registration)

**File**: `hooks/hooks.json:1-43`

- Top-level shape: `{ "hooks": { "<EventName>": [ { "matcher": …, "hooks":
  [ { "type":"command", "command":… } ] } ] } }`. Minimal schema — no
  `timeout`, `cwd`, or `env` fields anywhere.
- `SessionStart` entries (`hooks/hooks.json:3-30`):
  - vcs-detect (`hooks/hooks.json:3-12`) — matcher `""`,
    command `${CLAUDE_PLUGIN_ROOT}/hooks/vcs-detect.sh`. **AC8 requires this
    entry to remain byte-identical.**
  - config-detect (`hooks/hooks.json:13-21`).
  - migrate-discoverability (`hooks/hooks.json:22-30`).
- `PreToolUse` entry (`hooks/hooks.json:32-42`) — matcher `"Bash"`,
  command `${CLAUDE_PLUGIN_ROOT}/hooks/vcs-guard.sh`. Not changed by 0058
  but useful to remember as the natural location for the future PreToolUse
  enforcement (out of scope per the work item).
- `${CLAUDE_PLUGIN_ROOT}` resolves at runtime to the plugin root (directory
  containing `.claude-plugin/plugin.json`). Hooks that wish to run outside a
  session export it themselves — see `hooks/migrate-discoverability.sh:23`:
  `PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$SCRIPT_DIR/.." && pwd)}"`.
  The `vcs-detect.sh` extension may want to adopt the same fallback to make
  manual invocation work, particularly for golden-snapshot capture.
- `.claude-plugin/plugin.json:1-27` does not register hooks — discovery is
  by convention at `hooks/hooks.json`.

### `hooks/vcs-guard.sh` (sibling hook, contrast)

`PreToolUse` hook for `Bash` (`hooks/vcs-guard.sh`). Reads tool-invocation
JSON from stdin (`hooks/vcs-guard.sh:27-28`), exits 0 early if cwd is not a
jj repo (`hooks/vcs-guard.sh:19-24`), splits compound commands on
`&& || ; |` and matches each subcommand against a fixed git-VCS pattern
(`hooks/vcs-guard.sh:46-70`), then emits `{"decision":"block",…}` in pure
jj (`hooks/vcs-guard.sh:97-100`) or `{"decision":"allow",
hookSpecificOutput:{"systemMessage":…}}` in colocated mode
(`hooks/vcs-guard.sh:103-108`). It uses the same `find_repo_root` and is
*not* in scope for 0058 — but if 0058 changes are extended later to a
PreToolUse boundary guard, this file is the precedent for that hook shape.

### Test harness

- **Framework**: plain bash `test-*.sh` scripts sourcing the assertion
  library at `scripts/test-helpers.sh`. No bats, no shunit2.
- **Assertion library** (`scripts/test-helpers.sh:1-301`): the relevant
  helpers for AC1-AC9 are
  `assert_eq` (`:19-30`), `assert_contains` (`:31-…`),
  `assert_file_content_eq` (`:154-171`),
  `assert_json_eq` (`:287-301` — uses `jq -r <filter> <file>`),
  `assert_exit_code`, `assert_stderr_empty`, `test_summary`.
- **Runner**: `mise run test` → `tasks/test/{unit,integration,e2e}.py`
  (`mise.toml:88-135`). Discovery is done by
  `tasks/test/helpers.py:13-34` (`run_shell_suites(context, subtree)`)
  which globs `**/test-*.sh` under the named subtree and runs each one.
- **CI**: `.github/workflows/main.yml:14-31` runs `mise run test` on
  `ubuntu-latest` via `jdx/mise-action@v4`.
- **Gap**: there is **no subtree task pointing at `hooks/`**. The existing
  `hooks/test-migrate-discoverability.sh` is therefore not run by CI today
  — its header (`hooks/test-migrate-discoverability.sh:5`) advertises a
  manual invocation. For 0058 to satisfy AC5 in CI, a new
  `test:integration:hooks` task is needed:
  - Add `def hooks(context): run_shell_suites(context, "hooks")` to
    `tasks/test/integration.py` (precedent: `tasks/test/integration.py:21-30`
    for `config`).
  - Add `[tasks."test:integration:hooks"]` block to `mise.toml` and list it
    in `test:integration.depends` (precedent: `mise.toml:88-122`).
- **Nearest hook-test template**: `hooks/test-migrate-discoverability.sh`
  — bootstrap at `:7-13`, `run_hook()` wrapper at `:16-24`, per-test pattern
  at `:29-33`, `test_summary` at `:105`. Reuse the bootstrap shape directly.
- **Counter convention** (from review of
  `2026-05-08-0052-documents-locator-config-driven-paths`,
  `meta/reviews/plans/…review-1.md:131`): "delegate all counter management
  to `assert_*` helpers exclusively" — do not mix manual `PASS=$((PASS+1))`
  with assertion helpers.

### Fixture construction (new territory)

The repo has **no existing helpers** that create real `jj init`, `jj
workspace add`, or `git worktree add` fixtures. Two patterns exist for
faking VCS state:

- **Faked `.git` directory** (`skills/integrations/jira/scripts/test-jira-auth.sh:19-24`):
  `mkdir -p "$d/.git"` — fast but **inappropriate for 0058** because
  `vcs-detect.sh` needs authoritative probes (`jj workspace root`,
  `git rev-parse --git-dir` vs `--git-common-dir`, file-vs-directory tests
  of `.jj/repo`); a `mkdir .git` does not exercise these.
- **Real `git init`** (`skills/integrations/jira/scripts/test-jira-auth.sh:27-33`):
  `(cd "$d" && git init -q && git config user.email ... && git config user.name ...)`.
  This is the closest in-tree precedent. 0058 needs to extend it to also
  invoke `jj git init`, `jj workspace add <path>`, and `git worktree add
  <path>`. CI runs on `ubuntu-latest` so `jj` and `git` must both be
  available there — verify by checking `.github/workflows/main.yml` and
  `mise.toml` for declared tool versions; add a dependency note if `jj` is
  not currently installed.

### Golden snapshot patterns (for AC5)

Three patterns exist; the work item's wording ("byte-identical to … golden
snapshot in `tests/fixtures/vcs-detect-pre-0058/`") matches **Pattern A**
plus a strict equality assertion:

- Pattern A — `assert_eq "$(cat $GOLDEN)" "$OUTPUT"`. Precedent at
  `scripts/test-config.sh:1935-1939` with fixture at
  `scripts/test-fixtures/config-read-review/work-item-mode-golden.txt`
  (note: that file uses a sibling `test-fixtures/` dir; the work item
  prescribes a different top-level location, `tests/fixtures/`).
- Pattern B — pipe-delimited table-driven golden iterated line-by-line
  (`skills/work/scripts/test-work-item-scripts.sh:419-455`). Overkill here.
- Pattern C — `tree_hash` portable digest helper for directory-equivalence
  (`skills/config/migrate/scripts/test-migrate.sh:30-41`). Not needed for
  AC5 but useful if hook output ever grows.

**AC5 capture procedure (per review pass-2 major #19)**: the snapshots
must be captured *before* any modification to `vcs-detect.sh` or
`vcs-common.sh`. Procedure:
1. Add the new test file and fixture directory layout.
2. From a clean checkout (no 0058 code yet), construct each main-checkout
   fixture (main jj workspace, main git worktree) via the new fixture
   helpers, run `bash hooks/vcs-detect.sh` from the fixture cwd, capture
   stdout, and write it to `tests/fixtures/vcs-detect-pre-0058/<case>.json`.
3. Commit those snapshots before implementing the boundary-detection
   changes. AC5 then asserts the post-change output for each main case is
   byte-identical to the captured snapshot.

### Prior decisions and constraints (from prior research/plans/reviews)

#### `meta/reviews/work/0058-workspace-worktree-boundary-detection-review-1.md`

Final recommendation (`:288`): "address the two new majors (canonical
phrasing for AC1 prohibitions; explicit snapshot capture step) and the
implicit-prerequisite Dependencies bullet, then mark complete". Concrete
implementer-binding rules:

- **AC1 prohibitions must be canonical strings, not loose substrings**
  (review pass 2 major #18, `:267`). The work item already specifies them
  verbatim — `do not edit files in <parent>`, `do not run VCS commands
  against <parent>`, `do not grep, find, or research files in <parent>` —
  with `<parent>` substituted; the review confirms these must appear as
  contiguous substrings, not just as bag-of-keywords. Match this exactly
  in both the hook output and the assertion.
- **AC5 snapshots must be captured before any code change** (review pass 2
  major #19, `:268`). The work item now reflects this in AC5 text
  ("MUST be captured into that fixture directory before any implementation
  work on `vcs-detect.sh` or `vcs-common.sh` begins").
- **`realpath` normalisation is global, not local to AC4** (review minor
  #11, `:75-77`). The work item now states this as a Requirements bullet
  ("Emit all paths normalised via `realpath` …"); apply to every emitted
  path field and inside the prohibition substitutions.
- **AC3 colocated case needs a pinned JSON-ish shape**: "(a) the shared
  boundary path emitted once as a single labelled field, (b) the jj parent
  repo path emitted as a separate labelled field, (c) the git parent repo
  path emitted as a separate labelled field". Don't invent a different
  layout.
- **AC7 helpers contract**: "after `source scripts/vcs-common.sh`, each
  helper has a named function, expected stdout, and exit code 0" (review
  `:140`). `classify_checkout` must print exactly one of `main`,
  `jj-secondary`, `git-worktree`, `colocated`, `none` on stdout.

#### `meta/research/codebase/2026-03-16-jujutsu-integration-and-vcs-autodetection.md`

Foundational research that established the three-layer architecture
(skill split + SessionStart hook + PreToolUse guard). The hook contract
recorded there — "JSON on stdout with `hookSpecificOutput.additionalContext`;
exit 0 = success" (`:432-440`) — is the same one 0058 must preserve. The
research did **not** consider workspace/worktree boundaries, file-vs-
directory `.git`/`.jj/repo` markers, or cross-VCS nesting; 0058 is filling
those explicit gaps.

#### `meta/work/0020-vcs-abstraction-layer.md`

ADR-creation task summarising the three-layer decision (`:67-68`). It
records the headline detection rule (`.jj/ + .git/ = colocated`, `.jj/`
only = pure jj, otherwise git) but introduces no new constraints for
0058 — its accepted negatives ("Command-splitting heuristic in the guard
hook is best-effort", "jq dependency in hooks", `:84-85`) concern the
guard hook, not detection. **No ADR has yet been produced from 0020**,
so the 0058 implementer is not bound by a written ADR — only by the de
facto contracts in `vcs-detect.sh`/`vcs-common.sh`. The 0058 review
flagged that 0020 should be mirrored in 0058's Dependencies; the work
item now lists it as Related.

#### `meta/plans/2026-03-18-vcs-skill-improvements.md`

The implementation plan that built `find_repo_root`, `vcs-detect.sh`, and
`vcs-common.sh`. Confirms the binding contracts:

- `find_repo_root` is **directory-only** (`-d` markers, `:212-223`) — the
  exact gap 0058 must address with its new `*main*_root` helpers.
- `VCS_MODE` set from `.jj` / `.git` directory presence at `REPO_ROOT`
  (`:246-257`) — the existing branches must keep emitting their current
  strings unchanged to satisfy AC5.
- JSON output shape `{"hookSpecificOutput":{"additionalContext": <string>}}`
  with `jq` dependency (`:233-236, :305-311`) — the new boundary fields
  must be folded into the same `additionalContext` *string*, not new
  top-level fields, unless the work item author wants a schema change
  (work item wording leaves room either way: AC3 says "labelled field",
  which can be satisfied by structured prose inside the string).
- Hook registered with empty matcher `"matcher": ""` (`:322`) — AC8.

## Code References

- `hooks/vcs-detect.sh:1-83` — SessionStart hook to extend; envelope at
  `:77-82`; jj/git branches at `:32-55`/`:57-72`.
- `hooks/vcs-guard.sh:1-108` — sibling PreToolUse hook (out-of-scope but
  relevant precedent for future boundary enforcement).
- `hooks/hooks.json:3-12` — registration entry that must remain unchanged
  (AC8); rest of file at `:1-43`.
- `hooks/test-migrate-discoverability.sh:1-105` — template for the new
  hook test file.
- `scripts/vcs-common.sh:1-19` — library to extend; `find_repo_root` body
  at `:8-18`.
- `scripts/config-common.sh:5` — documents the inherit-shell-options
  convention for `*-common.sh` libraries.
- `scripts/test-helpers.sh:19-30,154-171,287-301` — assertion library
  (`assert_eq`, `assert_file_content_eq`, `assert_json_eq`).
- `scripts/test-config.sh:1935-1939` — golden-text fixture precedent.
- `scripts/test-fixtures/config-read-review/work-item-mode-golden.txt` —
  example golden fixture layout (note: 0058 uses `tests/fixtures/…`
  instead).
- `tasks/test/helpers.py:13-34` — shell test discovery; needs a new
  `hooks` (or `tests`) subtree call.
- `tasks/test/integration.py:21-30` — pattern for adding a new
  `test:integration:hooks` task.
- `mise.toml:88-122` — pattern for adding a new task block + depends list.
- `.github/workflows/main.yml:14-31` — CI invocation; depends on
  `jdx/mise-action@v4`.
- `skills/integrations/jira/scripts/test-jira-auth.sh:19-33` — fixture
  helpers (`setup_repo`, `setup_git_repo`) to model new jj/git fixture
  helpers on.
- `.claude-plugin/plugin.json:1-27` — plugin manifest (no `hooks` field;
  discovery is by `hooks/hooks.json` convention).

## Architecture Insights

- **Single-hook, single-envelope discipline.** The work item, the review,
  and the existing hook all push toward folding boundary detection into
  the existing `vcs-detect.sh` envelope. Splitting it into a sibling hook
  would force two `additionalContext` blocks that share most of their
  computation (`REPO_ROOT`, `VCS_MODE`) and double the per-session
  payload.
- **Detection helpers vs detection hooks.** The convention is to push pure
  detection logic into `scripts/vcs-common.sh` and keep `hooks/vcs-*.sh`
  as thin orchestrators that source, dispatch on results, and emit JSON.
  AC7 reinforces this: the three new helpers must be independently
  callable after `source scripts/vcs-common.sh`. This also makes them
  unit-testable without invoking the hook.
- **Authoritative probes over path-walking.** The work item explicitly
  rejects path-walking (`Requirements §4`) in favour of `jj workspace
  root`, `git rev-parse --git-dir` vs `--git-common-dir`, and the
  `.jj/repo` file-vs-directory test. This is a step-change from
  `find_repo_root`'s lexical walk, and the new helpers will probably
  shell out to `jj` and `git` directly (carrying a runtime dependency
  that the review flagged as missing from `Dependencies`).
- **Realpath normalisation is a global rule now.** Until 0058,
  `vcs-common.sh` and `vcs-detect.sh` did no path canonicalisation. The
  work item lifts `realpath` into a global Requirements bullet, so the
  new helpers and the hook should canonicalise consistently — and any
  follow-up that compares paths against `find_repo_root` output may
  expose normalisation drift in *existing* callers (out of scope here
  but worth flagging).
- **No ADR for the hook architecture.** The design lives in research
  doc + implementation plan, with ticket 0020 still outstanding as the
  ADR-creation task. 0058 does not need to produce an ADR but its
  rationale (extend, not split) should be recorded in the top-of-file
  comment per AC9 — that comment is currently the only durable design
  artefact for the placement decision.

## Historical Context

- `meta/research/codebase/2026-03-16-jujutsu-integration-and-vcs-autodetection.md`
  — foundational research; established the SessionStart-injection contract
  but did not consider workspaces/worktrees.
- `meta/work/0020-vcs-abstraction-layer.md` — ADR-creation task for the
  three-layer design; ADR not yet produced.
- `meta/plans/2026-03-18-vcs-skill-improvements.md` — implementation plan
  that produced today's `vcs-detect.sh`, `vcs-common.sh`, and
  `vcs-guard.sh`; binds the JSON envelope shape and the `find_repo_root`
  contract.
- `meta/reviews/work/0058-workspace-worktree-boundary-detection-review-1.md`
  — already-completed review of the work item itself; two pass-2 majors
  carried into the live work item (canonical AC1 prohibition phrasing;
  AC5 pre-implementation snapshot capture).
- `meta/reviews/plans/2026-05-08-0052-documents-locator-config-driven-paths-review-1.md:131`
  — documents the "delegate counter management to `assert_*` helpers"
  test-style convention.
- `meta/reviews/plans/2026-04-29-jira-integration-phase-1-foundation-review-1.md:608`
  — documents the unprefixed function-naming convention for `*-common.sh`
  libraries.

## Related Research

- `meta/research/codebase/2026-03-16-jujutsu-integration-and-vcs-autodetection.md`
- `meta/research/codebase/2026-03-18-adr-support-strategy.md`
- `meta/research/codebase/2026-03-28-initialise-skill-requirements.md`

## Open Questions

1. **AC3 emission shape inside the `additionalContext` string.** The work
   item says "exactly one `additionalContext` block … structured as: (a)
   the shared boundary path emitted once as a single labelled field, (b)
   the jj parent repo path emitted as a separate labelled field, (c) the
   git parent repo path emitted as a separate labelled field". The
   existing envelope carries `additionalContext` as a single human-readable
   string. Decide before implementation: does "labelled field" mean
   structured prose lines (e.g., `Boundary: <path>\nParent (jj): <path>\n…`)
   inside the existing string, or does the JSON shape grow new sibling
   fields next to `additionalContext`? The work item's mandate to "retain
   the existing `SessionStart` entry unchanged" (AC8) is about
   `hooks.json`, not about the envelope's keys — so either interpretation
   is technically open. Pick prose lines for AC5 byte-identity safety on
   the negative cases.
2. **Runtime availability of `jj` in CI.** The new helpers will shell out
   to `jj workspace root` in the fixture-construction tests. Confirm `jj`
   is installed in CI (`.github/workflows/main.yml`, `mise.toml`); if not,
   add it. The review flagged jj/git/realpath CLI runtime requirements as
   a missing Dependencies bullet on the work item (pass 2 minor #26).
3. **Whether to add the new hook tests under `hooks/test-vcs-detect.sh` or
   `tests/test-vcs-detect.sh`.** The work item names
   `tests/fixtures/vcs-detect-pre-0058/` for fixtures, which suggests a
   top-level `tests/` location for the test file. The existing precedent
   is `hooks/test-migrate-discoverability.sh`. Either way, a new
   `run_shell_suites` task is needed.
4. **Whether `find_repo_root` should be retired or kept.** The new
   helpers supersede its detection use case but not all of its callers
   (e.g., the visualiser launches use it purely as a "find the project
   root" anchor). Recommend leaving `find_repo_root` unchanged and adding
   the new helpers alongside; out of 0058's scope to refactor callers.
