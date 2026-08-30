---
type: "codebase-research"
id: "2026-08-05-0169-vcs-subdomain-and-hooks-migration"
title: "Research: VCS Subdomain and Hooks Migration (0169)"
date: "2026-08-05T13:57:49+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0169"
parent: "work-item:0169"
relates_to: ["codebase-research:2026-07-29-0169-vcs-subdomain-and-hooks-migration"]
topic: "VCS Subdomain and Hooks Migration (0169)"
tags: ["research", "codebase", "vcs", "hooks", "rust-cli", "migration"]
revision: "d325ca978cb9b29fef6ba2e18f9e0c9042cfb123"
repository: "accelerator"
last_updated: "2026-08-05T13:57:49+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: VCS Subdomain and Hooks Migration (0169)

**Date**: 2026-08-05T13:57:49+00:00
**Author**: Toby Clemson
**Git Commit**: d325ca978cb9b29fef6ba2e18f9e0c9042cfb123
**Branch**: (workspace `build-system`, working copy at an empty commit atop `main`)
**Repository**: accelerator

## Research Question

For the story at `meta/work/0169-vcs-subdomain-and-hooks-migration.md`: build a
comprehensive picture of the codebase context needed to plan its
implementation — the existing shell VCS detection/guard logic to be
reproduced, the 0188 library-backed adapter crates it builds on, the 0187
sub-binary registration surface it must use, the 0167 hook-envelope pattern it
extends, and the `skills/vcs/commit` repoint it performs.

## Summary

0169 is close to a green field on the Rust side and a fully-mapped,
byte-precise target on the shell side:

- **The shell source is small, and its taxonomy contract is exact.**
  `scripts/vcs-common.sh`'s `classify_checkout` produces a six-line
  `KEY=VALUE` record via a first-match-wins seven-arm cascade where ordering
  (`colocated` before `nested-*`) is load-bearing. Mode detection is
  duplicated four different ways across `vcs-common.sh`, `vcs-detect.sh`,
  `vcs-guard.sh`, `vcs-status.sh` and `vcs-log.sh` — two of those five
  (`vcs-detect.sh`, `vcs-guard.sh`) inline a `-d "$REPO_ROOT/.git"` check that
  misclassifies a colocated checkout whose `.git` is a *file* (worktree,
  submodule); the other two (`vcs-status.sh`, `vcs-log.sh`) check only `.jj`
  and are unaffected. `vcs-guard.sh`'s compound-command splitter is a literal
  four-pass `sed` (`&&`, `||`, `;`, `|` → newlines) feeding a `while read`
  loop that stops at the first of 13 blocked git subcommands.
- **0188 delivered the adapter foundation but wired none of it to a CLI
  subcommand.** `cli/vcs` (pure domain, `RepoRoot`/`VcsProbe` ports,
  no dependencies) and `cli/vcs-adapters` (`library::InProcessProbe`'s six
  taxonomy queries — `is_bare`, `worktree`, `superproject`,
  `jj_workspace_root`, `jj_repository`, `dual_roots` — plus a `subprocess`
  pair already wired to the crate-level `vcs_adapters::facts()` helper) both
  exist and are workspace members today. No `vcs` token exists anywhere in
  the launcher, and the four value types the classifier needs
  (`WorktreeFacts`, `JjWorkspaceRole`, `JjRepositoryFacts`, `DualRoots`) still
  live in `vcs-adapters::library`, not in the domain crate — moving them is
  additive work this story owns, gated by the domain crate's cargo-pup import
  restriction.
- **0187's registration surface is generalised and has one worked example**
  (`visualiser`), documented as a thirteen-point checklist in
  `tasks/README.md:304-456`. Registering `accelerator-vcs` needs a new
  package directory outside `cli/vcs/` (because the domain crate already owns
  that name and path), a `_SUBBINARY_MANIFESTS` entry, workspace membership,
  and a skill binding — but genuinely zero launcher code changes, since
  dispatch is a generic clap `external_subcommand` capture plus a
  by-name manifest lookup.
- **0167's `--format=hook` envelope pattern is established but has no
  PreToolUse slot and no shared module yet.** `hook_envelope()`
  (`config_command/render/summary.rs:66-74`) hand-builds a compact
  `{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"…"}}`
  string (not `serde_json`), and the empty-summary → zero-bytes decision lives
  one layer up in `resolve_summary` (`inbound/cli.rs:282-300`). No
  `crate::hooks::envelope` module or `HookEvent` enum exists anywhere in
  `cli/**` yet — this story's open question about where that module lives is
  genuinely unresolved on disk, and a cargo-pup rule
  (`config_command_may_not_import_adapters_or_launch`) constrains where it can
  go relative to `config_command`.
- **The `skills/vcs/commit` repoint has a direct precedent.**
  `skills/visualisation/visualise/SKILL.md` already carries two sibling
  scoped `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator <subcommand> *)` rules
  (`config *` and `visualiser *`) — 0169 adds a third pattern instance
  (`config *` + `vcs *`), dropping the broad `scripts/*` rule entirely. The
  permission lint (`skill_permissions.py`) enforces coverage of the two `!`
  command lines regardless of what they name, so the repoint and the
  frontmatter edit must land together.
