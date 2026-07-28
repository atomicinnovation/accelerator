---
type: codebase-research
id: "2026-07-27-0182-plugin-root-self-location-implementation-surface"
title: "Research: Implementation surface for 0182 — bootstrap self-location and the ACCELERATOR_PLUGIN_ROOT rename"
date: "2026-07-26T23:35:47+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0182"
parent: "work-item:0182"
relates_to: ["codebase-research:2026-07-19-0167-config-command-and-invocation-contract-migration"]
topic: "Implementation surface for 0182: verifying the rename set, the bootstrap's gate/argv structure, the R4 seam mechanism, the lint-guard and hook patterns, and the build-order dependency"
tags: [research, codebase, cli, launcher, bootstrap, plugin-root, hooks, lint-guards, build-system]
revision: "9f21238da828e777af109b37389e1c523e1c688f"
repository: "accelerator"
last_updated: "2026-07-26T23:35:47+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Implementation surface for 0182 — bootstrap self-location and the `ACCELERATOR_PLUGIN_ROOT` rename

**Date**: 2026-07-26 23:35 UTC
**Author**: Toby Clemson
**Git Commit**: `9f21238da828e777af109b37389e1c523e1c688f`
**Branch**: (jj working copy, no bookmark)
**Repository**: accelerator

## Research Question

Research the codebase for the bug at
`meta/work/0182-cli-derives-plugin-root-from-own-location.md` — verify the work
item's enumerated rename set, gate counts, and stated mechanisms against the
live tree, and establish the patterns each of R1–R12 must follow.

## Summary

The work item is **substantially accurate on inventory and wrong on three
mechanisms**. Every file:line in its rename table exists as claimed, the
`cli/` set is provably complete (21 occurrences, all `CLAUDE_PLUGIN_ROOT`; no
other `CLAUDE_*` variable exists anywhere under `cli/`), the name
`ACCELERATOR_PLUGIN_ROOT` is unused across the repo, and every pattern R7/R9/R10
needs — grep-guard lint task, gitignore-honouring walk, hook harness,
`build:cli:dev` prerequisite — already exists with a close precedent to copy.

Four findings change the plan, three of them established by direct measurement
against a freshly built launcher:

1. **Only `config template <name>` is root-sensitive.** Measured: `config
   agents`, `paths`, `path <k>`, `instructions <skill>`, `context --skill`,
   `work <k>` and `review <k>` produce **byte-identical output with and without
   a plugin root** — they read project config, not `<root>/templates/`. This
   narrows the *silent-degradation* hazard to the template family and makes two
   acceptance criteria unsatisfiable as written (below).
2. **Acceptance criterion 1 cannot pass.** It asserts `config instructions
   commit --fail-safe` emits "a known substring of the shipped commit
   instructions". There are no shipped commit instructions — `config
   instructions` reads *user* per-skill overrides. Measured: 0 bytes, exit 0,
   both rootless and rooted.
3. **R4's stated mechanism is wrong twice over, and the breakage is caused by
   R3, not R1.** Under `mise run` the seam never reaches the bootstrap
   (`ACCELERATOR_BIN` is set), and a rootless launcher does **not** exit 0 for
   `config template work-item` — it exits **1** fail-closed. So the seam
   survives R1 untouched and breaks only when R3 renames the launcher's reader
   alongside `tasks/test/helpers.py`. R4's prescribed fix remains correct; its
   justification and its sequencing claim do not.
4. **`bin/accelerator` has 16 `fail()` gates, not 14**, and the automated
   `!`-extraction suite is far simpler than scoped: all **204** extracted
   commands are static literals with **zero** argument interpolation, so that
   branch of the criterion is dead — but ~80 of them legitimately emit empty
   stdout, and which ones depends on the *project's* config, so the harness
   needs a fixture project rather than this repo.

`mise.local.toml` is **tracked** (staged `A`), so R6 is a real deletion, and it
is the confirmed source of the local masking: this session's Bash tool shows 9
`CLAUDE_*` variables where a clean environment shows 8, the extra one carrying
`mise.local.toml`'s tell-tale trailing slash.

## Detailed Findings

### 1. The bootstrap: gates, argv, and symlink prior art

`bin/accelerator` (263 lines) reads `CLAUDE_PLUGIN_ROOT` at **13 code sites**
(`:25`, `:26`, `:27`, `:44`, `:70`, `:73`, `:101`, `:107`, `:117`, `:122`,
`:135`, plus comments at `:93`, `:260`). It never `export`s it — the launcher
inherits it only because it was already in the bootstrap's own environment.

**`fail()` is the single abort funnel** (`bin/accelerator:20-23`) — there is no
other `exit 1` in the file — with **16** call sites, not the 14 the work item
states (R8, Technical Notes):

| # | Line | Gate | # | Line | Gate |
|---|---|---|---|---|---|
| 1 | 25 | root unset | 9 | 74 | public key missing |
| 2 | 27 | root not a directory | 10 | 107 | no usable cache dir |
| 3 | 35 | unsupported architecture | 11 | 134 | dev launcher refused |
| 4 | 40 | unsupported OS | 12 | 163 | could not hash shim |
| 5 | 45 | plugin.json not found | 13 | 168 | could not stage shim |
| 6 | 49 | version unreadable | 14 | 170 | could not chmod shim |
| 7 | 67 | no curl/wget | 15 | 208 | lock timeout |
| 8 | 72 | verify shim missing | 16 | 254 | fetch-and-verify failed |

Gates 1–2 are the ones R1 deletes, leaving 14 — which is likely where the
work item's number came from. R8 should say 16 (or 14 post-R1).

**argv is completely untouched before `exec`.** `"$@"` appears only at `:142`
(dev-override exec) and `:262` (verified exec); there is no `shift`, no `$#`
test, no `case "$1"`. Every other `$1`/`$2` is a function parameter. So the
R8 argv scan can sit anywhere in lines 19–24 — after `set -uo pipefail` (`:18`)
and before gate 1 — and a flag consumed inside `fail()` makes **all 16** gates
fail-safe-aware with no per-site edits. Under `set -u` with bash 3.2 the
belt-and-braces iteration form is `for arg in ${1+"$@"}`.

**Symlink chase — no prior art in the tracked tree.** `while [ -L` matches only
`meta/` documents; every tracked `-L` use is a one-shot *rejection*
(`bin/accelerator:121`, the Jira/Linear attach and transition flows), and the
only `readlink` uses are the GNU-only best-effort
`readlink -f "$path" 2>/dev/null || true` (e.g.
`skills/integrations/jira/scripts/jira-attach-flow.sh:124`), documented as a BSD
no-op. The repo-wide idiom is directory-only:
`SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"`, ~60 sites.

A **reviewed, previously-shipped implementation** of exactly the required chase
survives outside the tracked tree:

