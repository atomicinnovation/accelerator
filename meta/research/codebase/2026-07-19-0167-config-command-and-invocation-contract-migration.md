---
type: codebase-research
id: "2026-07-19-0167-config-command-and-invocation-contract-migration"
title: "Research: Built-in config Command and Invocation-Contract Migration (0167)"
date: "2026-07-19T17:39:37+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0167"
parent: "work-item:0167"
relates_to:
  - "codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"
  - "codebase-research:2026-07-07-0178-config-crates-native-yaml-reader"
  - "codebase-research:2026-07-19-0180-atomic-store-primitives-corpus-adapters"
  - "codebase-research:2026-06-11-0106-bare-path-script-invocation-call-sites"
topic: "Built-in config Command and Invocation-Contract Migration (0167)"
tags: [research, codebase, rust, config, cli, skills, invocation-contract, allowed-tools, hooks, atomic-write, migration]
revision: "1de6019f4e9e7955a0771b555967ebb6baa97081"
repository: "build-system"
last_updated: "2026-07-19T17:39:37+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Built-in config Command and Invocation-Contract Migration (0167)

**Date**: 2026-07-19 17:39 UTC
**Author**: Toby Clemson
**Git Commit**: `1de6019f4e9e7955a0771b555967ebb6baa97081`
**Repository**: build-system (jj workspace of `accelerator`)

## Research Question

What does the codebase actually look like for work item 0167 — the built-in
`accelerator config` command and the SKILL.md invocation-contract cutover? In
particular: what is the bash surface being replaced, what is the real shape of
the invocation contract, what has already landed on the Rust side, how do the
shell suites repoint, what do the hooks require, and where does the work item's
own text diverge from what the tree and the governing ADRs say?

## Summary

The work item is broadly accurate and unusually well-specified, and the tree is
in better shape than it assumes. Nine areas of research produced four headline
results:

1. **Q2 is resolved, and benignly.** The 46-vs-35 gap is real and is *not*
   missing declarations. Eleven files invoke config scripts under a broader
   `allowed-tools` rule: ten integration write-skills declare bare `- Bash`, and
   `skills/vcs/commit/SKILL.md:7` declares
   `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/*)`. Nothing is broken today. Exactly one
   file — `skills/vcs/commit/SKILL.md` — can silently break, and only if the
   replacement moves outside `${CLAUDE_PLUGIN_ROOT}/scripts/`. Since the
   bootstrap path is `bin/accelerator`, **it will**, so that file needs a rule
   added rather than rewritten.

2. **Repointing is more viable than the work item claims for the bulk, and less
   viable at the edges than it claims.** `test-config.sh` has 21 path bindings
   and 329 of 367 references are the uniform `bash "$VAR"` form. But the
   non-repointable remainder is **larger than the four enumerated members**: it
   also includes ~60 in-process calls into `config-common.sh` functions, 29
   `source` sites, two exec-form call sites the `bash "$VAR"` pattern misses, the
   whole of `test-config-read-doc-type-paths.sh` (which never uses `bash "$VAR"`),
   and `scripts/test-design.sh`, which asserts SKILL.md invocation shape and is
   not named anywhere in 0167.

3. **Three defects in the work item that no review pass caught**, each cheap now
   and expensive after the bash suites are deleted: the ADR-0021 **exit-2
   semantics are inverted**; `doc-type-paths` is **mis-classed as scalar** when it
   emits 13 tab-separated lines; and **ADR-0047's arbitrary YAML nesting** — the
   headline net-new capability — falls through both parity gates by construction.

4. **The `store` consolidation question is settled by inspection, not judgement.**
   0180 has landed (its work item still says `ready`; the plan and validation say
   done, and the code is in the tree). There are now **two** `atomic_write`
   implementations plus a third temp-and-rename shape in the launcher cache.
   `corpus-adapters` uses a same-directory temp; `config-adapters` uses a `tmp`
   subdirectory. **Neither has a symlink-escape refusal** — 0167 builds that new.

Two counts also need correcting in the work item: `_EXPECTED_CONFIG_SUITES` has
**zero headroom** (21 = 21 discovered), and `test-init.sh` is **not the only
orphaned suite** — four `skills/design` suites share the same blind spot.

## Detailed Findings

### 1. The bash removal set

22 files in `scripts/config-*.sh` plus one skill-local scaffolder. Two are
sourced libraries; the other 20 are executables. Every value read funnels through
`scripts/config-read-value.sh` (awk over YAML frontmatter of
`.accelerator/config.md` then `config.local.md`, last-writer-wins); the
higher-level scripts are thin default-supplying or prose-rendering wrappers.

**Exit-code shape.** Everything is 0/1 except the template-management trio, where
exit 2 is load-bearing — and note what actually triggers it:

