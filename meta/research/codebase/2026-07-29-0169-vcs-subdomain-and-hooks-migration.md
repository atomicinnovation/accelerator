---
type: "codebase-research"
id: "2026-07-29-0169-vcs-subdomain-and-hooks-migration"
title: "Research: VCS Subdomain and Hooks Migration (0169)"
date: "2026-07-29T16:20:46+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0169"
parent: "work-item:0169"
relates_to: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture", "codebase-research:2026-07-19-0167-config-command-and-invocation-contract-migration", "codebase-research:2026-07-11-0179-corpus-crates-parsing-conventions", "codebase-research:2026-07-03-0164-launcher-and-git-style-dispatch"]
topic: "Implementation surface for the accelerator-vcs subdomain and the SessionStart/PreToolUse hooks migration"
tags: ["research", "codebase", "vcs", "hooks", "rust", "cli", "migration", "gix", "jj-lib"]
revision: "0b2f8920ae677b141a161c78fb35d4e7bb2ae0db"
repository: "accelerator"
last_updated: "2026-07-30T00:00:00+00:00"
last_updated_note: "Added empirical gix/jj-lib probe and latency measurements; recorded sub-binary, in-scope probe_dir-fix and permissionDecision-envelope decisions"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: VCS Subdomain and Hooks Migration (0169)

**Date**: 2026-07-29 16:20 UTC
**Author**: Toby Clemson
**Git Commit**: `0b2f8920ae677b141a161c78fb35d4e7bb2ae0db`
**Branch**: working copy on the `0182-plugin-root-rename-and-terminal-surface` lineage (`main` = `66abd9ce` is an ancestor; this revision is unpushed, so references below are local paths, not permalinks)
**Repository**: accelerator

## Research Question

What is the implementation surface for work item 0169 — building the
`accelerator-vcs` subdomain (`detect`/`status`/`log`/`guard`) over the
`vcs`/`vcs-adapters` crates, and migrating the SessionStart VCS-detection and
PreToolUse guard hook logic into the CLI behind a `--format=hook` switch?

## Summary

The story is well-shaped and its dependencies have all landed, but research
surfaced **four findings that change the work** and **ten stale claims** in the
work item text.

The four substantive findings:

1. **The story's central open probe is answered, in the story's favour.**
   `hooks.json`'s `command` field *does* expand `${CLAUDE_PLUGIN_ROOT}` and *does*
   tokenize arguments (it is handed to `sh -c`), and there is additionally an
   *exec form* — a sibling `args` array — that passes argv tokens verbatim with no
   shell at all. Either way, `hooks/config-detect.sh` can be deleted and the
   wrapper is unnecessary. The last clause of the final AC is unblocked.

2. **The PreToolUse envelope the story pins as its golden fixture is the
   deprecated shape — so the story emits the current one instead.** Claude Code
   documents `{"hookSpecificOutput":{"hookEventName":"PreToolUse",
   "permissionDecision":…,"permissionDecisionReason":…}}` with `systemMessage` as
   a *top-level* field. `hooks/vcs-guard.sh:103-108` nests `systemMessage`
   *inside* `hookSpecificOutput`, where the schema has no such field — so the
   colocated "prefer jj" warning is almost certainly discarded today.
   **Decision (author, 2026-07-30): emit the `permissionDecision` envelope.**
   That makes the guard a deliberate behavioural change rather than a port, so
   the AC splits into decision-parity plus a golden fixture on the *new* shape
   (§4). Note `permissionDecision: "allow"` skips the permission prompt, so the
   colocated "warn but permit" case must emit a bare top-level `systemMessage`
   with no decision at all.

3. **The per-Bash-call cost is real but lives in the bootstrap, not in the
   sub-binary choice — and fixing it is now in scope** (measured — §12). Warm,
   on darwin-arm64: today's `vcs-guard.sh` is **35 ms**; bootstrap + launcher is
   **149 ms**. But the launcher binary itself is only **3 ms** and a minisign
   verify of the 8 MB launcher only **2.3 ms**. **~108 ms is `probe_dir`**
   (`bin/accelerator:166-180`) writing a fresh executable and running it (macOS
   charges a first-exec check), and a further **~23 ms** is hashing the 475 K
   verify shim twice to content-address a *same-directory* copy. Both are
   avoidable, which would put the bootstrap near **18 ms** — below the shell
   guard it replaces. **Decisions (author, 2026-07-30): ship the guard as a
   sub-binary, and land the `probe_dir` fix in this story** rather than
   deferring it, since the guard runs on every Bash call.

4. **The token `vcs` collides with the existing domain crate, twice — and this
   now matters, given the sub-binary decision.**
   `tasks/manifest.py:56-57` would resolve the sub-binary's manifest to
   `cli/vcs/Cargo.toml` — the pure domain crate, which has no `description` and no
   `[[bin]]`, raising `ManifestError`. And cargo-pup's
   `vcs_domain_imports_only_permitted` matches `^vcs($|::)` — the *whole crate
   name* — so a binary crate named `vcs` could not import its own adapters.
   Resolution in §8.

**The `gix`/`jj-lib` risk is now retired empirically** (§9). Measured against a
verbatim copy of `cli/deny.toml`, the combined tree passes `bans`, `advisories`
and `sources`; the sole licence rejection is `uluru` (MPL-2.0), cleared by the
one-line per-crate exception the repo's own policy prescribes. A binary calling
both libraries cross-compiles to a **statically linked** musl ELF that
`_assert_static_elf` accepts. jj-lib's workspace loader reproduces the shell's
secondary-workspace rule exactly, and needs no `UserSettings`.

The remaining understatement is scope: only ~11 of `hooks/test-vcs-detect.sh`'s
42 cases are repointable, and `vcs guard` has **no test coverage at all** today.

## Detailed Findings

### 1. Current state of the `vcs` / `vcs-adapters` crates

Both crates exist (delivered by 0179) and are much thinner than the story
implies. `cli/vcs/src/lib.rs` is 193 lines with **zero dependencies**.

The entire public surface:

| Item | Location |
| --- | --- |
| `enum VcsKind { Jj, Git, None }` | `cli/vcs/src/lib.rs:14-17` |
| `VcsKind::as_str` | `cli/vcs/src/lib.rs:22` |
| `struct RepoFacts { root, name, kind, revision }` | `cli/vcs/src/lib.rs:38-43` |
| `trait RepoRoot { discover, repository_root }` | `cli/vcs/src/lib.rs:46-57` |
| `trait VcsProbe { kind, revision }` | `cli/vcs/src/lib.rs:60-67` |
| `fn facts(start, &dyn RepoRoot, &dyn VcsProbe)` | `cli/vcs/src/lib.rs:74-91` |

**There is no checkout-classification model.** The taxonomy the story must add is
absent in every arm:

- *bare* — not representable. No `.git` marker ⇒ `discover` returns `None` ⇒
  `facts` returns `None`, indistinguishable from "no repository at all"
  (`cli/vcs-adapters/tests/detection.rs:251-277` pins this deliberately).
- *worktree* — a `.git` *file* is intentionally indistinguishable from a `.git`
  directory; discovery tests existence only (`cli/vcs-adapters/src/lib.rs:38`).