- Spec with rationale — `meta/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding.md:620-655`:
  `while [ -L "$SELF" ]` over `BASH_SOURCE[0]`, `readlink` **without** `-f`,
  `case "$LINK_TARGET" in /*)` to split absolute from relative targets, relative
  resolution via `"$(cd "$(dirname "$SELF")" && pwd -P)/$LINK_TARGET"`, a hop
  counter, then `cd -P`.
- As-shipped — `workspaces/visualisation-system/skills/visualisation/visualise/cli/accelerator-visualiser:17-32`
  (a jj workspace checkout; reference only).
- Its **review history is the most valuable input** —
  `meta/reviews/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding-review-1.md`:
  `:392` (use `while [ -L`, no `readlink -f`), `:738` (why a `readlink -f`
  capability probe and a `perl` fallback were both rejected — pure bash +
  `pwd -P` is identical on BSD and GNU), `:757` (**cycle detection is
  mandatory** or `ln -sf a b; ln -sf b a` hangs), `:856` (40 is Linux's
  `SYMLOOP_MAX`; Darwin's is 32 — lower it or state the over-approximation),
  and `:835` (**a cyclic chain cannot be exercised via direct exec** —
  `execve()` returns `ELOOP` first — so the test must invoke via
  `bash "$CYCLE_A"` to make the in-script counter fire).

`scripts/lint-bashisms.sh:32-33` explicitly appends the extensionless
`bin/accelerator` to its scan (the `*.sh` glob misses it), and that inclusion is
pinned by `tests/unit/tasks/test_bootstrap_coverage.py:25-27`. Denylist at
`scripts/lint-bashisms.sh:48-55`.

### 2. The entrypoint suite: two tests break, and they break dangerously

`tests/integration/entrypoint/test_accelerator_entrypoint.py` (832 lines, 26
test functions) builds its fixture with `make_harness` (`:188-222`): a
`tmp_path/rootN` containing synthetic `.claude-plugin/plugin.json`
(`{"version":"9.9.9-test"}`, `:206-208`), a real `keys/accelerator-release.pub`
(`:209`), `bin/` doubling as the cache dir (`:205`), the **repo bootstrap
`shutil.copy`'d in at `root/bin/accelerator`** (`:210-212`), and the real
cargo-built verify shim (`:213-215`, module-scoped `shim_bin` fixture at
`:132-153`).

`_run_bootstrap` (`:225-252`) runs `bash <root>/bin/accelerator` with a **fully
explicit** environment (no inheritance): `PATH`, `HOME`,
`CLAUDE_PLUGIN_ROOT=str(root)`, `ACCELERATOR_BOOTSTRAP_DOWNLOADER`,
`ACCELERATOR_RELEASE_BASE_URL=https://example.invalid/...`, `SERVER_DIR`,
`DL_LOG`.

**Self-location therefore resolves to the fixture root and 24 of 26 tests are
unaffected** — the injected variable simply becomes inert (a stale seam worth
deleting). Path assertions all read the filesystem rather than comparing
bootstrap-computed strings, and pytest's `tmp_path` is already canonical so
`/private/var` normalisation under `pwd -P` is harmless.

The two exceptions are the only tests that run the **repo** file (`_BOOTSTRAP`,
`:31`) rather than the fixture copy, and they must be **deleted, not
re-asserted** — as R3 already says, but for a sharper reason than stated:

- `:260` `test_unset_plugin_root_is_a_named_error` — its env passes only `PATH`,
  so with self-location it resolves the **real repo root**, satisfies gates 5/6
  (all four `bin/accelerator-verify-*` triples are committed), 8, 9 and 10, and
  then falls through to **real `curl` against the real GitHub release URL**,
  writing a cached launcher into the working tree's `bin/`. Leaving it in place
  is a network call and a working-tree write, not just a red test.
- `:273` `test_non_directory_plugin_root_is_a_named_error` — same shape, same
  hazard.

`tests/unit/tasks/test_bootstrap_coverage.py` (44 lines) **does not count gates
or map tests to gates** — despite the name it is four lint/discovery assertions
(shfmt/shellcheck discovery, bashisms discovery, exec bit, shared-key
coherence). Adding or removing gates needs **no change** there. Two textual
couplings do constrain a rewrite: the literal `keys/accelerator-release.pub`
must survive in `bin/accelerator` (`:39-43`), and the literal
`.accelerator-dev-launcher` must survive it too
(`test_accelerator_entrypoint.py:818`).

**No existing test invokes the bootstrap through a symlink** — the shape R1/R10
exist to support is entirely uncovered.

The gated dev override is exercised by 7 tests (`:527`–`:687`) which supply a
**stub shell launcher**, not a cargo artefact: `_local_launcher` (`:499-507`)
writes `_LAUNCHER_SRC` into `<harness root>/cli/target/debug/accelerator`,
`_write_marker` (`:510-511`) creates the marker, and the two env vars go through
`extra_env`. Note `test_accelerator_entrypoint.py:823-829` **asserts the marker
does not exist in the real repo root** — so no new test may create it there.

### 3. The `cli/` rename set is complete and exact

All 21 `CLAUDE_` occurrences under `cli/` were reconciled against the work
item's tables (excluding the gitignored `cli/target/`, `cli/.pup/` and
`cli/visualiser/frontend/node_modules/`; hidden files were searched and are
clean; `Cargo.toml`, `deny.toml`, `pup.ron`, `rustfmt.toml` are clean).
**Nothing is missing, no line number has drifted, and `CLAUDE_PLUGIN_ROOT` is
the only `CLAUDE_*` variable that appears at all.**

Production readers, with the behavioural asymmetry that matters:

| Site | Behaviour |
|---|---|
| `cli/launcher/src/main.rs:176` (doc `:173`) | `std::env::var_os(...).map(PathBuf::from)` → `Option`. **No empty-string filter**, unlike `cache_root.rs:27` — an empty value becomes `Some("")` here. |
| `cli/launcher/src/launch/outbound/resolve/cache_root.rs:26` (docs `:3`,`:38`; error text `:60`) | `Option`, empty-filtered; `ResolutionError::CacheRootUnavailable` when absent. |
| `cli/visualiser/server/src/main.rs:69-70` | let-else → `eprintln!` + `ExitCode::from(2)`. |

`plugin_root()` has **one** call site — `main.rs:161`,
`composed.store.with_plugin_root(plugin_root())` — inside `compose_stack`,
itself passed to `dispatch` as a **lazy closure** (`main.rs:196-198`), so a
`version` or external subcommand never reads it. `FileConfigStore` is `Clone`
and is boxed into **six** ports (`config_command/core/mod.rs:92-122`), so one
`Option` fans out to every config command.

The `Option` is consumed at five places in `cli/config-adapters/src/store.rs`:
`:78` (`display_path` prefix shortening), `:282` (`known_skill_names` → `Ok(vec![])`,
suppressing skill-name validation), `:343` (the plugin-default template tier),
`:357` (`template_names` → `vec![]`), and `:413-417` (`plugin_template_path`,
feeding `plugin_default` at `:382` and `TemplateOverride::eject` at `:429`).