- **Two dependency-status claims in 0169's own text are now stale, both in
  the story's favour**: 0167 and 0182 both show `status: done` on disk today,
  though 0169's prose still describes 0167 as needing to be "closed out" and
  0182 as "in-progress, code landed." The two blockers 0169's `blocked_by`
  list still names — 0187 and 0188 — are genuinely still `status: ready`
  (not `done`) in their own frontmatter, confirming those are the real
  remaining blockers. (One inconsistency worth resolving before planning:
  session memory records 0187 as "all 5 phases done+committed 2026-08-03,"
  which conflicts with the frontmatter `status: ready` found directly on disk
  — the code may be landed with the work item's status field simply not
  updated; this should be checked, not assumed, before treating 0187 as
  cleared.)

## Detailed Findings

### The shell VCS source being ported

All paths below are relative to this workspace
(`/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/build-system/`).

**`scripts/vcs-common.sh`** is the probe layer:

- `find_repo_root()` (`scripts/vcs-common.sh:8-18`) walks up testing
  `[ -e "$dir/.jj" ] || [ -e "$dir/.git" ]` — existence, not directory. This
  is deliberately unaffected by 0169's correction and already handles
  `.git`-as-a-file (fixed for the 0124 bug).
- `vcs_mode(root)` (`:27-36`) is the "jj-outranks-git dispatch": tests `.jj`
  existence first, then `.git`, else `none`. A comment (`:20-26`) states this
  is a **command-set** decision, not a topology one — even in a colocated
  checkout, `.jj` wins because git's index lags jj's working-copy commit.
- `classify_checkout(dir)` (`:177-280`) is the taxonomy function. Contract
  (`:157-176`): always exits 0, prints a six-line `KEY=VALUE` record —
  `KIND`, `BOUNDARY`, `JJ_PARENT`, `GIT_PARENT`, `JJ_MISSING`, `GIT_MISSING`.
  The jj probe (`:182-201`) and git probe (`:203-227`) each independently
  absolutise their parent-root candidates; the **classify cascade**
  (`:229-272`) is first-match-wins in this exact order, with `colocated`
  explicitly required to precede the `nested-*` arms because a true
  colocated checkout also satisfies both nested predicates
  (comment `:232-239`):
  1. `none` — neither in_jj nor in_git.
  2. `colocated` — `jj_secondary && git_worktree` with both parent roots
     resolved.
  3. `nested-jj-in-git` — `jj_secondary && in_git`, parents resolved and
     unequal.
  4. `nested-git-in-jj` — `git_worktree && in_jj`, parents resolved and
     unequal.
  5. `jj-secondary` — `jj_secondary` fallback.
  6. `git-worktree` — `git_worktree` fallback.
  7. `main` — else.
- Missing-binary handling: `jj_missing`/`git_missing` are set only when an
  ancestor carries the corresponding marker but the binary itself is absent
  (`_ancestor_has_marker`, `:45-62` — diagnostic-only, never used for primary
  classification).

**`scripts/vcs-status.sh` and `scripts/vcs-log.sh`** (~14 lines each) branch
only on `[ -d "$REPO_ROOT/.jj" ]` — they never inspect `.git` at all. This is
why the work item's `.git`-as-file correction does not reach them (unlike
`vcs-detect.sh`/`vcs-guard.sh`) and they stay strict parity.

**`hooks/vcs-detect.sh`** (SessionStart hook):

- Three-valued mode detection (`:22-37`) — the bug source: if
  `REPO_ROOT` has `.jj` and `[ -d "$REPO_ROOT/.git" ]` is true, mode is
  `jj-colocated`; else if only `.jj`, mode is `jj`; else `git`. A `.git`
  that's a *file* (worktree/submodule) is missed by this `-d` check and
  falls through to plain `jj` — the departure 0169 must correct.
  `_emit_parent_block` (`:94-100`) is the single source of truth for the
  boundary-prohibition triplet text (`do not edit files in…`,
  `do not run VCS commands against…`, `do not grep, find, or research files
  in…`).
- Final SessionStart envelope (`:177-181`):
  ```
  jq -n --arg context "$CONTEXT" --arg sys "$SYSTEM_MESSAGE" \
    '{hookSpecificOutput: {hookEventName: "SessionStart", additionalContext: $context}}
     + (if $sys == "" then {} else {systemMessage: $sys} end)'
  ```

**`hooks/vcs-guard.sh`** (PreToolUse hook):

- `gh`/`rtk` are always allowed via a whole-string grep (`:35-42`) before any
  splitting.