| Script | Exit 2 fires when |
|---|---|
| `config-eject-template.sh` | target **exists** without `--force` |
| `config-diff-template.sh` | **no** user override |
| `config-reset-template.sh` | **no** user override |

**Fail-closed behaviours** (the three 0167 names) are precisely located:

- *Frontmatter writeback* — `scripts/config-common.sh:128-310`. The awk returns
  distinct codes (3 = no frontmatter, 4 = unclosed, 5 = field absent, 6 =
  duplicate match), all collapsing to `return 1` with the file byte-unchanged.
  Values pass via `ENVIRON["CSF_VALUE"]`, not `awk -v`, so `&`/`/`/`\` cannot
  corrupt the substitution. `_config_fm_integrity_check` re-parses the *candidate*
  text and asserts read-back equality before the rename ever happens.
- *`work.integration` enum* — `scripts/config-read-work.sh:46-58` hard-fails via
  `log_die` → exit 1. `scripts/config-dump.sh:214-226` deliberately does **not**
  fail, rendering `(invalid: must be …)` inline, because the diagnostic surface
  must not hard-fail.
- *Doc-type path safety* — `scripts/config-read-doc-type-paths.sh:93-107`. Two
  abort gates per row: tab/newline in the value, and an unsafe-relpath glob case.
  Rationale in the header comment: these dirs scope **in-place mutation** during
  migration.

**A fourth fail-closed path 0167 does not name**: `config_assert_no_legacy_layout`
(`scripts/config-common.sh:55`) `exit 1`s — not `return`s — when
`.claude/accelerator.md` exists without `.accelerator/config.md`. It is called by
only seven of the twenty executables, and *not* by `config-read-path.sh`,
`config-read-work.sh`, `config-read-agents.sh`, `config-read-agent-name.sh`,
`config-read-context.sh`, `config-read-all-paths.sh`,
`config-read-doc-type-paths.sh`, `config-read-browser-executor.sh`, or any
template script. That asymmetry is behaviour, and it has no criterion.

**Injection blocks** — more than the three 0167 enumerates. `## Agent Names`
(`config-read-agents.sh:118-124`, always emitted, exactly 9 bullets),
`## Project Context` (`config-read-context.sh:27-32`, **emits nothing** if all
bodies trim empty), `## Review Configuration` (`config-read-review.sh:482-599`,
mode-dependent, includes a `### Lens Catalogue` table), plus
`## Skill-Specific Context`, `## Additional Instructions`, `## Configured Paths`,
`## Browser Executor`, and `## Effective Configuration`.

**`config-dump.sh` duplicates the parser** (`_read_from_file`, lines 28-92) rather
than reusing `config-read-value.sh`, because attribution needs per-file reads. Any
Rust reader must expose both "effective value" and "which file it came from", and
must redact `*.token`/`*.token_cmd` as `*(set — hidden)*` (lines 257-264).

**`config-common.sh` has 39 `.sh` sourcers plus one inline SKILL.md body** (40
call sites). 17 are cluster scripts; **18 are non-cluster production sourcers** —
`hooks/migrate-discoverability.sh:7`, all seven migrations, the work-item
scripts, the Jira/Linear scripts, the visualiser config writer. Those source it
for `config_extract_frontmatter` / `config_extract_body` / the writeback
primitives, **not** for config reading. Those functions must survive 0167
independently of the `accelerator config` surface.

### 2. The invocation contract — Q2 answered

**247 call sites across 46 SKILL.md files** verifies exactly, and **all 247 are
perfectly uniform**: `` !`${CLAUDE_PLUGIN_ROOT}/scripts/config-<name>.sh[ <one-token-arg>]` ``.
No pipes, no `&&`, no `$(…)`, no `bash ` prefix, no quoting, no multi-arg calls,
no unbraced variant, no two sites on one line.

**35 files declare the `config-*` glob** — the value lives in a YAML *block
sequence*, which is why a naive `allowed-tools:.*config-` grep returns zero.

**The 11-file difference, enumerated:**

- **Group A (1 file)**: `skills/vcs/commit/SKILL.md:7` declares
  `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/*)` — broad because it also calls
  `vcs-status.sh` and `vcs-log.sh`. **This is the one file at risk.**
- **Group B (10 files)**: the `disable-model-invocation: true` integration *write*
  skills, each declaring bare `- Bash`, each with the identical 3-call-site shape
  (`config-read-context.sh`, `config-read-skill-context.sh <name>`,
  `config-read-skill-instructions.sh <name>`).

**Surface the `!`-scoped view misses** — and this contradicts the work item's own
Context, which calls 247 the complete population:

- **`skills/config/configure/SKILL.md`** — 9 invocations in fenced code blocks at
  lines 925, 937, 953, 962, 968, 975, 993, 1012, 1023. These are the **only**
  flagged and multi-arg calls anywhere (`--dry-run`, `--force`, `--confirm`,
  `--all`). The file has **no `allowed-tools` key at all**. This is the file the
  ADR-0045 round-trip proof runs against.
- **`skills/config/init/SKILL.md:45`** — `bash "${CLAUDE_PLUGIN_ROOT}/…/init.sh"`
  inside a ```` ```bash ```` block, not a `!` site.
- **`skills/visualisation/visualise/SKILL.md:31`** — the only `!` site anywhere
  taking a quoted argument and interpolating `$ARGUMENTS`.
- **`skills/review/output-formats/work-item-review-output-format/SKILL.md:63`** —
  a prose reference needing a text update.
- **`hooks/migrate-discoverability.sh:7`** — a `source` of `config-common.sh`,
  structurally un-migratable to a binary call.
- **`agents/`** — zero references of any kind.

**`config-read-browser-executor.sh` has exactly one call site**
(`skills/config/browser-executor/SKILL.md:24`), in a file whose entire `config-*`
footprint is that line. The same-commit re-homing requirement protects a single
line.

### 3. The Rust workspace as it stands

`cli/` is a **10-crate** workspace (`launcher`, `kernel`, `verify`, `document`,
`config`, `config-adapters`, `corpus`, `corpus-adapters`, `vcs`, `vcs-adapters`).
0179 **and** 0180 have landed. Only `launcher` produces a shipped binary.

**There is no `config` subcommand.** The config hexagon is reachable only from a
test fixture binary; nothing in `launcher` references `config` or
`config-adapters`. Dispatch is `cli/launcher/src/launch/mod.rs:22-39`, with a
two-variant `Command` enum (`Version`, and an `external_subcommand` catch-all) at
`cli/launcher/src/launch/inbound/cli.rs:7-22`. Adding `config` means: a `Config`
variant with a nested `Subcommand`, a `config_command` module following the
`version/` hexagon shape, a `dispatch` arm, the two path deps, and a `pup.ron`
module rule.

**The bootstrap path is `bin/accelerator`** — a bash 3.2 script that maps `uname`
to a platform triple, reads the version from `plugin.json`, holds a PID-owner
mkdir lock, fetches and minisign-verifies into `.tmp-launcher-$$`, and finally
`exec`s the launcher (line 194). Cache root is `${CLAUDE_PLUGIN_ROOT}/bin` with
**no XDG fallback** — the comment at lines 87-88 records why: an XDG-resident
binary would break the `allowed-tools` glob. `ACCELERATOR_<SUB>_BIN` short-circuits
resolution, which is the test-fixture seam.

**`config` already resolves 55 keys across 6 groups plus 13 doc types**
(`cli/config/src/catalogue.rs:203-212`), and a bash-parity drift test shells out
to the shell scripts and asserts the catalogues are identical
(`catalogue.rs:343-412`).

**The write path already exists.** `WriteConfigLevel for FileConfigStore`
(`cli/config-adapters/src/store.rs:103-118`) and `ConfigAccess::set` with
`PathConflict` detection (`cli/config/src/service.rs:160-212`) are implemented.
What is missing: any CLI surface, unset/delete, typed set (values are always
`Scalar::String`), key enumeration, and any consumer of `catalogue::default_for`
— defaults are declaration-only today. `ConfigError::NotFound` is defined but
never constructed.

**Every temp-file-plus-rename in `cli/`** — four, so the consolidation check has
more to flag than the work item's "one if 0180 has not landed, two if it has":

| # | Location | Shape |
|---|---|---|
| 1 | `cli/config-adapters/src/store.rs:58-80` | `.accelerator/tmp/config-<pid>-<n>.tmp` + `fs::rename`; manual cleanup; no EXDEV handling; no RAII |
| 2 | `cli/corpus-adapters/src/store.rs:48-85` | `NamedTempFile::new_in(target_dir)` + `persist`; RAII; EXDEV → `StoreError::CrossFilesystem` |
| 3 | `cli/launcher/src/launch/outbound/resolve/cache.rs:88-127` | `.tmp-<stem>-<pid>-<seq>` in the cache root, 0600, chmod +x, `fs::rename` |
| 4 | `cli/corpus-adapters/src/lock.rs:106-117` | rename-as-claim for stale-lock reclaim (not a write, but the grep will hit it) |

**None of the four does a symlink-escape or containment check.** 0167 must decide
whether #3 and #4 are duplicates or intentional exclusions, or its committed
check will flag them forever.

**Enforcement configs.** `cli/pup.ron` has five `RestrictImports` rules; the
whole-crate domain rules (`^config($|::)`, `^corpus($|::)`, `^vcs($|::)`) force
single-item `crate::`-qualified imports inside domain crates. `cli/deny.toml`'s
`[bans]` fences `serde-saphyr` behind `wrappers = ["document"]` — the "infra out
of domain" fitness function. An infrastructure `store` crate needs **no** pup
rule; it needs a `deny.toml` entry only if it introduces a new licence or wraps a
fenced crate (it uses `std` + `tempfile` today, so probably not — but confirm).

**Test idioms already exist for everything 0167 asks for**: `CARGO_BIN_EXE_*`
black-box spawning (`cli/launcher/tests/dispatch.rs:12-13`), fixture workspaces
under `CARGO_TARGET_TMPDIR` seeded with a `.git` marker so root discovery is
bounded (`cli/config-adapters/tests/config_reader.rs:18-24`), a fixture
sub-binary exposing behaviours by argument, a std-only mock HTTP server, and a
`bash-parity` cargo feature gating differential suites.

### 4. Shell suites and the repoint

**`scripts/test-config.sh`** — 6289 lines, **21 path bindings** (10 in the header
block at lines 11-21, 11 mid-file at 2441/3040/3355/3766/4172/4228-4233). Of 367
references to those variables:

- **329** are `bash "$VAR" …` — mechanically repointable.
- **29** are `source "$VAR"` (`DEFAULTS_FILE`, `WORK_COMMON`) — bash array
  introspection, no CLI analogue.
- **2** (`:5975`, `:5977`) are the bare exec form `"$READ_VALUE" …` — a
  `bash "$VAR"` search-and-replace **misses these**.
- The rest use the variable as a *file path*: `:1013` and `:1023` grep the
  **source text** of `config-read-agents.sh` and `config-dump.sh` to extract their
  `AGENT_KEYS` array literals.

**Assertion counting is not what the work item assumes.** 337 is the static
`^\s*assert_` grep. There are also **208 inline `PASS=$((PASS + 1))`** blocks
bypassing the helpers, so the true runtime total is ≈545 — and some assert sites
sit inside loops. Any parity gate should compare `test_summary` output, not grep
counts.

**The non-repointable remainder is bigger than the four enumerated members:**

- `:24` sources `config-common.sh` inline (not variable-bound), followed by **~60
  in-process calls** to its functions — this tests a bash *library*.
- The SKILL.md censuses are more extensive than the flagged spots: `:1095-1101`
  (asserts `config-read-context.sh` appears exactly 42 times, `config-read-agents.sh`
  exactly 22), `:1139-1194` (ordering), `:2995-3032`, `:4985-4991` (skill-context
  and skill-instructions exactly 42 each), `:4993-5027` (skill-context must be
  exactly one line after context), `:5029+` (skill-instructions must be the last
  `^!\`` line). `SKILLS_DIR`/`SKILLS_GREP` are referenced **42 times**.