The tier skip (`store.rs:335-353`) is an `if let` that falls through to
`Ok(None)` — **indistinguishable from a missing template file**, with no warning
of its own:

```rust
if let Some(plugin) = &self.plugin_root {              // :343
    let default = plugin.join("templates").join(format!("{name}.md"));
    if default.is_file() { /* PluginDefault */ }
}
Ok(None)                                               // :353
```

`UnixExec::exec` (`cli/launcher/src/launch/outbound/exec.rs`, 23 lines total)
is confirmed to contain **no** `.env()`, `.envs()`, `.env_clear()` or
`.env_remove()` — `Command::new(program).args(args).exec()` at `:17`. So one
`export` in the bootstrap reaches the launcher and, through `exec`, the
visualiser server.

`ACCELERATOR_PLUGIN_ROOT` appears **nowhere in the repo** outside three `meta/`
documents. It fits the existing family cleanly — the other location-ish members
being `ACCELERATOR_CACHE_DIR` (`cache_root.rs:23`) and
`ACCELERATOR_RELEASE_BASE_URL` (`main.rs:40`), alongside `ACCELERATOR_LOG`
(`kernel/src/logging.rs:27`), the derived `ACCELERATOR_<SUB>_BIN` override
(`launch/core.rs:204-230`, prefix at `:230`), and four
`ACCELERATOR_VISUALISER_*` variables.

One vestige worth cleaning opportunistically: `ACCELERATOR_MIGRATION_MODE` is
**set** at `cli/config-adapters/tests/config_reader.rs:34` but **read nowhere**
in `cli/` (only a doc-comment mention at `store.rs:20`).

### 4. Measured behaviour — only the template family is root-sensitive

The tree-local `cli/target/debug/accelerator` was **stale** (v1.24.0-pre.13,
built 2026-07-13, predating the `config` builtin — it routed `config` through
the external resolver and so reported `CacheRootUnavailable`). After
`cargo build --manifest-path cli/Cargo.toml --bin accelerator` (v1.24.0-pre.16),
running each command twice — once under `env -u CLAUDE_PLUGIN_ROOT`, once with
`CLAUDE_PLUGIN_ROOT=<repo>`:

| Command (`--fail-safe`) | Rootless | Rooted |
|---|---|---|
| `config agents` | rc 0, 739 B | rc 0, 739 B |
| `config paths` | rc 0, 473 B | rc 0, 473 B |
| `config path work` | rc 0, 9 B | rc 0, 9 B |
| `config instructions commit` | rc 0, **0 B** | rc 0, **0 B** |
| `config context --skill commit` | rc 0, **0 B** | rc 0, **0 B** |
| `config instructions create-plan` | rc 0, 680 B | rc 0, 680 B |
| `config work integration` | rc 0, 6 B | rc 0, 6 B |
| `config review plan` | rc 0, 1320 B | rc 0, 1320 B |
| **`config template adr`** | **rc 1**, 0 B, `Template 'adr' not found. Available templates:` | rc 0, 1672 B |
| `config templates list` | rc 0, header-only empty table | rc 0, 10+ rows |

Three consequences:

**(a) The silent-degradation hazard is confined to the template family.**
Everything else reads project config and is root-independent. The research doc's
Hypothesis 4 measurement (`templates list` → empty table, exit 0) **reproduces
exactly** and remains the correct justification for R2 — but its generalisation
to the whole `config` family does not hold.

**(b) `config template <name>` is fail-closed even under `--fail-safe`.**
`resolve_template` (`config_command/inbound/cli.rs:238-247`) maps `None` to
`Failure::Refusal(not_found(...))`, and `finish` (`:452-470`) degrades only
`Failure::Read`:

```rust
Err(Failure::Read(error)) if on_failure == OnFailure::Degrade => { ... Ok(()) }
Err(Failure::Read(error) | Failure::Refusal(error)) => Err(error),
```

So a non-exporting bootstrap fix would leave the 20 `!` sites that call
`config template <name>` failing **loudly**, not silently. R2 is still
mandatory — `config templates list` is the silent case — but the failure mode
is mixed, not uniformly silent.

**(c) The `accelerator:`-on-stderr discriminator only detects bootstrap-layer
aborts.** The launcher's own diagnostics (`Template 'adr' not found`) carry no
`accelerator:` prefix, because that prefix is `fail()`'s
`printf 'accelerator: %s\n'`. The acceptance criteria's fail-safe signature is
therefore correct for what it claims to separate, but must not be read as
"stderr is clean iff the CLI fully succeeded".

### 5. R4: the mechanism is misdescribed and the sequencing claim is wrong

`skills/work/scripts/work-item-template-field-hints.sh` self-locates at `:11-12`
(`pwd`, not `pwd -P`), and calls the CLI at `:52-56`:

```bash
TEMPLATE_OUTPUT=$("${ACCELERATOR_BIN:-$PLUGIN_ROOT/bin/accelerator}" config template work-item 2>/dev/null) || {
  hardcoded_fallback "$FIELD"
  exit 0
}
```

It **never reads `CLAUDE_PLUGIN_ROOT`** — confirmed by full read. The fallback
function is `hardcoded_fallback()` (`:22-49`), with two further entries on a
*successful* call (`:69-72` no matching `^${FIELD}:` line; `:75-78` no `#` in
the matched line).

The four seam sites are `test-work-item-scripts.sh:1053` (Test 6, literal
7-value status list) and `:1086`, `:1096`, `:1106` (Test 8, the tripwire, which
parses `templates/work-item.md` itself and compares).