- **Compound-command splitting** (`:44-70`) is a single `sed` pass replacing
  each of `&&`, `||`, `;`, `|` with a newline, then a `while read` loop that
  trims whitespace per line and stops at the first line matching
  `^\s*git\s+(status|diff|add|commit|log|branch|checkout|switch|merge|rebase|reset|stash|show)(\s|$)`
  — the 13 blocked subcommands (`:53`). Everything else, including the 7
  named allowed ones (`push pull fetch remote clone config tag`), passes
  implicitly.
- **Mode determination** (`:76-81`) is the guard's own `-d
  "$REPO_ROOT/.git"` check — the second instance of the misclassification bug
  (`vcs-guard.sh:77`): a colocated checkout whose `.git` is a file is read as
  `pure-jj` and **blocks** instead of **warns**.
- **Emitted shapes, current (deprecated) form** (`:95-109`): a top-level
  `{"decision":"block","reason":…}` for pure-jj, and
  `{"decision":"allow","hookSpecificOutput":{"systemMessage":…}}` for
  colocated warn — both explicitly named by the work item as shapes the Rust
  port must **not** reproduce (departure 1).

**`hooks/config-detect.sh`** is a 14-line pure exec wrapper —
`exec "$ACCELERATOR" config summary --format=hook --fail-safe` — with no
detection logic of its own; 0169 inlines this directly into `hooks.json` and
deletes the file.

**`hooks/hooks.json`** registers 5 hooks today (4 SessionStart + 1
PreToolUse); the 3 command strings 0169 must produce verbatim are quoted in
the work item (lines 161-166) and match what these agents independently
confirmed from the source.

**`hooks/test-fixtures/vcs-detect/*.json`** — two goldens
(`main-git-checkout.json`, `main-jj-workspace.json`); notably the "jj" fixture
is actually built via `jj git init --quiet` (colocated) so its golden mode is
`jj-colocated`, not plain `jj`.

**`hooks/test-vcs-detect.sh`** (~713 lines) is the 42-case parity gate: fixture
builders for all seven `classify_checkout` shapes plus missing-binary and
`hooks.json`-literal cases, organized under AC1–AC9 labels inherited from the
*original* 0058 work item (not 0169's own ACs). The work item's required
partition (27 in-process cases → a new `scripts/test-vcs-common.sh`;
subprocess cases repointed by swapping the `HOOK` constant; missing-binary
cases deleted; two singletons — the AC9 comment-block grep deleted, the AC8
`hooks.json` literal updated and made order-independent) is derivable from
this structure but was **not** independently re-summed by this research pass
— the review history (below) flags this exact partition as having drifted
across editing passes, so re-deriving and stating it as a line-range
partition in the plan is safety-critical, not optional.

### The 0188 foundation: `cli/vcs` and `cli/vcs-adapters`

`cli/vcs/src/lib.rs` is a small pure-domain crate: `VcsKind` (`Jj`/`Git`/
`None`), `RepoFacts { root, name, kind, revision }`, and two ports —
`RepoRoot` (`discover`, defaulted `repository_root`) and `VcsProbe` (`kind`,
`revision`) — composed by `pub fn facts(start, root: &dyn RepoRoot, probe:
&dyn VcsProbe) -> Option<RepoFacts>` (`cli/vcs/src/lib.rs:74-91`). It has
**no dependencies** and is restricted by cargo-pup rule
`vcs_domain_imports_only_permitted` (`cli/pup.ron:75-89`) to
`std`/`core`/`alloc`, `kernel::Error`, and `crate` imports only — matching
`^vcs($|::)`.

`cli/vcs-adapters/src/library.rs` implements `InProcessProbe` with six
taxonomy query methods, all `Result<Option<T>, Error>` except the infallible
`dual_roots` (which wraps a per-side `Result`):

```rust
pub fn is_bare(&self, start: &Path) -> Result<Option<bool>, Error>                       // :204
pub fn worktree(&self, start: &Path) -> Result<Option<WorktreeFacts>, Error>              // :215
pub fn superproject(&self, start: &Path) -> Result<Option<PathBuf>, Error>                // :251
pub fn jj_workspace_root(&self, start: &Path) -> Result<Option<PathBuf>, Error>           // :279
pub fn jj_repository(&self, start: &Path) -> Result<Option<JjRepositoryFacts>, Error>     // :300
pub fn dual_roots(&self, start: &Path) -> DualRoots                                       // :337
```

Key implementation notes load-bearing for the classifier this story writes:

1. **Three walks, not one** — `jj_workspace_root`/`jj_repository`/the jj half
   of `dual_roots` resolve through a `.jj`-only ancestor walk
   (`carries_jj_marker`), never the combined `.jj`-or-`.git` walk; feeding
   `DefaultWorkspaceLoaderFactory::create` the combined boundary makes it
   report absence on nested-git-in-jj shapes where `jj workspace root` should
   report presence.
2. `superproject` gates on the git-dir-vs-common-dir comparison, not on
   `kind()` — `kind()` is measured wrong in both directions for linked
   worktrees of/in submodules.
3. `jj_repository` derives `JjWorkspaceRole::Main` vs `Secondary` by
   comparing the loader's canonicalised `repo_path()` against
   `<root>/.jj/repo`, with a post-condition guard requiring
   `<main_root>/.jj/repo` to actually be a directory
   (`Error::JjStoreLayout` otherwise).
4. `gix` 0.85 cannot read sha256-format repositories — every gix-backed query
   returns `Err` for those; reftable reads normally.

`WorktreeFacts`, `JjWorkspaceRole`, `JjRepositoryFacts`, and `DualRoots` are
all currently declared **inside** `vcs-adapters::library`
(`library.rs:157-189`), not in `vcs`. Since `vcs`'s import-restriction rule
forbids depending on `vcs-adapters`, moving these types to the domain crate —
which the work item's Amendment (finding 5) says this story must do when it
defines its classifier's port — is purely additive to `vcs` plus a
re-export/removal in `vcs-adapters`, and will churn
`cli/vcs-adapters/tests/queries.rs`'s expected-value tables.

The crate-level `pub fn facts()` (`cli/vcs-adapters/src/lib.rs:24-26`)
already wires the **subprocess** pair (`MarkerWalkRoot`, `CommandProbe`), not
`InProcessProbe` — the library-backed six-query probe is currently only
exercised by its own tests and the `vcs-adapters-fixture`/`-stub` binaries,
not by any production call site.

`cli/vcs-test-support::fixtures::pure_jj` (`cli/vcs-test-support/src/fixtures.rs:531-551`)
is the named builder the work item requires reuse of for the latency
benchmark, and `Matrix::build_in`/`build_or_adopt` (`:109-186`) builds/adopts
the ~34-fixture matrix already used by `cli/vcs-adapters/tests/queries.rs`'s
oracle-mapping table.

**Confirmed: no `vcs` dispatch token, `accelerator-vcs` package, or launcher
wiring exists anywhere yet.** `cli/Cargo.toml`'s workspace members list
includes `vcs`, `vcs-adapters`, `vcs-test-support` as libraries only (plus two
non-shipped fixture binaries inside `vcs-adapters`); a repo-wide grep for
`vcs` under `cli/launcher/src` returns nothing. This story is genuinely
greenfield on the launcher/dispatch side.

### The 0187 sub-binary registration surface

`_SUBBINARY_MANIFESTS` (`tasks/manifest.py:48-56`) maps a dispatch token to
its crate's `Cargo.toml` path only when that path isn't the default
`cli/<token>/Cargo.toml`. The **only** worked example today is `visualiser`
(`tasks/shared/paths.py:29` → `DISPATCHED_SUBBINARIES = ("visualiser",)`;
manifest path `cli/visualiser/server/Cargo.toml`; package name
`accelerator-visualiser`). Because `cli/vcs/Cargo.toml` already exists with
`package.name = "vcs"` (the domain crate from 0179/0188), `accelerator-vcs`
must live in a **sibling directory** — the work item states this explicitly
(lines 193-197) and the 0187 research independently confirms the domain-crate
collision is exactly why.

`validate_dispatch_coherence` (`tasks/shared/dispatch_coherence.py:143-219`)
enforces token format (`^[a-z][a-z0-9-]*$`), reserved-token exclusion
(`verify`, `launcher`), no shadowing of `BUILTIN_SUBCOMMANDS`
(`version`, `config`, `help`), and — critically — that every skill invoking
`${CLAUDE_PLUGIN_ROOT}/bin/accelerator <token>` carries a `Bash(...)` rule
naming exactly that token's subcommand segment, with no bare-`Bash`,
bare-launcher, or wildcarded-segment rule anywhere in that skill's
frontmatter disqualifying it as a witness. This is called both from
`tasks/manifest.py:138` (blocking a fresh manifest write) and as
`mise run lint:dispatch-coherence:check`.

Launcher dispatch needs **zero code changes** per new token: clap's
`#[command(external_subcommand)]` on `Command::External(Vec<OsString>)`
(`cli/launcher/src/launch/inbound/cli.rs:16-29`) captures any unrecognised
subcommand and its full argument tail; `ExternalCommand::from_raw`
(`cli/launcher/src/launch/core.rs:19-32`) splits it into `name`/`args`;
`LazyProductionResolver::resolve` (`cli/launcher/src/main.rs:56-69`) checks a
generic `ACCELERATOR_<TOKEN>_BIN` override first, then falls back to a
by-name lookup in the fetched/verified/cached signed manifest.

The full thirteen-point checklist lives at `tasks/README.md:304-456`, each
point tagged with where a mistake surfaces (`[PR]`/`[release]`/`[author]`).
Points 1, 2, 3, 4, 7, and 8 are explicitly called out as needing to land in
the **same change**. Point 3 requires `package.description` (the manifest
sources it directly and fails hard if absent), inherited `version.workspace`
(a hardcoded version desyncs silently at the next workspace bump), and either
`[lints] workspace = true` or an explicit crate-local `[lints.clippy]` table.
Point 8's binary-name discipline matters mechanically:
`accelerator-<token>` (the `[[bin]] name`), not the bare token, must go into
`_CLI_RELEASE_BINARIES` (`tasks/build.py:35`), because staging paths and
manifest asset paths only line up with that prefix present. `cargo-pup` and
`cargo-deny` obligations for the new binary crate are likely minimal (it
consumes already-vetted `vcs`/`vcs-adapters` dependencies), but the existing
`vcs_adapters_library_reads_in_process` cargo-pup rule
(`cli/pup.ron:100-121`) needs widening to cover wherever `vcs status`/`vcs
log` land, per the work item's own Amendment.

### The 0167 hook-envelope pattern and its PreToolUse gap

`cli/launcher/src/config_command/render/summary.rs:66-74`:

```rust
pub fn hook_envelope(summary: &str) -> String {
    format!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"SessionStart\",\
         \"additionalContext\":\"{}\"}}}}",
        json_escape(summary)
    )
}
```

Hand-built via `format!`, not `serde_json` — the module doc states this keeps
the hexagon's import surface narrow. `json_escape` (`:76-101`) is a hand-rolled
RFC 8259 escaper (quote, backslash, the named control escapes, and `\u00XX`
for any other control byte). The empty-summary → zero-bytes decision is
**not** in this file: it's one layer up, in `resolve_summary`
(`cli/launcher/src/config_command/inbound/cli.rs:282-300`), which
three-way-matches `(summary_render::body(&summary), hook)` — `(None, _)` →
empty string; `(Some(text), false)` → `text\n`; `(Some(text), true)` →
`hook_envelope(text)\n`. 0169's contract needs a *fourth*, disjoint outcome
(an adapter-failure `systemMessage` object) that this renderer has no slot
for — the work item's own "Notes from 0167" section names this gap
explicitly.

`--fail-safe` is implemented in two independent layers:

- **Rust-side**, per-subcommand: a clap flag converted to `OnFailure::Fail`
  vs `OnFailure::Degrade` (`cli/launcher/src/launch/mod.rs:22-28`), threaded
  into `config_command::core`, which distinguishes `Failure::Read` (IO,
  degradable under `Degrade`) from `Failure::Refusal` (never degradable,
  classified via `ConfigError::is_refusal()`,
  `cli/config/src/error.rs:76-85`) in `finish`
  (`cli/launcher/src/config_command/inbound/cli.rs:453-472`). Exit-code
  mapping happens centrally in `cli/launcher/src/main.rs:203-212`:
  `kernel::Error::Refusal` → exit 2, everything else non-Ok → exit 1.
- **Bootstrap-side**, independent of the Rust flag: `bin/accelerator:28-39`
  scans *all* argv tokens (any position, up to the first `--`) for the
  literal `--fail-safe`, setting `abort_status=0` if found — this governs
  every `fail()`/`fail_integrity()` call in the bootstrap script itself
  (fetch/verify/trust-chain failures), independent of and prior to any Rust
  code running. This is why 0169's "fails open on trust-chain and fetch
  failures" requirement is largely automatic at the bootstrap layer, provided
  the `hooks.json` registration includes the literal `--fail-safe` token.

**No `hooks`/`envelope` module and no `HookEvent` enum exist anywhere in
`cli/**` today** — `"SessionStart"` is hard-coded as a string literal inside
`hook_envelope`'s `format!` template, and a repo-wide grep for
`hookEventName`/`permissionDecision`/`PreToolUse` inside `cli/**` Rust source
returns matches only in the config-command renderer, its doc comments, and
one integration test. The work item's open question about where a shared
`crate::hooks::envelope` module should live is genuinely unresolved on disk.
A relevant constraint: cargo-pup rule
`config_command_may_not_import_adapters_or_launch` (`cli/pup.ron:131-144`)
denies `config_command` from importing `^config_adapters(::|$)` or
`^crate::launch(::|$)` — so if the shared envelope module lands under
`crate::hooks` inside the **launcher** crate, it must sit outside
`crate::launch` specifically for `config_command` to reach it; `kernel`
(described in its own module doc as "the lowest crate... cannot name a
subdomain's types") is the cleaner alternative the work item names.

### The `skills/vcs/commit` repoint

`skills/vcs/commit/SKILL.md` frontmatter currently carries:

```yaml
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator config *)
```

with body lines 13-14 invoking
`!`${CLAUDE_PLUGIN_ROOT}/scripts/vcs-status.sh`` and
`!`${CLAUDE_PLUGIN_ROOT}/scripts/vcs-log.sh``. The direct precedent for the
target shape is `skills/visualisation/visualise/SKILL.md:6-9`, which already
carries two sibling scoped rules (`config *` and `visualiser *`) rather than
one broad rule — 0169 should drop `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/*)`
entirely and add `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs *)` alongside
the existing `config *` rule, repointing the two `!` lines to
`accelerator vcs status`/`accelerator vcs log`.

Two lint guards interact with this change:

- `tasks/lint/skill_permissions.py` (`EXPECTED_INJECTION_SKILLS = 42`,
  line 48) requires every plugin-prefixed `!` invocation to be covered by a
  `Bash(...)` rule (`_command_violations`, `:64-108`) — it doesn't know or
  care about the old script names specifically, so it will pass equally
  whether the invocation names the old scripts or the new subcommand, **as
  long as the frontmatter rule covers whatever's actually invoked**. This
  means the frontmatter narrowing and the body repoint must land together, or
  the lint goes silent on a stale reference (the broad `scripts/*` rule would
  still "cover" a leftover old-script line).
- `tasks/lint/call_site_migration.py` guards only `scripts/config-*` call
  sites (a 0167-specific anti-regression check) — it would **not** catch a
  stray `scripts/vcs-*.sh` reference left behind anywhere.

Dispatch needs no launcher change (confirmed independently by both the 0187
and the skill-repoint research passes — `cli/launcher/src/launch/inbound/cli.rs:15-22`).
No other repo surface (`README.md`, `tasks/README.md`) currently references
the five files being retired; `docs-site/src/content/docs/internals.md` was
not directly checked and is worth a targeted look during planning, per the
work item's own AC naming `docs/internals.md` as a doc surface to update.

### Historical and document context

**Prior research** (`meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`,
complete, 2026-07-30) recorded four decisions now baked into 0169's current
text: (1) `${CLAUDE_PLUGIN_ROOT}` expansion in `hooks.json` works, so
`config-detect.sh` can be inlined and deleted; (2) the PreToolUse envelope
must use the current `permissionDecision` shape, with the colocated
warn-but-permit case emitting a bare `systemMessage` and **no**
`permissionDecision` at all (emitting `"allow"` would silently skip the
permission prompt — a privilege widening); (3) ship the guard as a
sub-binary and fix the bootstrap's exec-probe cost in this story (later
carved out as 0186, since done); (4) the `vcs` token collides with the
existing domain crate under both `tasks/manifest.py`'s default path
resolution and cargo-pup's domain-import rule, resolved by naming the
**package** `accelerator-vcs` in a sibling directory while keeping the
**dispatch token** `vcs`.

**Review history** (`meta/reviews/work/0169-vcs-subdomain-and-hooks-migration-review-2.md`)
ran four editing passes with major-finding counts of 19 → 15(+1 critical) →
14 → 14 — never reaching APPROVE. Pass 2's sole critical (no criterion
verified the PreToolUse envelope is honoured at the plugin's v2.1.144 floor —
an undetectable safety regression if not) was resolved by pass 3's addition
of a floor-verification criterion, now present in the work item. Pass 4's
measurement — 11 of 14 majors were defects *introduced by pass 3's own
fixes*, not new problems — is what drove the decision to split rather than
keep editing: sub-binary distribution → 0187, the `gix`/`jj-lib` swap → 0188,
the exec-probe fix → 0186, and `corpus-adapters` convergence (flagged in pass
3 as orthogonal scope) → 0185. 0169 retained only the pieces judged
undeliverable separately: the subdomain, the hooks migration, and the skill
repoint.

**ADR-0048** (Four-Toolchain Split, accepted 2026-06-27) is the authority for
moving hook logic into the CLI — its text states directly: "the hook logic is
implemented in the CLI, fronted by a thin shell shim only if the hook entry
point demands a shell command." There is no separate hooks-specific ADR.

**ADR-0053** (Thin CLI over a Hexagonal Ports-and-Adapters Core, accepted
2026-06-27) is the template `vcs`/`vcs-adapters` already follow and that any
new `vcs` subcommand hexagon must follow: domain core with inbound/outbound
ports as traits, adapters at the edges, composition at a root, inward
dependency direction mechanically enforced by `cargo-deny` between crates and
`cargo-pup` within a crate.

**Dependency-status corrections** (frontmatter checked directly against
0169's prose claims):

| Item | Frontmatter status | 0169's text | Verdict |
|---|---|---|---|
| 0164 | done | done | matches |
| 0166 | done | done | matches |
| 0167 | **done** | "code landed... work item still `ready`, close out before starting" | **stale — 0167 is done; the caveat is moot** |
| 0179 | done | done | matches |
| 0186 | done | done | matches |
| 0187 | ready | listed in `blocked_by`, no status asserted | consistent — genuinely still open (see caveat below) |
| 0188 | ready | listed in `blocked_by`, no status asserted | consistent — genuinely still open |
| 0182 | **done** | "in-progress, code landed" | **stale — 0182 is done** |

**Related, non-blocking work items**, briefly: 0125 (draft — the two-strategy
VCS-detection convergence; 0169 explicitly does not close it, and actually
*adds a third* detection implementation alongside the two shell ones, though
it dissolves 0125's stated rationale for the lexical fallback); 0165 (done —
the release pipeline 0169's sub-binary distribution depends on); 0183 (draft
— the SessionStart-advisories-on-stderr audit that 0169's new `vcs detect`
site falls under); 0185 (draft, blocked by 0188 — converges
`corpus-adapters` onto the library-backed adapter, explicitly not this
story's scope).

## Code References

- `scripts/vcs-common.sh:8-18` — `find_repo_root`
- `scripts/vcs-common.sh:27-36` — `vcs_mode` (jj-outranks-git dispatch)
- `scripts/vcs-common.sh:157-280` — `classify_checkout` contract and cascade
- `scripts/vcs-status.sh:8-13`, `scripts/vcs-log.sh:8-13` — `.jj`-only mode
  branch (unaffected by the `.git`-as-file correction)
- `hooks/vcs-detect.sh:22-37` — three-valued mode detection (bug source)
- `hooks/vcs-detect.sh:94-100` — `_emit_parent_block` prohibition triplet
- `hooks/vcs-detect.sh:177-181` — SessionStart envelope construction
- `hooks/vcs-guard.sh:44-70` — compound-command splitter
- `hooks/vcs-guard.sh:53` — 13 blocked git subcommands
- `hooks/vcs-guard.sh:76-81` — colocated/pure-jj mode determination (second
  bug instance)
- `hooks/vcs-guard.sh:95-109` — deprecated PreToolUse shapes (not reproduced)
- `hooks/config-detect.sh:13` — the wrapper's sole `exec` line
- `hooks/hooks.json:1-53` — current 5-hook registration
- `hooks/test-vcs-detect.sh` — 42-case parity gate (full structure)
- `cli/vcs/src/lib.rs:14-91` — `VcsKind`, `RepoFacts`, `RepoRoot`, `VcsProbe`,
  `facts()`
- `cli/vcs-adapters/src/library.rs:204-388` — the six `InProcessProbe`
  taxonomy queries plus `RepoRoot`/`VcsProbe` impls
- `cli/vcs-adapters/src/library.rs:157-189` — `WorktreeFacts`,
  `JjWorkspaceRole`, `JjRepositoryFacts`, `DualRoots` (to move to `cli/vcs`)
- `cli/vcs-adapters/src/lib.rs:24-26` — crate-level `facts()` (wires
  subprocess, not library, adapters)
- `cli/vcs-test-support/src/fixtures.rs:531-551` — `fixtures::pure_jj`
- `cli/pup.ron:75-89` — `vcs_domain_imports_only_permitted`
- `cli/pup.ron:100-121` — `vcs_adapters_library_reads_in_process`
- `cli/pup.ron:131-144` — `config_command_may_not_import_adapters_or_launch`
- `tasks/manifest.py:48-95` — `_SUBBINARY_MANIFESTS`, `collect_entries`
- `tasks/shared/dispatch_coherence.py:104-219` — `validate_dispatch_coherence`
- `tasks/shared/paths.py:29-37` — `DISPATCHED_SUBBINARIES`,
  `SKILL_EXEMPT_SUBBINARIES`, `DEBUG_ARCHIVE_DIRS`
- `tasks/build.py:35,151-179` — `_CLI_RELEASE_BINARIES`,
  `_assert_static_elf`
- `tasks/README.md:304-456` — thirteen-point sub-binary registration
  checklist
- `cli/visualiser/server/Cargo.toml:1-19` — worked registration example
- `cli/launcher/src/launch/inbound/cli.rs:16-29` — `Command::External` clap
  catch-all
- `cli/launcher/src/launch/core.rs:19-32` — `ExternalCommand::from_raw`
- `cli/launcher/src/main.rs:56-69` — `LazyProductionResolver::resolve`
- `cli/launcher/src/config_command/render/summary.rs:18-101` — `body`,
  `hook_envelope`, `json_escape`
- `cli/launcher/src/config_command/inbound/cli.rs:282-300` —
  `resolve_summary` (three-way empty/plain/hook match)
- `cli/launcher/src/config_command/inbound/cli.rs:414-472` — `Failure`,
  `finish` (degrade-on-read-failure logic)
- `cli/launcher/src/launch/mod.rs:22-28` — `on_failure` (`OnFailure`
  conversion)
- `bin/accelerator:28-39` — bootstrap `--fail-safe` global-token scan
- `cli/main.rs` (`cli/launcher/src/main.rs:203-212`) — exit-code mapping
  (`Refusal` → 2, else → 1)
- `skills/vcs/commit/SKILL.md:7-14` — current broad permission + script
  invocations to repoint
- `skills/visualisation/visualise/SKILL.md:6-9` — precedent for sibling
  scoped `bin/accelerator <subcommand> *` rules
- `tasks/lint/skill_permissions.py:48,64-108` — `EXPECTED_INJECTION_SKILLS`,
  coverage check
- `tasks/lint/call_site_migration.py:26-40` — scoped to `scripts/config-*`
  only

## Architecture Insights

- **Hexagonal by convention, not by tooling accident.** ADR-0053's
  ports-and-adapters shape is enforced two ways at once: crate-boundary
  `cargo-deny` ban-lists between crates, and `cargo-pup` module-import
  restrictions *within* a crate (e.g. `vcs`'s import allowlist, `vcs-adapters
  ::library`'s `std::process` denial, `config_command`'s deny-list on
  `launch`/`config_adapters`). Both mechanisms recur throughout this domain
  and any new `vcs` hexagon should expect to define its own pup rule(s) if it
  introduces new module boundaries.
- **Dispatch is fully generic; registration is the ceremony.** The launcher
  needs zero code changes to add a subcommand — the entire cost of adding
  `vcs` as a dispatched token lives in the Python build-system registries,
  the Cargo workspace wiring, and the release/signing pipeline, not in Rust
  dispatch logic. This makes 0187's checklist the load-bearing artefact for
  planning the registration phase, not the launcher's `cli.rs`.
- **A domain crate's own name can collide with a sub-binary token.** `vcs`
  the crate (pure domain) and `vcs` the dispatch token (sub-binary) are two
  different naming systems (`tasks/manifest.py`'s default-path resolution,
  cargo-pup's module-name matching) that silently assume they're the same
  string unless told otherwise via `_SUBBINARY_MANIFESTS`. This is a
  documented trap, not a novel discovery of this research pass, but it is a
  concrete planning input: the new binary package's directory name and its
  `_SUBBINARY_MANIFESTS` entry are both required, non-optional pieces.
- **Hand-built JSON envelopes are the established idiom for hook output**,
  specifically to keep hexagon cores free of a `serde_json` dependency. Any
  new envelope work (the PreToolUse `permissionDecision` shape, the merged
  `systemMessage` sibling) should extend `json_escape`/`hook_envelope`'s
  approach rather than introducing `serde_json` into a crate that currently
  avoids it, unless that decision is revisited explicitly.
- **`--fail-safe` is a two-tier mechanism, not one.** The bootstrap's own
  argv scan (any position, any subcommand) already fails open on trust-chain
  and fetch failures independent of anything the Rust binary does; the
  Rust-side `OnFailure::Degrade` mechanism only governs failures *after* the
  binary is already running (e.g. an unreadable repository). Planning should
  keep these two failure domains distinct: "the release host is unreachable"
  is a bootstrap-layer concern already solved; "the repository is corrupt" is
  a `vcs`-subcommand-layer concern this story must implement.

## Historical Context

- `meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`
  — prior research; source of the four decisions (hooks.json expansion
  confirmed, PreToolUse shape resolved to `permissionDecision`, sub-binary +
  exec-probe fix scoped in then later carved out, `vcs`-token collision
  resolved) now embedded in the work item's current text. Also catalogued
  ten now-mostly-corrected "stale claims" worth a final spot-check during
  planning.
- `meta/reviews/work/0169-vcs-subdomain-and-hooks-migration-review-2.md` —
  four-pass review history explaining *why* the story is scoped the way it
  is today (the split into 0185/0186/0187/0188) and flagging that the 42-case
  test partition and the guard's "×4 repo modes" multiplier have drifted
  across edits before — both worth re-deriving from source rather than
  trusting the current prose at face value.
- `meta/decisions/ADR-0048-four-toolchain-split.md` — authority for hook
  logic living in the CLI.
- `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`
  — the hexagon template this story's new subcommand(s) must follow.

## Related Research

- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
  — the epic-level research 0169 derives from.
- `meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`
  — the immediate predecessor to this document; still the primary
  implementation-surface reference, now supplemented by this pass's direct
  reading of the 0188/0187/0167 code that has landed since.

## Open Questions

- **Where does the shared hook-envelope module live?** Confirmed genuinely
  open on disk — no `crate::hooks::envelope` or equivalent exists anywhere in
  `cli/**`. The work item's default (a new launcher module) is constrained by
  the `config_command_may_not_import_adapters_or_launch` cargo-pup rule; the
  `kernel`-crate alternative avoids that constraint entirely since `kernel`
  sits below every subdomain by design.
- **Is `permissionDecision`/top-level `systemMessage` honoured at Claude Code
  v2.1.144?** Still unresolved and still gates planning per the work item's
  own Sequencing Constraint 1 — this research pass found no new evidence
  either way; it remains an empirical check to run early.
- **Is 0187 actually done, or only its underlying registration surface
  (which pre-existed via the `visualiser` example)?** Session memory says
  "done" for 0187; the work item frontmatter read directly during this
  research shows `status: ready`. Worth reconciling — check whether 0187's
  own acceptance criteria are met on disk — before treating it as a cleared
  blocker in planning.
- **The 42-case test partition and the guard's row-count multiplier** — both
  flagged by the review history as having drifted across prior editing
  passes. This research pass read the *shell* source closely enough to
  confirm the underlying fixture/case shapes exist, but did not independently
  re-sum the exact partition the work item's ACs assert (27 + subprocess +
  missing-binary + 2 singletons = 42; 34 × 4 = 136 + 1 = the guard total).
  Re-deriving these sums directly from `hooks/test-vcs-detect.sh` line ranges
  should be an early step in planning, not assumed from the work item's
  prose.
