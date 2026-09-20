---
type: "codebase-research"
id: "2026-09-20-0286-eradicate-direct-git-calls-from-skills"
title: "Research: Make Accelerator skills VCS-agnostic by eradicating direct git calls (0286)"
date: "2026-09-20T21:28:29+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0286"
parent: "work-item:0286"
topic: "Make Accelerator skills VCS-agnostic by eradicating direct git calls"
tags: ["research", "codebase", "vcs", "skills", "cli", "guard", "jj"]
revision: "f418ac31ce06d1088ccb3b4546132108cc963ce4"
repository: "accelerator"
last_updated: "2026-09-20T21:28:29+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Make Accelerator skills VCS-agnostic by eradicating direct git calls (0286)

**Date**: 2026-09-20T21:28:29+00:00
**Author**: Toby Clemson
**Git Commit**: f418ac31ce06d1088ccb3b4546132108cc963ce4
**Branch**: working-copy commit (jj; detached HEAD, unpushed)
**Repository**: accelerator

## Research Question

What does the codebase look like for work item 0286 — making every Accelerator
skill VCS-agnostic by eradicating direct `git` calls? Specifically: where do the
three affected skills issue backend-specific VCS commands; what does adding an
`accelerator vcs root` subcommand actually require; how is the SessionStart VCS
Command Reference produced and where would the two new idioms go; and what
binding design decisions and precedents constrain the implementation?

## Summary

The work item is accurate on the destination but carries **three inaccuracies in
its Technical Notes** that change the implementation shape. Every VCS reference
in the affected skills is **prose or an illustrative fenced block, never
`!`-executed** — no skill grants `git`/`jj` in `allowed-tools`, and the only
`!`-preprocessed commands anywhere are `accelerator …` invocations. So the
rewrites are wording changes, plus one thin CLI surface and one cheat-sheet
extension.

The three corrections, each verified against source:

- ❌ **`repository_root` is not consumed by `accelerator vcs detect`.** Its sole
  caller is `vcs::facts` (`cli/vcs/src/lib.rs:145`), which uses it to derive the
  repository *name*. `detect` finds the boundary via `jj_workspace_root` /
  `dual_roots` / `jj_repository`. The work item's Context and Technical Notes
  both misattribute the consumer.
- ❌ **Adding `vcs root` does not follow the thirteen-point dispatched-sub-binary
  checklist.** That checklist is per *token*; `vcs` is already a registered
  token (`accelerator-vcs`). `vcs root` is a second-level clap subcommand inside
  the existing binary and touches almost none of the thirteen points.
- ⚠️ **`vcs status` / `vcs log` do not shell out.** The "`jj status` /
  `git diff --cached --stat`" phrasing is conceptual (doc-comments); the
  implementation reads in-process via `gix` and `jj-lib` (ADR-0066, work item
  0198). Any mental model that has the skills or CLI invoking real binaries is
  wrong.

One genuinely unresolved design question surfaced: `repository_root` and
`jj_workspace_root` **diverge for a jj secondary workspace** — `repository_root`
returns the shared main repo, whereas `git rev-parse --show-toplevel` in a git
worktree returns the worktree. The acceptance-criteria fixtures (git-only,
colocated, pure-jj) do not exercise a secondary workspace, so `vcs root` could
pass its criteria while returning the "wrong" root in exactly the checkout this
research is running in.

The good news for scope: the **user-identity probe already exists** (`vcs::user_name`
/ `InProcessProbe::user_name`), so the identity idiom is a text edit; and
`vcs-cli` is public-API-exempt with both needed imports already present, so the
whole `vcs root` change is confined to `cli/vcs-cli/`.

## Detailed Findings

### The affected skills — every VCS reference is prose, none executed

The critical structural fact: the `!` preprocessor executes only inline
single-backtick `` !`command` `` forms, and in all four skills those are
exclusively `accelerator config …` / `accelerator corpus …`. Every `git`/`jj`
token below is prose guidance to the model or an illustrative command inside a
triple-backtick block — **none is executed, and no skill's `allowed-tools`
grants `git` or `jj`.**

| Skill | Site | Current text | Nature |
|---|---|---|---|
| `validate-plan` | line 58 | `git log --oneline -n 20` | fenced bash |
| `validate-plan` | line 59 | `git diff HEAD~N..HEAD` | fenced bash |
| `validate-plan` | line 62 | `cd $(git rev-parse --show-toplevel) && make check test` | fenced bash |
| `validate-plan` | lines 48, 247 | "through git", "analyze the git history" | prose |
| `config/migrate` | lines 272-273 | `` `jj status`/`git status` `` | prose |
| `refine-work-item` | lines 247-249 | `jj config get user.name` → `git config user.name` | prose |
| `research-issue` | lines 63-67 | "the session's VCS log/diff command" | prose (reference) |