**The values coincide exactly.** `templates/work-item.md:8-10` carries
`# draft | ready | in-progress | review | done | blocked | abandoned`,
`# story | epic | task | bug | spike`, `# high | medium | low` — byte-identical
to `hardcoded_fallback`'s constants. The render wraps content in a
```` ```markdown ```` fence (`config_command/render/template.rs:14-19`) but does
not indent, so the script's `^${FIELD}:` anchor still matches. So all four
assertions **pass vacuously** if the CLI succeeds — confirming the work item's
core worry.

But the work item's two supporting claims are both wrong:

- *"works today solely because the **bootstrap** fails its non-directory
  gate"* — only true standalone. Under `mise run`, `tasks/test/helpers.py:33-37`
  sets `ACCELERATOR_BIN=cli/target/debug/accelerator`, so the **bootstrap is
  bypassed entirely**; the seam works because the *launcher* resolves
  `/nonexistent/templates/work-item.md`, gets `Ok(None)`, and refuses.
- *"a rootless launcher exits **0** with the plugin-default tier skipped, so the
  fallback still would not fire"* — measured **exit 1** (§4). Renaming the seam
  variable to `ACCELERATOR_PLUGIN_ROOT="/nonexistent"` **would** restore it.

**Therefore the breakage is caused by R3, not R1.** R1 changes only the
bootstrap, which this path does not traverse under `mise run`; even standalone,
a self-locating bootstrap pre-R3 still hands the launcher
`CLAUDE_PLUGIN_ROOT=/nonexistent` and the fallback fires. The vacuity appears
the moment R3 renames the launcher's reader **and** `tasks/test/helpers.py`
writes the new name — at which point the injected old name is inert and the
overlay supplies the real repo root.

This contradicts the *If split* clause: "R4 and R5 are not separable from R1:
the moment self-location lands, R4's four assertions pass vacuously". The
correct statement is *not separable from R3*. R4's prescribed fix
(`ACCELERATOR_BIN="/nonexistent/accelerator"`) is unaffected and remains the
right choice — it is bootstrap- *and* launcher-independent.

Suite conventions: source `scripts/test-helpers.sh`; helpers are **label-first**
(`assert_eq <label> <expected> <actual>` at `scripts/test-helpers.sh:20`, plus
`assert_contains:33`, `assert_exit_code:197`, `assert_stderr_contains:257`,
`assert_grep_empty:335`); output is two-space-indented `  PASS:`/`  FAIL:`; the
file ends with a bare `test_summary` (`:1822`), which returns 1 if `FAIL > 0`.
**There is no symlink assertion helper** — an R10 shell test needs
`assert_eq` over `readlink` output.

### 6. R7: the lint guard has a near-exact precedent

`tasks/lint/store_duplication.py` (66 lines) is the template — a cli/-scoped
regex guard with a named `ALLOWLIST: frozenset[str]` carrying per-entry reasons,
a pure `violations(root: Path) -> list[str]` returning `path:line`, and a thin
`@task check` raising `Exit(..., code=1)` whose message **names the constant and
file to edit**. `tasks/lint/call_site_migration.py:26-93` is the second example
and appends `:{line.strip()}` — the better report format here, since the reader
needs to see *which* variable was found.

`tasks/lint/skill_permissions.py` (247 lines) supplies the reusable `!`-block
machinery for the final acceptance criterion — all public:
`preprocessor_commands(text)` (`:105-107`, regex `_PREPROCESSOR = re.compile(r"!`([^`]*)`")` at `:47`),
`is_plugin_invocation(command)` (`:110-112`) against
`_PLUGIN_PREFIX = "${CLAUDE_PLUGIN_ROOT}/"` (`:55`), plus
`frontmatter_bash_rules`, `frontmatter_name`, `covered_by`, `has_metacharacter`.
Its docstring (`:1-26`) enumerates 7 enforced points, and `_PLUGIN_PREFIX`
plus the `_BARE_LAUNCHER` probe string (`:43`) are the two sites that *model*
the matcher — the reason R7 must be a separate guard.

**Discovery must not use `rglob`.** `tasks/shared/sources.py` holds the
gitignore-honouring walk, with the never-reintroduce-`git ls-files` rule in its
module docstring (`:1-17`): git is blind inside a jj workspace, which "silently
emptied the scan and let unformatted scripts reach CI". `shell_sources()`
(`:58-98`) is **not** generic — hard-wired to `.sh` at `:80` and appending
`bin/accelerator` at `:55` — but `_ignore_spec()` (`:40-48`) is the reusable
primitive, and there is already precedent for reusing it for another extension:
`tests/unit/tasks/test_python_coverage.py:31,68-94` imports `_ignore_spec` and
reimplements the prune loop for `.py`. **Promote the loop into a generic
`sources(root, suffixes, subtree)` rather than writing it a third time.** This
matters concretely: `.gitignore:20-21,25` ignore `cli/target/`, `cli/.pup/` and
`node_modules/`, and a bare `(root/"cli").rglob("*.rs")` would descend into
thousands of vendored files. `tasks/lint/scripts.py:8-11` documents the
fail-closed convention — an empty match set must fail, not pass green.

**Registration touches four places, and the non-obvious one is (d):**

- (a) `tasks/lint/__init__.py:1-25` — add to both the import and `__all__`.
- (b) `tasks/__init__.py:83-114` — `ns_lint.add_collection(Collection.from_module(...))`;
  underscores become hyphens automatically.
- (c) `mise.toml` — a leaf modelled on `lint:store-duplication:check`
  (`:392-395`), with the mandatory `depends = ["deps:install:python"]`.
- (d) **`mise run check` does not run `lint:check`.** `check` depends on the
  seven `<component>:check` roll-ups (`mise.toml:465-467`); `cli:check`
  (`:407-409`) is where `lint:store-duplication:check` and
  `lint:vendor-shims:check` are wired, and `.github/workflows/main.yml:257`
  runs `mise run cli:check`. **No CI job runs `lint:check` or the bare
  `check`.** So a cli/-scoped guard belongs in `cli:check.depends` (`:409`);
  add it to `lint:check.depends` (`:451`) as well for completeness.
- (e) Test at `tests/unit/tasks/test_<module>.py`, following
  `test_store_duplication.py` exactly: `REPO_ROOT = Path(__file__).resolve().parents[3]`,
  per-branch negative tests over `tmp_path`, an allowlist round-trip
  (`for rel in ALLOWLIST`), and `violations(REPO_ROOT) == []` as the durable
  enforcement. `violations(root)` **must** take `root` as a parameter — the
  whole testability of the pattern rests on that injection.

One hazard, documented at `tests/unit/tasks/test_python_coverage.py:138-145`:
never write a sentinel violation into the live tree — `test:unit:tasks` runs
concurrently with `cli:check` under `mise run`, so an in-tree sentinel makes
the checks flake. Keep every probe in `tmp_path`.

`tests/unit/tasks/test_mise.py:17-35` guards `_CHECK_GATES = ["cli:check",
"deny:check", "pup:check"]` reachability from `check` — the natural place to add
a companion assertion.

### 7. R9/R9a: hooks, and the two conventions that constrain the new hook

`hooks/hooks.json` (44 lines) declares `SessionStart` as an **array of
matcher-groups**, house style one hook per group, command expressed as the
literal un-expanded string `"${CLAUDE_PLUGIN_ROOT}/hooks/<name>.sh"`.

**The new entry must be appended (index 3), never inserted at 0** —
`hooks/test-vcs-detect.sh:615-634` hard-codes `.hooks.SessionStart[0]` and
asserts it is `vcs-detect.sh` with `matcher == ""`, `hooks|length == 1`.

`hooks` is **not** a declared field in `.claude-plugin/plugin.json` (which
registers only `skills`), so hooks are discovered by convention and no
`plugin.json` edit is needed to register one.

The self-location idiom to copy is `hooks/config-detect.sh:9-10` (mirrored at
`hooks/migrate-discoverability.sh:6,:23`):

```bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$SCRIPT_DIR/.." && pwd)}"
```

**Output convention — this is the load-bearing constraint on "prints the path it
refreshed" (R9).** Every stdout byte any `SessionStart` hook emits today is
JSON. Three established channels:

| Channel | Precedent | Effect |
|---|---|---|
| stdout `{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"…"}}` | `hooks/vcs-detect.sh:177-181` (pretty, via `jq -n`); `config_command/render/summary.rs:66-74` (compact) | injected into the model's context |
| stdout top-level `"systemMessage"` sibling | `hooks/vcs-detect.sh:14,:180-181` | user-visible warning, not context |
| **stderr, plain text, `[accelerator] …` prefix, `exit 0`** | `hooks/migrate-discoverability.sh:68-73` | informational only |

Bare plain text on **stdout** has no precedent. For a diagnostic path the
`migrate-discoverability.sh` stderr form is the right match; if the path must
reach the model it has to go through `additionalContext`. All hooks exit 0 on
every path.

`CLAUDE_PLUGIN_DATA` appears **nowhere** in code, scripts, hooks, docs or tests
— the new hook would be its first consumer. Two `meta/` records bear on it:
`meta/reviews/work/0182-...-review-1.md:155-160` flags the `/bin`
absolute-path degeneration if the variable is unset (so the inertness guard must
test `[ -n "${CLAUDE_PLUGIN_DATA:-}" ]` **before** composing any path), and
`meta/plans/2026-05-06-design-skill-localhost-and-mcp-issues.md:125` records an
earlier deliberate decision of "**No `${CLAUDE_PLUGIN_DATA}` dependency**".

**R12 has nowhere to land a machine-readable floor.**
`.claude-plugin/plugin.json` (29 lines) has no `claudeCodeVersion`, `engines` or
`minimumClaudeCodeVersion` field — the only `requirements` entry is a free-text
Node string. The v2.1.144 floor exists **only in prose**, at `CLAUDE.md:123` and
`docs/releases-and-compatibility.md:36`. So "raise the declared floor in
`.claude-plugin/plugin.json`" is not performable as written; R12 must either
target those two prose sites or introduce the field.

Hook test harness — copy `hooks/test-migrate-discoverability.sh` (107 lines):
`set -euo pipefail`, `SCRIPT_DIR`/`PLUGIN_ROOT`/`HOOK`, `source
"$PLUGIN_ROOT/scripts/test-helpers.sh"`, `TMPDIR_BASE=$(mktemp -d)` with
`trap 'rm -rf "$TMPDIR_BASE"' EXIT`, a `run_hook()` wrapper applying the env
overlay per invocation (`:22-25`), and a closing bare `test_summary`. Neither
hook suite sets `HOME` today; `test-vcs-detect.sh:38-40` exports
`GIT_CEILING_DIRECTORIES` to stop discovery escaping the fixture — the
equivalent containment lever for R9's snapshot-diff criterion.

**Registration for a new shell suite: none.** `run_shell_suites`
(`tasks/test/helpers.py:40-73`) globs `<subtree>/**/test-*.sh` and filters on
`os.access(p, os.X_OK)` — so the requirements are (1) name it `test-*.sh` under
`hooks/`, (2) `chmod +x` and **commit the mode**. There is no registry, and
unlike config/work/migrate/integrations the `hooks` subtree has **no floor
count** (`tasks/test/integration.py:119-122` is the whole task) — worth adding
`_EXPECTED_HOOKS_SUITES` for symmetry.

Both new `.sh` files are **entrypoints**: `chmod 0755`, committed, and **not**
added to `SHELL_LIBRARIES` (`tasks/lint/scripts.py:18-49`) — adding either would
trip the `chmod -x` branch (`:130-133`) and break the pinned-membership test
`tests/unit/tasks/test_exec_bits.py:288-292`, whose `_RECONCILED_LIBRARIES`
(`:244-275`) is a hand-duplicated copy that must be mirrored on any change.
`tasks/README.md:61-72` documents the invariant. `.editorconfig:36-39` sets
2-space indent with `switch_case_indent` for `.sh`; `:7-8` sets 80 columns.

### 8. R9a: documentation targets

`docs/internals.md` (91 lines) is a flat three-section document with **no `###`
subheadings**: `# Internals` (`:1`), `## The meta/ Directory` (`:3`),
`## Agents` (`:39`), `## VCS Detection` (`:66`), `---` (`:88`), footer nav
(`:90`). A "Terminal invocation" section belongs as a **fourth `##` inserted at
line 88, before the `---`** — same class as VCS Detection (host-environment
mechanics) and least likely to be what a reader opens the file for. The footer
nav chain (`visualiser.md` → `internals.md` → `configuration.md`) must stay
intact.