- *colocated* — collapsed into `VcsKind::Jj` by marker precedence
  (`cli/vcs-adapters/src/lib.rs:98-104`); unobservable from `RepoFacts`.
- *nested* — the ancestor walk stops at the first marker, with no record that
  nesting occurred (`cli/vcs-adapters/src/lib.rs:37-42`).
- *`GIT_DIR`* — appears only as a variable to be scrubbed
  (`cli/vcs-adapters/src/lib.rs:141`), never read or honoured.
- *jj secondary workspace* — the **only** topology distinction present, and not
  as a type: it is the defaulted `RepoRoot::repository_root` hook, resolved by
  pure file reads (no subprocess) in `jj_repository_root`
  (`cli/vcs-adapters/src/lib.rs:57-68`).

`VcsKind`'s doc comment (`cli/vcs/src/lib.rs:9-12`) already encodes the
jj-outranks-git rationale the story wants preserved — "a colocated checkout
carries both markers, and `Jj` wins there because git's index lags the jj
working-copy commit".

**Only two subprocesses exist today**, and all spawning funnels through one site:

- `jj --color=never --no-pager log -r @ --no-graph -T commit_id` —
  `cli/vcs-adapters/src/lib.rs:110-120`
- `git -c color.ui=false rev-parse HEAD` — `cli/vcs-adapters/src/lib.rs:124-125`
- the single spawn chokepoint: `command.spawn()` at
  `cli/vcs-adapters/src/lib.rs:168`

That chokepoint is the natural place to instrument the story's "zero spawns" AC.

Two extension frictions worth knowing:

- `vcs_adapters::facts(start)` (`cli/vcs-adapters/src/lib.rs:225-227`)
  hard-wires `MarkerWalkRoot` + `CommandProbe::new()` with no injection variant,
  and the sole production caller (`cli/corpus-adapters/src/metadata.rs:201`)
  calls *that*, not `vcs::facts` — so nothing downstream can be tested against a
  fake probe.
- `RepoFacts.root` and `.kind` currently have **no production consumer**
  (`corpus-adapters` reads only `.name` and `.revision` at
  `cli/corpus-adapters/src/metadata.rs:185-186`), which gives latitude to reshape.

**The `bash-parity` feature name is misleading.** Despite it,
`cli/vcs-adapters/tests/detection.rs` never shells out to any bash helper — it
spawns `git`/`jj` only to *build* fixtures. There is no VCS shell-parity harness
today; genuine differential suites live in `cli/corpus-adapters/tests/parity.rs`.

### 2. The shell behaviour to port — and four divergent copies of mode detection

`scripts/vcs-common.sh` (280 lines) is the reference. The load-bearing parts:

- `find_repo_root` (`:8-18`) — lexical ancestor walk for `.jj`/`.git`, using
  `-e`. Note it ignores any argument and always walks from `$PWD`.
- `vcs_mode` (`:27-36`) — `.jj`-wins command-set selector, using `-e`. Prints
  `jj|git|none`.
- `classify_checkout` (`:177-280`) — the six-line `KEY=VALUE` record; the
  first-match-wins cascade at `:240-272` with the documented requirement that
  `colocated` precede the `nested-*` arms.
- `find_jj_main_workspace_root` (`:90-114`), `find_git_main_worktree_root`
  (`:127-155`) — the probe layer, including `GIT_DIR` scrubbing at `:132-135`
  and the bare-repo rejection at `:137-139`.

**The hooks do not use `vcs_mode()`, and neither do the status/log scripts.**
This is a parity trap the story does not name — there are four independent
implementations of mode detection:

| Site | Test | Values produced |
| --- | --- | --- |
| `scripts/vcs-common.sh:29,31` | `-e` | `jj` / `git` / `none` |
| `hooks/vcs-detect.sh:28-29` | `-d` | `jj` / `jj-colocated` / `git` |
| `hooks/vcs-guard.sh:22,77` | `-d` | pure-jj / colocated |
| `scripts/vcs-status.sh:9`, `scripts/vcs-log.sh:9` | `-d` | jj-arm / git-arm |

Two consequences. `vcs-detect.sh` produces a **three-valued** mode that
`vcs_mode()`'s two-valued contract cannot express. And in a checkout where
`.git` is a *file* rather than a directory, `hooks/vcs-guard.sh:77` classifies a
colocated repo as `pure-jj` and **blocks** where it should warn. Porting
"`vcs-common.sh` semantics" is therefore insufficient for hook parity — the
hooks' own inline divergences are the real reference.

### 3. The hook envelope contract as 0167 actually shipped it

There is **no shared hook-envelope module**. The envelope is one hand-written
`format!` plus a private RFC-8259 `json_escape`, in a renderer for a single
subcommand:

- `hook_envelope` — `cli/launcher/src/config_command/render/summary.rs:66-74`
- `json_escape` — `:76-101` (private, not exported)
- byte-exact contract test — `:112-120`

It is **compact** (no pretty-printing), single-key, and deliberately not
`serde_json` — even though the launcher already depends on `serde_json`
(`cli/launcher/Cargo.toml:35`). The choice is architectural, per the module
docstring at `:1-5`.

The emptiness rule is a single choke point —
`cli/launcher/src/config_command/inbound/cli.rs:290`:

```rust
let stdout = match (summary_render::body(&summary), hook) {
    (None, _) => String::new(),
    (Some(text), false) => format!("{text}\n"),
    (Some(text), true) => format!("{}\n", summary_render::hook_envelope(&text)),
};
```

`--fail-safe` maps to `OnFailure::Degrade`
(`cli/launcher/src/launch/mod.rs:22-28`), and `summary` uses
`Degrade::Suppress` (`inbound/cli.rs:212-216`) so a degraded run emits nothing on
stdout — deliberately, because a `## Unavailable` notice would be spliced into
the model's prompt. The swallow/refuse split is decided by
`ConfigError::is_refusal()` (`cli/config/src/error.rs:76-87`), whose exhaustive
in-crate match is a pattern worth copying: a new variant will not compile until
it is classified.

**Where a `vcs` sibling cannot simply reuse it.** cargo-pup denies
`config_command` importing `crate::launch` (`cli/pup.ron:99-112`), and by
symmetry a `vcs_command` hexagon must not reach across into
`crate::config_command`. The `vcs` domain crate is restricted to
`std`/`kernel::Error`/`crate::` (`cli/pup.ron:75-89`). The neutral homes that fit
the existing rules are a new shared launcher module (e.g.
`crate::hooks::envelope`) or `kernel`.

**The envelope must be extended, not reused.** `vcs-detect.sh:177-181` merges an
optional `systemMessage` into the *same* jq object as `hookSpecificOutput`;
`render/summary.rs:68-74` has **no `systemMessage` slot**. The story acknowledges
the sibling key at `:205-207` but no AC covers the merge.

### 4. Hook protocol facts (from the Claude Code documentation)

- **`command` parsing.** Shell form (no `args` key) hands the whole string to
  `sh -c`: `${CLAUDE_PLUGIN_ROOT}` is expanded and arguments are tokenized, so
  `"${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs detect --format=hook"` works
  directly. Exec form adds an `args` array whose elements are passed as exact
  argv tokens with no shell. **This resolves the story's probe.** Caveat: the docs
  do not date the `args` field, and the plugin's floor is Claude Code v2.1.144 —
  shell form is the safer choice if `args` postdates that.