- `:2525-2532` asserts the set of files defining the defaults arrays equals
  exactly `"./scripts/config-defaults.sh"` — so it fails on **deletion** as well
  as addition.
- **`scripts/test-design.sh`** does the same class of thing at `:42`, `:157-161`,
  `:427`, `:444-446`, `:471-472` — and **0167 does not name it anywhere**.

**`scripts/test-config-read-doc-type-paths.sh`** uses the bare exec form
`"$RESOLVER"` at all 8 call sites — never `bash "$VAR"`. Repointing is still a
one-line change to `:12`, but a pattern-based sweep misses the file entirely. Its
tail (`:96-114`) sources `config-defaults.sh` for array introspection.

**`skills/config/init/scripts/test-init.sh`** — 19 static assertions, runtime
count 44. **It would fail today, by at least 5 assertions.** It expects
`meta/research`, `meta/design-inventories`, `meta/design-gaps` with `.gitkeep`
files; `init.sh:25-31` actually creates `meta/research/codebase`,
`meta/research/design-inventories`, `meta/research/design-gaps` (14 dirs, not 12).
This is exactly what the characterise-then-retire sequence exists to surface.

**Discovery mechanics.** `run_shell_suites` is `tasks/test/helpers.py:17-44`: it
globs `**/test-*.sh`, filters on `is_file()`, name not in
`EXCLUDED_HELPER_NAMES`, and **`os.access(p, os.X_OK)`**, then runs each
*directly* via `context.run(suite)` — relying on the shebang and exec bit, not
`bash <path>`. The **eight subtrees** are the eight `run_shell_suites` call sites
in `tasks/test/integration.py` (lines 53, 87, 109, 125, 131, 137, 150, 163).