House style: British spelling; third-person declarative about the product,
second person for user actions; each section is prose → pipe table or code
fence → rationale paragraph; tables are the documented exception to the 80-column
rule (rows run to ~230 chars); `bash` fences use **aligned trailing `#`
comments**. No formatter or linter runs over `docs/`, so the 80-column prose
wrap is hand-maintained.

`README.md:46` `## Documentation`, `**Concepts**` list at `:48-63`. The
`Internals` entry (`:57-58`) currently enumerates the file's three sections —
"the `meta/` directory deep-dive, the agent roster, and VCS detection" — so that
gloss **goes stale** when a fourth section lands and must be updated alongside.

**`bin/accelerator` is mentioned nowhere in `docs/` or `README.md`** — the entry
point is entirely undocumented for users. The sole precedent for the idiom is
`docs/visualiser.md:24`, `accelerator-visualiser  # CLI wrapper — optionally
symlink onto $PATH` (referring to the pre-fold wrapper name), with a follow-on
at `:38`. `~/.local/bin` appears nowhere in the repo docs.

### 9. Build order: the dependency edge already exists, and position means nothing

**`build:cli:dev` already exists** (`mise.toml:107-109` → `tasks/build.py:248-260`)
and is already the declared launcher prerequisite for **six** integration tasks
(`test:integration:{visualiser,config,decisions,migrate,work,integrations}`,
`mise.toml:171,186,201,215,220,225`). Reuse it; do not invent a new one.

**The critical mechanic: mise resolves `depends` as a DAG and runs independent
nodes in parallel.** There is **no `wait_for` and no `depends_post` anywhere in
`mise.toml`**, and array position conveys no ordering. So adding `build:cli:dev`
to `default.depends` would merely *race* the test suites. The only correct place
is the specific leaf task's own `depends` — which is exactly what
`tasks/build.py:252-255` and `tasks/test/helpers.py:27-30` both say in prose
("build ordering lives in the task graph, not in ad-hoc cargo calls").
mise deduplicates, so the launcher compiles once per run.

Concretely, for 0182:

- The new `!`-extraction suite, if Python: add `build:cli:dev` to
  `test:unit:tasks` (`mise.toml:146-149`) or `test:integration:tasks`
  (`:160-163`).