`validate-plan` (`skills/planning/validate-plan/SKILL.md`) is the only real
breakage. Its evidence-gathering block (lines 55-63) is a ```` ```bash ```` fence
carrying all three raw commands, and line 62 additionally hardcodes
`make check test` alongside `git rev-parse --show-toplevel`. Its `allowed-tools`
(lines 7-11) grants only `Bash(accelerator config *)`, `Bash(accelerator corpus
metadata derive)`, `Bash(accelerator corpus frontmatter validate *)` — no VCS
permission, confirming the git commands are model-facing prose, not tool calls.

`research-issue` (`skills/research/research-issue/SKILL.md`) is the **template to
mirror**, already VCS-agnostic. Its exact vocabulary (lines 61-67): "its last
~20 revisions", "using the session's VCS log command", "using the session's VCS
diff command if needed", the generic noun "revisions"/"commits", and it obtains
the revision from `accelerator corpus metadata derive` (line 96) rather than
`git rev-parse`. Its author chain (line 108) reads "config → VCS user → prompt"
— the abstract label the identity idiom formalises.

`config/migrate` (`skills/config/migrate/SKILL.md`) names both tools at lines
272-273 in the dirty-path confirmation prose; its two other VCS references
(lines 11, 283) already say "VCS revert" and are clean.

`refine-work-item` (`skills/work/refine-work-item/SKILL.md`) has the ordered
author chain at lines 247-249: parent's `author` → configured `author` →
`jj config get user.name` → `git config user.name` → ask the user. The git leg
is load-bearing for git-only repos and must be **preserved through the identity
idiom**, not deleted.

⚠️ **Scope-adjacent findings the work item does not list.** `validate-plan` line
247 ("analyze the git history") is a second prose git reference beyond the flagged
block. `refine-work-item` names `jj restore` in three prose sites (lines 172,
188, 214). The acceptance-criteria sweep targets only `git <subcommand>`, so
`jj restore` prose would pass the sweep, but neither is backend-neutral. Decide
explicitly whether they are in or out.

### `accelerator vcs root` — the real (minimal) touch list

The command surface is **clap derive** in `cli/vcs-cli/src/cli.rs`: a `Cli`
struct (`#[command(name = "accelerator-vcs")]`) holding a `Command` enum with
four variants — `Detect`, `Status`, `Log`, `Guard`. `main.rs` parses and matches
them. `bin/accelerator` is a pure passthrough (`exec "${launcher}" "$@"` at line
455); the token→binary resolution (`vcs` → `accelerator-vcs`) happens in the Rust
launcher (`cli/launcher/src/launch/core.rs`), and the `vcs` token is registered
once in `tasks/shared/paths.py` (`DISPATCHED_SUBBINARIES`) and
`tasks/manifest.py` (`_SUBBINARY_MANIFESTS`).

Because `vcs` is already a token, a new subcommand rides it. The complete touch
list:

1. `cli/vcs-cli/src/cli.rs` — add a `Root { … }` variant to `Command` (its
   doc-comment is the clap help text). Mirror `Status`/`Log`, which carry only a
   (launcher-consumed, handler-ignored) `--fail-safe`.
2. `cli/vcs-cli/src/main.rs` — add `mod root;`, a `run_root()` thunk, a
   `Command::Root { … } => run_root()` arm, and extend the crate doc-comment
   (line 1) that enumerates `detect|status|log|guard`.
3. `cli/vcs-cli/src/root.rs` — new handler. **Mirror `detect.rs`, not
   `status.rs`/`log.rs`**: `status`/`log` route through the never-fail
   `report::run` boundary because they need a `VcsReporter` report type; a root
   that prints a path uses the probe ports directly.

The handler calls `InProcessProbe`, which already implements `vcs::RepoRoot`
(`cli/vcs-adapters/src/library.rs:511`). Both imports it needs are already in the
crate — `use vcs::RepoRoot as _;` (`cli/vcs-cli/src/report.rs:21`) and
`use vcs_adapters::library::InProcessProbe;` (`main.rs:15`, `report.rs:25`) — so
no new dependency. `vcs-cli` is a composition root and **exempt** from
cargo-public-api pinning (`tasks/public_api.py:70`), so a change confined to it
needs no snapshot update. `Bash(accelerator vcs *)` (`skills/vcs/commit/SKILL.md:7`)
already permits `vcs root`; skills use the bare `accelerator` form since work
item 0245.