**`_EXPECTED_CONFIG_SUITES = 21` (`tasks/test/integration.py:16`) has zero
headroom** — discovery currently finds exactly 21. Retiring two suites drops it to
19 and hard-fails the build unless the constant moves in the same change. Sibling
floors are equally tight: work = 6/6, integrations = 32/32, migrate = 4/4. Total
discovered across all eight subtrees: **75**.

**Five orphaned suites, not one.** Beyond `test-init.sh`:
`skills/design/inventory-design/scripts/test-ensure-playwright.sh`,
`test-notify-downgrade.sh`, `test-validate-source.sh`, and
`playwright/test-run.sh`. Any "walk `skills/` wholesale" fix sweeps all five.

**Two Rust tests pin the shell surface and will break on deletion**:
`cli/config-adapters/tests/parity.rs:42-43` asserts `config-read-value.sh` **is a
file** and `:113-121` shells out to it; `cli/corpus-adapters/tests/doc_type_single_source.rs:189-220`
sources `config-defaults.sh`. Two shell suites also write `exec` **stub scripts**
hard-coding the resolver's path (`test-validate-corpus-frontmatter.sh:412`,
`test-migrate-0007.sh:2208`).

### 5. Hooks

`hooks/hooks.json` confirms four registrations, each SessionStart hook in its
**own group object** with its own `matcher` (lines 9, 18, 27, 38).

**`hooks/config-detect.sh` is 24 lines** and contributes only the jq guard, the
emptiness test, and the envelope. Everything semantic is in
`scripts/config-summary.sh` (158 lines). Its four output states:

| Condition | stdout | exit |
|---|---|---|
| jq absent | literal `{"systemMessage":"WARNING: jq is not installed…"}` | 0 |
| summary non-empty | pretty-printed 2-space `hookSpecificOutput` envelope | 0 |
| summary empty | **nothing** | 0 |
| summary script exits non-zero | **nothing** (`\|\| SUMMARY=""` swallows it) | 0 |

The hook is **never non-zero**, and never emits both `additionalContext` and
`systemMessage` — unlike `vcs-detect.sh:177-181`, which merges an optional
top-level `systemMessage`. `vcs-guard.sh` emits two *different* PreToolUse shapes:
`{decision: "block", reason: …}` and `{decision: "allow", hookSpecificOutput:
{systemMessage: …}}` — note `systemMessage` nested in one and top-level in the
other. `migrate-discoverability.sh:68-70` emits plain text to **stderr** only.
0167's "no single envelope" finding is confirmed in full.

**`config-detect.sh` is covered — from the `scripts/` suite, not `hooks/`.**
`test-config.sh:19` binds `CONFIG_DETECT`, and two blocks exercise it:
`:758-800` (empty output when initialised with no config; init-hint in JSON;
`hookEventName == "SessionStart"`) and `:5079-5108`, which asserts the
unrecognised-skill warning appears **on stderr and not in stdout JSON**. That
stdout/stderr separation is an explicit contract the Rust replacement must keep.

**Migration hazard**: `hooks/test-vcs-detect.sh:620-634` hard-codes
`.hooks.SessionStart[0]` and asserts `SessionStart[0].hooks | length == 1`.
Reordering or restructuring the SessionStart groups turns that red even though
`vcs-detect` is untouched.

**Plugin-root resolution is inconsistent.** Only `migrate-discoverability.sh:23`
honours `${CLAUDE_PLUGIN_ROOT}`. `config-detect.sh:9` derives `SCRIPT_DIR` from
`BASH_SOURCE` and invokes by bare relative path (`$SCRIPT_DIR/../scripts/…`).
Swapping it for `accelerator config summary --format=hook` needs a resolution
story the current hook does not have.

**No spec-level documentation of the envelope exists.** The only constraints are
behavioural: byte-identity golden fixtures for `vcs-detect` (so jq's 2-space
pretty-print is frozen *for that hook*; `config-detect` has no golden and so has
formatting latitude), stdout-must-be-valid-JSON-or-empty, warnings-to-stderr, and
silence as a legal state.

### 6. Luminosity as precedent — and where it has none

Luminosity's CLI is a five-crate workspace with a **deliberately minimal** config
surface: only `config get` and `config set`.

**Transferable precedents:**

- **Per-command output split, no `--format`.** There is no `--format` flag
  anywhere in the workspace. Scalar reads live in `config get`; prose injection
  lives in a separate top-level `context` command. This directly supports 0167's
  closed decision.
- **`--fail-safe` semantics**, with two refinements 0167 should adopt: the failure
  notice deliberately does **not** impersonate a real block
  (`## Project Context Unavailable`), and `--explain` degrades too, so a
  diagnostic flag cannot punch a hole in the fail-safe boundary
  (`cli/launcher/src/context_command/inbound/cli.rs:38-46`, `:161-179`).