- `test:integration:work` (R4's suite) **already** has the edge (`:220`).
- **CI needs no edit** for anything under `test-unit` or `test-integration` —
  both already carry the `RUSTUP_HOME` routing step and `workspaces: cli` cargo
  caching (`.github/workflows/main.yml:70-71,81-88`). CI *would* need two new
  steps only if `check-scripts` (`:147-163`) or `check-build-system`
  (`:165-181`) started requiring a cargo build; neither has rustup routing or a
  cargo cache today, and per `:244-247` each cargo job needs its own
  `cache_key_prefix`.
- **No test asserts that any `test:*` task depends on `build:cli:dev`** —
  `test_mise.py` covers only `_CHECK_GATES`. A new task could silently ship
  without the prerequisite; this is the natural regression guard to add.

The build system sets **none** of `ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER`,
`ACCELERATOR_LAUNCHER_BIN` or `.accelerator-dev-launcher` — zero hits in
`tasks/`. Those three are exercised only by the entrypoint suite, in synthetic
harness roots with a **stub** launcher. The shared preconditions clause in the
acceptance criteria mandates "the locally built binary supplied via the gated
dev override", which requires all three gates simultaneously **and** that the
binary's canonical parent sit inside `${CLAUDE_PLUGIN_ROOT}/cli/target/`
(`bin/accelerator:119-129`) — satisfiable only when the fixture root *is* the
repo root, which conflicts with the fixture-tree precondition and with the
marker-absence assertion at `test_accelerator_entrypoint.py:825`. The existing
suite's stub-launcher approach is the workable shape.

### 10. Out-of-tree writers, and the full exemption inventory

The four writers are confirmed, with one correction: **`tests/integration/dev/dev_integration_driver.py:138`
feeds nothing.** Its value reaches the fake Python server via `copy_env`, but
`_SERVER_TEMPLATE` (`:45-63`) discovers its root from `os.getcwd()` and never
reads the variable. It is a production-parity mirror of `tasks/dev.py:48`;
rename it for fidelity, not function.

- `tasks/dev.py:48` (comment `:45`) — `{**os.environ, "CLAUDE_PLUGIN_ROOT": str(REPO_ROOT)}`,
  reaching the dev server two ways: directly (`:171-173`) and via the circus
  arbiter (`:70` → `tasks/shared/dev/circus.py:191-203` → `copy_env = true` at
  `:75`). Feeds the **hard-exit** reader; without it `mise run dev:*` dies
  with exit 2.
- `tasks/shared/dev/circus.py:43` — doc comment only; no code names the
  variable (the mechanism is `copy_env`). A stale comment if missed, nothing
  more.
- `tasks/test/helpers.py:36` (docstring `:18,:25,:29`) — the shell-suite
  overlay, `os.environ.get("CLAUDE_PLUGIN_ROOT", str(repo))`.

The `grep -rl` acceptance criterion's exemption set is confirmed against the
tree, with counts:

| Location | Occurrences | Kind |
|---|---|---|
| `skills/**/SKILL.md` | 410 across 48 files | substitution tokens (`allowed-tools` rules + `!` commands) |
| `agents/**` | **0** | — the criterion lists `agents/**` as an exemption but there is nothing there to exempt |
| `hooks/**` | 17 across 6 files | 2 production reads, 4 `hooks.json` tokens, 11 test writes/asserts |
| `scripts/**` | 5 across 2 files | `interactive-harness.sh:29` (the one hard `:?` read), `test-design.sh:153,155` (literal asserts) |
| `skills/config/migrate/**` | 8 production reads + 16 fixture reads + 12 in-heredoc + 81/74/15/1 test writes | migrations `0001`–`0007` + runner + interactive lib |
| `tests/unit/tasks/test_call_site_migration.py` | `:26,:35` | fixture strings |
| `tests/unit/tasks/test_skill_permissions.py` | `:14,15,30,32,42,54,55,75,87` | **not listed in the criterion** — 9 further token fixtures |
| `tasks/lint/skill_permissions.py` | `:43,:55` | the matcher model — **not listed in the criterion** |
| `.shellcheckrc:51`, `CLAUDE.md:65` | 2 | prose/comment |
| `bin/accelerator` | 13 | the subject of R1 |

So the criterion's enumeration is **incomplete in three places**:
`tests/unit/tasks/test_skill_permissions.py`, `tasks/lint/skill_permissions.py`,
and `.shellcheckrc`/`CLAUDE.md` — and lists `agents/**`, which has no
occurrences. As written it would fail.

Also note `skills/config/migrate/scripts/run-migrations.sh:643` and
`interactive-lib.sh:433,744` **write** `CLAUDE_PLUGIN_ROOT` into migration
children, satisfying `scripts/interactive-harness.sh:29`'s hard `:?` read. That
is a self-contained shell→shell contract entirely inside the adapter layer, so
it is correctly exempt — but it means the variable has a *legitimate* producer
outside Claude Code, which the layer table does not currently say.

### 11. The `!`-extraction criterion is simpler and harder than scoped

Extracting every `` !`…bin/accelerator config…` `` from `skills/**/SKILL.md`
yields **204 commands**, and:

- **Zero contain any `$` interpolation** beyond the `${CLAUDE_PLUGIN_ROOT}`
  prefix. Every one is a static literal. So the criterion's "commands carrying
  skill-time argument interpolation are supplied fixture arguments or skipped
  with a logged reason" describes an **empty set** — that branch can be dropped.
- All 204 carry `--fail-safe` (already enforced by `skill_permissions.py`
  point 3).
- The distinct command set is ~125. Shape distribution: `config agents` ×22,
  `config path <key>` ×~60 across 17 keys, `config template <name>` ×20 across
  14 names, `config instructions <skill>` ×43, `config context --skill <skill>`
  ×43, plus `config work`/`review`/`paths` singletons.

The hard part is the **non-empty-stdout assertion**. Per §4, `config
instructions <skill>` and `config context --skill <skill>` return empty unless
the *project* configures a per-skill override — that is ~86 of the 204
commands, or 42%. In this repo only `create-plan`, `implement-plan` and
`review-plan` are configured, so `config instructions commit` is legitimately
0 bytes. The exceptions list is therefore both large and **project-dependent**:
run against a fixture project with a known `.accelerator/config.md`, or the
enumeration silently changes with the repo's own configuration. The criterion's
"commands whose empty output is legitimate (a list with nothing configured) are
enumerated as exceptions" understates this considerably.

Recommended shape: a fixture project whose config configures a known
per-skill set, and classify assertions by family — non-empty required for
`agents`/`paths`/`path`/`template`/`work`/`review`, and exact-match against the
fixture's configured set for `instructions`/`context`.

### 12. `mise.local.toml` and the masking, confirmed

`mise.local.toml` is **tracked** (staged `A` in git status; `.gitignore` has no
`mise.local` entry — only `/.accelerator-dev-launcher` at `:29`). Contents:

```toml
[env]
CLAUDE_PLUGIN_ROOT = "/Users/tobyclemson/.claude/plugins/cache/atomic-innovation-prerelease/accelerator/1.24.0-pre.16/"
```

Because `accelerator_env()` reads `os.environ.get("CLAUDE_PLUGIN_ROOT", str(repo))`
(`tasks/test/helpers.py:36`), this **overrides the repo-root default for every
shell suite run locally**, pointing them at the installed prerelease cache
rather than the working tree. It would also break the dev-override containment
check, since `cli/target/` would resolve under the installed cache.

Corroboration of the export-scope claim, measured in this session: the Bash tool
environment carries **9** `CLAUDE_*` variables where the research doc's clean
nested probe found **8** — the extra one being `CLAUDE_PLUGIN_ROOT`, carrying
`mise.local.toml`'s tell-tale **trailing slash** (Claude Code's own substituted
value has none). Claude Code v2.1.220. This is consistent with the research
doc's conclusion and is *not* independent evidence of the export: it is a direct
observation of the masking.