- **SessionStart output.** `{"hookSpecificOutput":{"hookEventName":"SessionStart",
  "additionalContext":…}}` is current. `systemMessage` is a **top-level** field
  and may be emitted alone. Empty stdout at exit 0 is a no-op. One JSON object
  per invocation.
- **PreToolUse output.** The documented shape is
  `{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":…,
  "permissionDecisionReason":…}}`, with `permissionDecision` ∈
  `deny|allow|ask|defer`. The top-level `{"decision":"block","reason":…}` form is
  **not documented for PreToolUse**. Exit 2 is a blocking error whose stderr
  becomes Claude's feedback (and where JSON is ignored).
- **Multiple hooks per event** run in parallel; all `additionalContext` is
  concatenated; for PreToolUse the most restrictive decision wins
  (`deny` > `defer` > `ask` > `allow`).

**Decision (author, 2026-07-30): emit the `permissionDecision` envelope, not the
deprecated shape.** The guard's mapping from today's shell behaviour:

| Case | `vcs-guard.sh` today | New envelope |
| --- | --- | --- |
| pure-jj + blocked git subcommand | `{"decision":"block","reason":…}` (`:97-100`) | `{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":…}}` |
| colocated + blocked git subcommand | `{"decision":"allow","hookSpecificOutput":{"systemMessage":…}}` (`:103-108`) | top-level `{"systemMessage":…}` with **no** `permissionDecision` — see below |
| no match | exit 0, no output (`:72-74`) | unchanged |

**The colocated case needs care: `permissionDecision: "allow"` is not neutral.**
Per the documentation it *skips the interactive permission prompt* (settings-level
deny rules still apply). The shell's intent there is "let it through but advise" —
a warning, not a grant of auto-approval. Emitting `"allow"` would therefore be a
privilege change smuggled in under a format migration. Emitting only a top-level
`systemMessage` preserves the intent exactly: normal permission flow proceeds and
the advice is surfaced. (`"ask"` would force a prompt where none is currently
forced, so it is also not parity.)

Two user-visible consequences to flag in the story:

- **The colocated warning will start appearing.** `vcs-guard.sh:103-108` nests
  `systemMessage` inside `hookSpecificOutput`, where the schema has no such
  field, so it is almost certainly discarded today. Moving it to the top level
  fixes a silent no-op — correct, but it is new output users have not seen.
- **This is a behavioural change, not a port**, so the AC must split: parity on
  *decisions* (which Bash-call classes block versus pass — the allow/deny fixture
  set the story already asks for at `:96-99`) plus a golden fixture pinning the
  *new* envelope. Byte-parity against the shell is explicitly not the target.

### 5. `hooks.json` has five hooks, not four

| Index | Script | Owner |
| --- | --- | --- |
| SessionStart[0] | `hooks/vcs-detect.sh` (`:9`) | **0169 removes** |
| SessionStart[1] | `hooks/config-detect.sh` (`:18`) | 0167 re-homed; 0169 may fold in |
| SessionStart[2] | `hooks/migrate-discoverability.sh` (`:27`) | deferred to 0172 |
| SessionStart[3] | `hooks/launcher-link-refresh.sh` (`:36`) | **no story owns it** |
| PreToolUse(Bash) | `hooks/vcs-guard.sh` (`:47`) | **0169 removes** |

`hooks/launcher-link-refresh.sh` arrived with 0182, after 0169 was written. It
must survive untouched. It also documents two constraints 0169 inherits, at
`hooks/launcher-link-refresh.sh:16-17`: *"at most one JSON object may reach
stdout, and two conditions can hold in one run"* — hence its accumulate-then-
emit-once `finish()` pattern (`:20-27`) — and it emits a bare `{systemMessage}`
with no `hookSpecificOutput`, a third envelope shape beyond the two the story
enumerates.

### 6. Test surface: the parity gate is only ~1/4 repointable

`hooks/test-vcs-detect.sh` (712 lines, 42 cases) is a hybrid:

- **~11 subprocess cases** (golden JSON `:190-252`, boundary blocks `:482-594`,
  missing-binary `:666-709`) — repointable by editing `HOOK=` at `:20` and
  dropping `"$BASH_BIN"` at `:161`, `:233`, `:705`.
- **27 in-process cases** (`:256-480`) — these `source scripts/vcs-common.sh` at
  `:254` and call `classify_checkout`, `find_repo_root`, `vcs_mode`,
  `_jj_workspace_is_secondary`, `find_jj_main_workspace_root`,
  `find_git_main_worktree_root` as shell functions. A binary cannot host these
  unless a `classify`-style subcommand emits the same `KEY=VALUE` record. Since
  `vcs-common.sh` survives this story, they stay meaningful and should be split
  into their own suite rather than repointed.