- **Positional key/value + named `--level` as `Option<ValueEnum>`**, where `None`
  means "resolve across levels" and the default is applied in the handler, not by
  clap (`config_command/inbound/cli.rs:51`).
- **A CLI-layer `Level` enum bridged by `From`** so the domain crate stays
  clap-free.
- **`refuse_escaping_path`** (`cli/config-adapters/src/store.rs:115-130`) — the
  symlink guard Accelerator lacks. It canonicalises **both sides** before
  comparing; the doc comment explains why a string prefix would be wrong (a
  `.luminosity-evil/` sibling passes) and why canonical-against-raw would be wrong
  (macOS `/tmp` → `/private/tmp`). **This is the design to lift into `store`.**
- **`render(existing, document)`** (`config-adapters/src/document.rs:24-45`) —
  body preservation where the discarded `parse_frontmatter` call is a fail-closed
  gate, so a malformed existing file errors before `atomic_write` is reached.
- **The test harness**: `CARGO_BIN_EXE_*` + `CARGO_TARGET_TMPDIR` + PID/counter
  dirs + a `.git` boundary marker, asserting raw `output.stdout` bytes.

**Where luminosity has no precedent — 0167 is the originating implementation:**

- **Exit code 2.** Luminosity is strictly 0/1, funnelled through a stringly
  `kernel::Error::Failed(String)`. Accelerator's `kernel::Error` is the same
  shape, so **exit 2 needs a new boundary variant** that `main` maps to
  `ExitCode::from(2)`. This is design work, not mirroring.
- **Template subcommands.** Luminosity's work item 0019 is `draft` with zero code
  — and it cites *Accelerator* as its reference. The borrowing runs Accelerator →
  luminosity here.
- **Hook envelopes.** No mechanism at all; injection is `!` preprocessor only.
- **A shared `store` crate.** `atomic_write` is a private method on
  `FileConfigStore`, in a `.luminosity/tmp` subdirectory — i.e. luminosity has the
  *same* shape as Accelerator's `config-adapters`, and Accelerator's
  `corpus-adapters` is ahead of both.
- **`--help` snapshot testing.** Substring assertions only
  (`launcher/tests/config.rs:260-279`), though those do pin doc-comment text into
  the contract.

### 7. Cross-item records

| Record 0167 owes | State |
|---|---|
| 0106: `accelerator` variant of the canonical directive | **Outstanding** |
| 0107: migrate-then-build disposition + `accelerator` shape + Q1's answer | **Outstanding** |
| 0166: `store` split amendment | **Already written** (`0166:221-242`) — verify only |
| 0169: SessionStart envelope, "bootstrap path" naming, PreToolUse is 0169's | **Outstanding** |
| 0169/0173/0174 reciprocal `blocked_by` | **Done** |

**0106's authoritative sentence** is in the *plan* at
`meta/plans/2026-06-11-0106-invoke-plugin-scripts-by-bare-path.md:164-166`:

> Run the bare path **directly** as an executable; never prefix it with
> `bash`/`sh`/`env` (a wrapper prefix escapes the skill's `allowed-tools`
> permission and forces an unnecessary prompt).

The work item's blockquote (`0106:76-78`) is two sentences and worded differently
("Do **not** prefix the invocation with"), so the "updated to match" criterion
means editing both. The plan generalises the invariant at `:695-702`: **"the
invocation's first token must be the bare braced path"** — unquoted and braced,
not `"${…}"`, not `$CLAUDE_PLUGIN_ROOT`. An `accelerator` variant must pin the
equivalent first-token shape. Note 0106 explicitly *forbade* `allowed-tools`
changes (`0106:83-84`), so 0167's variant is a deliberate departure.

**0169 needs three corrections**, all currently wrong in its text: it says
"wrapper" three times (`:32`, `:52-55`, `:83`); its AC2 (`:67-69`) asserts one
shared `--format=hook` envelope for both SessionStart and PreToolUse, which
inspection disproves; and it names `accelerator config detect` (`:53-54`) against
0167's `config summary`. Its AC4 (`:72-74`) also claims removal of all four hook
`.sh` files including `config-detect.sh`, which 0167 owns.

**0180's true state**: work item says `ready`, plan says `done`, validation says
`pass` with commits `338dcd37 → accc29a5 → 76753652 → 609bb999`. **The code is in
the tree.** Its `atomic_write` signature is
`fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), StoreError>` — a free
function taking `&[u8]`; `config-adapters`' is a method taking `&str` returning
`ConfigError`. The extraction must reconcile all three axes.

## Code References