None of `tasks/`, the launcher, the manifest, `.gitignore`, `docs-site`, the
`cli/Cargo.toml` members list, or `deny.toml` needs touching. Verify with
`mise run cli:check`.

### `repository_root` vs `jj_workspace_root` — the semantic question

Both live in `vcs-adapters`; neither shells out (the crate bans `std::process`,
`cli/vcs-adapters/src/lib.rs:6`). They answer different questions:

| Capability | Kind | Signature | Returns for a jj secondary workspace |
|---|---|---|---|
| `repository_root` | `vcs::RepoRoot` trait method | `(&self, working_copy_root: &Path) -> PathBuf` (infallible) | the shared **main** repo |
| `jj_workspace_root` | `InProcessProbe` inherent + `ModeProbe` | `(&self, start: &Path) -> Result<Option<PathBuf>, Error>` | the **workspace's own** root |

`repository_root` (`cli/vcs-adapters/src/library.rs:516-519`) delegates to
`jj_repository_root`, which loads the workspace via `jj-lib`'s
`DefaultWorkspaceLoaderFactory` and returns `repo_path().parent().parent()`
(grandparent-of-store). For git-only/none it falls back to canonicalising the
working-copy root it was handed. Tests (`cli/vcs-adapters/tests/library.rs:264-300`,
`bash-parity`-gated) pin exactly this: a secondary workspace resolves to its main
repo; a main workspace resolves to itself.