- **1 dead case**: `:642-664` greps the *shell file's* leading comment block for
  four phrases (0058's AC9). Nothing to grep once it is Rust.
- `:620-634` asserts the exact `hooks.json` command literal and hard-codes the
  positional index `SessionStart[0]`.

Fixtures: only two goldens (`hooks/test-fixtures/vcs-detect/main-jj-workspace.json`,
`main-git-checkout.json`), compared byte-for-byte at `:209`/`:221`, plus a
host-path-artefact guard at `:196-202`. `CAPTURE-SOURCE.txt` is written by
`regenerate.sh:49-60` but **nothing reads it**.

Crucially, the boundary-block assertions extract via
`jq -r '.hookSpecificOutput.additionalContext'` (`:486`, `:693`) — so config's
compact JSON versus `vcs-detect.sh`'s pretty `jq -n` output is **transparent** to
them. Only the byte-exact goldens at `:209`/`:221` would need re-baselining.

**`hooks/vcs-guard.sh` has zero test coverage** — no shell suite, no pytest, no
Rust test. Its only reference outside `meta/` is its registration. So the story's
allow/deny AC builds *new* coverage; there is no behaviour lock to check the
Rust guard against, which argues for capturing the shell's current decisions as
fixtures *before* rewriting.

### 7. CI and lint machinery that must move in lockstep

| Guard | Location | Effect |
| --- | --- | --- |
| Hooks suite floor | `tasks/test/integration.py:47` (`_EXPECTED_HOOKS_SUITES = 2`), enforced `:159` | Holds if the suite stays a `hooks/test-*.sh`; must drop to 1 if converted. Never to 0 — `tests/unit/tasks/test_integration.py:84-96` stops discriminating on an empty list. |
| Launcher-build edge | `mise.toml:209-211`; pinned by `tests/unit/tasks/test_mise.py:47-56`, `:96-109` | `test:integration:hooks` today has **no** `build:cli:dev` dep and passes **no** env. Driving a binary requires all three of: move to `_LAUNCHER_DEPENDENTS`, add `depends`, pass `accelerator_env()` at `tasks/test/integration.py:158`. |
| `CLAUDE_*` coupling | `tasks/lint/claude_coupling.py:12`, `:31`, `_MIN_SCANNED = 200` at `:43` | `cli/**` must never name a `CLAUDE_*` variable — the binary reads `ACCELERATOR_PLUGIN_ROOT`. The harness's `CLAUDE_PLUGIN_ROOT=` overlay at `test-vcs-detect.sh:161` must change. |
| Exec-bit invariant | `tasks/lint/scripts.py:97-145`; `SHELL_LIBRARIES` at `:18-49` | **No edit needed** — all three files being deleted are entrypoints, not libraries. `scripts/vcs-common.sh:28` stays. |
| Stale library entry | `tasks/lint/scripts.py:113-118` + `tests/unit/tasks/test_exec_bits.py:254` | Coupled pair; only relevant when `vcs-common.sh` eventually goes (0174). |
| Skill permissions | `tasks/lint/skill_permissions.py:41-44`, `:183-188` | A `Bash(...)` rule must name the subcommand; an ancestor glob would pre-authorise every future sub-binary. |
| Store duplication | `tasks/lint/store_duplication.py:38-60` | Any temp-plus-rename write under `cli/**/src` outside `cli/store/` fails unless allowlisted. |

Shell-suite discovery is glob-based (`tasks/test/helpers.py:45-78`), so deleting
a `test-*.sh` auto-drops it from the run — and the floor then fails, by design.

### 8. Sub-binary distribution: registration points and the warm-path cost

Dispatch: only `version` and `config` are built in
(`cli/launcher/src/launch/inbound/cli.rs:16-29`); everything else falls to
`#[command(external_subcommand)]` and is resolved by fetch-verify-cache. Adding a
built-in touches five exhaustive matches (`cli/launcher/src/main.rs:98-109`,
`:139-144`, `:182-189`, `:191-201`, and `launch/inbound/cli.rs:487`).

The **warm path makes no HTTP request at all** — not even for `manifest.json`
(`cli/launcher/src/launch/outbound/resolve/mod.rs:179-211`). But it does
re-verify: `reverify` (`:90-109`) re-runs minisign over the cached bytes because
the cache dir is user-writable, and `bin/accelerator:336-338` independently
re-verifies the cached launcher on every invocation. So a PreToolUse guard served
as a sub-binary pays, per Bash call: bash bootstrap + launcher minisign verify +
sub-binary minisign verify + 3 execs — against one bash spawn today.

Registration points for a new `accelerator-vcs`:

| # | Location | Note |
| --- | --- | --- |
| 1 | `tasks/shared/paths.py:25` | `DISPATCHED_SUBBINARIES = ("visualiser",)` — the single allowlist; drives signing, manifest, upload, re-verify |
| 2 | `tasks/manifest.py:51-53` | `_SUBBINARY_MANIFESTS` override — **required**, see the token collision below |
| 3 | new crate `Cargo.toml` | `package.description` mandatory (`tasks/manifest.py:60-66`); `version.workspace = true` (else `tasks/build.py:74-101` coherence) |
| 4 | `cli/Cargo.toml:4` | workspace members |
| 5 | `.gitignore:37` | `bin/visualiser-*` needs a `bin/vcs-*` sibling, else warm cache entries appear untracked in the shipped `bin/` |
| 6 | `cli/launcher/tests/fixtures/manifest.example.json` | shared golden contract, also read by `tests/unit/tasks/test_manifest.py:38` |
| 7 | `tasks/build.py:189-208` | `validate_dispatch_coherence` is hardcoded to the visualiser — the SKILL↔producer binding would be unenforced |
| 8 | `tasks/build.py:37` / `:290-331` | cross-compile staging; musl builds get `_assert_static_elf` (`:132-159`) |

Already parameterised, needing no edit: `tasks/signing.py:50-73`,
`tasks/github.py:218-235`, `:270-293`. Nothing in `.claude-plugin/plugin.json`
enumerates binaries. The SLSA globs at `.github/workflows/main.yml:423-425` already
cover `accelerator-*`.

**The token collision, and how to resolve it.** `tasks/manifest.py:56-57`
defaults a token to `cli/<token>/Cargo.toml` — for `vcs` that is the pure domain
crate (no `description`, no `[[bin]]`), so `_read_description` raises. And
cargo-pup's `vcs_domain_imports_only_permitted` matches `^vcs($|::)`, the whole
crate name, so a *binary* crate named `vcs` would be restricted to
`std`/`kernel::Error`/`crate::` and could not import `vcs_adapters`.

Both dissolve if the **package** is named `accelerator-vcs` while the
**dispatch token** stays `vcs`:

- pup matches the resolved module path, and `accelerator-vcs` resolves to
  `accelerator_vcs`, which `^vcs($|::)` does not match. No new pup rule is
  needed for the binary; mirror rule 6 (`cli/pup.ron:99-112`) with a `denied`
  list if you want to stop it reaching sideways into `crate::launch`.
- the directory cannot be `cli/vcs/` (taken by the domain crate), so it needs a
  sibling — the visualiser precedent is a nested path (`cli/visualiser/server/`,
  package `accelerator-visualiser`, token `visualiser`), but `cli/visualiser/` is
  not itself a crate whereas `cli/vcs/` is. A sibling directory
  (e.g. `cli/vcs-command/`) is the clean analogue.
- add the `_SUBBINARY_MANIFESTS` entry (`tasks/manifest.py:51-53`) mapping token
  `vcs` → that directory's `Cargo.toml`, which is exactly what that override map
  exists for. The user-facing `accelerator vcs detect` UX is unaffected.

`derive_override_var` then yields `ACCELERATOR_VCS_BIN`
(`cli/launcher/src/launch/core.rs:215-240`), which is legal.

Env-var derivation is fine: `vcs` → `ACCELERATOR_VCS_BIN`
(`cli/launcher/src/launch/core.rs:215-240`; a token containing `_` would be
rejected).

### 9. `gix` / `jj-lib` feasibility: measured, and it passes

Neither `gix`, any `gix-*`, `gitoxide`, `jj-lib`, `git2` nor `libgit2-sys`
appears in `cli/Cargo.lock` today — both trees would be new. There was **no
spike**; `meta/prs/24-description.md:47` records the choice as "the load-bearing
decision to confirm", and that PR was documentation-only.

**This research ran the checks empirically** — a scratch workspace with
`gix 0.85` + `jj-lib 0.43`, evaluated against a verbatim copy of `cli/deny.toml`:

| `cargo deny` check | Verdict |
| --- | --- |
| `bans` | **ok** |
| `advisories` | **ok** |
| `sources` | **ok** |
| `licenses` | one rejection — `uluru 3.1.0`, `MPL-2.0` |

Two assumptions behind the original risk assessment proved **stale**:

- **`jj-lib` 0.43 no longer depends on `git2`/`libgit2-sys`.** It has migrated to
  `gix`. That was the principal source of C linkage and of `openssl-sys`.
- **`gix`'s default features exclude the network transports.** Compression
  resolves to `zlib-rs` (pure Rust). No `openssl`, `openssl-sys`, `native-tls`,
  `curl`, `curl-sys`, `libz-sys` or `libssh2-sys` enters the graph, so the
  `cli/deny.toml:65-70` ban is satisfied with no feature surgery.

This holds because **every query 0169 needs is a local read** — classification,
status, log, guard, and even ahead/behind (remote-tracking refs live in
`refs/remotes`). No network transport is required at all.

**Static linking is not in tension with the TLS ban — they agree.**
`openssl-sys` is C and needs a system libssl or a vendored C build; `rustls` is
pure Rust. The ban's own rationale (`cli/deny.toml:61-64`) is to protect the
musl-static build. Verified end-to-end: a binary calling `gix::discover` and
jj-lib's workspace loader cross-compiles for `aarch64-unknown-linux-musl` via
`cargo zigbuild` and reports `ELF 64-bit LSB executable, ARM aarch64, statically
linked, stripped`; `tasks/build.py:132-159` `_assert_static_elf` accepts it
unmodified. The only `cc` build-dep in the graph arrives via
`iana-time-zone-haiku` (**Haiku OS**), absent from all four target triples.

**The one real finding — `uluru` is MPL-2.0.** It is `gix-pack`'s LRU object
cache, reached through both `gix` and `jj-lib`, so it cannot be feature-gated
away. `cli/deny.toml:35-40` legislates exactly this case ("must be justified
per-crate via `[[licenses.exceptions]]`, never added to this blanket allow").
Adding

```toml
[licenses]
exceptions = [
    { crate = "uluru", allow = ["MPL-2.0"] },
]
```

turns the run green (verified). MPL-2.0 is file-level copyleft, so consuming it
unmodified as a library imposes no obligation on surrounding code.

**The `jj-lib` unstable-API risk is also retired.** The story asks
(`:192-197`) to "confirm its workspace/repo-loading surface covers the
secondary-workspace and colocated cases before committing". It does, and
without `UserSettings`:

- `jj_lib::workspace::DefaultWorkspaceLoaderFactory` is public, and the
  `WorkspaceLoader` trait exposes `workspace_root()` and `repo_path()` with no
  settings argument (jj-lib 0.43 `src/workspace.rs:499-545`).
- Its private `DefaultWorkspaceLoader::new` (`src/workspace.rs:564-585`)
  implements **precisely** the shell's secondary-workspace rule — "If .jj/repo is
  a file, then we interpret its contents as a relative path to the actual repo
  directory", canonicalised — i.e. `_jj_workspace_is_secondary`
  (`scripts/vcs-common.sh:74-81`) and `find_jj_main_workspace_root` (`:90-114`)
  in one library call.
- Probed against four real fixtures (colocated main, secondary workspace, plain
  git, nested subdir) plus this repo's own `workspaces/build-system` checkout,
  the library answers **match `classify_checkout` exactly**: `workspace_root()`
  equals `BOUNDARY`, and `repo_path()` minus `/.jj/repo` equals `JJ_PARENT`.

Two caveats found while probing, both worth encoding in the adapter:

- **`gix::discover` walks up *past* a jj workspace boundary.** From inside
  `workspaces/build-system` it returned the parent repo's
  `/Users/.../accelerator/.git`. Boundary containment is the adapter's job (the
  reason `classify_checkout` exists); bound discovery with a ceiling rather than
  trusting `discover` to stop.
- **`UserSettings` is a trap for a library consumer.** Full `Workspace::load`
  requires it, and jj-lib's own defaults (`src/config/misc.toml`) are behind a
  private `DEFAULT_CONFIG_LAYERS` static (`src/config.rs:910-913`) — so a caller
  must supply every required key by hand, discovered one panic at a time
  (`user.name` → `operation.hostname` → `operation.username` →
  `debug.randomness-seed` → `signing.behavior` → …). **Avoid `Workspace::load`
  for detection**; the loader-only path needs none of it.

Two secondary notes:

- **Pin `gix` to match `jj-lib`'s** (0.85 at the time of writing). Declaring
  `gix = "0.86"` alongside `jj-lib` yields two complete `gix` trees — only a
  warning (`multiple-versions = "warn"`, `cli/deny.toml:57`), but it doubles the
  closure.
- `unknown-git = "deny"` (`:74`) matters only if jj-lib's unstable API ever
  forces pinning an unreleased commit. Both crates are on crates.io today and
  `sources` passes.

Also: `tasks/lint/cli.py:15` runs clippy with `--locked`, so `cli/Cargo.lock`
must be committed in the same change as the new dependencies.

One genuine upside the story does not claim: going in-process **dissolves 0125's
stated reason** for keeping the lexical fallback — that the helpers must work
with no `git`/`jj` on `PATH`, and that probing spawns 1-3 subprocesses per call.
Neither constraint survives a library-backed adapter.


### 10. Patterns to build on

- **Hexagon template.** `cli/config/` + `cli/config-adapters/` is the grown
  version; the shape is domain (`lib.rs` re-export face, `error.rs`,
  `service.rs`) + adapters (`compose.rs` as the one tested wiring helper,
  `store.rs`, private `mod`s with a curated `pub use`). Error convention: one
  `#[non_exhaustive]` enum per domain, hand-written `Display`, `impl Error`,
  `From<X> for kernel::Error` → `Failed(...)` (`cli/config/src/error.rs:36-147`).
- **Domain import style.** One item per `use`, `crate::`-qualified, never
  braces — cargo-pup resolves grouped `use foo::{a,b}` to an empty module name
  which matches no `allowed_only` pattern (`cli/pup.ron:93-96`). `#[cfg(test)]`
  modules are exempt in practice.
- **"Must never fire" assertions.** `cli/launcher/tests/crypto_provider.rs:1-56`
  is the template: a `Cell<bool>` spy port *plus* a panicking null (`PanicExec`).
  For exact counts, `AtomicUsize` as at `cli/corpus-adapters/src/lock.rs:211-225`.
- **Zero-fetch assertions already exist.**
  `cli/launcher/tests/resolution.rs:451-466` points a second resolver at
  `http://127.0.0.1:1` and proves the warm path is offline;
  `:214-224` asserts `hits(asset) == 1`. Per-path counters:
  `cli/launcher/tests/common/mod.rs:82-89`.
- **Golden output.** No `insta` anywhere. Checked-in files under
  `tests/fixtures/<case>/*.golden`, read by a `golden()` helper and compared as
  `Vec<u8>` against `output.stdout` (`cli/launcher/tests/config_read.rs:74-135`,
  `:490-497`).
- **CLI end-to-end.** `env!("CARGO_BIN_EXE_accelerator")` +
  `std::process::Command`, with a `run_in` helper that scrubs `ACCELERATOR_*`
  (`cli/launcher/tests/config_read.rs:59-72`). For a library crate with no bin,
  add a test-only fixture bin under `tests/fixtures/`
  (`cli/config-adapters/tests/fixtures/config_adapters_fixture.rs`).
- **Fixture repos.** `cli/vcs-adapters/tests/detection.rs:21-80` — `require()`
  hard-fails on an absent binary (Rust has no skip primitive, so an early return
  would register PASS), labelled `tempfile::Builder` prefixes, git identity passed
  per-invocation. Existing shapes: clean git, no-commits, bare, worktree-as-file,
  colocated (`:144-169`), jj secondary workspace (`:171-209`).
  **Missing and needed by AC2: dirty, detached-HEAD, ahead/behind.** Consider
  adopting `GIT_CEILING_DIRECTORIES` from `hooks/test-vcs-detect.sh:35-40` so
  `gix`'s own discovery cannot escape the fixture.

### 11. What survives this story

After deleting the two hooks, five `vcs-common.sh` functions lose **every**
production caller — `classify_checkout` (whose only call site anywhere is
`hooks/vcs-detect.sh:122`), `find_jj_main_workspace_root`,
`find_git_main_worktree_root`, `_jj_workspace_is_secondary`,
`_ancestor_has_marker`. Only `hooks/test-vcs-detect.sh` would reference them.

But the lexical helpers survive with a wide surface:

- `find_repo_root` — 20+ call sites, notably `scripts/config-common.sh:18`
  (the transitive hub), `skills/work/scripts/*`,
  `skills/integrations/{jira,linear}/scripts/*`,
  `skills/decisions/scripts/adr-next-number.sh:37`,
  `skills/design/inventory-design/scripts/playwright/run.sh:71`
- `vcs_mode` — exactly one: `skills/work/scripts/work-item-file-dirty.sh:45`

So `scripts/vcs-common.sh` cannot be deleted here, and the story is right not to
try. **After 0169 there will be three implementations, not two**: the Rust
classifier, the surviving shell probe layer, and the surviving shell lexical
walk.

The coupling that makes the SessionStart port load-bearing:
`skills/vcs/commit/SKILL.md:43-44,54-55` instructs Claude to "refer to the
session's VCS context" — text injected by the hook being replaced.

Two latent bugs a faithful port would preserve:

- `skills/planning/validate-plan/SKILL.md:56-60` instructs raw `git log` /
  `git diff`; both are in the guard's blocked pattern
  (`hooks/vcs-guard.sh:53`), so that skill is presumably blocked in pure-jj repos.
- `scripts/lint-bashisms.sh:31` still enumerates via `git ls-files` — the
  jj-workspace blindspot that `shell_sources()` was deliberately moved off.

### 12. Measured hook latency, and the bootstrap fix this story must land

The story carries per-call latency as an unverified assumption (`:161-164`).
Measured on darwin-arm64, warm cache, 20 iterations each:

| Path | ms/call |
| --- | --- |
| `hooks/vcs-guard.sh` (today's shell guard) | **35.1** |
| `bin/accelerator version` (bootstrap + launcher, warm) | **149.1** |
| launcher binary invoked directly, no bootstrap | **3.0** |
| minisign verify of the 8 MB launcher | **2.3** |
| `probe_dir` write + chmod + exec + rm | **107.9** |
| re-exec of a *pre-existing* probe file | **10.6** |
| one `sha256_file` of the 475 K verify shim | **11.7** |

Everything the fetch-verify-cache design is usually blamed for — minisign, the
launcher, dispatch — totals under 6 ms. The 149 ms decomposes almost entirely
into two avoidable costs:

| Warm-path step | Cost | Avoidable? |
| --- | --- | --- |
| `probe_dir` write+chmod+exec+rm (`bin/accelerator:166-180`, via `resolve_cache_dir:195`) | ~108 ms | **yes** |
| `sha256_file` ×2 over the 475 K shim (`:252`, `:256`) | ~23 ms | **yes** |
| `verify_launcher` — shim exec + minisign (`:310-312`, `:336`) | ~2.3 ms | no (trust posture) |
| `exec "${launcher}"` (`:352`) | ~3 ms | no |
| bash startup, `uname` ×2, `plugin.json` `sed` | ~12 ms | partly |

That is **~131 ms of 149 ms addressable**, which would put the bootstrap at
roughly 18 ms — *below* today's 35 ms shell guard rather than 4× above it.

**Decision (author, 2026-07-30): the `probe_dir` fix lands as part of this
story**, not as a follow-up. The guard runs on every Bash tool call, so the
story cannot honestly claim warm-cache parity with `vcs-guard.sh` while the
bootstrap costs 4× the thing it replaces.

Why the two costs are safe to remove:

- **`probe_dir` is redundant on the warm path.** Its purpose is to catch a
  `noexec` mount that a write-only check would miss. But the warm path then
  execs the shim from `cache_dir` (`verify_launcher`, `:311`) and execs the
  launcher from `cache_dir` (`:352`) — both are *stronger* exec tests than a
  synthetic probe, and a `noexec` dir makes `verify_launcher` fail, which falls
  through to the cold branch where the probe still belongs. Suggested shape:
  split `probe_dir` into `ensure_dir` (the `mkdir -p`, always) and the
  write+chmod+exec probe, and call the latter only before fetching. Note the
  probe's result is not used to *choose* a directory — `resolve_cache_dir`
  (`:184-193`) has no fallback — so moving it changes only where the diagnostic
  fires, not which path is selected.
- **The shim staging is a same-directory copy in the default configuration.**
  `shim_source` is `${plugin_root}/bin/accelerator-verify-${platform}` (`:158`)
  and `cache_dir` defaults to `${plugin_root}/bin` (`:190`) — so the warm path
  hashes 475 K twice to content-address a copy of a file into its own directory.
  The "plugin root may be noexec" rationale (`:246-247`) only bites when
  `ACCELERATOR_CACHE_DIR` points elsewhere. Cheapest correct fix: skip staging
  entirely when `shim_source`'s directory and `cache_dir` resolve to the same
  path, and keep the existing digest dance only when they differ.
  **Open sub-question for planning:** the second hash (`:256`) also defends
  against a planted stub being trusted by name. Running the shim from
  `shim_source` drops that check — though an attacker able to plant a stub there
  already owns the root of trust, and `bin/accelerator-verify.vendored.sha256`
  pins the vendored digest. Confirm the intended trust boundary before removing
  it; if in doubt, fix `probe_dir` alone (the ~108 ms) and leave the ~23 ms.

Two further consequences:

- **Built-in versus sub-binary is worth single-digit ms** (one extra in-process
  minisign verify plus an exec). It was never a latency decision. The author
  chose the sub-binary (2026-07-30); the parent research's "lean built-in" steer
  (`2026-06-28-0136-...:613-615`) was reasoning from an assumed per-call fetch
  cost the warm path does not actually pay.
- The magnitude is **macOS-specific** — the ~97 ms delta between executing a
  freshly written file and re-executing an existing one is macOS's first-exec
  check. Linux has no equivalent, so quote darwin as the worst case. Both are
  shipped platforms.

Suggested acceptance criteria to add to the story:

- The warm-cache bootstrap performs no write-and-exec probe: verified by
  asserting that a warm `bin/accelerator` invocation creates no
  `.accelerator-probe-*` path (e.g. a probe-name guard, or by asserting the
  cold path still does).
- Warm-path per-call latency is bounded — e.g. a warm `accelerator vcs guard`
  costs no more than today's `hooks/vcs-guard.sh` on the same host. This
  replaces the story's current unverified assumption at `:161-164`.
- The `noexec` diagnostic is preserved on the cold path: a cache dir that
  cannot execute still fails closed with the existing message, covered by its
  own test.
- The bootstrap remains bash-3.2 clean (`scripts/lint-bashisms.sh`), which the
  story already requires at `:124-125`.

Note this also speeds up `hooks/config-detect.sh` at every SessionStart, and
every future CLI-backed hook, for free.


## Code References

- `scripts/vcs-common.sh:27-36` — `vcs_mode`, the `.jj`-wins command-set selector
- `scripts/vcs-common.sh:177-280` — `classify_checkout`; cascade at `:240-272`
- `hooks/vcs-detect.sh:28-36` — inline three-valued mode detection (not `vcs_mode`)
- `hooks/vcs-detect.sh:177-181` — SessionStart envelope + merged `systemMessage`
- `hooks/vcs-guard.sh:53` — the blocked git-subcommand pattern
- `hooks/vcs-guard.sh:97-108` — the two guard envelopes (legacy shape)
- `hooks/hooks.json:9,18,27,36,47` — all five registrations
- `hooks/launcher-link-refresh.sh:16-27` — one-JSON-object constraint, `finish()`
- `hooks/test-vcs-detect.sh:20` — the hard-coded `HOOK` constant
- `hooks/test-vcs-detect.sh:254-480` — the 27 in-process library cases
- `cli/vcs/src/lib.rs:46-67` — the two outbound ports
- `cli/vcs-adapters/src/lib.rs:110-125` — the two subprocess arg-sets
- `cli/vcs-adapters/src/lib.rs:168` — the single `spawn()` chokepoint
- `cli/launcher/src/config_command/render/summary.rs:66-101` — the hook envelope
- `cli/launcher/src/config_command/inbound/cli.rs:290` — the emptiness rule
- `cli/launcher/src/launch/inbound/cli.rs:16-29` — built-ins vs external dispatch
- `cli/launcher/src/launch/outbound/resolve/mod.rs:179-211` — warm path + reverify
- `bin/accelerator:336-338` — warm launcher path, still minisign-verifies
- `cli/pup.ron:75-89` — `vcs_domain_imports_only_permitted`
- `cli/pup.ron:93-98` — the grouped-import gotcha
- `cli/deny.toml:41-70` — licence allow-list and the TLS bans
- `tasks/shared/paths.py:25` — `DISPATCHED_SUBBINARIES`
- `tasks/manifest.py:56-66` — default manifest path + mandatory description
- `tasks/test/integration.py:43-47,150-159` — the hooks floor and task body
- `tests/unit/tasks/test_mise.py:47-56` — `_NO_LAUNCHER_NEEDED` pin
- `tasks/lint/claude_coupling.py:12,31,43` — the `CLAUDE_*` prohibition on `cli/**`
- `cli/launcher/tests/crypto_provider.rs:1-56` — spy port + panicking null
- `cli/launcher/tests/resolution.rs:451-466` — offline warm-path proof

## Architecture Insights

- **The hexagon boundary is enforced mechanically, and it shapes the design.**
  Because `cli/pup.ron:75-89` restricts the `vcs` domain to
  `std`/`kernel::Error`/`crate::`, `gix` and `jj-lib` can only ever live in
  `vcs-adapters`. That is the right shape, but it means the full
  `classify_checkout` taxonomy must be expressible as plain domain values with no
  library types leaking in — the port signatures are the design work.
- **`--format=hook` is a rendering concern collapsed at the boundary.** 0167's
  precedent parses a `ValueEnum` then immediately flattens it to a `bool`
  (`cli/launcher/src/launch/mod.rs:112-117`), so the hexagon never sees a
  "format" concept. There is no shared `Format` type — `SummaryFormat` and
  `PathsFormat` are per-subcommand, and `PathsFormat` is parsed and ignored.
- **Envelope-per-hook-type is correct, and the story already knows it.** There is
  no single envelope spanning hooks: SessionStart carries `additionalContext`,
  PreToolUse carries a permission decision, and `launcher-link-refresh` emits a
  bare `systemMessage`. What the story does not yet reflect is that the *current*
  documented PreToolUse shape differs from the shell's.
- **Parity is the wrong frame for the guard's envelope.** Parity on *decisions*
  (which commands block, which pass) is worth locking. Parity on *bytes* would
  freeze both a deprecated schema and a probably-ineffective `systemMessage`
  placement. These should be separate criteria.
- **Fail-safe diagnostics currently go nowhere.** `--fail-safe` writes
  `eprintln!("{error}")` then exits 0
  (`cli/launcher/src/config_command/inbound/cli.rs:464`), and
  `render/mod.rs:43-48` writes *every* warning to stderr even on success. Per
  0183's analysis, stderr at exit 0 is discarded by Claude Code. Any new
  hook-mode command inherits this.
- **Fetch-verify-cache is trust-first at every step**, which is why it is not
  free: manifest signature verified before parsing (`resolve/mod.rs:130`), exact
  version equality as anti-rollback (`manifest.rs:103-108`), cached entries
  re-verified on every warm hit. Latency on a hot path is the cost of that
  posture, not an implementation defect to optimise away.

## Historical Context

- `meta/decisions/ADR-0048-four-toolchain-split.md:99-101` — **the story's
  citation is correct** despite the ADR's title: *"Hooks are part of this: the
  hook logic is implemented in the CLI, fronted by a thin shell shim only if the
  hook entry point demands a shell command."* Corroborated by
  `ADR-0049-bash-3.2-compatibility-floor.md:23`. There is no separate hooks ADR.
- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md:606-615`
  — Open Question 4 resolved the invocation mechanism, and on built-in vs
  sub-binary said *"lean built-in to avoid a per-Bash-call sub-binary fetch"*.
  0169 closed it the other way (`:82-85`, `:252-254`). Deliberate, but the
  research gives the opposite answer to anyone who reads it.
- `.../2026-06-28-...:641-646` — Open Question 8 pinned `hooks/` at 7 tracked
  `.sh` (4 prod). It is now 8 (5 prod), after 0182.
- `meta/reviews/work/0169-...-review-1.md` — five lenses, REVISE → APPROVE. All
  twelve pass-1 findings dispositioned; finding #6 is the one that produced the
  golden-envelope AC, and its fix is what this research now shows needs
  re-examining. Left open deliberately: the bundling scope point (`:420-423`).
- `meta/work/0125-converge-vcs-detection-on-probe-layer.md` — **open `draft`
  debt, not subsumed by 0169.** Two binding constraints it states are *not*
  restated in 0169: `find_repo_root` ("where am I") must stay distinct from
  `find_git_main_worktree_root` (canonical main) — `:96-101`; and `vcs_mode` is a
  command-set selector, so any delegation to `classify_checkout` must map the
  topology verdict *back* to a jj/git selector — `:102-105`. It also asks for a
  cross-strategy characterisation test (`:131-133`) that 0169 has no AC for.
- `meta/work/0183-session-start-hook-advisories-reach-nobody-on-stderr.md:49-55`
  — the one-JSON-object and stdout-is-context constraints; its audit AC
  (`:83-84`) requires every SessionStart hook to be converted or cleared.
- `meta/work/0174-retire-shell-tooling-and-ci-guards.md:44,59,88` — removes
  *tooling, not scripts*, and never names `vcs-common.sh` or any hook script. It
  expects the bootstrap, hook wrapper and Playwright executor to survive.
- `meta/work/0058-workspace-worktree-boundary-detection.md` and
  `meta/work/0124-find-repo-root-fails-in-git-worktrees.md` — the origin of
  `classify_checkout` and of the `-d`→`-e` fix in `find_repo_root`. Both are
  still recorded `draft` although 0124's fix has shipped.
- `meta/prs/24-description.md:29,47` — the `gix`/`jj-lib` refinement record,
  flagging `jj-lib`'s unstable API for early validation. Documentation-only;
  nothing was validated.

## Related Research

- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
  — the epic's scope and architecture; Phase 6 is this story
- `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`
  — the original shell inventory sweep
- `meta/research/codebase/2026-07-19-0167-config-command-and-invocation-contract-migration.md`
  — the `--format=hook` contract as designed
- `meta/research/codebase/2026-07-03-0164-launcher-and-git-style-dispatch.md`
  — dispatch and fetch-verify-cache
- `meta/research/codebase/2026-07-11-0179-corpus-crates-parsing-conventions.md`
  — the crate-pair template this subdomain mirrors
- `meta/research/codebase/2026-05-15-0058-workspace-worktree-boundary-detection.md`
  — the `classify_checkout` design being ported

## Stale Claims in the Work Item

Recorded so the plan can correct them rather than inherit them.

1. **`:49-50` "four bare `.sh` paths in `hooks.json`"** — there are five, since
   0182 added `launcher-link-refresh.sh`.
2. **`:151-152` and `:250-251` "removes three of the four hook scripts"** — it
   removes **two of five** (three if `config-detect.sh` is folded in). The review
   praised this invariant as consistently stated across five sections; it is now
   inaccurate in all five.
3. **`:146-147` and `:137-139` still say `config detect` is in scope** and that
   0167 delivers a command "this story extends with `config detect`", contradicting
   the Summary at `:36-40`. No `config detect` exists; the shipped surface is
   `config summary --format=hook --fail-safe`. The config half is out of scope
   except for the optional fold-in.
4. **`:143-149` dependency statuses** — 0164, 0166, 0179 are `done`; 0167's code
   has landed (verified: `cli/config/`, `cli/launcher/`, `bin/accelerator`,
   `hooks/config-detect.sh` all exist at `main`) though its work item is still
   `ready`. The remaining blocker is bookkeeping, not work.
5. **`:112-119` the PreToolUse golden envelope** — pins a shape the current docs
   do not describe for PreToolUse, and the shell it copies nests `systemMessage`
   in a position the schema has no field for. **Superseded by the 2026-07-30
   decision**: the AC should now require the `permissionDecision` envelope and
   split decision-parity from envelope shape (§4).
6. **`:228-229` "must slot into the same `additionalContext` shape"** — the 0167
   renderer has no `systemMessage` slot, so the envelope must be *extended*. The
   story requires the sibling key at `:205-207` but no AC covers the merge.
7. **`:238-243` the argument-splitting probe** — resolvable from documentation;
   both shell form and an `args` exec form work.
8. **`:82-85` "as the `accelerator-vcs` sub-binary"** — the token collides with
   the domain crate under both `tasks/manifest.py` and `cli/pup.ron`, and the
   parent research leaned built-in.
9. **`:168-171` Technical Notes source list omits `scripts/vcs-status.sh` and
   `scripts/vcs-log.sh`** — the actual implementations behind AC2's `vcs status`
   / `vcs log` parity claim, which cites `vcs-common.sh` instead.
10. **`:89-90` "the repointed `hooks/test-vcs-detect.sh` parity gate"** — only
    ~11 of 42 cases are repointable; 27 test the surviving shell library and 1 is
    unportable.
11. **`:168-171` Technical Notes must now also name `bin/accelerator`** — the
    `probe_dir` fix (§12) is in scope as of 2026-07-30, so the bootstrap is a
    modified file, not merely the invocation mechanism. It is already covered by
    the story's bash-3.2 AC (`:124-125`), which refers to "the wrapper"; that
    wording should be pinned to `bin/accelerator` explicitly (see stale claim 6
    on the wrapper/bootstrap ambiguity).

## Open Questions

1. ~~**Built-in or sub-binary for the guard?**~~ **DECIDED 2026-07-30: sub-binary**
   (author). Measurement (§12) supports it being a low-cost choice — the
   built-in/sub-binary delta is single-digit ms, while ~108 ms of the 149 ms
   bootstrap+launcher path is `probe_dir`'s write-and-exec, paid either way.
   Consequences to carry into the plan: name the **package** `accelerator-vcs` in
   a sibling directory (not `cli/vcs/`) with a `_SUBBINARY_MANIFESTS` entry
   keeping the token `vcs` (§8), and register the new token in
   `DISPATCHED_SUBBINARIES`, `.gitignore`, and the manifest fixture.
   The story's latency *assumption* (`:161-164`) can now be replaced with the
   measured figure rather than left unverified.
2. ~~**Does `jj-lib` cover the secondary-workspace and colocated cases, and do
   `gix` + `jj-lib` pass `cargo deny`?**~~ **RESOLVED 2026-07-30** by the probe
   in §9: yes on both counts, with a single `[[licenses.exceptions]]` entry for
   `uluru` (MPL-2.0) and a `gix` version pinned to match `jj-lib`'s. Static musl
   linking confirmed against `_assert_static_elf`. The per-query shell fallback
   is therefore an escape hatch, not the main path. Residual follow-ups: bound
   `gix::discover` so it cannot escape a workspace boundary, and avoid
   `Workspace::load` (and thus `UserSettings`) on the detection path.
3. ~~**What is the PreToolUse envelope's target shape?**~~ **DECIDED 2026-07-30:
   the `permissionDecision` envelope** (author). Mapping and rationale in §4. The
   AC splits into decision-parity plus a golden fixture on the new shape. Residual
   for planning: the colocated branch must emit a bare top-level `systemMessage`
   (no `permissionDecision`), because `"allow"` would skip the permission prompt
   and quietly widen privilege; and the story should note that the colocated
   warning becomes visible for the first time.
4. **Is the `args` exec form available at the plugin's Claude Code floor
   (v2.1.144)?** The docs do not date it. If not, shell form is the answer.
5. **Where should the 27 in-process library cases live?** Split into a new
   `scripts/test-vcs-common.sh` (keeping the hooks floor at 2 without a repointed
   suite), or re-home as Rust tests? This interacts with the floor decision at
   `tasks/test/integration.py:47`.
6. **Who owns `scripts/vcs-status.sh` / `scripts/vcs-log.sh` and
   `skills/vcs/commit`?** No epic-0136 story claims `skills/vcs/**`. Unless 0169
   also rewrites that skill's two `!`-preprocessor call sites, `vcs status` and
   `vcs log` ship with zero consumers and two shell scripts survive unowned.
7. **Does 0169 note or close 0125?** It currently does neither, and adds a third
   detection implementation. At minimum the plan should state that 0125 stays
   open and that its lexical-fallback rationale is dissolved by the in-process
   adapter.
8. **Does 0172 know it inherits this story's `hooks.json`?** The hand-off is
   asserted only in 0169; 0172 has no `blocked_by: 0169` and its source list omits
   `hooks/migrate-discoverability.sh` entirely.
9. **Removing the shim's double hash — what is the intended trust boundary?**
   The `probe_dir` fix is settled and in scope (§12). The adjacent ~23 ms of
   shim hashing is also removable, but the second hash (`bin/accelerator:256`)
   defends against a planted stub being trusted by name. Decide during planning
   whether running the shim from `shim_source` is acceptable given that
   `bin/accelerator-verify.vendored.sha256` already pins the vendored digest; if
   not, take the ~108 ms and leave the ~23 ms.

10. **Who owns `hooks/launcher-link-refresh.sh`?** No story does. It must survive
   0169 and 0174, so someone should say so explicitly.