- `scripts/config-common.sh:55` — `config_assert_no_legacy_layout`, the fourth fail-closed path
- `scripts/config-common.sh:128-310` — writeback primitives, integrity check, awk return codes
- `scripts/config-read-doc-type-paths.sh:93-107` — doc-type path safety gates
- `scripts/config-read-work.sh:46-58` — `work.integration` hard fail
- `scripts/config-dump.sh:28-92`, `:257-264` — duplicated parser; credential redaction
- `scripts/config-summary.sh:92-146` — `KNOWN_SKILLS` dynamic scan + per-skill customisations
- `skills/vcs/commit/SKILL.md:7` — the one at-risk `allowed-tools` rule
- `skills/config/configure/SKILL.md:925-1023` — the only flagged/multi-arg invocations; no `allowed-tools`
- `hooks/config-detect.sh:3-24` — jq guard, delegation, envelope
- `hooks/hooks.json:9,18,27,38` — four registrations
- `hooks/test-vcs-detect.sh:620-634` — index-sensitive `SessionStart[0]` assertion
- `scripts/test-config.sh:11-21`, `:2441-2533`, `:3040-4233` — path bindings
- `scripts/test-config.sh:1095-1101`, `:4985-5027` — SKILL.md censuses
- `scripts/test-design.sh:42,157-161,427,471-472` — unnamed censuses
- `tasks/test/helpers.py:17-44` — `run_shell_suites` discovery
- `tasks/test/integration.py:16` — `_EXPECTED_CONFIG_SUITES = 21`
- `bin/accelerator:87-88,114-145,194` — cache-root rationale, lock, exec
- `cli/launcher/src/launch/mod.rs:22-39` — dispatch
- `cli/launcher/src/launch/inbound/cli.rs:7-22` — `Command` enum
- `cli/config/src/service.rs:96-108`, `:160-212` — precedence; `set` with `PathConflict`
- `cli/config/src/catalogue.rs:203-212`, `:343-412` — 55 keys; bash drift test
- `cli/config-adapters/src/store.rs:58-80` — `atomic_write` (tmp subdirectory)
- `cli/corpus-adapters/src/store.rs:48-85` — `atomic_write` (same directory, EXDEV)
- `cli/launcher/src/launch/outbound/resolve/cache.rs:88-127` — third temp-and-rename
- `cli/config-adapters/tests/parity.rs:42-43,113-121` — Rust test pinning the shell script
- `cli/pup.ron:42-89` — whole-crate domain rules
- `cli/deny.toml:55-69` — `serde-saphyr` wrappers fence

Luminosity (reference repo, `…/company/luminosity`):

- `cli/launcher/src/launch/inbound/cli.rs:47-94` — `ConfigAction` + `Level` `ValueEnum`
- `cli/launcher/src/context_command/inbound/cli.rs:38-46,88-100` — fail-safe rationale, block rendering
- `cli/config-adapters/src/store.rs:115-154` — `refuse_escaping_path` + `atomic_write`
- `cli/config-adapters/src/document.rs:24-45` — body-preserving `render`
- `cli/launcher/tests/config.rs:20-35,248-279` — harness, byte assertions, help contract

## Architecture Insights

**The uniformity is genuine but bounded.** All 247 `!` sites are identical, which
makes the body rewrite mechanically safe. But the *non*-`!` surface is where all
the irregularity lives — flags, multi-arg calls, `bash` prefixes, `source`. The
work item's "no wrappers, no pipes, no quoting variants" claim is true of the 247
and false of the migration surface as a whole.

**Two counting habits recur and should be distrusted.** The 337 figure is a static
grep that misses 208 inline assertions. The 247/46/35 figures are `!`-scoped and
miss the code-block invocations. Both are cited in the work item as complete
populations.

**The floors are the sharpest tripwire in the build.** Four subtree floors sit at
exactly their discovered count. Any suite retirement is a same-change edit to a
Python constant, and `test-init.sh` being wired *in* mid-story moves the config
count up before it moves down.

**Repointing has a shape the work item does not model.** It is not "20 bindings,
mechanical". It is: 329 uniform invocations that repoint trivially; ~90
library-level and array-introspection calls that cannot repoint at all because
they test bash *as bash*; and a text-assertion layer over SKILL.md that breaks by
design across three files, not one.

**Everything the Rust side needs already has a working precedent in-tree.** The
black-box harness, the `.git`-bounded fixture workspace, the fixture sub-binary,
the bash-parity differential feature, the drift test against the shell catalogue
— all exist and are green. The new work is the command surface, the envelope, and
the `store` extraction, not the testing apparatus.

## Historical Context

- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
  — the source research. Q3 resolves the cache location under
  `${CLAUDE_PLUGIN_ROOT}` (strictly about the *binary cache*; 0167 extends it to
  the bootstrap script, reasonably but as an extension). Q4 resolves the hook
  envelope via `--format=hook` and names `config detect`. Q7 resolves the test
  strategy as repoint-then-retire — the position 0167 adopted at pass 5 — **and
  explicitly endorses a `--format` switch**, which 0167 rejects. 0167's closed
  `--format` decision argues against luminosity and bash, never against the
  research that recommended it.