❓ **The divergence matters for `vcs root`'s contract.** `git rev-parse
--show-toplevel` in a git worktree returns the *worktree* root — the analog of
`jj_workspace_root`, not `repository_root`. For a plain checkout (the common
case, and all three AC fixtures) the two coincide, so AC2 passes either way. But
`validate-plan` runs `make check test` from that root, and in a worktree /
secondary workspace you want the checkout you are in. The work item names
`repository_root`; whether that is the right semantics for the checks step is
unresolved and unexercised by the criteria. This very research runs inside the
`build-system` jj secondary workspace, where the two answers differ.

### The SessionStart VCS Command Reference and the two new idioms

The cheat sheet is emitted by `accelerator vcs detect --format=hook --fail-safe
--descriptive`, wired as the first SessionStart hook in `hooks/hooks.json:7-10`.
The `--descriptive` flag prepends the reference; without it, output is
structured-only (`cli/vcs-cli/src/detect.rs:161-177`).

The reference text lives in **`cli/vcs-cli/src/detect.rs`**, not `cli.rs`: two
`concat!` consts, `JJ_COMMAND_REFERENCE` (lines 11-39) and `GIT_REFERENCE`
(lines 41-59), selected by `reference_text(mode)` (lines 61-73). Both begin with
a "VCS Command Reference:" bulleted list. The two new idioms — the
trunk-divergence diff-range idiom and the user-identity idiom — are new `- Use …`
bullets added to *each* of the two lists, keeping the `\n` terminators, the
backslash line-continuation style, and the 80-column floor.

The underlying capability split for the idioms:

- **User-identity idiom — probe already built.** `vcs` defines
  `trait UserIdentityProbe` and free fn `user_name` (`cli/vcs/src/lib.rs:111-132`);
  `InProcessProbe` implements it (`cli/vcs-adapters/src/library.rs:545-562`),
  backed by `git_user_name` (gix) and `jj_user_name` (jj config-stack
  precedence). Surfacing the git/jj renderings is a `detect.rs` text edit.
- **Diff-range idiom — net-new text, no probe.** No diff-range capability
  exists; it stays model-driven, phrased as `git diff main...HEAD` /
  `jj diff --from 'fork_point(trunk())' --to @`.

⚠️ **Golden-test coupling.** The exact `--descriptive` output is pinned by
end-to-end goldens in `cli/vcs-cli/tests/detect_goldens.rs` (`bash-parity`)
against three fixtures in `cli/vcs-test-support/fixtures/vcs-detect/`:
`main-jj-workspace.json`, `main-git-checkout.json`, `colocated-git-as-file.json`.
Any new bullet requires regenerating all three. The `render`-level unit tests
(`detect.rs:216-413`) assert only `.contains(...)` substrings and tolerate
additions.

### The guard — why pure-jj breaks, and what it is not

The guard is two layered files, not a re-export pair: pure decision logic in the
`vcs` domain crate, mode gating + hook-envelope rendering in the `accelerator-vcs`
binary crate (`vcs-cli` depends on `vcs`).

- **Blocklist** (`cli/vcs/src/guard.rs:19-22`) — 13 entries exactly: `status,
  diff, add, commit, log, branch, checkout, switch, merge, rebase, reset, stash,
  show`. `rev-parse` and `config` are **absent** (an allow-list test asserts
  `git config user.name` is allowed).
- **Per-mode behaviour** (`cli/vcs-cli/src/guard.rs:39-80`) — mode is determined
  first; `Mode::Git` returns `Ok(None)` before `decide()` is ever called
  (allow-without-evaluating); `Mode::JjColocated` warns via a bare
  `systemMessage` (no `permissionDecision`, so the command still runs);
  `Mode::Jj` denies via `permissionDecision:deny`.
- **Suggestions** map per subcommand (`cli/vcs/src/guard.rs:71-85`), framed into
  a mode-specific sentence by `reason()` (`cli/vcs-cli/src/guard.rs:12-24`).

This confirms the spike 0200 framing: `validate-plan`'s pure-jj breakage is
**not** the blocklist. `git rev-parse` is not blocked, yet it fails in pure-jj
purely because there is no `.git` (`fatal: not a git repository`). Dropping
`log`/`diff` from the blocklist would make them *run* and hit the same fatal — a
worse failure than today's redirect. The remedy is to stop issuing raw git, which
is exactly 0286's approach.

## Code References

- `skills/planning/validate-plan/SKILL.md:55-63` — the evidence-gathering fence
  (raw `git log`/`git diff`/`git rev-parse` + hardcoded `make check test`)
- `skills/planning/validate-plan/SKILL.md:48,247` — soft prose "through git" /
  "analyze the git history"
- `skills/research/research-issue/SKILL.md:61-67` — the session-VCS phrasing to
  mirror
- `skills/config/migrate/SKILL.md:272-273` — `` `jj status`/`git status` `` prose
- `skills/work/refine-work-item/SKILL.md:247-249` — the author resolution chain
- `skills/work/refine-work-item/SKILL.md:172,188,214` — `jj restore` prose
  (scope-adjacent)
- `skills/vcs/commit/SKILL.md:7,18-23` — `Bash(accelerator vcs *)` +
  `!`-injection precedent for `vcs status`/`vcs log`
- `cli/vcs-cli/src/cli.rs:15-59` — the `Command` enum (add `Root` here)
- `cli/vcs-cli/src/main.rs:97-110` — dispatch match; `run_status`/`run_log`
  thunks at 37-47 to mirror
- `cli/vcs-cli/src/detect.rs:11-73` — `JJ_COMMAND_REFERENCE`, `GIT_REFERENCE`,
  `reference_text` (add both idioms here); probe-port handler pattern to mirror
- `cli/vcs-adapters/src/library.rs:511-519` — `impl RepoRoot for InProcessProbe`
  (`discover`, `repository_root`)
- `cli/vcs-adapters/src/library.rs:287-299,575-594` — `jj_workspace_root`,
  `jj_repository_root`
- `cli/vcs-adapters/src/library.rs:545-562` — `InProcessProbe::user_name`
- `cli/vcs/src/lib.rs:59-64,111-132,144-146` — `RepoRoot::repository_root` trait,
  `user_name`, and the `vcs::facts` sole call site of `repository_root`
- `cli/vcs/src/guard.rs:19-22,71-85` — blocklist + suggestions
- `cli/vcs-cli/src/guard.rs:12-24,39-80` — `reason()` + per-mode deny/warn/allow
- `hooks/hooks.json:7-10,43-47` — SessionStart `vcs detect --descriptive`;
  PreToolUse `vcs guard`
- `cli/vcs-cli/tests/detect_goldens.rs` + `cli/vcs-test-support/fixtures/vcs-detect/`
  — the three goldens to regenerate
- `tasks/README.md:444-611` — the dispatched-sub-binary checklist (per-token; not
  triggered by `vcs root`)
- `tasks/public_api.py:27,70` — `vcs` pinned, `vcs-cli` exempt

## Architecture Insights

- **Ports-and-adapters, VCS as an outbound port (ADR-0053).** The `vcs` crate
  holds ports/models/renderers; `vcs-adapters` holds the `InProcessProbe` git/jj
  implementations over `gix`/`jj-lib`. This is why "eradicate direct git" is
  tractable: the backend-neutral capabilities already exist behind ports.
- **"We own the format" (ADR-0066 / 0198).** `vcs status`/`vcs log` render one
  backend-neutral text format in-process; byte-parity with native CLIs is an
  explicit non-goal. The sole consumer injects the text as human orientation via
  the `!` preprocessor, one repo at a time. `vcs root` extends this
  deterministic-CLI-injected pattern.
- **Two dispatch levels.** Level one (`accelerator <token>` → signed sub-binary)
  is governed by the thirteen-point checklist and the `DISPATCHED_SUBBINARIES`
  registry. Level two (subcommands within a binary) is plain clap. Conflating the
  two is the work item's second inaccuracy.
- **The reference is the steering mechanism, the guard is the backstop.** The
  SessionStart cheat sheet tells the model which backend command to use; the
  PreToolUse guard only deny/warns raw git in jj modes. 0286 leans on the
  reference (extended with two idioms) and treats the guard as secondary — no
  guard change is in scope.

## Historical Context

- `meta/decisions/ADR-0066-vcs-agnostic-status-log-output-format.md` (accepted)
  — the canonical spec for backend-neutral command output that 0286 builds on.
- `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`
  (accepted) — VCS-as-a-port; the reason abstraction is possible.
- `meta/decisions/ADR-0054-git-style-modular-cli-of-on-demand-static-binaries.md`
  (accepted) — how `accelerator vcs …` sub-binaries are packaged/dispatched.
- **Gap:** no ADR covers the VCS guard, colocated support, or session-VCS
  injection; that design lives only in work items, plans and research.
- `meta/reviews/work/0286-eradicate-direct-git-calls-from-skills-review-1.md` —
  three verdicts (REVISE → COMMENT → APPROVE). Review 1 *forced all three scope
  expansions into existence* to close testability/dependency gaps: the diff-range
  anchor (trunk divergence, over first-commit "fragile to squash/reorder" and a
  named bookmark "needs consistent naming"), `vcs root`, and the reference
  extension. It also fixed AC4 to spell the jj revset and left the land-order
  (reference + `vcs root` before the skill rewrites) as prose-only unless the
  bundle is split.
- `meta/work/0200-decide-vcs-guard-log-diff-blocklist-membership.md` (done) —
  spawned 0286; decided keep-both; framed the pure-jj `.git`-absent breakage as
  distinct from guard denial. Notably it *recommended* a broad `accelerator vcs
  diff`, which 0286 review 1 then rejected in favour of a model-driven diff plus
  the narrow `vcs root`.
- `meta/work/0198-vcs-agnostic-status-log-renderer.md` (done) — built the
  in-process renderer and the session-VCS precedent; deleted the
  `vcs_adapters::subprocess` shell-out module.
- `meta/work/0169-vcs-subdomain-and-hooks-migration.md` (done) — ported the
  13-subcommand blocklist verbatim and built the `vcs detect|status|log|guard`
  subdomain + hook wiring; deliberately declined to change blocklist membership.
- `meta/work/0136-migrate-shell-scripts-to-rust-cli.md` (epic) — parent; migrate
  shell into a typed Rust CLI. 0286 is a VCS-residual child.
- `meta/work/0188-library-backed-vcs-adapter.md` (done) — delivered the
  `vcs-adapters` crate. `meta/work/0267-move-vcs-kind-detection-into-vcs-crates.md`
  (draft) is the closest open follow-on.

## Related Research

- `meta/research/codebase/2026-08-30-0198-vcs-agnostic-status-log-renderer.md`
  and `2026-08-31-0198-…` — the renderer 0286 extends.
- `meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`
  and `2026-08-05-0169-…` — the subdomain + hook migration.
- `meta/research/codebase/2026-08-02-0188-library-backed-vcs-adapter.md` — the
  adapter crate.
- `meta/research/codebase/2026-03-16-jujutsu-integration-and-vcs-autodetection.md`
  — foundational VCS-agnostic direction.
- `meta/plans/2026-03-18-vcs-skill-improvements.md` — origin of the session-VCS
  command pattern and the guard.

## Open Questions

- ❓ **`vcs root` semantics for a secondary workspace.** Should it surface
  `repository_root` (main repo, as the work item names) or `jj_workspace_root`
  (the checkout, matching `git rev-parse --show-toplevel` in a worktree)? The AC
  fixtures do not disambiguate; `validate-plan` runs checks from this root.
- ❓ **Scope of the scope-adjacent references.** Are `validate-plan` line 247
  ("analyze the git history") and `refine-work-item`'s three `jj restore` prose
  sites in scope? The `git <subcommand>` sweep passes them, but they are not
  backend-neutral.
- ❓ **Correcting the work item's Technical Notes.** The `repository_root`
  consumer and the "follows the dispatched-sub-binary checklist" claims are both
  inaccurate; the plan should not inherit them uncritically.