**R11 cannot be answered from the codebase** — it is a live-session probe of
Claude Code's `allowed-tools` matcher and must be run interactively per the work
item's own procedure, in permission mode `default` with no broad Bash allow
rules. Nothing in the tree can settle it.

## Code References

- `bin/accelerator:20-23` — `fail()`, the single abort funnel; 16 call sites
- `bin/accelerator:25-27` — the two gates R1 deletes
- `bin/accelerator:119-129` — `dev_launcher_contained`, the containment check
- `bin/accelerator:142,262` — the only two uses of `"$@"`
- `cli/launcher/src/main.rs:175-177` — `plugin_root()`, no empty-string filter
- `cli/launcher/src/main.rs:161,196-198` — the single call site, behind a lazy closure
- `cli/config-adapters/src/store.rs:343-353` — the plugin-default tier skip
- `cli/config-adapters/src/store.rs:282,357` — the other two silent `None` degradations
- `cli/launcher/src/config_command/inbound/cli.rs:238-247` — `resolve_template` → `Failure::Refusal`
- `cli/launcher/src/config_command/inbound/cli.rs:452-470` — `finish`; `Refusal` ignores `--fail-safe`
- `cli/launcher/src/launch/outbound/exec.rs:17` — no `.env()`; wholesale inheritance confirmed
- `cli/launcher/src/launch/core.rs:204-230` — `derive_override_var`, the `ACCELERATOR_` prefix
- `tests/integration/entrypoint/test_accelerator_entrypoint.py:188-222` — `make_harness`
- `tests/integration/entrypoint/test_accelerator_entrypoint.py:225-252` — `_run_bootstrap`
- `tests/integration/entrypoint/test_accelerator_entrypoint.py:260,273` — the two tests to delete
- `tests/integration/entrypoint/test_accelerator_entrypoint.py:823-829` — marker-absence assertion
- `tests/unit/tasks/test_bootstrap_coverage.py:21-43` — four lint assertions, no gate count
- `skills/work/scripts/work-item-template-field-hints.sh:11-12,22-49,52-56` — self-location, fallback, CLI call
- `skills/work/scripts/test-work-item-scripts.sh:1053,1086,1096,1106` — the R4 seam
- `templates/work-item.md:8-10` — the values the seam's assertions duplicate
- `tasks/lint/store_duplication.py` — the R7 template, whole file
- `tasks/lint/skill_permissions.py:47,55,105-112` — reusable `!`-extraction machinery
- `tasks/shared/sources.py:1-17,40-48,58-98` — the anti-`git ls-files` rule and the walk
- `tests/unit/tasks/test_python_coverage.py:31,68-94` — precedent for reusing `_ignore_spec`
- `tests/unit/tasks/test_store_duplication.py` — the R7 test pattern, whole file
- `tasks/test/helpers.py:17-37,40-73` — `accelerator_env()` and glob-based suite discovery
- `tasks/test/integration.py:7-41,119-122,131-141` — floors; `hooks` has none
- `tasks/lint/scripts.py:13-17,18-49,120-133` — `SHELL_LIBRARIES` and the exec-bit branch
- `tests/unit/tasks/test_exec_bits.py:244-275,288-292` — the pinned duplicate list
- `hooks/hooks.json:3-31` — `SessionStart` schema
- `hooks/test-vcs-detect.sh:615-634` — the `SessionStart[0]` index assertion
- `hooks/config-detect.sh:9-11` — the canonical self-location idiom
- `hooks/vcs-detect.sh:173-181` — the `jq -n` JSON envelope
- `hooks/migrate-discoverability.sh:66-73` — the stderr `[accelerator]` convention
- `hooks/test-migrate-discoverability.sh:1-34,105-107` — the harness to copy
- `scripts/test-helpers.sh:20,33,197,257,335,371-381` — label-first asserts, `test_summary`
- `.claude-plugin/plugin.json` — no version-floor field exists
- `mise.toml:107-109,146-149,220,407-409,449-451,465-467,471` — `build:cli:dev` and the check topology
- `tests/unit/tasks/test_mise.py:17-35` — `_CHECK_GATES`
- `.github/workflows/main.yml:70-71,81-88,147-163,257` — CI ordering and the two cargo-less jobs
- `docs/internals.md:66-90` — the insertion point for R9a
- `README.md:57-58` — the gloss that goes stale
- `docs/visualiser.md:24,38` — the only `symlink onto $PATH` precedent

## Architecture Insights

**The layer table needs a fifth row, or the fourth widened.** The adapter layer
does not merely *read* `CLAUDE_PLUGIN_ROOT` — `run-migrations.sh:643` and
`interactive-lib.sh:433,744` **write** it into migration children to satisfy
`interactive-harness.sh:29`'s hard `:?` read. That is a legitimate, self-contained
shell→shell producer/consumer pair with no Claude Code involvement. The
work item's "Root handling after this change" column says only "Keeps reading
`CLAUDE_PLUGIN_ROOT`", which understates it and makes the exemption look
weaker than it is.

**Two variables, two ownership models — and the repo already demonstrates why
the split works.** `ACCELERATOR_BIN` is the established "address the reader
directly, bypass the chain" seam: 28 files use
`"${ACCELERATOR_BIN:-$PLUGIN_ROOT/bin/accelerator}"`, and the test overlay sets
it precisely so suites never traverse the fetch/verify bootstrap. R4's fix is
therefore not a workaround but the *existing convention* applied correctly —
which is a stronger argument for it than the (incorrect) mechanical one the work
item gives.

**The `Option`-shaped root is the design flaw the rename does not fix.**
`plugin_root() -> Option<PathBuf>` fans out to six ports and degrades to
`Ok(None)`/`vec![]` at four of its five consumers. Renaming the variable leaves
that intact: a future caller who forgets the export still gets an empty template
list, an empty template-name set, and suppressed skill-name validation, all at
exit 0. The work item files this under "Deliberately unchanged — fold in only if
cheap"; §4 shows the affected surface is narrower than feared (templates only),
which makes it *cheaper* than assumed and worth reconsidering. Making
`plugin_root` non-optional at the config-command boundary would convert three of
those four silent paths into named errors.