- `meta/decisions/ADR-0021-template-management-subcommands.md` — exit 2 is
  "destructive action requires confirmation", mandates a `--confirm`/`--force`
  two-phase flow, Tier-1 reset semantics, and out-of-project re-confirmation.
  None of the latter three appear in 0167.
- `meta/decisions/ADR-0047-multi-level-userspace-configuration-model.md` — drops
  the two-level `section.key` cap in favour of **arbitrary YAML structure**, and
  names the **dual agent-name strategy** (inline `config-read-agent-name.sh` for
  exact `subagent_type` values plus one per-skill table call for prose). Both are
  in-scope for Phase 4 per the research; neither appears in 0167.
- `meta/decisions/ADR-0020-per-skill-customisation-directory.md` — mandates
  `KNOWN_SKILLS` derived by frontmatter scan excluding `configure`, advisory
  stderr warning on unknown dirs, empty files → zero output, and enumeration of
  detected customisations **in the SessionStart summary**. Also specifies
  `.claude/accelerator/skills/<skill>/`, which ADR-0047 relocates under
  `.accelerator/` without flagging the change.
- `meta/reviews/work/0167-…-review-1.md` — five passes. Its strongest process
  finding: every incremental patching round introduced a new critical; the single
  rewrite round did not. **Do not patch the AC section serially.**
- `meta/work/0166-shared-config-corpus-store-crates.md:221-242` — the `store`
  amendment 0167 owes is already written and already references 0167.

## Related Research

- `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`
- `meta/research/codebase/2026-07-07-0178-config-crates-native-yaml-reader.md`
- `meta/research/codebase/2026-07-19-0180-atomic-store-primitives-corpus-adapters.md`
- `meta/research/codebase/2026-07-03-0164-launcher-and-git-style-dispatch.md`
- `meta/research/codebase/2026-06-11-0106-bare-path-script-invocation-call-sites.md`

## Open Questions

**Still genuinely open:**

- **Q1 — does `*` span `/` in the `allowed-tools` prefix matcher?** Unanswered
  upstream; the research never addressed it. One piece of indirect evidence
  nobody has cited: the existing taxonomy already relies on whole-directory globs
  **including nested `playwright/*`**, which only works if `*` spans `/`. That is
  suggestive, not proof — the empirical probe against v2.1.144 is still required.

**Newly surfaced, needing a decision before implementation:**

- **ADR-0021 exit-2 semantics are inverted in 0167.** The criterion asserts exit 2
  from `eject|diff|reset` against the **not-customised** fixture. Under ADR-0021,
  `eject` and `reset` exit 2 when an override **exists**; only `diff` matches the
  work item's phrasing — and the research calls that one the "exit-code-as-signal
  hack". The bash confirms ADR-0021, not 0167.
- **`doc-type-paths` is mis-classed as `scalar`** ("exactly the value plus a
  single `\n`") when it emits 13 `type<TAB>dir` lines under `LC_ALL=C`. It belongs
  in `block`.
- **ADR-0047's arbitrary YAML nesting has no criterion and falls through both
  parity gates** — a repointed bash suite cannot assert a capability the bash
  never had, and it is not one of the four enumerated remainder members.
- **ADR-0020 behaviours are unowned**: `KNOWN_SKILLS` scan, the advisory stderr
  warning, empty-file-yields-zero-bytes, and the summary's enumeration of detected
  customisations. The last means the summary golden may miss a mandated section if
  the baseline fixture has no per-skill dirs.
- **`config_assert_no_legacy_layout`** — a fourth fail-closed path, applied
  asymmetrically across the twenty scripts, with no criterion.
- **`scripts/test-design.sh` is unnamed in 0167** but asserts SKILL.md invocation
  shape and the browser-executor's existence in the same class as the flagged
  regions.
- **Which temp-and-rename shapes count as duplicates** — the launcher cache and
  the lock reclaim will both trip a naive committed grep.
- **Whether the `!`-scoped 247 or the full call-site surface is the denominator**
  — the Context and the Grep A criterion currently contradict each other on this.
- **The per-skill customisation root** — ADR-0020 says `.claude/accelerator/…`,
  ADR-0047 says `.accelerator/…`; 0167 records neither.

**Stale text in 0167 worth cleaning:** two Drafting Notes still say the `store`
decision "expands 0180's scope and must be agreed there", contradicting the
settled Dependencies text ("0180 needs no amendment, no scope expansion"); and the
~40-line "Answering the scope objection" section argues against the
deleted-not-repointed premise that the AC rewrite reversed.