**Fail-closed vs fail-safe is bifurcated inside the config command.**
`Failure::Read` degrades under `--fail-safe`; `Failure::Refusal` never does
(`inbound/cli.rs:452-470`), and `ConfigError::Invalid` maps to `Refusal` by
`From` (`:246-252`). A missing template is a `Refusal`. So the blanket
assumption that "the `config` family is read-only and already `--fail-safe`
guarded" — used to justify the automated criterion — holds for most of the
family but not for `config template <name>`, which is 20 of the 204 `!` sites.

**CI enforcement flows through `<component>:check`, not the families.** No CI
job runs `lint:check` or the bare `check`; `.github/workflows/main.yml` invokes
the seven roll-ups individually. Two existing guards
(`lint:skill-permissions:check`, `lint:call-site-migration:check`) get their CI
teeth only from their paired unit test's `violations(REPO_ROOT) == []`
assertion, not from the lint task at all. R7 should do **both**, and should ride
inside `cli:check` to be reached by `check-cli`.

**mise `depends` is a set, not a sequence.** Nothing in `mise.toml` orders
siblings, and the repo has internalised this as "build ordering lives in the
task graph" — meaning per-leaf `depends`, never aggregate position. The build-order
dependency 0182 records is real but already solved: the edge exists, and six
tasks use it.

## Historical Context

- `meta/research/issues/2026-07-26-cli-requires-claude-plugin-root-env-var.md` —
  the source RCA. Its Hypothesis 4 measurement reproduces exactly; its
  generalisation of that measurement to the whole `config` family does not
  (§4). Its Fix Options table and Scope Extension are the direct ancestors of
  R1–R3.
- `meta/reviews/work/0182-cli-derives-plugin-root-from-own-location-review-1.md` —
  APPROVE. Highest-density plugin-root document in the repo. `:155-160` is the
  `${CLAUDE_PLUGIN_DATA}` `/bin`-degeneration hazard that R9's inertness guard
  answers; `:1130` prescribes the temp-tree `HOME`/`CLAUDE_PLUGIN_DATA` approach
  the snapshot-diff criterion uses; `:809-823` correctly anticipated that there
  is nowhere to declare a version floor (§7 confirms).
- `meta/reviews/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding-review-1.md` —
  the reviewed symlink-chase design (`:392`, `:738`, `:757`, `:835`, `:856`).
  This is the single most directly reusable historical artefact for R1: it
  already litigated `readlink -f` vs `pwd -P`, mandated cycle detection, fixed
  the hop count against `SYMLOOP_MAX`, and identified that a cycle test must go
  through `bash "$CYCLE_A"` because `execve()` returns `ELOOP` first.
- `meta/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding.md:620-655` —
  the corresponding specification.
- `meta/research/issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md` —
  the prefix-match mechanism behind the approval prompt; the precedent R11's
  probe extends from `bash`/`sh` wrappers to env-assignment prefixes.
- `meta/plans/2026-07-19-0167-config-command-and-invocation-contract-migration.md`
  (status `ready`) and
  `meta/validations/2026-07-19-0167-...-validation.md` (result **`partial`**) —
  0167 routed the skills through the bootstrap; its partial validation is the
  nearest prior signal that the invocation contract had a gap.
- `meta/plans/2026-07-03-0164-launcher-and-git-style-dispatch.md` (status `done`)
  and `meta/reviews/work/0164-...-review-1.md` (**REVISE**) — where the
  bootstrap and its environment dependency were introduced.
- `meta/decisions/ADR-0048-four-toolchain-split.md` — why R7 is a Python invoke
  task, as the work item states. Also relevant:
  `ADR-0054-git-style-modular-cli-of-on-demand-static-binaries.md` (the
  dispatch/on-demand design authority), `ADR-0045-skills-vs-cli-division-of-labour.md`
  (the boundary R3 enforces), `ADR-0049-bash-3.2-compatibility-floor.md`,
  `ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`.
- `meta/plans/2026-05-06-design-skill-localhost-and-mcp-issues.md:125` — an
  earlier deliberate "**No `${CLAUDE_PLUGIN_DATA}` dependency**" decision, which
  R9 reverses. Worth acknowledging rather than silently overriding.
- No ADR exists about environment variables, the CLI/Claude Code env boundary,
  or lint guards beyond ADR-0049/0050 — and `${CLAUDE_PLUGIN_DATA}` appears in
  **zero** decisions. The boundary rule R7 encodes ("nothing under `cli/` may
  name, read, or require a `CLAUDE_*` variable") is currently unrecorded as a
  decision; it is a candidate ADR.

## Related Research

- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- `meta/research/codebase/2026-07-03-0164-launcher-and-git-style-dispatch.md`
- `meta/research/codebase/2026-07-19-0167-config-command-and-invocation-contract-migration.md`
- `meta/research/codebase/2026-07-22-0167-config-command-refactoring-opportunities.md`
- `meta/research/codebase/2026-06-11-0106-bare-path-script-invocation-call-sites.md` —
  origin of the `${CLAUDE_PLUGIN_ROOT}` addressing convention
- `meta/research/codebase/2026-06-23-0120-prevention-tests-agent-invocation-path.md`
- `meta/research/codebase/2026-06-22-0124-find-repo-root-fails-in-git-worktrees.md` —
  a prior root-derivation failure

## Open Questions

- **R11 is unanswerable from the tree** and remains genuinely open: does
  `${CLAUDE_PLUGIN_ROOT}` still substitute into `allowed-tools` Bash rules on
  v2.1.220? Must be probed live per the work item's procedure. Note this
  session's environment is masked by `mise.local.toml`, so any probe run from
  this repo is invalid evidence — which is itself an argument for landing R6
  first.
- **Should `plugin_root` become non-optional at the config-command boundary?**
  §4 shows the silent-degradation surface is only the template family, making
  this cheaper than the work item assumed. Currently filed as
  "fold in only if cheap".
- **Does the acceptance criteria's dev-override precondition work at all?**
  The three gates require the launcher's canonical parent to sit inside
  `${CLAUDE_PLUGIN_ROOT}/cli/target/`, which conflicts with both the
  fixture-root precondition and the marker-absence assertion at
  `test_accelerator_entrypoint.py:825`. The existing suite uses a **stub**
  launcher instead. Needs restating, probably toward `ACCELERATOR_BIN` or a stub.
- **Where does R12's floor actually go?** `.claude-plugin/plugin.json` has no
  version-floor field; the floor lives in prose at `CLAUDE.md:123` and
  `docs/releases-and-compatibility.md:36`.
- **How is the `!`-extraction harness's fixture project defined?** The
  `instructions`/`context` families are empty-by-default and their emptiness
  depends on project config, so running against this repo makes the exception
  list unstable (§11).
- Should `_EXPECTED_HOOKS_SUITES` be added? The `hooks` subtree is the only
  test-bearing subtree with no floor count, so a dropped exec bit on
  `test-shim-refresh.sh` would silently remove it from CI.
- `ACCELERATOR_MIGRATION_MODE` is set by a `cli/` test but read nowhere in
  `cli/` — dead, and cheap to remove while touching neighbouring code.
