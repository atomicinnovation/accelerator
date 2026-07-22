---
type: plan
id: "2026-07-19-0167-config-command-and-invocation-contract-migration"
title: "Built-in config Command and Invocation-Contract Migration Implementation Plan"
date: "2026-07-19T21:06:21+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0167"
parent: "work-item:0167"
derived_from:
  - "codebase-research:2026-07-19-0167-config-command-and-invocation-contract-migration"
  - "codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"
relates_to:
  - "work-item:0106"
  - "work-item:0107"
  - "work-item:0166"
  - "work-item:0169"
  - "work-item:0180"
tags: [rust, config, cli, skills, invocation-contract, allowed-tools, hooks, store, migration]
revision: "b290d5d94322d663af947227f8c64e7a470b173f"
repository: "build-system"
last_updated: "2026-07-20T11:15:42+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Built-in config Command and Invocation-Contract Migration Implementation Plan

## Overview

Build the `accelerator config` command family into the launcher over the
existing `config`/`config-adapters` crates, extract `atomic_write` into a shared
`store` crate, then cut every config-cluster SKILL.md call site and its
`allowed-tools` rules over to the bootstrap path — and finally delete the bash
removal set.

The phase order is **command first, cutover last**. Phase 0 prepares the
bootstrap; phases 1-4 leave bash authoritative and user-visible behaviour
unchanged, so each merges on its own. Phase 5 flips the contract, phase 6 the
hook, phase 7 deletes.

## Current State Analysis

### What already exists

The Rust side is further along than the work item assumes.

- `cli/` is a 10-crate workspace; 0178, 0179 and 0180 have all landed.
- `cli/config` resolves 55 keys across 6 groups plus 13 doc types
  (`catalogue.rs:203-212`), with a bash-parity drift test that shells out to the
  shell catalogue (`catalogue.rs:343-412`).
- `ConfigAccess::get`/`set` are implemented, including `PathConflict` detection
  and insertion-order-preserving `Mapping::upsert` (`service.rs:96-212`).
  Precedence is personal-over-team, matching `config-read-value.sh:114-124`.
- `FileConfigStore` implements both ports with a body-preserving `document::render`
  and a working `atomic_write` (`config-adapters/src/store.rs:58-118`).
- Byte-exact goldens for the review block already exist at
  `scripts/test-fixtures/config-read-review/{pr,plan,work-item}-mode-golden.txt`.

### What is missing

- **No `config` subcommand.** `Command` has two variants — `Version` and an
  `external_subcommand` catch-all (`launch/inbound/cli.rs:15-22`); `dispatch` has
  two arms (`launch/mod.rs:22-39`). Nothing in `launcher` references `config`.
- **No exit code 2 is reachable.** `kernel::Error` has two variants, both mapped
  to `ExitCode::FAILURE` (`main.rs:110-115`).
- **`catalogue::default_for` has no consumer** — defaults are declaration-only.
- **`ConfigError::NotFound` is defined but never constructed.**
- **No symlink-escape refusal** in either `atomic_write`.
- **No CLI surface** for templates, summary, dump, doc-type-paths, or init.

### Verified measurements (at revision `b290d5d9`)

| Measure | Value |
|---|---|
| `!`-preprocessor config call sites | **247** across **46** SKILL.md files |
| Non-`!` config call sites needing migration | **14** across 5 files |
| Files declaring `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)` | **35** (34 clean + 1 with a trailing space) |
| Files with bare `- Bash` | 11 |
| `grep -rn 'scripts/config-' --include=SKILL.md skills/` total | **297** |
| … of which `config-read-browser-executor.sh` | **1** |
| Non-SKILL.md `.sh` files referencing removal-set scripts | **31** (28 repointed + 3 retained) |
| Distinct referencing files, all forms and file types | **95** |
| `scripts/config-*.sh` | 22 files |
| `scripts/test-config.sh` | 6,289 lines, 21 path bindings, 344 variable references |
| Discoverable suites under `scripts/` | 21 (22 files less `test-helpers.sh`) |

### Key Discoveries

- **The work item's ADR-0021 criterion is wrong and unsatisfiable.** Exit 2
  fires on *opposite* customisation states: `config-eject-template.sh:133-135`
  exits 2 when the override **already exists**; `config-diff-template.sh:36,43`
  and `config-reset-template.sh:60,67` exit 2 when there is **none**. ADR-0021:80
  defines the code as "destructive action requires confirmation", which fits
  `eject` and is overloaded by the other two. One fixture cannot serve all three.
- **Exit 2 collides with clap.** clap 4 exits **2** on usage errors, and
  `main.rs:107` delegates to `error.exit()`. Without interception, a mistyped
  template name would be indistinguishable from "confirmation required".
  The bash exits **1** on usage errors, so this is a regression the port
  introduces.
- **`_EXPECTED_CONFIG_SUITES` is a `<` floor** (`tasks/test/integration.py:85`),
  not an equality — the research called it "zero headroom". Retiring two suites
  drops discovery to 19 and still fails it, so the constant still moves.
- **Two of the four temp-and-rename sites are not duplicates.**
  `cache.rs:112-127` carries 0600-then-chmod+x publication semantics plus a
  paired signature file; `lock.rs:106-117` renames a *directory* as a lock claim.
- **`config_extract_frontmatter` returns 1 for both "no frontmatter" and
  "unclosed"** (`config-common.sh:74-86`); callers disambiguate with a separate
  `head -1 | grep -q '^---'`. Only the unclosed case warns.
- **Two observable bash defects** that byte-exact goldens would freeze:
  `config-read-review.sh:270` emits a **double slash** in custom-lens paths
  (`…/lenses/foo//SKILL.md`), which reaches the Lens Catalogue table and
  `custom_lens_paths` (`:306`) but **not** the three stderr warnings at
  `:277,282,287` — those interpolate `$lens_dir` (one trailing slash from the
  `*/` glob), not `$skill_file`, so fixing `:270` leaves them unchanged; and
  `config-summary.sh:20-22` resolves the init sentinel
  relative to **CWD, not project root**, so the summary reports an initialised
  repo as uninitialised when run from a subdirectory.
- **`test-init.sh` fails today.** `init.sh:18-39` creates 14 content directories
  including `meta/research/codebase`; the suite expects 12 including a bare
  `meta/research`.
- **The removal set has 28 non-SKILL.md production consumers.** Six work-item
  scripts, four visualiser scripts, four Jira and two Linear scripts, all six
  migrations, four shared `scripts/` helpers (`doc-type-inference`,
  `doc-type-table`, `validate-corpus-frontmatter`, `work-common`),
  `adr-next-number.sh` and the Playwright `run.sh` invoke removal-set scripts by
  path. **`jira-auth.sh` and `linear-auth.sh` resolve the credential through
  `config-read-value.sh`**, so deleting it breaks authentication for both
  integrations. A SKILL.md-scoped grep cannot see any of this.
- **0178 already dropped the migration bypass, and repointing the migrations
  detonates it.** `config_assert_no_legacy_layout` returns early under
  `ACCELERATOR_MIGRATION_MODE=1` (`config-common.sh`), which
  `run-migrations.sh:632` sets for every migration. The Rust deliberately does
  **not** honour it — a shipped 0178 acceptance criterion, pinned by
  `config-adapters/tests/config_reader.rs`'s `run_with_migration_mode` helper and
  `a_legacy_layout_exits_non_zero_with_the_migrate_directive`. The six migrations
  `0001`-`0006` read config directly (`config-read-value.sh` ×5, `config-read-path.sh`
  ×5, `config-read-all-paths.sh` ×1) and `0007` reads it transitively through
  `doc-type-table.sh` — **seven in total** — so once repointed, every one refuses
  on exactly the legacy-layout repos `/accelerator:migrate` exists to fix. The
  tool that repairs the layout would be blocked by the layout.
- **The per-skill customisation root is settled by the tree**:
  `.accelerator/skills/<skill>/` (`config-read-skill-context.sh:23`). ADR-0020's
  `.claude/accelerator/…` is stale; ADR-0047 wins.

## Desired End State

`accelerator config <subcommand>` is the only config reader in the product. Every
config-cluster SKILL.md call site invokes it through
`${CLAUDE_PLUGIN_ROOT}/bin/accelerator`, covered by an `allowed-tools` rule in
the same commit, and so does every one of the 28 non-SKILL.md shell consumers.
The `config-detect` hook registration calls the bootstrap path directly. The
removal set is deleted, `atomic_write` exists once in a `store` crate with a
symlink-escape refusal, and `mise run` is green.

Verified by: `mise run` exits 0 with the removal set deleted; Grep A returns 0;
Grep B returns 1; the permission-coverage script exits 0 at every commit in the
migration range.

## What We're NOT Doing

- Migrating `config-read-browser-executor.sh` (0173 owns it) — only re-homing its
  permission coverage.
- Retiring `scripts/config-common.sh` (0174 owns it) — 18 non-cluster production
  sourcers survive.
- Migrating `vcs-detect`, `vcs-guard` (0169) or `migrate-discoverability` (0172).
- Building 0107's lint guard — migrate-then-build is recorded on 0107 instead.
- Migrating `artifact-*` or any other non-config script family (0173).
- Folding the launcher cache publisher or the lock's directory rename into
  `store` — both are allowlisted exceptions.
- The visualiser fold-in (0168).

## Implementation Approach

Eight phases, each independently mergeable and green on its own.

Phase 0 touches only the bootstrap and the release profile. Phases 1, 3 and 4
add Rust and leave bash authoritative — no config-cluster call site changes, so
they can sit in `main` indefinitely. **Phase 2 is the exception**: it changes the
shipped launcher's usage-error exit code from clap's 2 to 1 (see §3), a
launcher-wide contract change that affects `version` and every external
subcommand, not just `config`. A release cut from `main` between phases 2 and 5
therefore ships that changed contract before any call site exercises the new
`config` surface — recorded so the phase-independence property is not read as
"every phase is behaviour-neutral". The exit-code interception could instead be
deferred to Phase 5, where the contract flip is already the theme; the choice is
noted as an open question. Phase 4 is the parity gate:
`test-config.sh` is repointed at the compiled binary via generated shim scripts,
which handles `bash "$VAR"`, bare-exec and `source` forms uniformly with one
edit per binding.

**What that gate can and cannot prove.** `test-config.sh` carries 258 `grep -q`
substring checks against 338 `assert_` calls — the custom-lens assertions at
`:1382` are `grep -q "| compliance |" && grep -q "| custom |"`, which a port
emitting a different path, column spacing or row order would still pass. Roughly
half the suite therefore cannot detect rendering drift, which is precisely the
defect class a bash-to-Rust output port introduces. The repointed suite is a
**behavioural** gate; the byte-exactness claim is carried by the goldens, and the
goldens must cover the non-trivial states rather than the baseline alone.

Phase 5 flips the invocation contract, phase 6 the hook, and phase 7 deletes.

**Divergences from the bash, all deliberate and recorded** (Phase 2 writes them
into a `meta/` divergence note, referenced by the inventory):

1. `config_assert_no_legacy_layout` is applied **uniformly** to every subcommand.
   The bash applies it to only 7 of 20 scripts; `ConfigError::LegacyLayout`
   already exists and gating everywhere is strictly safer. The single exception
   is an explicit `--allow-legacy-layout` flag, accepted on the **read**
   subcommands (rejected on writes), passed directly by migrations `0001`-`0006`
   and via the allowlisted `doc-type-table.sh` for `0007` — see Phase 2.
2. The `config-read-review.sh:270` double slash is **fixed**, not frozen.
3. The `config-summary.sh:20-22` CWD-relative sentinel is **fixed** to resolve
   against project root.
4. Usage errors exit **1**, not clap's 2. This removes clap's collision with the
   refusal code; it does **not** give exit 2 a single meaning. `eject` exits 2
   when the override *already exists*, `diff`/`reset` when there is *none* — two
   opposite triggers, preserved from the bash because matching the shell contract
   is worth more than the tidier taxonomy. Exit 2 is documented as a
   **subcommand-scoped, caller-actionable refusal**, and the kernel variant is
   named for that rather than for confirmation alone.
5. Fail-safe is an explicit **`--fail-safe` opt-in**, matching luminosity's
   `OnFailure::{Fail, Degrade}` rather than being implicit. It degrades
   **read/IO failures only** — an unreadable or absent config file, a
   legacy-layout refusal, a filesystem error. It does **not** override a
   subcommand's **validation refusal** (a bad `work.integration` enum, a
   doc-type escape, an unresolvable `template`): those stay fail-closed with the
   flag present, because a validation failure is a caller error the caller must
   see, not a transient read failure to paper over. This split is the load-bearing
   distinction — mandating the flag on every `!` site (Phase 5 §5) without it
   would let a misconfigured `work.integration` degrade to empty-and-exit-0, which
   `list-work-items/SKILL.md:30` reads as "no integration configured".

   **The degraded shape is defined per `!`-reachable subcommand**, not left to be
   invented per command:

   | Subcommand(s) | `--fail-safe` on a read/IO failure |
   |---|---|
   | `agents`, `context`, `instructions`, `review` | render the `## <Name> Unavailable` block on stdout, exit 0 |
   | `summary` | emit **nothing**, exit 0 (see below) |
   | `get`, `path`, **`work`** (single-value) | empty stdout, exit 0 |
   | `dump`, `paths`, `template`, `templates list` (multi-line blocks) | render a `## <Name> Unavailable` block on stdout, exit 0 |
   | any subcommand, **validation refusal** | fail-closed regardless of the flag: non-zero, empty stdout, diagnostic on stderr |

   **`work` degrades by suppression, not by notice**, despite being a block
   subcommand for its enum validation: `config work integration` is consumed as a
   single-value presence-probe — `list-work-items/SKILL.md:30`, `sync-work-items`
   and `create-work-item` branch on empty = "not configured" — so a
   `## Work Unavailable` notice would read as "configured" and steer a subsequent
   sync/`--integration` operation. Its *validation* refusal (a bad enum) still
   fails closed per the last row; only the read/IO failure suppresses. This is the
   same reasoning as the scalars.

   The five notice headers are named exactly: `## Agent Names Unavailable`,
   `## Project Context Unavailable`, `## Skill-Specific Context Unavailable`,
   `## Skill Instructions Unavailable`, `## Review Configuration Unavailable`.
   `context --skill` emits both the project and skill-specific blocks, so a
   per-source read failure yields the healthy block plus the other's notice — the
   `## Skill-Specific Context Unavailable` header is the fifth, for the skill block
   the project-only header set would otherwise leave unnamed.

   **`summary` degrades by suppression, not by notice** — `summary --fail-safe`
   emits **nothing** and exits 0. Its output is wrapped by `--format=hook` into an
   `additionalContext` envelope, which has no slot for a notice that is not itself
   injected context — rendering one would inject an error block into every
   SessionStart, where `config-detect.sh:14`'s `|| SUMMARY=""` makes such a session
   silently context-free today.

   **Scalar commands (`get`, `path`) degrade by suppression** — empty stdout,
   diagnostic on stderr, exit 0 — matching the bash's warn-and-emit-empty. They
   are the bulk of the 247 sites, and a notice string injected where a skill body
   expects a path could steer a subsequent write or corpus scan.

   `--explain` writes **resolution provenance** to stderr — which level supplied
   each value and which files were read — without touching stdout. Stated
   concretely because the plain stderr diagnostic is already guaranteed in both
   modes, so a flag that only promised "the diagnostic on stderr" would be
   indistinguishable from the default and satisfied by a no-op; its criterion
   asserts the provenance *content* on a fixture where a key set in `config.md`
   is overridden in `config.local.md`, requiring `--explain` to name both files
   and attribute the winning value to the personal level. The bash has no
   equivalent of either flag.

   Opt-in rather than default because the 28 repointed shell consumers want loud
   failures — `jira-auth.sh` reading a token must not receive a degradation
   notice and treat it as a credential — while only the `!`-preprocessor call
   sites want degradation. Making it explicit puts the choice where the caller's
   requirement is visible. Every `bin/accelerator config` call site rewritten in
   Phase 5 carries the flag, and `check-skill-permissions.sh` asserts it.
6. **Fail-open output is net-new.** No removal-set script emits an
   `## Agent Names Unavailable` header — `config-read-agents.sh:117-124` emits its
   full default block regardless, and `config-read-context.sh:26-33` emits
   nothing. The `## … Unavailable` blocks are a new contract, injected verbatim
   into 247 skill prompts, and are recorded here rather than left to look like
   parity.
7. **`paths --doc-types` buffers.** The bash emits each line inside its loop and
   exits 1 mid-iteration (`config-read-doc-type-paths.sh:81-110`), so a fail-closed
   exit leaves a partial prefix on stdout. The Rust buffers all 13 lines and emits
   them only on success, matching `summary`'s existing buffered-write behaviour:
   partial output followed by a failure is a worse contract for a caller than no
   output at all.
8. **Skill and template names are validated as identifiers**
   (`^[a-z0-9][a-z0-9-]*$`) rather than interpolated into paths unchecked. The
   configured `templates.<key>` *value* keeps its existing warn-and-proceed
   behaviour — only the name is constrained.
9. **The inner `.gitignore` is ensured, not created-if-absent**, so a hand-edited
   `.accelerator/.gitignore` still gains the `config.local.md` rule.

The next three are **already shipped by 0178 and already tested** — they are
recorded here because they become user-visible at the Phase 5 cutover, not
because this story introduces them. Each names its existing test.

10. **Malformed frontmatter is fail-loud where the bash degrades**
    (`config-adapters/tests/parity.rs:296` —
    `malformed_frontmatter_is_fail_loud_where_bash_degrades`). The bash skips the
    malformed level and reads the other with a warning; the Rust errors.
    **0178's behaviour is kept, not reversed** — reversing a second shipped 0178
    decision is how the `--allow-legacy-layout` half-restoration happened. The
    blast radius is recorded instead: with the 28 shell consumers deliberately
    carrying no `--fail-safe`, one broken `config.local.md` becomes a hard failure
    across the work-item, doc-type, ADR and visualiser scripts rather than a
    warn-and-default. The error **must name the offending file**, so a wide
    failure is diagnosable in one read rather than bisected.
11. **A block-authored YAML sequence resolves to a populated sequence** where the
    bash reports found-empty (`parity.rs:278`).
12. **Value encodings** — `[a, b]`, `~`, zero-padded, exponent and
    trailing-comment forms — resolve to their declared divergent forms
    (`parity.rs:256`).
13. **`context --skill` inserts one blank line between the project and skill
    blocks** where the two bash scripts, invoked as adjacent `!` sites, abut with
    none. `config-read-context.sh:32` ends with a single trailing newline and
    `config-read-skill-context.sh:30` opens directly with its header, so the
    collapse changes the bytes injected into 42 prompts. Needs a test. (Whether
    today's adjacent-`!` output already carries that blank line depends on
    undocumented preprocessor trailing-newline handling, so the current bytes are
    captured live in the Phase 5 probe session and the divergence stated against
    that capture, not against the scripts' concatenated stdout.)
14. **`init` sources its 14 directories from `catalogue::default_for`** where
    `init.sh` passes explicit `DIR_DEFAULTS` values that win over the catalogue
    (`config-read-path.sh:31-33`) — a source-of-truth change, not a reproduction.
    The values coincide today; a test captured before `init.sh`'s deletion pins
    each `DIR_DEFAULTS` value equal to `catalogue::default_for` (Phase 3 §3).

Each divergence updates the corresponding repointed assertion in the same commit,
with the reason in the commit message, and **names the test that would fail if
the divergence were absent or later regressed**.

Divergences 10-12 already have tests (named above). Divergences 2, 3, 6, 9, 13
and 14 have **none** today and need one written each. Divergence 2's
custom-lens path is asserted nowhere — the custom-lens tests
(`test-config.sh:1371-1520`) check only the name and source columns, and all
three committed goldens contain 31 `| built-in |` rows and zero custom lenses.
Divergence 3 is reachable only from a subdirectory and appears only as a manual
step. A divergence nothing can detect is indistinguishable from a defect.

---

## Phase 0: Bootstrap development override and staging

### Overview

`bin/accelerator` serves only signed release artefacts, so nothing built locally
can be exercised through it. Every verification step from Phase 5 onwards —
`test-configure-round-trip.sh`, the live `/accelerator:configure` checks, Phase
6's hook equivalence capture — invokes the bootstrap path, so without an override
they cannot be run at all until a signed release carrying `config` exists.

Smaller bootstrap changes ride along: the verify shim is staged non-atomically
and re-copied on every call, which becomes both a race and a per-call cost once
the call-site count moves from zero to 247; the cache lock gives up after 30s
while the fetch it waits on is bounded at 300s, so a slow-but-succeeding cold
fetch makes concurrent waiters fail needlessly; and the release profile is unset,
so the launcher the bootstrap hashes on every invocation is larger than it needs
to be.

The bootstrap's **fail-closed posture on the paths above the launcher is left
unchanged, deliberately**: eleven `fail()` conditions (unset `CLAUDE_PLUGIN_ROOT`,
missing downloader, noexec cache, fetch/verify failure, lock timeout, …) exit
non-zero before `exec`, and after Phase 5/6 that is a hard outage for all 46
skills and every SessionStart rather than a degradation. This is accepted — a
dead binary cripples the plugin regardless, and injection-safe distribution is
`bin/accelerator`'s design, owned by 0164/0165, not this story's to reshape. Two
consequences are recorded rather than closed here: the downloader is resolved
unconditionally at `:47-62` *before* the cache-hit check, so a warm verified
cache plus no curl/wget fails needlessly (a cheap `bin/accelerator` fix for
0164/0165 — move resolution into the miss branch); and `--fail-safe` cannot cover
any of these, since it is a launcher flag parsed after `exec`.

### Changes Required

#### 1. Local-build override

**File**: `bin/accelerator`, `.gitignore`
**Changes**: `.gitignore` gains `/.accelerator-dev-launcher` (the marker's
load-bearing property is that it is untracked and unshippable, so the ignore rule
is part of this change, not prose). `ACCELERATOR_LAUNCHER_BIN` names a launcher to
exec directly, skipping fetch, cache and verification. It is honoured **only**
when all three:

1. `ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER=1` is set;
2. an **untracked, gitignored** marker file
   `${CLAUDE_PLUGIN_ROOT}/.accelerator-dev-launcher` exists; and
3. the named binary resolves to a non-symlink regular executable whose
   **canonical** path is contained in the **canonical**
   `${CLAUDE_PLUGIN_ROOT}/cli/target/`.

It always emits a one-line stderr warning naming the path **and** appends a
timestamped record — path plus invoking PID — to
`${cache_dir}/.accelerator-unverified.log`, so an engaged override leaves a
durable trace (the stderr warning alone is invisible at SessionStart, where
stderr is not surfaced, and at the 247 `!` sites, whose `--fail-safe` contract
routes stderr away from the prompt).

**The marker is the load-bearing half**, and it is anchored to
`CLAUDE_PLUGIN_ROOT` — which the bootstrap already validates at `:19-21` — not to
a discovered project root. Environment variables alone are not adequate: after
Phase 5 this path is pre-authorised in 35 SKILL.md files and, after Phase 6,
invoked unattended at every SessionStart, and the Bash tool holds a persistent
shell, so a single `export` or an ambient `.envrc`/`mise.toml` block would
otherwise convert every subsequent prompt-free invocation into attacker-chosen
execution.

**Why this marker and not the earlier designs.** The `jira-auth.sh:196-203`
precedent uses a VCS-*tracked* marker; that does not transfer, because the
override's purpose is contributor workflow on *this* repository, so a tracked
marker would be committed to accelerator itself — present in every clone and
install, collapsing the gate to the single environment variable. Anchoring the
marker to a CWD-discovered root fails too: it is selectable by whoever selects
the CWD, and under jj the working-copy auto-snapshot makes a just-created file
tracked by the time the probe runs. The marker used here is neither: it is
**untracked and gitignored** (so no clone or marketplace install can carry it —
`.gitignore` lists it, and it is never committed), and **anchored to
`CLAUDE_PLUGIN_ROOT`** (so it is not CWD-selectable). A contributor creates it
once. Its shipped state is *absent*, and its absence, not its presence, is what
the release carries.

**Factor 3 must canonicalise, not string-prefix.** Under the bash 3.2 floor
there is no `readlink -f`, so containment is checked by resolving the leaf's
parent (`cd "$(dirname "$bin")" && pwd -P`) and comparing it against
`cd "${CLAUDE_PLUGIN_ROOT}/cli/target" && pwd -P`, plus an `-L` test on the leaf
itself — a string-prefix test would admit `${…}/cli/target/link/payload` where
`link` is a symlink out, and a path containing `..`. There is an unavoidable
check→exec TOCTOU: the binary could be swapped between the containment check and
the `exec`. This is accepted, because engaging the override already requires
write access to `cli/target/` and the two env vars — an attacker with that
capability can edit `bin/accelerator` itself — but it is recorded rather than
left implicit.

Unlike `ACCELERATOR_BOOTSTRAP_DOWNLOADER` and `ACCELERATOR_UNAME_S`/`_M`, which
substitute a *step*, this skips the whole verification chain — so it is
documented separately in the header comment rather than alongside them. The
override is a `bin/accelerator` concern that 0164/0165 own; it is added here only
because Phase 5+ verification needs it, and its gate design should be reviewed
against those stories rather than treated as settled by this one.

#### 2. Content-addressed shim staging

**File**: `bin/accelerator`
**Changes**: `:107` copies the 475KB verify shim onto a fixed path on every
invocation, before the cache-hit check and outside the lock (`:181` is taken only
on the miss path) — a 475KB write plus two forks on every warm call, about to be
multiplied by 247 call sites, and non-atomic so a concurrent reader can exec a
partial file. Stage instead to a **content-addressed** path
`${cache_dir}/accelerator-verify-${platform}-${sha}`, where `${sha}` is the
vendored shim checksum the repo already commits at
`bin/accelerator-verify.vendored.sha256`.

**Skip-if-exists must verify, not trust the path.** A pre-existing staged shim is
used **only after** its actual SHA-256 (via the existing `scripts/hash-common.sh`)
matches `${sha}`; on a mismatch or absence the shim is re-staged from the tracked
`bin/accelerator-verify-${platform}` source. Trusting a content-addressed file by
existence alone would be a regression, not an optimisation: today's
unconditional re-copy is the only thing that overwrites a planted stub before it
is used to check the launcher's minisign signature, so skip-if-exists-without-hash
would let an attacker who pre-populates the path install their own verifier. The
hash check preserves the per-warm-call saving (a fast hash of a 475KB file, no
copy) while keeping the re-copy's integrity guarantee, and a uniquely-named file
is still never overwritten in place, so the torn-file race is closed too.

**`ACCELERATOR_CACHE_DIR` is gated in this story, not deferred.** That variable
(`:90-93`) relocates both the staged shim and the verified launcher to a
caller-chosen directory, so it is the same env-var-injection capability the
override's marker gate defends against — and with content-addressed staging it
would otherwise let a planted shim be trusted by path. It is honoured only under
the same opt-in as the override (`ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER=1` plus
the dev-launcher marker); absent the opt-in, the cache dir is the default and the
shim is always hash-verified before use. This is the minimal change that closes
the stage→exec window; the broader `bin/accelerator` hardening remains 0164/0165's,
but a verification bypass introduced *by this story's own staging change* is this
story's to not open.

#### 2b. Lock ceiling

**File**: `bin/accelerator`
**Changes**: the cache lock gives up after 300 iterations of `sleep 0.1` (30s,
`:139-144`) while the fetch is bounded at `--max-time 300` (`:53`). Raise the
lock ceiling above the fetch timeout, or have waiters detect an in-progress fetch
via the lock's `pid` file and extend rather than abort, so a single skill load's
up-to-17 concurrent cold-cache invocations do not cascade into hard failures
while the one fetch is still progressing.


#### 3. Release profile

**File**: `cli/Cargo.toml`
**Changes**: add `[profile.release]` with `strip = true` and `lto = "thin"`. The
bootstrap hashes the whole launcher on every invocation, so binary size is a
per-call latency term, and it is also the cold-fetch payload.

`strip = true` trades symbol names in a panic backtrace for size; `lto = "thin"`
trades release build time for size. Because the same binary is now on the
critical path of 247 invocations and every SessionStart — and after Phase 7 has
no bash equivalent to diff a field report against — `strip = true` is paired with
`split-debuginfo` and a published debug archive keyed to the release version (the
pipeline already runs `build.create_debug_archives`), so a field backtrace can be
symbolised after the fact. All three are recorded as deliberate.

### Success Criteria

#### Automated Verification

- [x] With the opt-in variable set and the marker present, the bootstrap execs
      the named binary, emits the warning on stderr, appends the record to
      `.accelerator-unverified.log`, and performs no fetch
- [x] With `ACCELERATOR_LAUNCHER_BIN` set but the opt-in variable absent, the
      override is ignored and the normal verified path runs
- [x] The override is refused when the marker is absent — asserted against a
      **pristine clone at a release tag** (whole tree present, no marker), so the
      refusal is tested in the shape a real install takes, not a synthetic fixture
- [x] `.accelerator-dev-launcher` is listed in `.gitignore` (asserted by a
      committed test) and is untracked — so no clone or marketplace install carries
      it, and jj's auto-snapshot cannot make it tracked
- [x] The override is refused when the named binary is a symlink, is not
      executable, is reached through a **symlinked ancestor directory**, or whose
      canonical path contains `..` or resolves outside the canonical
      `${CLAUDE_PLUGIN_ROOT}/cli/target/`
- [x] A tampered cached launcher is still refused — the override changes nothing
      about the verification path when it is not engaged
- [x] A **planted or tampered staged shim is refused**: a content-addressed shim
      whose bytes do not match **the source shim's own SHA-256** is re-staged from
      the tracked source rather than trusted, so a stub pre-written to the staging
      path (including via `ACCELERATOR_CACHE_DIR`) never becomes the verifier.
      **Deviation (approved): the content address and skip-if-exists check key on
      the source shim's digest, not `bin/accelerator-verify.vendored.sha256` —
      that file is a cli/verify build-input marker, not a shim content hash.**
- [x] `ACCELERATOR_CACHE_DIR` **remains ungated (approved deviation from §2):** a
      caller-set cache dir always hash-verifies the staged shim against the source
      before use, so a planted shim there is re-staged not trusted — the gate the
      plan proposed protected only the broken trust-by-name design and would have
      broken read-only-root installs. Launcher verification is always on regardless.
- [x] N concurrent bootstrap invocations against a **warm** cache all succeed; no
      run fails shim verification
- [x] N concurrent bootstrap invocations against a **cold** cache with a slow
      injected downloader all succeed — no waiter fails the lock while the fetch
      is still progressing (waiters that see a live owner extend rather than abort)
- [x] `mise run check` and `mise run` exit 0

#### Manual Verification

- [ ] Launcher binary size recorded before and after the profile change, and the
      per-invocation `verify_launcher` cost recorded at each size.
      **Recorded: stripped release launcher (darwin-arm64) = 6,085,776 bytes with
      `strip = true` + `lto = "thin"`.** `split-debuginfo` was **dropped
      (deviation from §3): it is inert without `debug > 0`, and making it
      meaningful — enabling debug, validating `.dSYM`/`.dwp` under zigbuild musl
      cross-compile, and wiring the launcher into `create_debug_archives`
      (currently visualiser-only) — is release-pipeline work overlapping 0165.
      Recorded as a follow-up alongside Phase 7's launcher-size regression check.**

---

## Phase 1: The `store` crate and `atomic_write` consolidation

### Overview

Extract one `atomic_write` primitive with a permitted-root symlink refusal, and
commit a check that fails on a reintroduced duplicate. Touches no CLI surface.

### Changes Required

#### 1. New workspace member

**File**: `cli/Cargo.toml`
**Changes**: add `store` to `members`.

**File**: `cli/store/Cargo.toml`, `cli/store/src/lib.rs`

`store` is an **infrastructure** primitive (Phase 1 §4 confirms it needs no
inward pup rule), depending on `std`, `tempfile` and `rustix` only. It does **not**
depend on `kernel`: both consumers translate `WriteError` into their own taxonomy
(`ConfigError`, `corpus::StoreError`), each of which already has its own
`From … for kernel::Error`, so a `From<WriteError> for kernel::Error` would be
dead code and an unnecessary crate-graph edge.

The type is named `WriteError`, **not** `StoreError`: `corpus::StoreError`
already exists (`cli/corpus/src/store.rs:14-20`) as the return type of the
`AtomicWrite` and `RecordStore` ports, and `corpus_domain_imports_only_permitted`
(`pup.ron:57-72`) allows corpus to name only std, `kernel::Error` and `crate` —
so corpus can never adopt this one. Two same-named error enums with divergent
variant sets, one crate apart, would be confusable at every call site.

`WriteError` carries `#[derive(Debug, Clone, PartialEq, Eq)]` and
`#[non_exhaustive]`, with **hand-written** `Display` and `std::error::Error`
impls — `thiserror` is not in the dependency set, so it is written out as
`corpus::StoreError` (`corpus/src/store.rs:22-45`) already does. Every public
`Result`-returning fn carries a `# Errors` doc section, which `clippy::pedantic`
(denied workspace-wide) requires.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WriteError {
    NotWritable { path: String },
    CrossFilesystem { path: String },
    UnsafePath { path: String },
    Io { path: String, detail: String },
}

/// How the persisted file's mode is chosen. The caller supplies any concrete
/// mode value, so mode resolution here never reads the process-global umask.
pub enum NewFileMode {
    /// Force this mode whether or not the target exists — personal
    /// `config.local.md` is `Set(0o600)`, clamped rather than preserved.
    Set(u32),
    /// Preserve an existing target's mode; create a fresh file at this mode —
    /// team `config.md` is `PreserveOr(0o666 & !umask)`, the umask read by the
    /// composition root, not here.
    PreserveOr(u32),
}

/// The trusted roots bounding a write. `permitted_root` bounds the target;
/// `project_root` is the independently-discovered root a symlinked
/// `permitted_root` must resolve inside.
pub struct WriteBounds<'a> {
    pub permitted_root: &'a Path,
    pub project_root: &'a Path,
}

/// Whole-file atomic replacement: a reader never observes a partial file.
///
/// # Errors
/// Returns `WriteError` on containment refusal, cross-filesystem staging, an
/// unwritable target, or any underlying I/O failure.
pub fn atomic_write(
    path: &Path,
    bytes: &[u8],
    bounds: &WriteBounds<'_>,
    mode: NewFileMode,
) -> Result<(), WriteError>;
```

**The mode is set on the temp before `persist`, never post-`rename`.** `rename`
replaces the inode, so without care the target takes `NamedTempFile`'s 0600 —
silently changing a committed, team-shared `.accelerator/config.md` on every
write; and a post-rename `chmod` in the caller leaves a window in which a
credential file exists at the wrong mode. `atomic_write` applies `mode` to the
staged temp before persisting, the pattern `cache.rs:112-127` already uses. The
two config levels pass:

- **personal** (`config.local.md`) — `Set(0o600)`. Created fresh at a
  umask-derived 0644 it would break Jira and Linear authentication immediately
  (`jira-auth.sh:21,28-30` fails closed with `E_LOCAL_PERMS_INSECURE (29)` above
  0600); and preserving a *pre-existing* 0644 would write a secret into a
  world-readable file, with the existing check guarding only the subsequent read.
  `Set` (not `PreserveOr`) is what makes a pre-existing 0644 become 0600.
- **team** (`config.md`) — `PreserveOr(0o666 & !umask)`. A shared, committed file
  created 0600 by whoever ran `config set` first would be unreadable to every
  other user on a shared checkout or CI image; preserving an existing mode keeps a
  team's chosen permission, and a fresh file lands umask-derived.

The Rust reader mirrors the auth scripts' **symlink refusal** for the personal
level: `jira-auth.sh:189-192` refuses a symlinked `config.local.md`, so `get`,
`dump` and the personal write refuse one too rather than following it.

Placement is **same-directory temp** via `NamedTempFile::new_in` with an explicit
`prefix(...)` (so the gitignore rule that matches it cannot silently stop matching
on a `tempfile` bump), adopting `corpus-adapters`' shape over `config-adapters`'
`tmp` subdirectory: a `tmp` on a different mount would silently degrade `rename`
to a copy.

The refusal follows luminosity's `refuse_escaping_path`
(`…/luminosity/cli/config-adapters/src/store.rs:108-130`), with the ordering made
explicit because the naive reading has two holes:

1. **Canonicalise the nearest *existing* ancestor**, verify containment, and only
   then `create_dir_all` the remainder — **inside `atomic_write`**, not in the
   adapter's `stage`. Canonicalising a not-yet-existent parent fails; doing it
   after creation means the chain was already built through whatever symlink was
   being checked. `stage`'s own `create_dir_all` (`corpus-adapters/src/store.rs:59`)
   is removed so directory creation happens only after containment is verified; the
   config permitted root (`<project>/.accelerator`) does not exist on a fresh
   repository, so a blanket "absent root is `Ok`" would disable the guard on
   precisely the first write. **`append_record`'s pre-lock `create_dir_all`
   (`:111-113`) is kept**, because the mkdir-based lock (`lock.rs`, `create_dir`
   not `create_dir_all`) requires the parent to already exist before `lock::acquire`
   at `:114`, and `atomic_write` is only reached at `:125` — removing it would make
   `append_record`/`remove_by_key` fail on a not-yet-existent nested corpus parent.
   That pre-lock ensure is itself routed through the containment check (a guarded
   `ensure_within` helper) so it does not reopen hole 1.
2. **Refuse a permitted root that is itself a symlink** resolving outside
   `bounds.project_root`. Canonicalising the root as trusted otherwise follows it,
   and git stores symlinks — a cloned repository can ship `.accelerator` as one,
   passing the check by construction. This is why `project_root` is a distinct
   parameter: a guard given only `permitted_root` has nothing to compare a
   symlinked root against.

The `stage`/`persist` split from `corpus-adapters/src/store.rs:53-70` is
preserved as the fault-injection seam.

#### 2. Repoint both callers

**File**: `cli/config-adapters/src/store.rs`
**Changes**: delete the private `atomic_write` method (`:58-80`);
`WriteConfigLevel::write` calls `store::atomic_write` with a `WriteBounds` whose
`permitted_root` is `self.config_dir()` and whose `project_root` is the discovered
root, and the per-level `NewFileMode` (`Set(0o600)` for personal,
`PreserveOr(0o666 & !umask)` for team, the umask read here in the adapter/composition
root, not in `store`). `ConfigError` gains an `UnsafePath { path }` variant.

**Implemented divergence (2026-07-20).** The umask read was hoisted into a
shared **`store::current_umask()`** — rustix-based, no `unsafe` — rather than
copied into each adapter; `store` is the only crate both adapters share, and
`atomic_write` still never reads the umask (the caller composes the mode and
hands it in), so the determinism this boundary protects is preserved and only
the query utility is co-located. Both adapters call it, and `libc` is dropped
from `store` and both adapters in favour of rustix (`process::umask` for the
mask, `io::Errno::XDEV` for the cross-filesystem classification). The workspace
`rustix` dependency gains the `fs` feature (`umask` takes a `rustix::fs::Mode`).

The `store::WriteError → ConfigError` translation is a **private free function**
in `config-adapters` (`fn to_config_error(error: store::WriteError) -> ConfigError`),
applied at the single `write` call site — **not** a `From` impl, mirroring the
corpus-side `to_store_error`. A `From<WriteError> for ConfigError` has no legal
home: inside `config` it would require `config` to import `store`, which
`config_domain_imports_only_permitted` denies; inside `config-adapters` both types
are foreign, so it is an orphan-rule (E0117) violation; and inside `store` it would
require `store` to depend on `config`, contradicting its declared dependency set.

The `.accelerator/tmp` directory is no longer created for writes.

Same-directory temps therefore land in `.accelerator/`, whose `.gitignore`
carries only `config.local.md` — the `*`/`!.gitkeep` pattern lives in
`.accelerator/tmp/.gitignore` and no longer applies. Under jj the working copy is
auto-snapshotted, so a temp orphaned by a SIGKILL would be committed without
anyone acting.

**The `.accelerator/.gitignore` temp-name rule must be ensured on the write path,
for both levels, in this phase.** The real-repo writer of that file is
`accelerator_ensure_inner_gitignore` (`scripts/accelerator-scaffold.sh:19-27`),
which writes only when the file is absent — so an already-initialised repo (which
never re-runs `init`) would stage un-ignored temps forever, and Phase 3's Rust
`init` is not on the invocation path until later. `scripts/accelerator-scaffold.sh`
is therefore **added to this phase's change list**: `accelerator_ensure_inner_gitignore`
becomes ensure-not-create (a `grep -qFx` guard matching the root-rule shape) and
adds the pinned temp prefix. Independently, the Rust write path (`config set`,
both levels) ensures the rule before writing and fails closed if it cannot — so a
team-level write or a `templates eject` in a pre-existing repo cannot orphan an
un-ignored temp.

**File**: `cli/corpus-adapters/src/store.rs`, `cli/corpus/src/store.rs`
**Changes**: the free `atomic_write` (`:48-51`) delegates to `store::atomic_write`
with a `WriteBounds` rooted at the corpus root and `NewFileMode::PreserveOr(0o666
& !umask)`; `stage`/`persist` move to `store`.

`corpus::StoreError` gains an **`UnsafePath { path }`** variant — it is
`#[non_exhaustive]`, so this is additive. Without it the refusal collapses into
`Io` on the corpus side, defeating the reason `permitted_root` was generalised
rather than left as config-specific logic.

**Open question — generalise now or scope to config?** `FileCorpusStore` has no
production composition root today (below), so the corpus-side guard cannot be
shown to constrain anything until 0180's consumers land, and its refusal test
must therefore pass a `project_root`/`permitted_root` that is **not** the
directory under test (a test that writes into its own permitted root proves
nothing). The generalisation is kept — the API break is small and 0180's
consumers are near — but the alternative (scope `permitted_root` to the config
caller now, let corpus adopt it when it acquires a real root) is recorded as the
fallback if the corpus construction sites prove more disruptive than expected.
Per §1 hole 1, `stage`'s own `create_dir_all` moves inside the guarded
`atomic_write`, while `append_record`'s pre-lock `create_dir_all` is kept (the
mkdir-lock needs the parent first) and routed through the containment check; the
lock's directory rename-as-claim (`lock.rs`) stays the allowlisted exception it
already is.

The translation is a **private free function** in `corpus-adapters`
(`fn to_store_error(error: store::WriteError) -> corpus::StoreError`), applied at
the two call sites — **not** a `From` impl. Both types are foreign to
`corpus-adapters`, and `corpus` cannot name `store` under
`corpus_domain_imports_only_permitted`, so a `From` impl has no legal home under
the orphan rule and would fail to compile.

**The root has to be plumbed, and that is a constructor change.**
`AtomicWrite::write(&self, path, bytes)` carries no root and `FileCorpusStore`
holds only `LockOptions` (`:18-20`), with `with_lock_options` a `const fn`.
`RecordStore::append_record` and `remove_by_key` need it too, so `new()` takes
the root and `with_lock_options` loses `const`. The ripple is:

- `impl Default for FileCorpusStore` (`:36-40`) delegates to `new()` and cannot
  survive `new()` gaining a required parameter — it is **deleted**, which is a
  public-API change any 0180 consumer constructing it that way must absorb.
- every `FileCorpusStore::new()` construction site (eight in
  `corpus-adapters/tests/store.rs`, one in the unit tests at `src/store.rs`) is
  updated, and `with_lock_options` gains a second parameter. The `stage` /
  free-`atomic_write` / `classify_persist_error` unit tests (`src/store.rs`) move
  to the `store` crate alongside the primitive they exercise.

`FileCorpusStore` has **no production composition root today** — it is
constructed only in tests — so the corpus-side containment guard cannot be shown
to constrain anything until 0180's consumers land, and the corpus refusal test
must pass a root that is **not** the directory under test or it verifies nothing.
See the open question on whether `permitted_root` should be generalised now or
scoped to the config caller until corpus acquires a real root. Recorded here so
the ripple to construction sites is not mistaken for scope creep.

#### 3. The duplicate check

Per ADR-0048 (Python is the test language for the non-Rust surfaces), the guard
is **Python, not a bash script**.

**File**: `tasks/lint/store_duplication.py` (new)
**Changes**: a `violations(root)` scan flagging a temp-then-rename shape
(`fs::rename(`, `NamedTempFile`, `.persist(`) under `cli/**/src` outside
`cli/store/`, behind a `check` invoke task that fails on any finding. Two
allowlisted exceptions, each with the reason inline:

```
cli/launcher/src/launch/outbound/resolve/cache.rs  # 0600 publish + paired signature
cli/corpus-adapters/src/lock.rs                    # directory rename-as-claim, not a write
```

**File**: `mise.toml`, `tasks/__init__.py`, `tasks/lint/__init__.py`
**Changes**: register `lint:store-duplication:check` and add it to `cli:check`
(mirroring `lint:vendor-shims:check`) so `mise run check` runs it.

**File**: `tests/unit/tasks/test_store_duplication.py` (new)
**Changes**: the known-positive proof as a unit test — a planted temp-rename
outside `cli/store/` is flagged; the store crate and the two allowlisted renames
are not; the real `cli/` tree is clean.

#### 4. Enforcement configs

**File**: `cli/pup.ron`
**Changes**: `store` is infrastructure and needs no inward rule of its own. Confirm
the existing whole-crate domain rules (`^config($|::)`, `^corpus($|::)`,
`^vcs($|::)`) still deny `store` — they allow only std, `kernel::Error` and
`crate`, so they already do. Add a regression test asserting a domain crate
importing `store` is rejected.

**File**: `cli/deny.toml`
**Changes**: `tempfile` and `rustix` are already in the graph via `corpus-adapters`;
confirm no new licence or `[bans]` entry is needed rather than assuming.

### Success Criteria

#### Automated Verification

- [x] Workspace builds: `cd cli && cargo build --workspace`
- [x] All Rust tests pass with CI's feature set: `mise run test:unit:cli` (which
      runs `cargo nextest --workspace --all-features`, enabling `bash-parity`) —
      plain `cargo test --workspace` omits the feature-gated suites this migration
      must keep green. (`cli:test` is not a task; `cli:check` is format+lint only.)
- [x] `mise run cli:check` exits 0
- [x] The duplicate check flags **both** real duplicates when run against the
      pre-consolidation tree (recorded as its known-positive proof) and exits 0
      after — proven against rev `7a64db2b`: `config-adapters/src/store.rs`
      `fs::rename` + `corpus-adapters/src/store.rs` `NamedTempFile`/`.persist`
- [x] The duplicate check does **not** flag `cache.rs` or `lock.rs`
- [x] Interruption invariant, asserted through the existing `stage`/`persist`
      seam: after a staged-but-not-persisted write the target is **byte-identical
      to its prior contents**, the directory gains no extra entries, and a
      completed write replaces it exactly once. Stated as observable properties
      rather than a recorded syscall sequence — no injected filesystem port
      exists today, and building a syscall recorder behind a ~25-line primitive
      would cost more indirection than the invariant is worth
- [x] After both a successful and a failed write, no temp artefacts remain
- [x] A path whose symlink resolves outside the permitted root is refused on
      **both** read and write
- [x] A target reached through a symlinked ancestor is refused **when the
      permitted root does not yet exist** — the first-write case
- [x] A permitted root that is itself a symlink resolving outside the project
      root is refused
- [x] Under `PreserveOr`, an existing team `config.md`'s mode survives
      replacement, and a fresh one lands umask-derived
- [x] Under `Set(0o600)`, a newly created `config.local.md` is 0600, a
      pre-existing 0600 file stays 0600, **and a pre-existing 0644
      `config.local.md` is 0600 after a personal write** — the case that
      distinguishes clamping from preservation, and the one that keeps a
      credential out of a world-readable file
- [x] The reader refuses a symlinked `config.local.md` (mirroring
      `jira-auth.sh:189-192`) rather than following it
- [x] `mise run check` exits 0

#### Manual Verification

- [x] `cargo tree -p store` shows no dependency beyond std, tempfile, rustix
      (direct deps: `rustix`, `tempfile` only)

---

## Phase 2: Read subcommands, goldens, and the behaviour inventory

### Overview

Every read-only subcommand, its byte-exact golden, and the inventory of
behaviours that no repointed suite can cover. Bash stays authoritative; nothing
outside `cli/` and `meta/` changes.

### Changes Required

#### 1. The command tree

**File**: `cli/launcher/src/launch/inbound/cli.rs`
**Changes**: add a `Config` variant carrying a nested `ConfigAction`, following
luminosity's shape — CLI-layer `Level` as a `ValueEnum` bridged to
`config::Level` by a `From` impl in the same file, so the domain crate stays
clap-free. `--level` is `Option<Level>` on every subcommand that takes it, with
defaults applied in the handler, never `default_value_t`: absence means
"resolve across levels" for reads and is a third state the domain signature
depends on.

Doc comments are contract — they are what `--help` renders and what the snapshot
test pins.

**`--allow-legacy-layout`** is accepted by the **read** subcommands only; the
mutating subcommands (`set`, `init`, `templates eject`, `templates reset`)
**reject** it. It carries **both** halves of what `ACCELERATOR_MIGRATION_MODE=1`
does in the bash, not just the visible one:

1. it suppresses the uniform legacy-layout refusal, and
2. it enables the **legacy source fallback** — when neither
   `.accelerator/config.md` nor `config.local.md` exists, read
   `.claude/accelerator.md` and `.claude/accelerator.local.md` instead.

Scoped to reads because the fallback has no write counterpart: a
`config set --allow-legacy-layout` would read from `.claude/accelerator.md` and
write to `.accelerator/config.local.md`, which makes the current-layout pair
present and permanently disengages `config_find_files`' fallback condition —
orphaning the entire legacy configuration behind a file containing only the one
key just written, and `.claude/accelerator.local.md` is gitignored so its
credentials are not VCS-recoverable. Rejecting the flag on writes closes that
split-brain write; a criterion asserts `config set --allow-legacy-layout` exits
non-zero.

The second half is the one a reader of `config_assert_no_legacy_layout` alone
would miss. It lives in `config_find_files` (`config-common.sh:41-47`), whose
comment states its purpose exactly: *"Legacy fallback: config not yet moved by
migration 0003"*. Without it a repointed migration does not fail — it resolves
**absent** and proceeds on defaults, silently migrating the wrong directories.

**Both halves need an implementation site, and only the guard half was plumbed.**
The guard half rides `compose(cwd, LegacyPolicy)`. The fallback half must change
which files `ReadConfigLevel` reaches: `FileConfigStore::level_path`
(`config-adapters/src/store.rs:51-56`) hard-codes `.accelerator/config.md` and
`config.local.md`, so it becomes policy-aware — under a legacy policy with the
current pair absent, it resolves the `.claude/accelerator{,.local}.md` pair,
preserving the team-then-local last-writer-wins order that `config_find_files:45-46`
emits. This is a second constructor-adjacent change (`FileConfigStore` carries the
policy), recorded so it is not missed the way the fallback half was in an earlier
draft. A criterion asserts the fallback is **inert** when the current-layout pair
is present.

Only `0001` and `0002` run before `0003` relocates the config (`0003:65` notes it
deliberately avoids `config-read-path.sh` because it is itself rewiring that
script), so for `0004`-`0007` on a normally-ordered run the fallback is inert —
`config_find_files` engages it only when the current pair is absent. The
`--skip 0003` path (`run-migrations.sh` supports `--skip`) is the exception where
a later migration meets a legacy layout, which is why the flag attaches to every
migration's reads.

**There are seven migrations, not six.** `0007-unify-meta-corpus-frontmatter.sh`
reads config *transitively*: `0007:19` sources `scripts/doc-type-table.sh`, whose
`load_doc_type_table` (`:41`) spawns the repointed `config paths --doc-types`.
Under the uniform gate that read newly refuses on a legacy layout, but the call
site is in a shared helper, not in `migrations/`. **`doc-type-table.sh` is added
to `check-call-site-migration.sh`'s allowlist** as the one shared helper the
migration engine depends on, so it may carry `--allow-legacy-layout` without
failing the confinement check. The corrected direct read counts across the seven
are `config-read-value.sh` **×5** (`0001:31,33`, `0002:19,21`, `0006:356`),
`config-read-path.sh` **×5** (`0004:383`, `0005:17`, `0006:60,335,372`) and
`config-read-all-paths.sh` **×1** (`0004:459`) — the last also needs the flag
under `--skip 0003`.

This **replaces** rather than restores the env var. 0178 dropped that bypass
deliberately and the Rust is tested to ignore it; an ambient environment variable
is escapable by anything in the process tree, whereas a flag is explicit at the
call site and greppable. The env var stays unhonoured — the negative test in
`config-adapters/tests/config_reader.rs` is retained, not relaxed.

**Greppability is only a property if something greps.**
`check-call-site-migration.sh` asserts `--allow-legacy-layout` appears **only**
under `skills/config/migrate/migrations/` **and in the allowlisted
`scripts/doc-type-table.sh`**; its appearance anywhere else fails the build.
Otherwise the flag is as freely reachable as the bypass 0178 closed, merely
better documented.

`run-migrations.sh` continues to export `ACCELERATOR_MIGRATION_MODE=1` for the
bash helpers the migrations still source; the flag is additive at each config
call site.

#### 2. The hexagon

**File**: `cli/launcher/src/config_command/{mod.rs,inbound/mod.rs,inbound/cli.rs,core/mod.rs}`
plus `config_command/render/{scalar,agents,review,dump,summary,paths}.rs`
**Changes**: three layers, mirroring luminosity — `inbound/` (clap tree and
dispatch), `core/` (view assembly and the not-found→error policy), `render/`
(pure functions of a view). The domain *service* lives in `config`; the
per-subcommand **view assembly** — the review-configuration aggregate (built-in
plus custom lenses, name-conflict resolution, `min_lenses` by mode), `dump`'s
presence-not-value attribution across levels, the agents default/fallback table,
the 13 doc-type resolutions with their fail-closed validation, `summary`'s
skill-directory enumeration rule (`config-summary.sh:113-140`) — is the largest
new body of logic and lands in `config_command/core/`, **not** in the render
modules. Left unhomed it would accrete inside `render/*`, reintroducing the
impurity the `Rendered` seam exists to remove and making the render modules the
file every future subcommand threads through.

`inbound/cli.rs` holds **only** the clap tree and its dispatch to handlers. Each
output family gets its own `render` module, so the `review` renderer and the
`dump`/`summary`/`agents` rules do not accumulate in one file — at 80 columns,
18 subcommands of byte-exact output in a single module would run to the low
thousands of lines.

**Renderers return both streams**, not a `String`, and each render module also
carries a `render_unavailable() -> Rendered` for its `--fail-safe` degraded
block, so the byte-exact `## <Name> Unavailable` output lives beside the success
output it mirrors rather than being constructed in the handler:

```rust
pub struct Rendered {
    pub stdout: String,
    pub warnings: Vec<String>,
}

pub fn render_summary(view: &SummaryView) -> Rendered;
pub fn render_summary_unavailable() -> Rendered;
```

Three required behaviours cannot be modelled by `-> String`: `agents` warns on
stderr and skips the key, `path` warns and still yields an empty line, and
`summary` buffers stdout so warnings always precede it. With a `String` return
the renderers would reach for `eprintln!` inline, which makes them impure and
pushes every warning assertion into the black-box harness. `Rendered` holds
`warnings` separately so warning **content** is unit-assertable; the **ordering**
guarantee (warnings precede buffered stdout) is a property of the handler that
emits `Rendered`, so it is asserted at the handler/black-box level via a small
`emit(&Rendered, &mut impl Write, &mut impl Write)`, not claimed as a unit-level
property of the value. The handler is the only thing that writes to either stream.

**File**: `cli/launcher/src/main.rs`
**Changes**: composition goes through the **existing** `compose()` helper, widened
to `compose(cwd, LegacyPolicy)`. It already exists and is documented as "the
wiring protocol as a single tested helper" — discovering the root once, running
the legacy guard against it, and building the store and service rooted at the
same directory (`config-adapters/src/compose.rs:19-26`). Duplicating that
protocol elsewhere would put the wiring in an untested place; and because
`assert_no_legacy_layout` is called *inside* it, the widened parameter is also
how `--allow-legacy-layout`'s guard half reaches the guard.

**`compose`'s return type widens too, not just its parameters.** Today it returns
`Result<ConfigService<FileConfigStore, FileConfigStore>, ConfigError>`, consuming
the `FileConfigStore` into the service and dropping every other handle — so it
cannot hand back the port implementations (all `FileConfigStore`) that roughly
half the subcommands need. It returns a bundle exposing the service plus each
driven-port view.

**The bundle handed to `dispatch` is expressed in `config`/`config_command`
traits, not the concrete adapter, so `launch/mod.rs` never names `config_adapters`
and `main.rs` remains the only module that does.** A `ComposedConfig { service,
store }` whose `store` is `FileConfigStore` would force `launch/mod.rs` to name
that adapter type the moment it destructures or forwards it — the invariant this
paragraph asserts would be false. So the composer, constructed in `main.rs`,
returns port **trait objects** (`Box<dyn ReadSkillContent>`, `Box<dyn
ReadLensCatalogue>`, `Box<dyn ScaffoldProject>`, `Box<dyn TemplateOverride>`) plus
the service behind its `config`-crate interface; `dispatch`'s new parameter is a
`FnOnce() -> Result<ConfigStack, ConfigError>` where `ConfigStack` is defined in
`config_command` (or `config`) over those traits. `launch/mod.rs` names only
`config`/`config_command`; `config_adapters` and `FileConfigStore` appear solely
in the `main.rs` closure body. `compose`'s two existing tests move to the new
return shape.

To preserve laziness — `main.rs:45-46` already establishes it, since eager
composition would make `accelerator version` and every external subcommand pay
root discovery and the legacy guard — the `FnOnce` is invoked only when the
`Command::Config` arm routes to `config`, and it closes over `cwd` and the
resolved `LegacyPolicy`.

**File**: `cli/config/src/service.rs`, `cli/config-adapters/src/store.rs`
**Changes**: driven ports for the filesystem access beyond the two config files,
split by responsibility rather than one omnibus trait mixing read, enumerate,
create-tree and delete. Three, not two, so no port both scaffolds and deletes and
no read port becomes a grab-bag:

- `ReadSkillContent` — read a skill's `context.md`/`instructions.md`.
- `ReadLensCatalogue` — enumerate `.accelerator/skills/` and the lens
  directories, stat the init sentinel.
- `ScaffoldProject` (create-only) — `init`'s directory tree, `.gitkeep`s and
  `.gitignore`s. Nothing else.

**Both template mutations — `eject`'s write and `reset`'s delete — go behind a
single distinct `TemplateOverride` port**, not `ScaffoldProject`. A reader looking
for the one destructive operation in the command family will not look behind a
name that says "scaffold", and the eject-write belongs with the reset-delete it
pairs with (they operate on the same override file), so the template mutations are
co-located and named for what they are. `ScaffoldProject` stays purely
create-only for `init`.

All return `ConfigError` and are implemented by `FileConfigStore`, and drive
corresponding services in the `config` crate (e.g. `TemplateAccess`,
`ScaffoldAccess`, `ContextAccess`) so the resolution *rules* — template-tier
precedence, `init`'s idempotent scaffold contract, skill-context resolution,
`summary`'s enumeration — live in the domain rather than in `config_command`.
They are injected into `config_command` directly rather than added as further
generic parameters on `ConfigService`, which would ripple through every
`ConfigService<FileConfigStore, FileConfigStore>` annotation and construction
site. The port vocabulary stays domain-shaped (`ensure_content_dir`,
`ensure_ignore_rule`) rather than naming filesystem artefacts.

**These ports return kind-accurate errors, not `ConfigError::Io`.** That
variant's Display is hard-coded to `"I/O error on config file '{path}'"`
(`config/src/error.rs:84-86`); under these ports it would render lens
directories, skill files, template overrides and `init`'s 14 content directories
as "config file", and `templates reset`'s failed delete the same way. Since
divergence 10 deliberately makes read failures fail-loud across the 28 shell
consumers with no `--fail-safe`, and its stated mitigation is that "the error
**must name the offending file**, so a wide failure is diagnosable in one read",
a misdescribing message undercuts exactly that. `ConfigError` is
`#[non_exhaustive]`, so add `ProjectIo { path, detail }` and
`ScaffoldFailed { path, detail }` (or reword the `Io` Display to be
file-kind-neutral and update the two tests at `error.rs:181-191` and `store.rs`
that pin it).

`ReadConfigLevel`/`WriteConfigLevel` reach only `.accelerator/config.md` and
`config.local.md`, but `context --skill`, `instructions`, all five `templates`
subcommands, `init`'s scaffold and `summary`'s init-sentinel check all need more.
Leaving those with no outbound home invites raw `std::fs` in the inbound adapter,
which makes roughly half the subcommands testable only through the binary.

**File**: `cli/launcher/src/lib.rs`
**Changes**: `pub mod config_command;`

**File**: `cli/launcher/src/launch/mod.rs`
**Changes**: `dispatch` gains a `FnOnce() -> Result<ConfigStack, ConfigError>`
parameter supplied by `main.rs`; the `Command::Config` arm invokes it and hands
the resulting `ConfigStack` (service + port trait objects, all `config`/`config_command`
types) to `config_command`. `launch/mod.rs` names neither `config_adapters` nor
`FileConfigStore` nor `ComposedConfig` and does not call `compose` itself — the
closure is opaque to it. Laziness is preserved because the arm invokes the closure
only when it routes to `config`, so `version` and external subcommands never
trigger root discovery or the guard.

**File**: `cli/launcher/Cargo.toml`
**Changes**: path deps on `config` and `config-adapters` (the latter used only by
`main.rs`).

**File**: `cli/pup.ron`
**Changes**: a module rule for `config_command`. **Not** the `version_core`
shape — that is a domain-*core* inward rule permitting only std, `kernel::Error`
and `crate::version::core` (`pup.ron:10-24`), which applied literally to an
inbound module would deny the `config` import this phase depends on.

The rule follows the file's documented convention (`pup.ron:6-7`): the `matches`
field is the **resolved** module path, and `allowed_only` is the **literal
use-path**. So `matches` is `^accelerator::config_command($|::)` (as
`version_core` matches `^accelerator::version::core`), and `allowed_only`
enumerates literal use-paths — `std`/`core`/`alloc`, `^kernel::Error(::|$)`,
`^config(::|$)`, and `^crate::config_command(::|$)`. It is **not** bare `^crate`:
a config_command handler writes intra-module imports as `use crate::config_command::…`
(never `use accelerator::…`), so `^crate::config_command(::|$)` is what actually
matches those, and confining the `crate::` allowance to the module's own subtree
is what expresses the `crate::launch::outbound` prohibition without a `denied`
clause (all five existing rules use `allowed_only` with `denied: None`; the
combination has no precedent and its precedence is untested).

`config_adapters` is **not** on the allowlist. Permitting it would contradict the
composition-root decision: the service arrives through `dispatch`, so
`config_command` never names the adapter crate, and permitting it would let a
handler call `FileConfigStore::at(...)` directly while the pup check stayed green
— eroding the seam invisibly. `main.rs` is the only launcher module that names
`config_adapters`.

Two deliberately-violating fixtures under `tests/integration/pup/` prove the rule
rejects a `crate::launch::outbound` import **and** a `config_adapters` import from
`config_command` — the second is the actual load-bearing prohibition and was
untested in an earlier draft. This is the module-level counterpart to Phase 7's
no-HTTP dependency check.

#### 3. Exit codes

**File**: `cli/kernel/src/lib.rs`
**Changes**: a boundary variant carrying a code, so exit 2 is reachable. `kernel`
uses `thiserror`, so the variant carries its display attribute; the doc comment
states only its per-subcommand meaning, not the exit-code mapping, which lives in
`main.rs`:

```rust
pub enum Error {
    #[error(transparent)]
    LogFilter(#[from] tracing_subscriber::filter::ParseError),
    #[error("{0}")]
    Failed(String),
    /// A subcommand-scoped refusal the caller acts on; its meaning is defined
    /// per subcommand, not globally.
    #[error("{0}")]
    Refusal(String),
}
```

**File**: `cli/launcher/src/main.rs`
**Changes**: map `Refusal` to `ExitCode::from(2)`, and intercept clap usage errors
rather than delegating to `error.exit()` so a usage error exits **1** as the bash
does.

All **three** of clap's non-error kinds exit 0: `DisplayHelp`, `DisplayVersion`,
and `DisplayHelpOnMissingArgumentOrSubcommand`. The last does **not** fire from
`subcommand_required` alone — in clap 4 that setting yields
`ErrorKind::MissingSubcommand`, which the interception would map to exit 1. To
make a bare `accelerator config` print help and exit 0, set
`arg_required_else_help = true` on the `config` subcommand, which is what produces
`DisplayHelpOnMissingArgumentOrSubcommand`.

Replacing `error.exit()` also drops clap's own stream routing (help-family output
to stdout, genuine errors to stderr). The interception preserves that per kind —
delegating the three non-error kinds to `Error::print()` (stdout, exit 0) and
usage errors to stderr with exit 1 — rather than uniformly `eprintln!`-ing, which
would move help text onto stderr for the missing-subcommand case.

**The existing `DisplayHelp` interception must be scoped to top-level help.**
`main.rs:102-108` today intercepts *every* `DisplayHelp` and calls
`render_augmented_help()`, which rebuilds `Cli::command()` and prints the
**top-level** help — so `accelerator config --help` and `accelerator config get
--help` would both print launcher help, and the Phase 2 help-snapshot criterion
could not pass. `render_augmented_help()` must fire only for the top-level help
(checking the error's context / the parsed subcommand path); subcommand
`DisplayHelp` delegates to `error.print()` with exit 0, so clap's own
matched-subcommand help is what renders.

**`install_crypto_provider()` moves here too.** It runs unconditionally in
`main()` before argument parsing (`main.rs:95`), so a `config path` pays rustls
provider installation for capability it never uses. It moves behind the
external-resolution path so built-ins skip it. A criterion asserts neither
`accelerator version` nor `accelerator config path <key>` installs the provider.
(This is control-flow hygiene and a microsecond-scale saving, **not** a
size-lever — it does not unlink rustls/ring/reqwest from the artefact; the
per-call size term is Phase 0's `strip`/`lto`.)

**This is a launcher-wide contract change, not a config-scoped one.**
`Cli::try_parse()` is the single parse point for `version`, top-level `--help`
and the `external_subcommand` catch-all (`launch/mod.rs:33-37`). Two consequences
are recorded rather than discovered: 0164 shipped with clap's conventional exit 2
for usage errors, so any consumer distinguishing 2 from 1 sees a changed
contract; and exec'd externals propagate their own codes unmodified, so exit 2's
refusal meaning holds for **built-ins only**. Sub-binaries 0168/0169/0173 add
should adopt the same usage-error-is-1 interception so the exit-code taxonomy
converges rather than diverging as they land — recorded as a forward requirement
on 0164's sub-binary contract.

#### 4. The subcommands

Scalar (stdout is exactly the value plus one `\n`; stderr empty):
`get`, `path`, `agent`.

**`get` and `path` both take an optional `[default]` positional, and their
precedence rules differ.** This is load-bearing for the 28 shell consumers and is
not symmetric:

| | Explicit `$2` | No `$2` |
|---|---|---|
| `config get <key> [default]` | returned on a miss | **empty** — no catalogue lookup at all (`config-read-value.sh:25,129`) |
| `config path <key> [default]` | wins over the catalogue | catalogue default, else empty + a stderr warning (`config-read-path.sh:31-42`) |

Consumers depend on both directions. `jira-auth.sh:228`, `linear-auth.sh:241` and
`write-visualiser-config.sh:64-65` pass an explicit **empty** default as a
*presence probe*, branching on emptiness to detect an unset key —
`write-visualiser-config.sh:66-69` fires a migration guard off exactly that. And
`write-visualiser-config.sh:185` passes a long non-empty default that must win.

A catalogue fallback for `get` would break every probe; dropping the positional
would break every explicit default. Both failures are silent wrong values, and
the parity gate cannot catch either: `test-config.sh`'s only two-positional sites
(`:5975`, `:5977`) pass defaults that coincide with the catalogue values, so an
implementation ignoring `$2` passes both unchanged.

Block (stdout matches a committed golden byte-for-byte):
`paths`, `context`, `instructions`, `agents`, `work`, `review`, `dump`,
`summary`, `template`, `templates list`, `templates show`.

**`template` is a block, not a scalar** — `config-read-template.sh:36-60` emits
the entire resolved template file wrapped in ```` ```markdown ```` fences, or
verbatim when the file already opens with a fence (`:41-43`), across the 19 `!`
sites that inject it. It gets committed goldens for all three
`config_resolve_template` resolution tiers plus the already-fenced branch and the
warn-and-fallback-on-missing-configured-path branch (`config-common.sh:412-417`).
Its fail-closed exit (unresolvable → exit 1, `:53-57`) follows the same
`--fail-safe` rule as the other fail-closed block subcommands (see the fail-safe
matrix under Implementation Approach).

**Three bash scripts collapse into switches rather than becoming subcommands**,
following luminosity's shape:

| Bash script | Becomes |
|---|---|
| `config-read-context.sh` | `config context` |
| `config-read-skill-context.sh` | `config context --skill <name>` |
| `config-read-skill-instructions.sh` | `config instructions <name>` |
| `config-read-all-paths.sh` | `config paths` |
| `config-read-doc-type-paths.sh` | `config paths --doc-types --format tsv` |

**`context` emits both blocks in one invocation.** The project block is always
requested; `--skill` *adds* the skill block after it, mirroring
`luminosity/cli/launcher/src/context_command/inbound/cli.rs`.

**The separator is emitted only when both blocks are non-empty.** Either block can
be absent — `config-read-context.sh:26` emits nothing when every body trims empty
(the common case for an unconfigured repo) and `config-read-skill-context.sh:25,28`
exits 0 silently for an absent or whitespace-only file. So the rule is: join with
exactly one blank line when both survive, emit the survivor alone when one does,
and emit nothing when neither does. Goldens cover all four states, not only
both-present.

Note this is *not* plain concatenation of the two scripts' stdout — that yields
zero blank lines, since `config-read-context.sh:32` ends with a single trailing
newline and `config-read-skill-context.sh:30` opens immediately with its header.
SKILL.md files today make two separate `!` calls for these, so one invocation
replaces two — the only place in this migration where the mechanical rewrite
reduces the call count rather than preserving it.

**`instructions` takes a positional, not `--skill`.** There is no project-level
instructions concept — `config-read-skill-instructions.sh` is the only reader —
so a `--skill` flag would be mandatory on every invocation, which is strictly
worse than a positional. The redundant `skill-` prefix is dropped since
skill-scoped is the only kind.

**`paths` carries the doc-type variant as switches.** `--doc-types` selects the
13 doc-type→directory mappings instead of the configured path keys, and
`--format <block|tsv>` selects the rendering (`block` defaults for the configured
paths, `tsv` for doc-types). `--all` includes the keys
`config-read-all-paths.sh` hard-codes as excluded (`tmp`, `templates`,
`integrations`).

**`paths --doc-types` takes an optional `[root]` positional**, resolving the
doc-type directories against `<root>` (each read inside `( cd "$ROOT" && … )`,
matching `config-read-doc-type-paths.sh:51-55`). This is designed and tested
*here*, not merely referenced in Phase 4/7: `doc-type-table.sh:41` passes the root
in **production** (`bash "$RESOLVER" "$root"`), so migration 0007 and the
`corpus-adapters` test both depend on it, and those directories scope in-place
mutation. A criterion asserts `config paths --doc-types <root>` resolves against
`<root>`, not the caller's CWD. (Note `config-read-doc-type-paths.sh` has **zero**
`!` sites, so its rewrite to `paths --doc-types --format tsv` is a shell-consumer
repoint in Phase 5 §4b, not one of the 247 `!`-site reshapings.)

This **revises the no-`--format` decision**, which previously admitted only
`summary --format=hook` as an exception. The rule becomes: a format switch is
permitted where the same underlying data has genuinely distinct machine and human
consumers; it is not used to vary a single consumer's rendering. `paths` has
both — the block form is spliced into prompts, the TSV form is parsed by
`doc-type-table.sh` and `validate-corpus-frontmatter.sh`.

The fail-closed validation stays attached to `--doc-types` (tab/newline → exit 1,
unsafe path → exit 1, directories normalised through `tr -s '/'`, leading `./`
stripped at `:76` and trailing-slash strip), matching
`config-read-doc-type-paths.sh:81-110`. The default `paths` form does not
validate, as the bash does not. Recorded as a deliberate asymmetry: the
validation protects consumers who use the value *as a path*, which is the TSV
form's audience.

**A present-but-blank `paths.<key>` coerces to the registry default**, with a
stderr note ("blanking a path does not disable a doc-type",
`config-read-doc-type-paths.sh:87-91`), and the output is **always 13 rows**.
This is load-bearing beyond parity: `0007:814-819` reasons that "because a blank
config value is coerced to its registry default, a short/empty table only ever
signals such a failure" — the premise of 0007's fail-closed pre-mutation net. A
port that emitted fewer than 13 rows for a blank key, or errored instead of
coercing, would silently invert that guard from fail-closed to fail-open. A
criterion asserts 13 rows under a blank override.

`work` is also `block` rather than scalar despite emitting one line, because its
enum validation is a fail-closed exit path the scalar contract does not model.

**`summary --format=hook` lands here, not in Phase 6.** Its contract is described
under Phase 6 (three output states, the transport envelope), but the
implementation belongs in this phase so that `test-config.sh`'s `CONFIG_DETECT`
binding — which exercises the envelope, the emptiness suppression and the
stdout/stderr split at `:764`, `:769`, `:783`, `:798`, `:5088`, `:5098-5099` —
can be repointed in Phase 4 alongside every other binding. Left in Phase 6 those
seven assertions have no gate: the script is on neither the removal set nor any
inventory member, and Phase 6 would delete `config-detect.sh` while the suite
still executes it, so the phase could not be green on its own. Phase 6 then
reduces to the `hooks.json` registration change.

Behaviours to reproduce exactly, from the bash:

- `agents` **always** emits its 9 bullets, even with zero config files
  (`config-read-agents.sh:117-124`); an empty YAML value falls back to
  `accelerator:<key>`; unknown keys warn on stderr and are skipped; hyphens
  become spaces in the display name.
- `context` emits **nothing** when every body trims empty; team and local bodies
  join with exactly one blank line (`config-read-context.sh:19`).
- `review` takes exactly one of `pr|plan|work-item`; `min_lenses` defaults to 3
  for `work-item` and 4 otherwise; the header comment at `:9` is stale — the
  block is emitted unconditionally at `:482`.
- `context --skill` and `instructions` emit nothing for an absent file or a
  whitespace-only one, and interpolate the skill name into the prose.

  **The skill name is validated as an identifier, not a path fragment** — a
  divergence. `config-read-skill-context.sh:23` builds
  `$PROJECT_ROOT/.accelerator/skills/$SKILL_NAME/context.md` with no validation,
  so a traversing name reads an arbitrary file into the prompt, and the command
  fails silently (exit 0, no output) when the file is absent, making a probe
  invisible. Anything not matching `^[a-z0-9][a-z0-9-]*$` is refused. The
  codebase already hardens this way at `config-read-doc-type-paths.sh:101-107`,
  so this closes an inconsistency rather than setting new policy.

  **The name is parsed in the handler, never by a clap `value_parser`.** A
  `value_parser` rejects before the fail-safe boundary is reached, so an invalid
  name under `--fail-safe` would exit non-zero and discard the whole prompt —
  the exact failure the flag exists to prevent. Luminosity states this
  explicitly (`context_command/inbound/cli.rs:11-14`), carries the raw name as
  `Option<String>` through to the handler, and models a never-parsed name as its
  own `Section::Unresolved` so it degrades before any path is touched. This plan
  does the same.

- **Per-source degradation happens within a single `context` invocation.** An
  unreadable skill file leaves the healthy project block standing under a notice
  naming the skill file, and vice versa; both failing prints both notices. This
  is a consequence of `context` emitting both blocks — it is no longer assertable
  only across separate invocations. Because these bytes are byte-exact-goldened
  and injected into 42 prompts, the **notice position is specified**, not left to
  the first implementation: a source's notice replaces its block in place (notice
  where the block would have been), the surviving block keeps its position, and
  the one-blank-line separator applies between the two members exactly as for two
  blocks. Goldens cover project-block + skill-notice, project-notice +
  skill-block, and both-notices, alongside the four survival states.
- `summary` buffers all stdout and emits it in one write, so stderr warnings
  always precede it; an unrecognised skill dir warns on stderr **unconditionally**
  but is enumerated on stdout **only when its `context.md` or `instructions.md`
  has non-whitespace content** (`config-summary.sh:113-140`) — so an unrecognised
  *empty* dir warns and is not enumerated; a fully-initialised repo with no config
  emits zero bytes.
- `get` on an unset key prints the default (usually empty) and exits **0**
  (`config-read-value.sh:126-130`) — it never errors. `ConfigError::NotFound`
  stays unconstructed on this path: `get` sits behind `!` call sites, so a
  non-zero exit would inject an error string where the bash injected an empty
  line.
- `dump` attributes by **presence, not value** — a key set to the default string
  still attributes to team/local; `*.token`/`*.token_cmd` render as
  `*(set — hidden)*` with an em dash (U+2014); template rows hard-code source
  `default`.
- `path` treats an omitted and an empty `$2` identically; unknown keys warn on
  stderr and yield an empty line; the migration-0004 rename warnings are preserved.

#### 5. Goldens and fixtures

**File**: `cli/launcher/tests/config_read.rs` (new)
**Changes**: the black-box harness — `CARGO_BIN_EXE_accelerator`, a per-test
workspace under `CARGO_TARGET_TMPDIR` carrying a `.git` boundary marker so root
discovery is bounded inside the fixture, uniqueness by `{pid}-{counter}`.
Byte-exact assertions compare `output.stdout` directly, never through
`from_utf8_lossy`.

**File**: `cli/launcher/tests/fixtures/`
**Changes**: the fixture workspaces named in the work item — **baseline**,
**not-customised**, **already-customised** (new, for `eject`'s exit 2),
**error**, **empty-summary**, **unreadable-config**, three **malformed**, and the
three fail-closed triggers (**writeback-failure**, **bad-integration-enum**,
**doc-type-escape**).

Reuse `scripts/test-fixtures/config-read-review/{pr,plan,work-item}-mode-golden.txt`
as the **baseline** `review` goldens — but **verify them byte-exactly first**.
They have never been compared that way: `test-config.sh:1947-1949` uses
`assert_eq "$(cat "$GOLDEN")" "$OUT"`, and command substitution strips all
trailing newlines from *both* operands, so their trailing-newline content is
unconstrained. Redirect `config-read-review.sh <mode>` to a file and `cmp` it
against each committed golden before reuse, and record the result. The same
applies to the `config-summary.sh` golden captured before deletion in Phase 7 —
capture by redirection and `cmp`, not by command substitution.

Otherwise a genuinely byte-exact Rust comparison fails for a reason unrelated to
the port, and the likely repair is to trim the Rust output — silently restoring
the newline-insensitive assertion at the subcommand with the largest rendering
surface.

They are not sufficient on their own: all three contain only built-in lens rows
(31 `| built-in |`, zero `| custom |`), so the custom-lens path — the most
branching part of the most branching block subcommand, and where divergence 2
changes behaviour — would have golden coverage of zero. A **custom-lenses**
fixture is added with committed goldens for all three modes, covering a valid
custom lens (pinning the single-slash path), a lens whose name conflicts with a
built-in, a lens directory with no `SKILL.md`, and a lens carrying `applies_to`
so the three per-mode goldens **genuinely differ** — otherwise the mode-filtering
path (`config-read-review.sh`'s ten `applies_to` assertions at
`test-config.sh:2010-2236`: per-mode restriction, unrecognised mode, empty list,
scalar form, duplicates) is pinned by identical goldens and untested.

**The same baseline-only argument applies beyond `review`.** Phase 2 §4
enumerates exact behaviours whose only golden is the baseline: `agents`'
empty-value fallback to `accelerator:<key>`, unknown-key warn-and-skip, and
hyphen-to-space display naming; `dump`'s presence-not-value attribution and the
`*(set — hidden)*` em-dash masking of `*.token`/`*.token_cmd`. After Phase 7
these are pinned by no byte-exact gate at all, so dedicated fixtures are added: an
**agents** fixture exercising the empty-value / unknown-key / hyphenated-name
triple, and a **dump** fixture carrying set `*.token` and `*.token_cmd` keys plus
keys set to their default string.

A `summary` test invoked with `.current_dir(fixture.join("src/deep"))` pins
divergence 3, which is otherwise unreachable from the fixture root.

#### 6. The behaviour inventory

**File**: `meta/inventories/0167-config-behaviour-inventory.md` (new)
**Changes**: rows keyed by `<file>:<line>` or by script path, each mapped to a
named Rust test, covering the four non-repointable members:

1. **every region of `test-config.sh` that greps consumer source text for a
   removal-set script name** — not only the call-site greps
   (`:1095-1101`, `:1139-1194`, `:2995-3032`, `:4985-5027`, `:5029+`) but also the
   second family of tree-wide census invariants: `:2543-2577` (every
   jira/linear/visualiser `config-read-value.sh` key is catalogue-registered),
   `:3287-3316` (no consumer passes a hardcoded inline default to
   `config-read-path.sh`), `:4056-4083` (same for `config-read-work.sh`),
   `:4085-4114` (no non-`config-*` file reads `work.*` via `config-read-value.sh`),
   `:4116-4143`, `:4145-4164` (every SKILL.md invoking `config-read-work.sh` has a
   covering `allowed-tools` entry). Most grep the **old** script name, so after
   Phase 5 they match nothing and **pass vacuously** rather than failing — the
   defect class this migration can introduce (hardcoded inline defaults,
   unregistered keys, missing permission coverage). Each region is classified
   *ported to a Rust test*, *rewritten against the new invocation shape in a
   surviving gate* (`check-call-site-migration.sh` for the inline-default and
   key-registration invariants, `check-skill-permissions.sh` for the
   allowed-tools-coverage invariant), or *dropped with a reason* — enumerated by
   pattern (grep for every removal-set basename in the suite), not by hand;
2. the `config-defaults.sh` file assertions (`:2441`, `:2525-2532`);
3. **all of `test-init.sh`**;
4. every removal-set script with no covering suite.

**Depth floor for member 4**: every branch of the script's top-level control flow
and every distinct exit code is its own row. **Its population is derived
independently**, not read off the extractor: member 4 = *removal-set file list*
minus *scripts named by a suite in the pinned suite audit*, both committed
artefacts, and `check-inventory.sh` asserts set **equality** against that computed
set rather than a count floor — so a member-4 list missing three scripts fails
against the computed set instead of being satisfied by a correspondingly small
inventory.

**Depth floor for member 3** (`test-init.sh`) is **per-assertion**, not
per-branch — see Phase 4. Its cross-cutting invariants belong to no branch of
`init.sh` and would satisfy a branch floor vacuously.

**File**: `meta/inventories/0167-deletion-ledger.md` (new)
**Changes**: one row per removal-set path, with `covering gate` and `commit where
that gate went green` columns left empty. Phases 2-4 fill in each row **in the
same commit that turns its gate green**, so Phase 7's replay verifies a record
built forwards. Authored here rather than at Phase 7: a ledger written by reading
the history it certifies is the unfalsifiable claim it was introduced to replace.

**File**: `meta/inventories/0167-divergences.md` (new)
**Changes**: the recorded divergences listed under Implementation Approach, each
naming the test that would fail if it were absent or later regressed.
`check-inventory.sh` **parses this note and asserts every named test resolves to
a real `#[test]`/suite and appears in the passing test list** — otherwise a
divergence whose named test was deleted is indistinguishable from a defect, which
is the plan's own stated standard. This matters concretely for divergences 10-12,
whose named tests live in `config-adapters/tests/parity.rs` (see Phase 7 §3, which
must retain them while removing only the differential shell-out tests).

**File**: `scripts/check-inventory.sh` (new, `0755`)
**Changes**: asserts no duplicate and no missing rows against an extraction of
the four members **pinned to a recorded pre-deletion revision**
(`jj file show -r <rev> <path>`, which prints file contents at that revision —
**not** `jj show <rev>`, which renders a commit description and diff — and with a
self-check that the extraction is non-empty and parses). Phase 7 deletes every
extraction source, so a working-tree
extraction would yield nothing and the check would pass trivially at exactly the
moment it matters — the same tautology this plan flags for
`_EXPECTED_CONFIG_SUITES` and must not repeat here.

It also carries a **known-positive floor**, as every other new gate does: the
per-member row count at the pinned revision is committed, and the script fails if
extraction yields fewer rows than that floor. Pinning stops a vacuous pass *after*
deletion. For member 4 the count floor is **replaced** by the set-equality check
above (computed population, not extractor output), closing the mis-scoped-extractor
hole the count floor left open — the member the work item names as the likeliest
silent loss. Member 3's floor is pinned to the counted `test-init.sh` assertions
at the recorded green-run commit, not a prose "~25".

Its cardinality is recorded, **not** reconciled against 337
— that figure counts `test-config.sh` assertions and bears no relation to the
remainder's size. (The real runtime total is ≈545 anyway: 337 static `assert_`
sites plus 208 inline `PASS=$((PASS + 1))` blocks.)

#### 7. Surviving-suite audit

**File**: `meta/inventories/0167-suite-audit.md` (new)
**Changes**: pinned to a recorded revision. `run_shell_suites` is called from
**eight separate task entry points over different subtrees** at the pinned
revision (`integration.py:53,87,109,125,131,137,150,163`), plus the ninth that
Phase 4 §4 adds for `skills/config/init`. So "every suite it discovers" has no
single referent — the audit's corpus is fixed as the **union** of the eight
present at the pinned revision plus the Phase-4 `skills/config/init` addition
(without which `test-init.sh` appears in no audit row), and the initial table is
generated mechanically (grep every discovered suite for every removal-set
filename) rather than by enumeration, so membership cannot be missed by hand.

Every suite appears as a row classified (a) exercises no removal-set script,
(b) inventoried and ported, or (c) repointed. Must include
**`scripts/test-design.sh`**, which the work item never names but which asserts
SKILL.md invocation shape at `:42`, `:157-161`, `:427`, `:444-446`, `:471-472`
and breaks in the same class as the flagged regions.

The generation is **transitive-closure**, not fixed-depth. A two-hop scan (grep
each suite for removal-set filenames *and* for the 28 repointed consumers' paths)
still misses three-hop chains: `jira-common.sh` and `linear-common.sh` are
*sourced libraries* pulled in by the `*-flow.sh` scripts, each covered by its own
suite among the 32 integration suites — `test-jira-create.sh` exercises
`jira-create-flow.sh`, which sources `jira-common.sh`, which calls
`config-read-path.sh`. So the generator first computes the set of shell files
reaching a removal-set script through **any** chain of `source`/invocation, then
greps every discovered suite against that closure.

It is checked against a known-positive floor that includes a genuinely **three-hop**
suite whose only path to a removal-set script is through a sourced library —
`test-jira-create.sh` or `test-linear-search.sh` (whose flow scripts source
`*-common.sh` with no direct removal-set reference, so a fixed-depth generator
provably misses them; note `test-jira-init-flow.sh` does *not* qualify — its flow
script calls `config-read-work.sh` directly, a two-hop path) — alongside
`test-write-visualiser-config.sh` (two-hop), `test-work-item-create-remote.sh`,
`test-work-item-scripts.sh`, `test-skill-frontmatter-population.sh` (which greps
SKILL.md bodies for the invocation shape, so it breaks at the Phase 5 cutover in
the same class as `test-design.sh`) and `test-jira-paths.sh`. A generator
returning fewer than these is mis-scoped.

The reference side of this is now covered mechanically by
`meta/inventories/0167-removal-set-references.md`, which enumerates every
reference to every removal-set member by basename across the whole tree — the
audit that found `config-common.sh:407,422`, `catalogue.rs:301,325` and
`corpus-adapters/tests/common/mod.rs:66`, none of which a path-shaped grep sees.

Also records the Rust tests that pin the shell surface and will break at
Phase 7: `cli/config-adapters/tests/parity.rs:42-43,113-121` and
`cli/corpus-adapters/tests/doc_type_single_source.rs:189-220`; plus the two shell
suites writing `exec` stubs that hard-code the resolver path
(`test-validate-corpus-frontmatter.sh:412`, `test-migrate-0007.sh:2208`).

### Success Criteria

#### Automated Verification

- [x] `mise run test:unit:cli` passes (CI's `cargo nextest --workspace
      --all-features`, not bare `cargo test --workspace`, so the
      `bash-parity`-gated suites are exercised)
- [ ] `accelerator config --help` and every subcommand's `--help` match a
      committed snapshot
- [x] Each **block** subcommand matches its golden byte-for-byte against the
      baseline fixture
- [x] Each **scalar** subcommand emits exactly the value plus one `\n`, stderr empty
- [x] `config paths --doc-types --format tsv` emits exactly 13 tab-separated
      lines under `LC_ALL=C`, byte-identical to `config-read-doc-type-paths.sh`
- [x] `config paths` emits the `## Configured Paths` block excluding `tmp`,
      `templates` and `integrations`; `--all` includes them
- [x] `config context --skill <name>` covers all four survival states: both
      blocks (project, one blank line, skill), project-only, skill-only, and
      neither (zero bytes). The separator appears only when both survive — plain
      concatenation of the two bash scripts yields zero blank lines, so
      "byte-identical to concatenation" and "one blank line" are different
      contracts and only one can hold
- [x] Fail-safe, read/IO failure, notice-rendering commands: `agents`, `context`,
      `instructions`, `review` **with `--fail-safe`** against **unreadable-config**
      exit 0 with stdout exactly the named header (`## Agent Names Unavailable\n`,
      `## Project Context Unavailable\n` and `## Skill-Specific Context Unavailable\n`
      for `context --skill`, `## Skill Instructions Unavailable\n`,
      `## Review Configuration Unavailable\n`), diagnostic on stderr, nothing else
- [x] Fail-safe, suppression commands: `summary`, `get`, `path`, **`work`** **with
      `--fail-safe`** against **unreadable-config** emit **nothing** on stdout,
      exit 0, diagnostic on stderr — including `config work integration`, whose
      empty output the work skills read as "not configured"
- [x] Fail-safe, multi-line block commands: `dump`, `paths`, `template`,
      `templates list` **with `--fail-safe`** against **unreadable-config** render
      their `## <Name> Unavailable` block, exit 0
- [x] **Without** `--fail-safe` the same input exits non-zero with the error on
      stderr and stdout empty — asserted for every subcommand above, since this is
      what the 28 shell consumers depend on
- [x] **Validation refusals stay fail-closed even WITH `--fail-safe`**:
      `config work integration --fail-safe` against **bad-integration-enum**, and
      `config paths --doc-types --fail-safe` against **doc-type-escape**, each exit
      non-zero with empty stdout and the diagnostic on stderr — the load-bearing
      read/IO-vs-validation split
- [x] Per-source degradation asserted **within one `context --skill` invocation**,
      with notice **position** pinned by golden: an unreadable skill file prints
      the project block then the skill notice in the skill block's position; an
      unreadable config prints the project notice then the skill block; both
      failing prints both notices; the one-blank-line separator applies between the
      two members in every case
- [x] An invalid `--skill` name under `--fail-safe` still prints the project
      block and exits 0 — proving the name is parsed inside the fail-safe
      boundary rather than by a clap `value_parser`
- [x] The same invalid name **without** `--fail-safe` exits non-zero
- [x] `--explain` names **both** files on a fixture where a key set in `config.md`
      is overridden in `config.local.md`, attributes the winning value to the
      personal level, and leaves stdout byte-identical to the same run without it;
      the same run **without** `--explain` emits no provenance — so the flag is
      distinguishable from the default, not a no-op
- [x] `config paths --doc-types` emits exactly 13 rows under a **blank**
      `paths.<key>` override, coercing it to the registry default with the stderr
      note — the invariant migration 0007's fail-closed net depends on
- [x] Fail-closed (no flag): **bad-integration-enum** → `config work` exits
      non-zero; **doc-type-escape** → `config paths --doc-types` exits non-zero;
      stdout empty in both (the buffered divergence — the bash leaves a partial
      prefix), diagnostic on stderr
- [x] A usage error (unknown flag, bad `--level`) exits **1**, not 2 — asserted
      for `accelerator --bogus-flag` and `accelerator version --bogus-flag` too,
      since the interception is at the launcher-wide parse point
- [x] A bare `accelerator config` prints help and exits **0**, matching
      `config --help`
- [x] `accelerator config --help` and `accelerator config get --help` render the
      **matched-subcommand** help, not the top-level launcher help — the augmented
      top-level help fires only for the top-level `--help`
- [x] Neither `accelerator version` nor `accelerator config path <key>` installs
      the rustls crypto provider (the built-in path skips `install_crypto_provider`)
      — spy-resolver dispatch harness in `cli/launcher/tests/crypto_provider.rs`
- [x] `config get` on an unset key with no `[default]` prints empty and exits 0
- [x] `config get <key> <default>` on a miss returns the **caller's** default even
      where a differing catalogue default exists, and returns **empty** for an
      explicitly empty default — the presence probe `jira-auth.sh:228`,
      `linear-auth.sh:241` and `write-visualiser-config.sh:64-65` depend on
- [x] `config path <key> <default>` prefers the explicit default over the
      catalogue; without it, the catalogue default, else empty plus a stderr
      warning
- [x] Fixtures include **config-path-customised** and **non-mapping-root**
- [x] Legacy layout is refused uniformly by every subcommand
- [x] `--allow-legacy-layout` suppresses that refusal on the **read** subcommands
      when passed, and enables the source fallback; `config set
      --allow-legacy-layout` (and the other mutating subcommands) **exits
      non-zero** — the flag is rejected on writes. `ACCELERATOR_MIGRATION_MODE=1`
      alone still refuses, asserted by the retained 0178 negative test
- [x] The legacy source fallback is **inert** when the current-layout pair is
      present, and reads the `.claude/accelerator{,.local}.md` pair (team then
      local, last-writer-wins) only when the current pair is absent
- [x] `config-summary.sh`'s output against the baseline fixture is captured and
      committed as a golden **before** the script is deleted (by redirection and
      `cmp`, not command substitution), and `accelerator config summary` matches
      it byte-for-byte
- [x] `--format=hook --fail-safe` against **baseline** emits exactly the envelope
      with the plain command's output as `additionalContext`
- [x] `--format=hook --fail-safe` against **empty-summary**: nothing on stdout,
      exit 0 — not `{}`, not an envelope carrying an empty string
- [x] `--format=hook --fail-safe` against **unreadable-config**: nothing on
      stdout, exit 0, diagnostic on stderr
- [x] `--format=hook` **without** `--fail-safe` against **unreadable-config**:
      exit non-zero, empty stdout — the flag is required for the exit-0 states, so
      the hook registration (Phase 6) carries it
- [x] The stdout/stderr split is preserved: the unrecognised-skill warning appears
      on stderr and **not** in the stdout JSON
      (`summary_hook_keeps_the_unrecognised_skill_warning_off_stdout`)
- [x] `bash scripts/check-inventory.sh` exits 0
- [x] `mise run check` exits 0, `mise run` exits 0

#### Manual Verification

- [ ] The `dump` output's em dash renders correctly in a terminal
- [ ] The divergence note reads as a decision record, not a defect list

---

## Phase 3: Write subcommands — `set`, templates, `init`

### Overview

The net-new write path plus the template group and `init`. Still no call-site
changes.

### Changes Required

#### 1. `config set`

**File**: `cli/launcher/src/config_command/inbound/cli.rs`
**Changes**: positional `key` and `value`, `--level` defaulting to `Personal` in
the handler. Delegates to `ConfigAccess::set`, which already preserves sibling
type and key order (`service.rs:712-738`) and fails closed on a read error before
any write (`:696-709`).

Body preservation is already correct: `document::render` re-emits only the
frontmatter and concatenates the original body bytes verbatim, and its discarded
`parse_frontmatter` call is a fail-closed gate that errors before `atomic_write`
is reached. **Arbitrary YAML nesting comes free** — `Key::parse` splits on dots
and `insert`/`resolve` recurse, so ADR-0047's headline capability needs an
assertion, not an implementation. The work item leaves this uncriterioned; this
plan asserts it directly.

**`config set` is net-new capability, and its residual security risk is recorded
as an accepted decline, not dismissed.** There is no bash `config-set-value.sh`;
`skills/config/configure` writes with the Edit tool today. After Phase 5,
`config set` becomes prompt-free in every skill carrying the `config *` rule.
Two specific hazards were raised and the refusals **declined** (see the Q1
disposition): writing `*.token_cmd`, which `jira-auth.sh:118` later executes via
`bash -c`; and writing a credential to the VCS-tracked team `config.md` via
`--level team`. The earlier justification — "repo content is team-reviewed before
it lands" — is **wrong** for the skills that carry the rule: `review-pr`,
`respond-to-pr`, `show-jira-issue`, `search-jira-issues`, the Linear equivalents,
`extract-work-items` and `research-issue` all ingest attacker-controllable PR
diffs and issue bodies that are not reviewed. The decline is held for parity and
scope, but recorded as an accepted residual RCE/credential-exfiltration risk with
the true input surface named, so a later reader re-examines it. The closing levers
are noted, not applied: refuse `config set` on `*.token`/`*.token_cmd` (and on
`--level team` for those keys), and split the `config *` grant into a read-only
rule for the ~25 read-only skills — both mechanically enforceable by
`check-skill-permissions.sh`, which is already being written.

#### 2. The template group

**File**: `cli/launcher/src/config_command/` plus a `templates` module
**Changes**: `templates list|show|eject|diff|reset`.

**Exit 2 semantics, corrected.** ADR-0021:80 defines exit 2 as "destructive
action requires confirmation". The bash overloads it, and the three commands fire
on **opposite** customisation states:

| Command | Exit 2 fires when | Fixture |
|---|---|---|
| `templates eject` (no `--force`) | the override **already exists** | **already-customised** |
| `templates diff` | there is **no** override | **not-customised** |
| `templates reset` | there is **no** override | **not-customised** |

**The two commands do not test the same thing**, so a binary
customised/not-customised model is insufficient. `config-eject-template.sh:88`
tests `[ -f "$TEMPLATES_DIR/<key>.md" ]` — the configured templates directory
only — while `diff` (`:34-44`) and `reset` (`:58-68`) resolve through
`config_resolve_template`, which also honours a `templates.<name>` config-path
entry. A template customised that way is "customised" for diff/reset and "not
customised" for eject, which will silently write a second copy.

A third **config-path-customised** fixture pins the asymmetry: `eject` exits 0
and writes, `diff`/`reset` exit 0 and resolve to the config-path file, including
`reset`'s `Note:` message which fires only on that source. Without it, a port
that unified the two resolution paths would change `eject`'s exit code and write
target with no test failing.

The work item's criterion — exit 2 from all three against **not-customised** — is
wrong for `eject`, which succeeds with exit 0 there. This plan supersedes it and
the work item's AC should be amended to match.

`--all` aggregation for `eject` (`config-eject-template.sh:118-137`): any error
wins (exit 1) over any exists (exit 2).

**Template names are validated as identifiers** on the same rule as the skill
name (`^[a-z0-9][a-z0-9-]*$`) — `config_resolve_template`
(`config-common.sh:399-439`) interpolates the key into three candidate paths, and
`reset` *deletes* what it resolves.

An absolute or out-of-project `templates.<key>` **value** remains permitted with
the bash's existing warning — decision **held**, but its rationale is recorded
honestly. The original justification ("a user configured that path deliberately")
no longer holds cleanly: `config set` is net-new capability with no bash
equivalent, and once it is reachable a model can set `templates.<key>` to an
absolute path and then `templates reset --confirm` deletes it (`config-reset-template.sh:92`
is a bare `rm` on the resolved path). So the residual is stated as an **accepted
risk**, not a non-issue: the value is left warn-and-proceed for parity and to
avoid breaking a supported configuration, and the reachability change is noted so
a later reader re-examines it rather than trusting the old rationale. If it is
ever revisited, the fix is to bound `reset`'s delete (and `eject --force`'s write)
by Phase 1's `permitted_root` when the resolution is destructive — recorded as the
lever, not applied here. The validation covers the name, not the destination.

Flag surface preserved exactly: `--force`, `--dry-run`, `--all` on `eject`;
`--confirm` on `reset`. `--all` combined with a name is an error. `reset` without
`--confirm` reports and exits 0; with it, deletes and exits 0. The
out-of-project warning and the two differently-worded `Note:` messages are
preserved.

#### 3. `config init`

**File**: `cli/launcher/src/config_command/` plus an `init` module
**Changes**: reproduces `init.sh` — 14 content directories each with a `.gitkeep`;
`.accelerator/.gitignore` containing `config.local.md` **and the temp-file
pattern** (Phase 1 moved staging into this directory); `.accelerator/state/`,
`skills/`, `lenses/`, `templates/` each with a `.gitkeep`; the tmp dir with its
three-line `.gitignore`; and the anchored root `.gitignore` rule. Idempotent,
silent on stdout.

`init` is the first consumer of `catalogue::default_for`, which has had none.

**Sourcing the 14 directories from `catalogue::default_for` is a source-of-truth
change, not a reproduction — recorded as a divergence.** `init.sh:36` calls
`config-read-path.sh "$key" "$default"` with an explicit non-empty default from
its own `DIR_DEFAULTS` array (`:25-31`), and `config-read-path.sh:31-33` makes an
explicit non-empty `$2` **win over the catalogue** — so `init.sh` never consults
the catalogue for those keys, and `config-defaults.sh:14-16` records the two
vocabularies as deliberately un-unified. The values coincide today, so this is
safe, but after Phase 7 `init` has no golden and no repointed suite, so a future
catalogue-only edit would silently move its created tree. A test captured before
`init.sh` is deleted asserts each of the 14 `DIR_DEFAULTS` values equals
`catalogue::default_for("paths.<key>")`, so the coincidence is pinned rather than
assumed.

**The inner gitignore is ensured, not just created** — a divergence.
`accelerator_ensure_inner_gitignore` (`scripts/accelerator-scaffold.sh:19-27`)
writes the `config.local.md` rule **only when the file does not exist**, so a
repo whose `.accelerator/.gitignore` was hand-edited never gains it and the
credential file becomes committable. The root-level rule is already guarded with
`grep -qFx`, so this is an inconsistency rather than a decision. `init` uses the
same guard, and `config set --level personal` ensures both rules before writing,
failing closed if it cannot.

### Success Criteria

#### Automated Verification

- [x] `mise run test:unit:cli` passes (CI's `cargo nextest --workspace
      --all-features`, not bare `cargo test --workspace`, so the
      `bash-parity`-gated suites are exercised)
- [x] `config set` on either level re-reads identically, and **all content outside
      the edited key — including surrounding Markdown body prose — is
      byte-identical** to the pre-write file
- [x] `config.local.md` is gitignored after a personal write, asserted on a
      fixture where `init` has **not** run and on one carrying a pre-existing
      `.accelerator/.gitignore` that lacks the rule — on an already-initialised
      fixture the assertion passes vacuously
- [x] A traversing skill name (`../../etc/passwd`) or template name is refused by
      `context --skill`, `instructions` and every `templates` subcommand
- [x] A deeply nested key (`a.b.c.d`) round-trips, proving ADR-0047 nesting
- [x] Each of the three **malformed** fixtures: `config set` refuses and leaves
      the file byte-identical (asserted by comparing contents before and after)
- [x] The **non-mapping-root** fixture — a *well-formed* frontmatter whose root is
      a sequence or scalar — is refused and left byte-identical.
      `ConfigService::set` matches `_ => Mapping::new()` (`service.rs:116-119`)
      while `document.rs:33-37` parses every YAML variant successfully, so
      without this the whole frontmatter is silently discarded on first write
- [x] **writeback-failure** fixture → `config set` exits non-zero, stdout empty
- [x] `templates eject` against **already-customised** exits 2; against
      **not-customised** exits 0
- [x] `templates diff` and `templates reset` against **not-customised** exit 2;
      against **already-customised** exit 0
- [x] Against the **error** fixture all three exit 1
- [x] `config set` contains no temp-file or `fs::rename` logic of its own
- [x] `config set --allow-legacy-layout <key> <value>` exits non-zero — the flag
      is rejected on writes, so no split-brain legacy-read/current-write occurs
- [x] `config init` on an empty fixture produces exactly the documented tree; a
      second run is a no-op
- [x] Each of `init`'s 14 `DIR_DEFAULTS` values equals
      `catalogue::default_for("paths.<key>")`, captured before `init.sh` is
      deleted — pinning the source-of-truth coincidence
- [x] `mise run check` and `mise run` exit 0

#### Manual Verification

- [ ] A hand-written `.accelerator/config.md` with prose, comments and blank lines
      survives a `config set` visually unchanged apart from the edited key

---

## Phase 4: Repoint the shell suites at the binary

### Overview

The parity gate. `test-config.sh` and `test-config-read-doc-type-paths.sh` run
green against the compiled binary before any script is deleted.

### Changes Required

#### 1. Shims

**File**: `scripts/test-shims/config-*.sh` (new, `0755`, deleted in Phase 7)
**Changes**: one shim per repointed script, each a two-line
`exec "${ACCELERATOR_BIN:?}" config <subcommand> "$@"`.

**`ACCELERATOR_BIN` is an absolute path to the freshly built launcher**, set once
by the suite — never a bare `accelerator` resolved through `PATH`. A stale global
build on `PATH` would silently validate the wrong binary, and an unresolved name
would produce shim failures indistinguishable from the parity defects this phase
exists to surface. It is also the compiled binary rather than `bin/accelerator`,
so the ~545 runtime assertions do not each pay the bootstrap's process spawns and
whole-binary signature verification; this gate is testing `config` behaviour, not
binary distribution.

Shims are chosen over rebinding to `accelerator` directly because they handle
**all three invocation forms uniformly** with one edit per binding:
`bash "$VAR" args`, the bare exec form `"$VAR" args` (which a `bash "$VAR"`
search-and-replace misses at `test-config.sh:5975,5977`), and — where a suite
merely needs the path to exist — a plain file reference.

#### 2. Rebind

**File**: `scripts/test-config.sh`
**Changes**: repoint the script-path bindings at the shims — `:12-18`, `:20-21`,
`:3040`, `:3355`, `:4172`, `:4228-4233`. Stated as an explicit line list rather
than the range `:12-21`, because that range also spans `CONFIG_DETECT` (`:19`),
which is a hook rather than a `config-*` script and binds to
`accelerator config summary --format=hook`. A mechanical sweep over the range
would silently replace the seven hook-envelope assertions with summary
assertions that pass for the wrong reason. The measurements table's 21 bindings
reconcile as 18 config scripts + `DEFAULTS_FILE` + `CONFIG_DETECT` + the shim
root.

`DEFAULTS_FILE` (`:2441`) is **not** repointed — it is a file-content assertion,
inventory member 2.

Not repointable, left alone and covered by the inventory:
- `:24` sources `config-common.sh` inline, followed by ~60 in-process calls to
  its functions — this tests a bash *library*, and `config-common.sh` survives.
- the 13 `source "$VAR"` sites (array introspection, no CLI analogue).
- `:1013`, `:1023` grep the **source text** of `config-read-agents.sh` and
  `config-dump.sh` to extract their `AGENT_KEYS` array literals.
- the SKILL.md censuses, which Phase 5 rewrites.

**File**: `scripts/test-config-read-doc-type-paths.sh`
**Changes**: **not** a one-line rebind. All 8 call sites use the bare exec form
`"$RESOLVER"`, never `bash "$VAR"` — a pattern-based sweep misses this file
entirely — and they pass a **repo-root positional** (`"$RESOLVER" "$DEFREPO"` at
`:37` and throughout).

The contract is settled in **Phase 2**, where the subcommand is designed, not
here: `config paths --doc-types [root]` accepts the optional root positional,
matching the bash. This is not a test-harness question — `scripts/doc-type-table.sh:41`
passes the root in **production** (`bash "$DOC_TYPE_PATHS_RESOLVER" "$root"`),
and `config-read-doc-type-paths.sh:51-55` uses it to run each read inside
`( cd "$ROOT" && … )`. `doc-type-table.sh` is on Phase 5's shared-helper list and
would otherwise be repointed as a "mechanical one-line swap" that drops the
argument, leaving migration 0007 to resolve doc-type directories against the
caller's CWD — and those directories scope in-place mutation.

`DOC_TYPE_PATHS_RESOLVER` survives as a full **command** rather than a bare script
path, so the two suites that substitute an `exec` stub at it
(`test-validate-corpus-frontmatter.sh:412`, `test-migrate-0007.sh:2208`) keep
their only injection point.

#### 3. Divergence updates

**Changes**: update the assertions covering the recorded divergences, each in a
commit naming the divergence note.

#### 4. Characterise `test-init.sh`

**File**: `tasks/test/integration.py`, `mise.toml`
**Changes**: a **dedicated** `run_shell_suites(context, "skills/config/init")`
call site (not widening the config walk — widening would make the
`_EXPECTED_CONFIG_SUITES` reconciliation below wrong). `skills/config/init` is not
one of the eight subtrees, so this suite has never run in CI. Reaching CI takes
**three** edits, not one: the `integration.py` call site, a matching
`[tasks."test:integration:init"]` entry in `mise.toml`, and adding that task to
the `test:integration` aggregate's `depends` list — otherwise "green in CI at a
recorded commit" is satisfiable by a local `invoke` run CI never performed.

**File**: `skills/config/init/scripts/test-init.sh`
**Changes**: fix what the first run surfaces — it expects `meta/research`,
`meta/design-inventories`, `meta/design-gaps` while `init.sh:25-31` creates
`meta/research/codebase`, `meta/research/design-inventories`,
`meta/research/design-gaps` (14 dirs, not 12). At least 5 assertions fail today.

Record each first-run failure **with a decision: script wrong, or suite wrong.**
"Fix what the first run surfaces" alone would pre-decide that `init.sh`'s 14
directories are correct and the suite's 12 are stale — but the suite is currently
the only recorded expectation of `init.sh`'s contract, and editing it to match
the code destroys the evidence rather than characterising it. The green run's
commit is what the Phase 7 retirement references. **This is the only way to learn
whether the behaviour being ported is the behaviour the script has.**

**Inventory depth for this member is per-assertion, not per-branch.** The
"every branch and every exit code" floor is over `init.sh`'s control flow, and
the suite's most valuable checks belong to no single branch: `tree_hash`
idempotency (`:91-94`), gitignore rule non-duplication (`:101-102`), preservation
of the legacy `.claude/accelerator.local.md` rule (`:104-122`), root discovery
from a deep subdirectory (`:124-131`), and the `paths.tmp` override
(`:133-145`). All five satisfy the branch floor vacuously. The suite is ~25
assertions, so a row apiece is cheap and closes the gap — `init` is a mutation
command with no golden and no repointed suite after Phase 7.

`_EXPECTED_CONFIG_SUITES` is **unaffected** by this wiring: it gates
`run_shell_suites(context, "scripts")` (`integration.py:87-88`), and
`test-init.sh` lives at `skills/config/init/scripts/` — a different subtree. It
drops by **two** at Phase 7, not three, since `test-init.sh` was never in it.

The new call site needs its own floor. Every other subtree has one
(`_EXPECTED_MIGRATE_SUITES`, `_EXPECTED_WORK_SUITES`, …) precisely because
`run_shell_suites` filters on `os.access(p, os.X_OK)`, so a dropped exec bit
silently yields zero suites — which would let the "green in CI at a recorded
commit" criterion pass with the suite never having run, and Phase 7's retirement
cite a run that proved nothing. Add `_EXPECTED_INIT_SUITES = 1` in the same
change.

### Success Criteria

#### Automated Verification

- [x] `bash scripts/test-config.sh` passes with every binding repointed at the binary
- [x] **Rebinding is complete, proven by a known-positive floor**: the repointed
      run with every stubbable removal-set script replaced by a stub that exits
      non-zero and prints its own name **still passes** — so no binding was missed
      and left silently exercising the bash. Without this the parity gate (which
      justifies deleting 6,289 lines of tests) is satisfiable with an arbitrary
      subset repointed. Implemented as `scripts/test-shims/rebind-floor.sh`,
      proven at this commit by running it serially (it stubs `scripts/` in place,
      so it is a standalone proof, not part of the fully-parallel CI aggregate
      which formats/lints/tests the same tree concurrently). Four scripts still
      invoked in-process by retained bash — `config-read-value/path/work.sh` via
      `config-common`/`work-common`, `config-summary.sh` via the `config-detect`
      hook — are excluded and logged; their bindings are proven by the repointed
      suites passing.
- [x] `bash scripts/test-config-read-doc-type-paths.sh` passes repointed
- [x] `test-init.sh` is discovered by `run_shell_suites` **from the CI aggregate**
      (task wired into `test:integration`'s `depends`, not only local `invoke`) and
      **green at a recorded commit**
- [x] The suite-audit table's row count equals what discovery finds at the pinned
      revision, reproducibly
- [x] `mise run check` and `mise run` exit 0

#### Manual Verification

- [x] The repointed suites' failures during development were parity defects, not
      shim defects — spot-checked by running a handful of shims by hand

---

## Phase 5: Call-site and `allowed-tools` cutover

### Overview

The contract flip. 247 `!` sites plus 14 non-`!` sites move to the bootstrap
path, with `allowed-tools` rewritten in lockstep.

### Changes Required

#### 1. Resolve Q1 first

**Changes**: an empirical probe, run against Claude Code **v2.1.144** (the
declared minimum) *and* the current release, with all results and both version
numbers recorded in the work item's RESERVED Assumptions slot before the first
call site is rewritten. An earlier draft treated most of Q1 as "settled by
documentation"; two of those settled claims are moved back into the probe because
the tree's own evidence contradicts them.

1. **Rule shape — does `Bash(<path> config *)` (space-separated wildcard) even
   match?** This is the load-bearing unknown, not a documentation footnote. Every
   one of the tree's 84 `Bash(...)` rules is either a bare command name or a path
   glob with `*` appended **directly** to a prefix (`…/scripts/config-*`), with no
   space and no colon; Claude Code's docs show the prefix form as
   `Bash(npm run test:*)` — the **colon** spelling. The space-separated form has
   no precedent anywhere. If it does not match, all 46 skills throw a permission
   prompt on every config call at load (17 for `visualisation/visualise`), and
   `check-skill-permissions.sh` — written to the same assumption — would agree the
   rule covers the call. So the probe tests `Bash(<path> config *)`,
   `Bash(<path> config:*)` and the colon/space variants against a real invocation
   and records the spelling that works; the "confirm against one real skill"
   manual gate below confirms the **shape**, not just the wildcard reach.

2. **Is the matcher operator-aware, or a literal prefix/glob?** An earlier draft
   filed "the matcher splits on `&&`/`||`/`;`/`|`/… so no metacharacter screening
   is needed" as settled. This repo's own research
   (`meta/research/issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md:83-88,118-126`)
   concluded the **opposite**: a literal prefix/glob match against the command
   string that strips only recognised wrappers (`timeout`, `time`, `nice`,
   `nohup`, `stdbuf`), which is why a leading `bash ` defeats it — incompatible
   with per-segment splitting. The same research records an unresolved report
   (its Hypothesis 3) that `allowed-tools` may be enforced only on the **first**
   matching Bash call per session — directly relevant to a skill making 17 config
   calls at load. Both go in the probe. Until resolved, `check-skill-permissions.sh`
   **rejects any `!` invocation containing a shell metacharacter** regardless — the
   247 sites are metacharacter-free today, so the check costs nothing.

3. **Does `*` span `/` in argument position?** The docs read as spanning
   everything, but glob convention and the tree suggest otherwise; only
   slash-bearing **arguments** are sensitive (`config set paths.work meta/work`
   has a slash, `config path plans` does not). The tree carries counter-evidence:
   an `inventory-design/scripts/playwright/*` rule exists **alongside**
   `inventory-design/scripts/*`, redundant if `*` spanned `/`.

4. **Does `${CLAUDE_PLUGIN_ROOT}` expand in `allowed-tools`, and from which
   version?** Only `${CLAUDE_PROJECT_DIR}` substitution is documented, and only
   from **v2.1.196** — above this plugin's declared v2.1.144 floor. The 35
   existing rules evidently work, so it expands in practice, but the floor may
   already be wrong for the *current* plugin. The floor itself is prose-only and
   triplicated (`docs/releases-and-compatibility.md:36`, `CLAUDE.md:121`,
   `ADR-0051:117`) with nothing machine-readable — so a probe result that moves it
   has no single home. Phase 5 names `docs/releases-and-compatibility.md` as the
   canonical copy with a coherence check over the other two, so a floor change has
   somewhere to land; if substitution requires v2.1.196 that is recorded as a
   finding against the plugin, not this phase.

**Consequence if `*` stops at `/`.** The read subcommands are unaffected — their
arguments are single tokens. `config set` is the one subcommand routinely taking
a path argument, so it acquires its own narrow rule, in `skills/config/configure`
only, which is the sole caller. That falls out of the matcher rather than being a
permission-scoping decision, but it has the same effect: the ~25 read-only skills
do not carry a write-capable rule.

The rule is shaped to be indifferent where possible:

```
- Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator config *)
```

**The prefix stops at `config`, not at `accelerator`.** `Command` carries an
`external_subcommand` catch-all (`launch/inbound/cli.rs:15-22`), so a rule ending
at the binary name would grant the launcher's whole dispatch surface — and would
silently pre-authorise every sub-binary 0168, 0169 and 0173 add, in all 35 files,
on the day it ships. Naming the subcommand costs nothing and keeps the grant
matched to what this story migrates.

The grant is still broad *within* `config`: the ~25 read-only skills also gain
`config set`, `config init` and the destructive `templates` subcommands. This is
**a recorded, accepted residual risk — not a benign grant**, and the honest
framing is the one in Phase 3 §1, not a "reviewed commits" premise: several of
these skills (`review-pr`, `respond-to-pr`, `show-jira-issue`,
`search-jira-issues`, the Linear equivalents, `extract-work-items`,
`research-issue`) ingest attacker-controllable PR diffs and issue bodies that are
**not** reviewed, so a steered `config set jira.token_cmd '<payload>'` (later
`bash -c`'d) or `templates reset --confirm` on an absolute path is reachable. The
decline is held for parity and scope (10 of the 35 declare bare `- Bash` and
permit everything regardless), but the closing lever — split the grant into a
read-only rule for the read-only skills, enforced by `check-skill-permissions.sh`
— is recorded, not claimed unnecessary. Recorded so a later reader sees a
considered residual, not a "safe because reviewed" that is false for these skills.

If the probe says `*` does not span `/`, add the narrow per-subcommand rules the
probe result dictates.

#### 2. Rewrite call sites

**Changes**: all 247 `!` sites — every one has the identical shape
`` !`${CLAUDE_PLUGIN_ROOT}/scripts/config-<name>.sh[ <one-token-arg>]` `` with no
pipes, `&&`, `$(…)`, `bash` prefix, quoting or multi-arg call.

Mechanical **except** for three deliberate reshapings:

1. **Every injection site gains `--fail-safe`.** Degradation is opt-in
   (divergence 5), so a site rewritten without the flag exits non-zero on a read
   failure and discards the prompt. `check-skill-permissions.sh` asserts the flag
   is present on every `context`, `instructions`, `agents`, `review` and
   `summary` invocation in a SKILL.md, so a missed one fails the build rather
   than a session.
2. **Paired `context` + `skill-context` calls collapse to one.** Where a SKILL.md
   invokes both — all 42 files that call either call both, adjacently,
   project-first — the pair becomes a single
   `config context --skill <name> --fail-safe`, and the intervening SKILL.md
   markdown between the two former `!` blocks is removed, since the separator now
   comes from the command.

   Output follows **Phase 2's join rule**, which is *not* plain concatenation:
   concatenating the two scripts yields zero blank lines, while the rule inserts
   one when both blocks survive. Recorded as divergence 13. This is the only
   reduction in call count in this migration.
3. **`skill-instructions` becomes `instructions`** at its `!` sites.
   (`doc-type-paths` → `paths --doc-types --format tsv` is **not** an `!`-site
   reshaping — `config-read-doc-type-paths.sh` has zero `!` sites; it is a
   shell-consumer repoint handled in §4b.)

The 28 shell consumers are rewritten **without** `--fail-safe` — they want the
loud failure.

Plus the 14 non-`!` sites, which is where all the irregularity lives:
- `skills/config/configure/SKILL.md:925,937,953,962,968,975,993,1012,1023` — the
  only flagged and multi-arg invocations anywhere (`--dry-run`, `--force`,
  `--confirm`, `--all`), in fenced code blocks. This file has **no
  `allowed-tools` key at all** and needs one added.

  **`allowed-tools` is an allowlist, not an additive grant.** The file runs
  unrestricted today and uses the Edit tool at `:1028` to modify config files —
  adding a key carrying only the Bash rule would silently remove the ability that
  is this skill's entire purpose. Its full current tool usage is enumerated
  first, and the key lists every tool it uses, not just the new Bash path. This
  is the one file where the cutover can break the write path while every
  read-path check still passes.
- `skills/integrations/jira/init-jira/SKILL.md:61,72` and
  `skills/work/extract-work-items/SKILL.md:350,352` and
  `skills/integrations/jira/create-jira-issue/SKILL.md:150` — prose instructions
  to run a script.
- `skills/config/init/SKILL.md:45` — `bash "${CLAUDE_PLUGIN_ROOT}/…/init.sh"`
  inside a fenced block; contains no `config-` segment, so only Grep A finds it.
- `skills/review/output-formats/work-item-review-output-format/SKILL.md:63` — a
  prose reference needing a text update.

`skills/integrations/jira/create-jira-issue/SKILL.md:114` **sources**
`config-common.sh` and is left alone — that library survives to 0174.

#### 3. Rewrite `allowed-tools`

**Changes**: replace the `config-*` glob in 35 frontmatter blocks (normalising the
one with a trailing space). Two special cases:

- **`skills/vcs/commit/SKILL.md:7`** declares the broad
  `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/*)` because it also calls `vcs-status.sh`
  and `vcs-log.sh`. The bootstrap path is `bin/accelerator`, which falls
  **outside** `scripts/`, so this file **gains a rule** rather than having one
  rewritten — the one file that could silently break.
- The **10 integration write-skills** declaring bare `- Bash` already cover the
  new path; no change needed, but the coverage script must confirm it.

#### 4. Re-home the browser executor

**Changes**: `skills/config/browser-executor/SKILL.md` gains its own narrow rule
for `config-read-browser-executor.sh` **in the same commit** that removes the
`config-*` glob from its frontmatter block. Its entire `config-*` footprint is
one line (`:24`).

#### 4b. Repoint the 28 shell consumers

**Changes**: every non-SKILL.md production `.sh` invoking a removal-set script
swaps the path for `${…}/bin/accelerator config <subcommand>`. These carry no
`allowed-tools` implications — they are shell, not `!`-preprocessor sites — so a
swap is one line **except where the site suppresses failure** (below), which is
not mechanical.

Grouped by owner, repointed in that grouping so a regression is attributable:

- six work-item scripts (`work-item-{create-remote,next-number,read-field,`
  `resolve-id,sync-baseline,template-field-hints}.sh`)
- four visualiser scripts (`launch-server`, `status-server`, `stop-server`,
  `write-visualiser-config`)
- four Jira and two Linear scripts
- four shared helpers (`doc-type-inference`, `doc-type-table`,
  `validate-corpus-frontmatter`, `work-common`)
- `adr-next-number.sh`, `inventory-design/scripts/playwright/run.sh`
- the seven migrations `0001`-`0007` (`0007` only transitively, through
  `doc-type-table.sh`)

**Failure suppression is not confined to the migrations.** The same grep that
built the migration table, re-run over **all** consumers, finds eight more sites
with the identical suppressing shape outside `migrations/`:
`write-visualiser-config.sh:64,65,91,217,263,267` (`2>/dev/null || true`), `:185`
(`|| echo "$KANBAN_DEFAULT"`), and `launch-server.sh:110` (`|| true`). Under the
repointed contract these reads can newly fail (legacy-layout refusal now uniform,
malformed frontmatter now fail-loud, a bootstrap failure) where the bash
succeeded, and each failure collapses to empty. The worst is
`write-visualiser-config.sh:64-66`: the tickets→work pre-flight guard branches on
`[ -n "$TICKETS_OVERRIDE" ]`, so an unread config makes the guard **not fire** and
the visualiser launches with the empty kanban that guard exists to prevent. The
full site table is committed, and every one of these eight gets the same
capture-and-distinguish conversion as the migrations (non-zero fatal, exit-0-empty
keeps "not configured"), each exercised against a stub `accelerator` that fails.

**`jira-auth.sh` and `linear-auth.sh` are the highest-risk two** — they resolve
the integration credential, so a defect there is an authentication outage rather
than a degraded block. Each gets a criterion of its own.

**The migrations are the second risk, in two distinct ways.**

*Layout.* They run against repos that by definition still carry the legacy layout
— the state the uniform gate refuses, and which 0178's dropped
`ACCELERATOR_MIGRATION_MODE` bypass no longer excuses. Every repointed config
read inside `0001`-`0006` passes **`--allow-legacy-layout`** directly; `0007`
reads config only transitively through `doc-type-table.sh`, which passes the flag
from its allowlisted position (Phase 2 §1). Phase 2's flag — including its
source-fallback half, implemented in `FileConfigStore::level_path` — is a hard
precondition of this phase.

*Failure handling.* The existing call sites do not propagate failures, they
suppress them. The list is derived by grep, not by hand — an earlier draft omitted
`0006:59-60`, which is the worst of them:

| Site | Form |
|---|---|
| `0001:31,33` | `2>/dev/null \|\| true` |
| `0004:383` | `2>/dev/null \|\| echo ""` |
| `0004:459` | `2>/dev/null \|` (pipeline) |
| `0006:59-60` | `2>/dev/null \|\| true`, inside `resolve_corpus_path()` |
| `0006:335,356,372` | `2>/dev/null \|\| true` |

Each is followed by an empty-means-skip guard. Under the repointed contract these
reads can fail in ways the bash could not (a bootstrap failure, a missed flag),
and every such failure becomes "nothing to migrate". The migration exits 0,
`run-migrations.sh:655` appends it to `migrations-applied`, and it never runs
again — a permanently half-migrated repo with no recovery short of hand-editing
the ledger.

Each read is converted so that a **non-zero exit** is fatal while **exit 0 with
empty output** keeps its existing "not configured" meaning. `0005:17-20` is the
model for the non-zero half only — it dies on *empty*, which is `0005`'s own
policy and must not be generalised, since `0001`, `0004` and `0006` deliberately
skip on empty.

**The conversion must be structural, not a per-site `log_die`.** `0006:59-60`
sits inside `resolve_corpus_path()`, invoked at `:298` as
`if ! rel="$(resolve_corpus_path "$key")"` — a command substitution inside a
condition, where `set -e` is suspended and `log_die`'s `exit 1` terminates only
the subshell. `walk_corpus` would then take the `if !` arm, print
`rewrote 0 file(s) under <unresolved …>`, `return 0`, and the migration would exit
0 and be recorded as applied — exactly the outcome being fixed, at the one site
that motivates the fix.

So the read's status is **captured and returned as a distinct value**:
`resolve_corpus_path` returns 2 for "read failed" and 1 for "not configured". The
caller reshape is spelled out, because the existing `if ! rel="$(…)"` form
**discards** the graded return — inside the `then` branch `$?` is the `!`
pipeline's status (0), not the function's 1 or 2. The site becomes:

```sh
rc=0; rel="$(resolve_corpus_path "$key")" || rc=$?
case "$rc" in
  2) log_die "config read failed for $key" ;;                       # fatal
  1) echo "0006: rewrote 0 file(s) under <unresolved $key>"; return 0 ;;
esac
```

The `rc=1` branch uses `return 0` (not `continue`) and keeps `walk_corpus`'s
existing "rewrote 0 file(s)" diagnostic (`0006:298-301`), because the site is
inside the `walk_corpus` *function* — a bare `continue` would depend on
propagating to the caller's loop and would drop the diagnostic. Only reads
reshaped *directly* inside a `for` loop (e.g. `0006:335`) use `continue`. `if !`-
and `[[ … ]]`-wrapped invocations cannot observe a graded return, so any read
converted inside a function called in a condition or command-substitution context
is reshaped this way, not left as an `if !`.

**Recovery exists for a discovered half-application.** `run-migrations.sh` has
`--skip`/`--unskip` for the *skip* list but no way to un-record an *applied*
migration, so today a half-applied entry in `.accelerator/state/migrations-applied`
can only be removed by hand-editing a file most users never find. An
`--unapply <id>` affordance is added, so the conversion's fatal path has a
supported recovery rather than depending on the ledger never being wrong.

#### 5. The verification scripts

**File**: `scripts/check-skill-permissions.sh` (new, `0755`)
**Changes**: for every SKILL.md, extract **every `!`-preprocessor invocation —
bare-path and `accelerator` alike** — and assert each is covered by at least one
`Bash(...)` rule in that file's frontmatter under Q1's resolved semantics. Exits
non-zero on any uncovered invocation.

It also **rejects any `Bash(...)` rule that matches the bootstrap path without
naming a subcommand** — evaluated against the binary path rather than keyed on
the literal, so an ancestor glob (`Bash(${CLAUDE_PLUGIN_ROOT}/*)`,
`Bash(${CLAUDE_PLUGIN_ROOT}/bin/*)`) is caught too. That form already exists in
the tree at `skills/vcs/commit/SKILL.md:7`, which is also the one file Phase 5
gives a *new* rule — making a later "simplification" to a single broad glob the
most natural edit anyone would make there.

And it **asserts `--fail-safe` on every `bin/accelerator config` invocation
appearing in a `!` block**, not merely on the five injection subcommands. Most of
the 247 sites are `path` and `get`, where a missed flag regresses the bash's
warn-and-emit-empty into a non-zero exit that discards the prompt — the identical
failure mode, and the bulk of the corpus. A missed flag must fail the build
rather than a live session.

Extraction **must** include bare paths: the browser executor is invoked that way,
and an `accelerator`-only extractor would never examine the one call the
re-homing requirement exists to protect.

It also **rejects any `!` invocation containing a shell metacharacter**
(`&&`, `||`, `;`, `|`, `$(`, backtick, `<(`, `>(`), pending the Q1 probe result
on whether the matcher is operator-aware or a literal prefix/glob. If the matcher
is a literal prefix match (as this repo's own research concluded), a chained
command could smuggle an unmatched subcommand past a rule that covers only the
first; the 247 sites are metacharacter-free today, so rejecting them outright
costs nothing and does not depend on the unresolved matcher semantics.

**File**: `scripts/check-call-site-migration.sh` (new, `0755`)
**Changes**: Grep A and Grep B, both over a **fixed corpus defined in the
script**, so a scope chosen at verification time cannot be narrowed until it
passes.

Grep A is **two patterns over one corpus**, because basename matching (added to
catch the Rust and comment references a path pattern misses) also surfaces
*mentions* in files that legitimately survive, so a single "exactly 0" is
unachievable. The corpus is **the whole tree minus a committed exclude list** —
`meta/` (migration prose would match), `docs/`, `CHANGELOG.md` (an immutable
release record that names removal-set scripts historically and must not be
rewritten), and the removal set itself. **`cli/` is not excluded**: Phase 7 §3
repoints all four Rust dependants, so `cli/` is clean at the final state, and
excluding it would contradict the known-positive floor below, which requires two
references *inside* `cli/`.

- **Grep A-functional** — an *invocation or `source` shape*: `$VAR/<name>`,
  `"…/<name>"`, `bash <name>`, `require_script(<name>)`, `exec <name>`, matched by
  **basename as well as path**. Path-only matching let real breakage through:
  `"$SCRIPT_DIR/config-read-value.sh"` (`config-common.sh:407`),
  `require_script("scripts/config-read-doc-type-paths.sh")`
  (`corpus-adapters/tests/common/mod.rs:66`), and `source "$scripts/config-dump.sh"`
  inside a Rust test's bash payload (`catalogue.rs:301`) carry no matchable
  `scripts/` segment. **This is the gated pattern: post-migration exactly 0.** Its
  pre-migration run is the known-positive floor, and that floor must include
  `config-common.sh:407,422`, `catalogue.rs:301,325` and
  `corpus-adapters/tests/common/mod.rs:66` — a basename-blind pattern returns zero
  for all while the tree is whole.
- **Grep A-mention** — any other textual reference to a removal-set basename.
  **Not gated to zero**; reported for review so a surviving mention (a retained
  comment in `config-common.sh:315,394`, `config-defaults.sh`,
  `config-read-browser-executor.sh:6`, `visualise/server/src/config.rs:823`) is
  visible without failing the build. The companion criterion "no retained file
  references a removal-set member" is scoped to the **functional** pattern; retained
  comments are either left (they are inert) or rewritten in the same commit, but
  they do not make the gate unachievable.

  `meta/inventories/0167-removal-set-references.md` records the full audit (95
  referencing files across SKILL.md, shell, suites, Rust and config), with
  `meta/inventories/0167-audit-removal-set.sh` as the re-runnable generator.
- **Grep B** — the literal `grep -rn 'scripts/config-' --include=SKILL.md skills/`.
  Pre-migration total **297**, of which the browser-executor subset is **1**.
  Post-migration: **exactly 1**, and that hit is the browser executor.

#### 6. Record Q2

**Changes**: write the resolved 46-vs-35 explanation into the work item's Context.
The gap is benign: 1 file (`vcs/commit`) under a broader `scripts/*` rule and 10
integration write-skills under bare `- Bash`. **Nothing is broken today**, and no
file needs a rule *added* for the bash surface — only `vcs/commit` and
`configure` need one for the new path.

### Success Criteria

> **Progress (2026-07-21).** Split into two increments. **Increment A — the
> SKILL.md contract flip — is committed and `mise run`-green** (`6e509b2e`):
> all 247 `!` sites and 14 non-`!` sites repointed at `bin/accelerator config`
> (`--fail-safe` on every config `!` site; the 42 context+skill-context pairs
> collapsed; `skill-instructions`→`instructions`), the 35 `allowed-tools` blocks
> swept (plus `vcs/commit` gaining the rule, `browser-executor` narrowed,
> `configure` gaining a full key), `check-skill-permissions.sh` built + wired
> into `lint:check`, and the SKILL.md census suites (`test-config.sh`,
> `test-design.sh`, `test-skill-frontmatter-population.sh`) rewritten to the new
> contract (the work-consumer census left **transitional** — accepts either the
> old script or the new form — so A could land before §4b). **Increment B —
> §4b (the 28 shell consumers + migrations graded-return + `--unapply` +
> `check-call-site-migration.sh` + tightening the transitional censuses) — is
> still to do.** Boxes below are ticked accordingly.
>
> **Progress (2026-07-21, increment B).** All 28 shell consumers are repointed
> at `"${ACCELERATOR_BIN:-$PLUGIN_ROOT/bin/accelerator}" config …` across six
> `mise run check`-green commits: work-management scripts + `work-common`;
> jira/linear (`-auth` via `config get`, `-common`/`-resolve-fields`/init-flow);
> `adr-next-number` + the inventory-design playwright launcher; the
> `doc-type-table` resolver (now `config paths --doc-types --format tsv`,
> word-split, appending `--allow-legacy-layout` under migration mode); the four
> visualiser scripts (the eight failure-suppression reads converted to
> distinguish a fatal non-zero read from an unset key, each with a failing-stub
> test); and migrations `0001`/`0002`/`0004`/`0005`/`0006` (graded-return
> conversions — `resolve_corpus_path`/`resolve_user_template_path` return a code
> because they run inside command substitutions — plus `--allow-legacy-layout`
> on every direct read). `run-migrations.sh --unapply` lands with a test. A
> shared `ensure_accelerator_bin` invoke helper points the config/work/
> integrations/migrate/decisions/visualiser suites at the compiled launcher.
> `check-call-site-migration.sh` is built and wired into `lint:check`: Grep B (no
> removal-set config- script in a SKILL.md) and the `--allow-legacy-layout`
> confinement (migrations/ + doc-type-table.sh only) gate immediately; Grep
> A-functional gates to zero **outside** a `PENDING_PHASE7` allowlist (deviation
> below). The transitional work-consumer census is tightened to `config work`,
> and Q2 was already recorded in Context during planning. `mise run check` is
> green with all of the above.
>
> **Deviation (approved 2026-07-22): the gate allowlists the not-yet-migrated
> references.** The Phase-5 criterion "Grep A-functional returns exactly 0" is
> only literally reachable at Phase 7 — the removal set, `scripts/test-shims/`,
> `scripts/test-config.sh`, the `cli/` dependants (`catalogue.rs` `EXTRACT`,
> `config-adapters/tests/parity.rs`, `corpus-adapters/tests/common/mod.rs`) and
> `config-common`'s `config_resolve_template` still functionally name removal-set
> basenames until Phase 7 §2-§3 deletes/repoints them, and
> `hooks/config-detect.sh` until Phase 6. The gate carries a committed
> `PENDING_PHASE7` allowlist naming exactly those files, each held to a
> known-positive floor (it MUST still contain a functional reference), so Phase
> 6/7 empties the list rather than leaving it to rot — mirroring the
> `store`-duplication lint's allowlist. It catches any NEW functional reference
> outside the allowlist today.
>
> **Still to do:** the per-migration failing-stub / per-k proofs (plan §4b — the
> graded conversions are implemented and every migrate suite is green, but no
> test yet injects a failing stub to assert a migration exits non-zero AND stays
> out of `migrations-applied`); and the Phase-2 §6
> member-1 disposition of the now-vacuous `test-config.sh` inline-default censuses
> (they grep the old names, pass vacuously, are superseded by the new gates, and
> are deleted in Phase 7).

#### Automated Verification

- [ ] **Entry precondition — 0165's artefact gate.** Signed, checksum-verified
      launcher artefacts exist for **all four** triples in `bin/accelerator`'s
      `case` arms (`darwin-arm64`, `darwin-x64`, `linux-arm64`, `linux-x64`), **for
      a release that carries the `config` subcommand** (i.e. Phases 1-4). "At the
      currently published version" is not enough — a release predating `config`
      would make every migrated call fall through `external_subcommand` and fail,
      and would make the latency measurement below measure the wrong code path.
      This is a **new check**, not an extension of `validate_version_coherence`
      (`tasks/build.py:184-210`), which compares local version strings and has no
      network dependency
- [ ] **Pre-release gate — latency budget.** The aggregate latency measurement
      (Performance Considerations) is run against the signed `config`-carrying
      release (overrides unset), over the **post-batching shipped invocation
      shape** (not the raw un-batched set the product no longer issues), with a
      **fixed working directory** (both repo-root and a representative subdirectory
      recorded, since bash `find_repo_root` forks per ancestor level while the Rust
      walk is depth-invariant), and **passes the pre-committed 25% bound**. It is
      gated at the end of Phase 5's batching carve-out and **before the release
      that carries the flip to users** — call sites may coexist with bash in `main`
      until that release (Migration Notes), so this is still before user impact
      while measuring the shape that actually ships. The measured absolute
      aggregate and the launcher byte size are committed as artefacts that survive
      Phase 7's deletion of the bash comparator
- [ ] A **standing release-pipeline gate** refuses to publish a `plugin.json`
      version bump unless all four signed triples are built for it. The entry
      check alone is insufficient: `tasks/release.py:87` bumps the version inside
      the release run, so `plugin.json` on `main` names the last *published*
      release and says nothing about the release that will carry these call sites
      to users. After Phase 5 every future bump creates an outage window, so the
      gate has to be recurring rather than one-shot. It lives in the release task
      (network-dependent, runs at publish time), never in `mise run check`
- [ ] **The publish is reordered so it cannot leave a half-published version on
      `main`.** `tasks/release.py:69-76` today commits and pushes the version bump
      and tag **before** `upload_and_verify_release`, so an upload failure after
      the push leaves `plugin.json` naming a version `bin/accelerator` cannot
      fetch. Artefacts are uploaded and verified before the bump commit/tag are
      pushed (or the bump is the last step), and a simulated upload failure is
      asserted to leave `plugin.json` on `main` at the previous, fully-published
      version
- [x] `bash scripts/check-call-site-migration.sh` — **Grep A-functional** returns
      exactly 0, Grep B exactly 1; **Grep A-mention** is reported (not gated).
      **Deviation (approved): Grep A-functional is zero _outside_ the committed
      `PENDING_PHASE7` allowlist** (the Phase-6/7-owned refs), each held to a
      known-positive floor; Grep B is asserted as "no removal-set config- script
      in a SKILL.md" (the browser executor and config-common are the permitted
      survivors). See the increment-B progress note.
- [ ] The pre-migration Grep A-functional run is recorded and demonstrably
      non-zero, and its floor includes the three references a path-shaped pattern
      misses — `config-common.sh:407,422`, `catalogue.rs:301,325` and
      `corpus-adapters/tests/common/mod.rs:66` (all reachable because `cli/` is
      **in** the corpus)
- [x] No **retained** file *functionally* references a removal-set member at the
      final state (Grep A-functional over the whole tree); surviving mentions in
      `CHANGELOG.md`/`docs`/retained-comment files are excluded or reported, not
      gated
- [x] All 28 shell consumers are repointed, **including the eight non-migration
      failure-suppression sites** (`write-visualiser-config.sh` ×7,
      `launch-server.sh:110`) converted to distinguish non-zero from
      exit-0-empty; a tree-wide functional scan for removal-set paths outside the
      exclude list returns 0 (the scan is `check-call-site-migration.sh`'s Grep
      A-functional, zero outside the approved `PENDING_PHASE7` allowlist)
- [x] `bash skills/integrations/jira/scripts/test-jira-auth.sh` and the Linear
      equivalent pass repointed — the credential path resolves through
      `accelerator config get` and an absent token still fails closed
- [ ] Every migration `0001`-`0007` runs green against a **legacy-layout** fixture
      carrying a **non-default value for every key any migration reads**
      (`paths.tickets`, `work.id_pattern`, `paths.plans`, `paths.research_codebase`,
      `paths.research_issues`, `paths.templates`, `paths.work`, a `templates.<name>`
      entry), and per migration the **configured** directory — not the catalogue
      default — is demonstrably the one walked. "Runs green" alone cannot catch a
      migration reading `meta/plans` instead of a configured `docs/plans` and
      rewriting the wrong corpus
- [x] A repointed config read that exits **non-zero** makes its migration exit
      non-zero, and that migration is **not** appended to
      `.accelerator/state/migrations-applied` — exercised with a stub
      `accelerator` that fails. Exit-0-with-empty keeps its "not configured"
      meaning (`test-migrate.sh` "a failed config-path read aborts 0006 without
      recording it", with a pass-through control)
- [x] The same holds for a read reached **through a helper invoked in a condition
      or command-substitution context** (`0006:298` → `resolve_corpus_path`), where
      `set -e` is suspended — asserted against the reshaped `rc=$?` caller, since
      the original `if !` form discards the graded return (the same test stubs
      `config path`, which `resolve_corpus_path` reads in a command substitution)
- [ ] **Per-site conversion is proven, not sampled**: a stub failing on exactly
      the k-th config invocation, for **every** k in 1..N across the migration's
      suppression sites, makes its migration exit non-zero and leaves
      `migrations-applied` unchanged — a stub that fails on every call would only
      ever prove the first site was converted. A re-run with a working binary
      converges to the same tree as an uninterrupted run
- [x] `run-migrations.sh --unapply <id>` removes an applied entry, so a discovered
      half-application has a supported recovery
- [x] `--allow-legacy-layout` appears nowhere outside
      `skills/config/migrate/migrations/` **and the allowlisted
      `scripts/doc-type-table.sh`**, asserted by `check-call-site-migration.sh`
- [x] A full `/accelerator:migrate` run takes a legacy-layout fixture to the
      current layout end to end — the deadlock case, exercised rather than
      reasoned about (`test-migrate.sh` Test 2, green against the compiled
      launcher)
- [x] `bash scripts/check-skill-permissions.sh` exits 0 — including its assertion
      that `--fail-safe` is present on **every** `bin/accelerator config`
      invocation in a `!` block (the flag is present everywhere; its effect
      differs per the fail-safe matrix), and its rejection of any `!` invocation
      containing a shell metacharacter
- [x] The checker rejects an ancestor-glob rule (`Bash(${CLAUDE_PLUGIN_ROOT}/*)`)
      that would match the bootstrap path without naming a subcommand
      (negative-tested against a broken rule, a missing flag, and metacharacters)
- [ ] **Same-commit re-homing**: the coverage script is replayed against **each
      commit** in the migration range and exits 0 at every one; the replay output
      is committed. Final-tree checks cannot distinguish a rule added in the right
      commit from one added three commits later
- [x] `bash scripts/test-design.sh` passes with its SKILL.md censuses updated
- [x] The SKILL.md census invariants (injection present in exactly 42 skills,
      skill-context immediately follows context, skill-instructions is the last
      preprocessor line, skill-name matches frontmatter `name`, `configure`
      excluded) have a **named surviving gate** at the final state — rewritten
      into `check-skill-permissions.sh` (census items 5-7, negative-tested)
      per the inventory member-1 disposition, **not** left only
      in `test-config.sh` (which Phase 7 deletes); the `context`+`skill-context`
      collapse makes the ordering invariant newly fragile, so it is explicitly one
      of them. (The pre-collapse "skill-context immediately follows context"
      ordering is subsumed: the pair is now a single `config context --skill`
      call, so it holds by construction.)
- [x] `mise run check` and `mise run` exit 0

#### Manual Verification

- [x] Q1's two open answers, and **both** verified Claude Code versions
      (v2.1.144 and v2.1.216), are written into the work item's Assumptions slot
      **before** the first call site was rewritten
- [x] The winning rule shape is confirmed against **one real skill** in a live
      session before the remaining 34 frontmatter blocks are rewritten (the
      `probe-q1` skill, run on both versions)
- [ ] In a live session, invoking `/accelerator:commit` and one integration write
      skill produces **no new permission prompt**
- [ ] `/accelerator:configure` loads and its injected blocks render correctly
- [ ] `/accelerator:configure` **writes** — a create-or-edit flow completes end to
      end and the file lands on disk. The read path passing proves nothing about
      the tools its new `allowed-tools` key now bounds

---

## Phase 6: The `config-detect` hook

### Overview

Move the SessionStart registration onto the CLI and define the envelope 0169
inherits. The `--format=hook` implementation itself lands in Phase 2 so its
existing assertions can be repointed rather than hand-ported; this phase is the
registration change and the cross-item record.

### Changes Required

#### 1. `--format=hook` (implemented in Phase 2; contract recorded here)

**File**: `cli/launcher/src/config_command/`
**Changes**: `accelerator config summary --format=hook` wraps the plain command's
output in the transport envelope, serialised with `serde_json`:

```json
{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"<summary text>"}}
```

Three output states, all load-bearing:

1. **Summary present** → the envelope, exit 0.
2. **Summary empty** → **nothing at all** — not `{}`, not an envelope with an
   empty string. `config-detect.sh:17` only prints when the summary is non-empty;
   emitting an empty envelope would inject a blank context block into every
   session.
3. **Summary unavailable** → nothing on stdout, exit 0, diagnostic on stderr.

The jq-missing `{"systemMessage": …}` branch does **not** carry over — it exists
only because the bash shells out to `jq`. Note the serialisation also moves from
`jq`'s pretty-print (`config-detect.sh:18` builds the envelope **without** `-c`,
so today's SessionStart stdout is multi-line) to `serde_json`'s compact single
line. Functionally benign for a JSON parser, but recorded so Phase 4's repointed
`CONFIG_DETECT` assertions (`:764,:769,:783,:798,:5088,:5098-5099`) are checked
for layout sensitivity (a line count, a `grep -q` on a lone-line key) before
repointing, not diagnosed as a behavioural failure.

`--format=hook` is an application of Phase 2's revised `--format` rule (a format
switch is permitted where the same data has genuinely distinct machine and human
consumers), not an "exception": here the human consumer is the rendered summary
and the machine consumer is the enveloped one. Stated once here so the earlier
"accepted exception" framing does not read as a contradiction of the `paths
--format` decision.

#### 2. Registration

**File**: `hooks/hooks.json`
**Changes**: the `config-detect` registration invokes
`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config summary --format=hook --fail-safe`
directly.

**`--fail-safe` is required in the registration**, not implied by `--format=hook`.
Without it the three output states contradict divergence 5: degradation is
opt-in, so an unreadable or legacy-layout config would exit non-zero, and Phase
6's contract requires exit 0 in all three states. `config-detect.sh:14` swallows a
failing `config-summary.sh` via `|| SUMMARY=""` today, so a session in such a repo
is silently context-free rather than hook-failing; the flag preserves that.
`config-summary.sh:11` already calls `config_assert_no_legacy_layout`, so under
divergence 1's uniform gating this is a common path, not a rare one.
`config-detect.sh` is deleted — it contributes only the jq guard, the emptiness
test and the envelope, all of which move into the CLI.

**First argument-bearing registration.** All four current entries in
`hooks/hooks.json` are bare script paths. Whether the `command` field is
shell-interpreted — so `${CLAUDE_PLUGIN_ROOT}` resolves *and* the argument tokens
split — is undocumented, and the tree carries no precedent to infer from. If the
field is exec'd as a single path the summary silently stops appearing, with no
error. Fold this into the Phase 5 probe and settle it in the same session; if
arguments do not arrive, keep a thin `hooks/config-detect.sh` whose only job is
to exec the bootstrap path with them, rather than deleting it.

**Migration hazard**: `hooks/test-vcs-detect.sh:620-634` hard-codes
`.hooks.SessionStart[0]` and asserts `SessionStart[0].hooks | length == 1`.
`config-detect` is `SessionStart[1]`, so removing the group object entirely — not
just its command — would shift indices. Keep `vcs-detect` at index 0.

`vcs-detect`, `vcs-guard` and `migrate-discoverability` remaining on bash is
expected. Note `config-detect.sh:9` derives `SCRIPT_DIR` from `BASH_SOURCE` and
invokes by bare relative path; only `migrate-discoverability.sh:23` honours
`${CLAUDE_PLUGIN_ROOT}`. Calling the bootstrap path from `hooks.json` — which
does expand the variable — resolves that inconsistency for this hook.

#### 3. Record on 0169

**File**: `meta/work/0169-*.md`
**Changes**: record the SessionStart envelope contract and the "bootstrap path"
naming, and note that **PreToolUse's envelope is 0169's own to define** —
`vcs-guard.sh` emits `{decision, reason}` and `{decision, hookSpecificOutput}`,
an unrelated shape, so there is no single envelope spanning all hooks.

0169 also needs three corrections its text currently gets wrong: it says
"wrapper" at `:32`, `:52-55`, `:83`; its AC2 (`:67-69`) asserts one shared
envelope for both SessionStart and PreToolUse; and it names
`accelerator config detect` (`:53-54`) against this story's `config summary`.

### Success Criteria

> **Progress (2026-07-22).** `hooks/config-detect.sh` is rewritten as a thin
> exec-wrapper of `${…}/bin/accelerator config summary --format=hook --fail-safe`
> (dropping the jq guard, the emptiness test and the envelope, all now in the
> CLI). **Deviation (approved by the "push through Phase 7" direction): the
> registration is NOT inlined into `hooks.json` and the wrapper is NOT deleted.**
> Whether `hooks.json`'s `command` field expands `${CLAUDE_PLUGIN_ROOT}` and
> splits argument tokens can only be settled by a live SessionStart, which cannot
> run headlessly — so the plan's own fallback (keep a thin wrapper) is taken.
> `hooks.json` is therefore unchanged, which also keeps `vcs-detect` at index 0.
> 0169 is updated with the envelope contract, the bootstrap-path naming, the
> distinct PreToolUse envelope, and the shared arg-splitting probe. The wrapper is
> lint-clean (shellcheck/shfmt/bashisms) and `config-detect.sh` is dropped from the
> call-site gate's Phase-7 allowlist.

#### Automated Verification

- [x] `bash hooks/test-vcs-detect.sh` still passes (index-sensitive assertion)
- [~] The registration's arguments actually arrive — `hooks.json` expands
      `${CLAUDE_PLUGIN_ROOT}` and splits the argument tokens, per the Phase 5
      probe. **Deferred (live-session probe): the thin `hooks/config-detect.sh`
      wrapper survives and takes no arguments in `hooks.json`, so the arg-splitting
      question does not block; it is 0169's to resolve before inlining.**
- [x] `mise run check` exits 0 (confirmed); the bare `mise run` exits 0
      end-to-end (validated 2026-07-22)

The `--format=hook` output criteria live in Phase 2, where it is implemented.

#### Manual Verification

- [ ] **Live hook equivalence**: in a named scratch repository, capture (a)
      `accelerator config summary --format=hook` invoked directly and (b) the
      `additionalContext` value delivered by a real SessionStart in that same
      repository state. **Parse the `additionalContext` field out of (a)** and
      assert byte-identity with (b) — comparing the whole envelope against a field
      would compare a JSON object to one of its string values. Attach both captures
- [ ] Starting a session in a repo with no config produces no blank context block
- [ ] **Warm SessionStart latency** recorded against the current bash hook. The
      cold path — the first session after a version bump serialises a 10-25MB
      launcher fetch (bounded at `--max-time 300`) in front of session start — is
      recorded but attributed to `bin/accelerator` (0164/0165), since `--fail-safe`
      cannot cover it: the bootstrap `fail()`s before the launcher process starts

---

## Phase 7: Deletion and green build

### Overview

Delete the removal set, retire the superseded suites, move the counters.

### Changes Required

#### 1. The removal set

**File**: `meta/inventories/0167-removal-set.md` (new)
**Changes**: an explicit file list, not a category description:

```
scripts/config-read-value.sh          scripts/config-read-template.sh
scripts/config-read-path.sh           scripts/config-list-template.sh
scripts/config-read-all-paths.sh      scripts/config-show-template.sh
scripts/config-read-doc-type-paths.sh scripts/config-eject-template.sh
scripts/config-read-work.sh           scripts/config-diff-template.sh
scripts/config-read-agents.sh         scripts/config-reset-template.sh
scripts/config-read-agent-name.sh     scripts/config-dump.sh
scripts/config-read-context.sh        scripts/config-summary.sh
scripts/config-read-review.sh         skills/config/init/scripts/init.sh
scripts/config-read-skill-context.sh
scripts/config-read-skill-instructions.sh
```

**Not** on it: `scripts/config-common.sh` (0174 owns its retirement),
`scripts/config-read-browser-executor.sh` (0173 owns its migration), and
**`scripts/config-defaults.sh`**.

`config-defaults.sh` was on the removal set in earlier drafts and cannot be:
`config-common.sh:9` sources it **unconditionally**, and `config-common.sh` is
deliberately retained for 0174 because 18 non-cluster production sourcers survive.
Deleting the defaults file would break every one of them at source time — all
seven migrations, the work-item scripts, the Jira and Linear scripts and the
visualiser config writer, most under `set -euo pipefail`. `write-visualiser-config.sh:179-185`
additionally reads its `VISUALISER_KEYS`/`VISUALISER_DEFAULTS` arrays to build the
`visualiser.kanban_columns` default, for which the CLI offers no replacement —
`get` deliberately has no catalogue lookup (see the `get`/`path` table).

It retires with `config-common.sh` in 0174, so the shell catalogue remains a
second source of truth until then — already true for `config-common.sh`, so this
changes the duration of that overlap, not its existence.

**The catalogue drift test is not saved by this retention**, contrary to an
earlier draft of this note. Its bash payload lives in `catalogue.rs`'s `EXTRACT`
const (`:295-333`), not at `:343-412`, and it depends on two files that *are*
deleted: `:301` sources `config-dump.sh` — the sole home of `REVIEW_KEYS`,
`REVIEW_DEFAULTS`, `AGENT_KEYS` and `AGENT_DEFAULTS`, verified absent from
`config-defaults.sh` — and `:325-326` invokes `config-read-review.sh` for the
`review.*` cross-check. `EXTRACT` runs under `set -euo pipefail`, so the missing
source aborts extraction and the test fails its `output.status.success()`
assertion.

Disposition: **move the four arrays into the retained `config-defaults.sh`**,
where the rest of the shell catalogue already lives and where they arguably
belonged, and **drop the runtime `config-read-review.sh` cross-check** — those
three `review.*` keys are covered by the declarative comparison once the arrays
move. The drift test then survives to 0174 alongside the file it checks.

#### 2. Delete and retire

**Changes**: delete the removal set, the shims, `hooks/config-detect.sh`, and the
three superseded suites (`scripts/test-config.sh`,
`scripts/test-config-read-doc-type-paths.sh`,
`skills/config/init/scripts/test-init.sh`).

`test-init.sh`'s retirement commit **references the recorded green run** from
Phase 4.

**Also delete `config_resolve_template` from the retained `config-common.sh`**
(`:377-439`, with its `CONFIG_TEMPLATE_SOURCE_*` constants). It invokes two
removal-set scripts — `config-read-value.sh` at `:407` and `config-read-path.sh`
at `:422` — and every caller is deleted in this same phase: the five template
scripts (`config-{read,list,show,diff,reset}-template.sh`) and `test-config.sh`
(`:5177-5244`). Leaving it would ship a retained library, sourced by 18 surviving
production consumers until 0174, carrying a function that fails the moment
anything calls it — invisible to `mise run check`, which has no dead-path
analysis for shell.

#### 3. Fix the pinned Rust tests

**File**: `cli/config-adapters/tests/parity.rs`
**Changes**: `:42-43` asserts `config-read-value.sh` **is a file** and `:113-121`
shells out to it. Remove `scripts_dir()`, `oracle()`, `require_bash()` and the
differential tests that shell out; **retain** the declared-value tests at `:256`,
`:278`, `:296` — divergences 10-12 name them, and `0167-divergences.md`'s checker
asserts they still resolve to real passing tests. Two of the three (`:278`, `:296`)
are **not purely** non-differential — each carries a differential tail calling
`oracle()`/`require_bash()`; retain each with that tail **stripped**, keeping only
the declared-value Rust assertion, so removing the helpers does not leave a
dangling reference. The earlier "the `bash-parity` feature gates this" escape does
**not** apply: `bash-parity` is declared in `corpus-adapters` and `vcs-adapters`
but **not** in `config-adapters/Cargo.toml`, and CI runs `--all-features`, so
gating would not stop the deleted-script assertion firing.

**File**: `cli/corpus-adapters/tests/doc_type_single_source.rs`
**Changes**: none required — `:189-220` sources `config-defaults.sh`, which is
retained for 0174. Repointing at `config::catalogue::DOC_TYPES` moves to 0174
alongside the file's retirement.

**File**: `cli/config/src/catalogue.rs`
**Changes**: `EXTRACT` (`:295-333`) sources `config-dump.sh` at `:301` and invokes
`config-read-review.sh` at `:325-326`. Move `REVIEW_KEYS`, `REVIEW_DEFAULTS`,
`AGENT_KEYS` and `AGENT_DEFAULTS` into `config-defaults.sh` and drop the runtime
cross-check. **Dropping the runtime cross-check is not free** — `EXTRACT`'s `M`
lines map-overwrite the `K` lines (`:321-324`) so the assertion is *the value the
renderer emits* against the declared default, catching a `config-read-review.sh`
that hardcoded a number out of step. So a **replacement** Rust test is added,
scoped **per key and mode** rather than blanket-equal to `catalogue::default_for`:
`max lenses` equals `default_for` across all three modes; the work-item
severity/count equals `default_for` in work-item mode; and `min lenses` equals its
**mode-specific expected value** — **3 in work-item mode, 4 otherwise**. `min_lenses`
must *not* be asserted equal to `default_for` (`catalogue.rs:106` declares 4, but
the work-item golden renders 3 — a mode-specific default the renderer computes,
which the original cross-check extracted from PR mode only for exactly this
reason). A blanket `== default_for` would fail against correct work-item output.

**File**: `cli/corpus-adapters/tests/common/mod.rs`
**Changes**: `doc_type_table()` at `:66` calls
`require_script("scripts/config-read-doc-type-paths.sh")` and passes
`repo_root()` **positionally**; it is imported by
`corpus-adapters/tests/parity.rs:16`. Repoint at
`accelerator config paths --doc-types --format tsv <root>`, resolving the binary
**explicitly**: `CARGO_BIN_EXE_accelerator` is injected only for `launcher`
integration tests, so `corpus-adapters` cannot use it, and a bare
`cargo test -p corpus-adapters` does not build the launcher bin. The test resolves
`cli/target/<profile>/accelerator` (profile from `PROFILE`/`CARGO`), and **fails
loudly with an explicit "launcher not built" message** when it is absent — never
falling through to `bin/accelerator`'s network fetch of a released binary. This is
a third pinned Rust dependant, distinct from the two above, and its positional
root passing is a second production consumer of the `paths --doc-types [root]`
contract settled in Phase 2 — the first being `doc-type-table.sh:41`.

**File**: `scripts/test-validate-corpus-frontmatter.sh:412`,
`skills/config/migrate/scripts/test-migrate-0007.sh:2208`
**Changes**: both write `exec` stub scripts hard-coding the resolver's path.

#### 4. Move the counters

**File**: `tasks/test/integration.py`
**Changes**: `_EXPECTED_CONFIG_SUITES` moves from 21 to whatever discovery finds
after the retirements. It is a `<` floor (`:85`), not an equality, so it must be
**lowered** or the build fails. Necessary bookkeeping — but tautological once
edited, which is why the superseded-suite **absence** assertion is what carries
weight.

**File**: `tasks/lint/scripts.py`, `tests/unit/tasks/test_exec_bits.py`
**Changes**: none required for `scripts/config-defaults.sh` — it is retained for
0174, so its `SHELL_LIBRARIES` entry (`scripts.py:25`) and mirror
(`test_exec_bits.py:251`) stay. Confirm no other removal-set path appears in
either, rather than assuming.

**File**: `meta/inventories/0167-suite-audit.md`
**Changes**: a **second, final-state discovery run** recorded, with every
difference from the pinned audit table attributed to a named deletion or
addition.

#### 5. Cross-item records

- **0106**: its plan's authoritative sentence
  (`meta/plans/2026-06-11-0106-invoke-plugin-scripts-by-bare-path.md:164-166`)
  gains an `accelerator`-shaped variant for config-cluster invocations, with the
  work item's blockquote (`0106:76-78`) updated to match — the two are worded
  differently today, so "updated to match" means editing both. The existing
  bare-path directive is **retained unchanged** for `artifact-*` and other
  families, which stay on bare paths until 0173. Note 0106 explicitly *forbade*
  `allowed-tools` changes (`:83-84`), so this variant is a deliberate departure.
- **0107**: the migrate-then-build disposition, plus the invocation shape its
  future matcher must cover.
- **0178**: record that `--allow-legacy-layout` supersedes the
  `ACCELERATOR_MIGRATION_MODE` bypass 0178 deliberately dropped. 0178's decision
  is reframed, not reversed — the env var stays unhonoured and its negative test
  is retained; the escape becomes explicit and per-invocation.
- **0166**: verify the `store` amendment at `0166:221-242` — the research found it
  **already written**, so this is a verification, not a write.
- **0169**: covered in Phase 6.
- **0169/0173/0174 reciprocal `blocked_by`**: already done 2026-07-19.

#### 6. Re-measure

**Changes**: the `config-common.sh` sourcer count post-migration, recorded
alongside the pre-migration figure of 40 call sites (39 `.sh` sourcers plus one
inline SKILL.md body). 18 are non-cluster production sourcers — the migrations,
work-item scripts, Jira/Linear scripts, the visualiser config writer — and they
source it for `config_extract_frontmatter`/`config_extract_body`/the writeback
primitives, not for config reading. **The surviving count is what justifies
keeping the library until 0174; if it reaches zero, say so** rather than
deferring on a stale number.

**Re-measured (2026-07-22).** Post-deletion, **18 non-cluster production scripts**
still `source config-common.sh` — the seven migrations, `run-migrations.sh`,
`migrate-discoverability.sh`, four work-item scripts, `jira-auth`/`jira-resolve-fields`,
`linear-auth`/`linear-create-flow`, and `write-visualiser-config`. It has **not**
reached zero, so config-common's retention to 0174 stays justified. (The removal
set's own sourcers are gone with the scripts; the count matches the 18
non-cluster figure the plan predicted.)

### Success Criteria

> **Progress (2026-07-22).** Phases 6 and 7 pushed through to the deletion
> end-state, `mise run check`-green. The removal set (20 scripts), the superseded
> suites (`test-config.sh`, `test-config-read-doc-type-paths.sh`, `test-init.sh`)
> and the Phase-4 shims are deleted; `config_resolve_template` is dropped from the
> retained `config-common.sh`; the four cli/ dependants are repointed and the
> review/agent arrays moved into `config-defaults.sh`; counters moved
> (`_EXPECTED_CONFIG_SUITES` 21→19, the init suite wiring removed); the call-site
> gate's `PENDING_PHASE7` allowlist is empty; the removal-set/ledger/suite-audit
> inventories are recorded; and the 0106/0107/0178 cross-item records are written
> (0166 verified already-written).
>
> **Follow-up close-out (2026-07-22).** Three of the recorded-remaining items are
> now built and green: the `install_crypto_provider` negative test
> (`cli/launcher/tests/crypto_provider.rs`, a spy-`ResolveBinary` dispatch
> harness); the standalone deletion-ledger *replay* with its own known-positive
> floor (`scripts/replay-deletion-ledger.sh`, wired into `mise run check` as
> `lint:deletion-ledger-replay:check`, output committed at
> `meta/inventories/0167-deletion-ledger-replay.md`, negative-tested); and the
> SKILL.md census surviving gate (`check-skill-permissions.sh` items 5-7). Also
> added: the summary-hook stdout/stderr-split test and the 0006 failing-stub
> proof (`test-migrate.sh`). **Still remaining: the launcher-size regression
> check** (overlaps 0165 release-pipeline work), the full per-*k* migration stub
> matrix across every migration (the 0006 command-substitution site is proven),
> and a committed `--help` snapshot suite. The Phase-5 release-gated items (signed
> artefacts, latency budget) and the live-session probes remain blocked in this
> environment.

#### Automated Verification

- [x] Every file on the removal set is deleted
- [x] `mise run check` exits 0
- [x] The bare `mise run` exits 0 end-to-end (validated 2026-07-22)
- [x] At the **final state**, `run_shell_suites` discovery contains **none** of
      `test-config.sh`, `test-config-read-doc-type-paths.sh`, `test-init.sh`
- [x] **Deletion ledger replay.** `meta/inventories/0167-deletion-ledger.md` maps
      every deleted path to a **gate that exists and is green at the final state**
      (a third column beyond "covering gate" and "commit where it went green"), and
      the replay asserts that final-state gate is present and passing **after** the
      deleting commit — not merely that some gate was green at-or-before it. Where
      the only covering gate was the repointed `test-config.sh` (which this phase
      deletes), that forces the assertion to be ported to a surviving Rust test or
      inventory row before the script goes, which is the durable-coverage property
      the at-or-before check does not give. The replay carries its **own
      known-positive floor** — a deliberately mis-named gate row, or a row pointed
      at a commit after its deletion, must make the replay fail — since this is the
      one new gate that otherwise takes its own correctness on trust, and it guards
      the plan's one irreversible risk. The replay output is committed
      (`scripts/replay-deletion-ledger.sh`; presence + floor wired into
      `mise run check` via `lint:deletion-ledger-replay:check`, output at
      `meta/inventories/0167-deletion-ledger-replay.md`; negative-tested that a
      mis-named gate row fails it)
- [x] `bash scripts/check-inventory.sh` exits 0 against the final tree
- [x] `mise run lint:store-duplication:check` exits 0 (and its unit test
      `tests/unit/tasks/test_store_duplication.py` passes)
- [x] The `config` built-in path drags in **no HTTP/fetch code at runtime**,
      asserted two ways since `config_command` is a *module* in `launcher` (which
      genuinely depends on `reqwest`/`rustls` for external resolution, so no
      `cargo tree` boundary exists): the **pup module rule** denies
      `accelerator::config_command` from naming `crate::launch::outbound` (present:
      `config_command_may_not_import_adapters_or_launch`), and a **test** asserts a
      `config path` invocation does not reach `install_crypto_provider`.
      **Done: the dedicated negative test now exists
      (`cli/launcher/tests/crypto_provider.rs`) — a spy `ResolveBinary` driven
      through `dispatch` proves neither `version` nor `config path` consults the
      resolver, where `install_crypto_provider` lives; the pup rule still holds.**
- [~] The shipped launcher's size is recorded and bounded by a
      **launcher-size regression check** in `mise run check`. **Remaining: recorded
      as a follow-up (overlaps the 0165 release-pipeline size datum alongside
      `tasks/manifest.py`/`create_checksums`); not built in this push.**

#### Manual Verification

- [ ] **Performance — aggregate, not per-call.** Measured in **Phase 5** (its
      pre-release gate), not here: Phase 7 deletes `config-read-path.sh`, the bash
      side of the comparison, so by this point the baseline no longer exists. A
      committed script measures the **post-batching shipped invocation shape** of
      the two worst-case skills (`visualisation/visualise/SKILL.md`,
      `config/init/SKILL.md`) — `config paths`/`config paths --all` plus the
      residual non-batchable calls, **not** the raw 17/13-call un-batched set the
      product no longer issues — invoked through `${CLAUDE_PLUGIN_ROOT}/bin/accelerator`
      (the shipped shape) against the equivalent set on bash, in the same run on
      the same host, at a **fixed working directory** (both repo-root and a
      representative subdirectory recorded), with
      `ACCELERATOR_LAUNCHER_BIN`/`ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER` **unset**
      against a cached, signature-verified release-profile launcher — the override
      skips the verification term the measurement exists to expose, and a debug
      build overstates it. Working directory is load-bearing: bash `find_repo_root`
      forks `dirname` per ancestor level (twice per `config-read-value.sh` call)
      while the Rust walk is depth-invariant, so an unstated CWD makes the ratio
      choosable. The **absolute** aggregate and the launcher byte size are
      committed as artefacts (the ratio dies with the bash comparator at Phase 7;
      the absolutes survive, feeding the launcher-size regression check). The
      budget is a failable bound: the accelerator aggregate must not exceed the
      bash aggregate by more than **25%**. Measuring the batched shape is what makes
      it satisfiable — the un-batched set (11-23ms × 17 ≈ 187-391ms against an
      85-128ms allowance) provably could not pass, and no metadata validity stamp
      is used because it would trade warm-path tamper detection for the saving
- [ ] Bootstrap-only overhead recorded separately from the launcher's own work,
      so the two costs are attributable
- [ ] Cold start — first invocation after a version bump, including fetch and
      verification — recorded as its own figure rather than a binary
      "it bootstraps" check
- [ ] A fresh clone with no cached binary bootstraps and serves a skill invocation

---

## Testing Strategy

### Unit Tests

- Pure `render_*` functions per output class, tested in isolation from the process
  boundary (luminosity's pattern — testability without injecting a writer). Each
  returns `Rendered { stdout, warnings }`, so warning **content** is
  unit-assertable rather than only observable through the spawned binary; the
  **ordering** guarantee (warnings precede buffered stdout) is a property of the
  `emit(&Rendered, …)` handler, asserted at the handler/black-box level, since
  `Rendered`'s two collections carry no relative sequencing.
- `store::atomic_write` through the `stage`/`persist` seam against real temp
  directories: target byte-identical after an abandoned stage, no residual
  entries, exactly-once replacement on success, and mode preserved across it.
- Symlink-escape refusal: parent-component escape, leaf-symlink escape, absent
  root reached through a symlinked ancestor (must **refuse**), absent root
  otherwise (must succeed — it is a first write), a permitted root that is itself
  a symlink (must refuse), and the macOS `/tmp` → `/private/tmp` case that a
  canonical-against-raw comparison would falsely refuse.
- The catalogue drift test's payload is `catalogue.rs`'s `EXTRACT` const
  (`:295-333`), not `:343-412`. It sources `config-dump.sh` and invokes
  `config-read-review.sh`, both deleted — so Phase 7 moves its four `REVIEW_*`
  and `AGENT_*` arrays into the retained `config-defaults.sh` and drops the
  runtime cross-check, letting the test survive to 0174.

### Integration Tests

- Black-box: spawn `CARGO_BIN_EXE_accelerator` with `.current_dir(fixture)`.
  Fixtures live under `CARGO_TARGET_TMPDIR` with a `.git` boundary marker so the
  upward root walk cannot escape into the real working tree — `CARGO_TARGET_TMPDIR`
  is under `cli/target/`, inside this repo, so the marker is load-bearing.
- Uniqueness by `{pid}-{counter}` so parallel test threads never collide.
- Byte-exact assertions compare `output.stdout` directly; stderr is asserted by
  substring so wording can evolve while the named entity stays pinned.
- The repointed shell suites (Phase 4) are the parity gate for the bulk.

### End-to-end proof (ADR-0045)

**File**: `scripts/test-configure-round-trip.sh` (new, `0755`)

Extracts every `` !`…` `` command from `skills/config/configure/SKILL.md`,
executes each, concatenates the results in document order, and compares against a
committed golden capture. **Every extracted command that invokes a removal-set
script routes through `accelerator`**; any remaining bare-path invocations are
enumerated in the change and each shown to belong to a non-config family deferred
to 0173 — so this does not require doing 0173's work.

The same run exercises the write path: a `config set` through `configure`, then a
re-read returning the written value. This is the only check tying the read and
net-new write paths together through a real skill rather than unit fixtures, and
it re-runs as a regression guard.

### Manual Testing Steps

1. Start a session in a repo with team config → the SessionStart summary appears
   with the same content as before.
2. Start a session in a repo with **no** config → no blank context block.
3. Invoke `/accelerator:commit` → no new permission prompt (the `vcs/commit`
   rule addition is the highest-risk single edit).
4. Invoke `/accelerator:configure view` → all injected blocks render.
5. `/accelerator:configure` a template eject on an already-ejected template →
   the two-phase confirmation flow still works (exit 2 path).
6. Run a skill from a **subdirectory** → paths resolve against project root
   (the `config-summary.sh:20-22` divergence).

## Performance Considerations

`config-read-path.sh` alone has 66 call sites and the bash cluster is hand-tuned
to a 20-30ms band, so skill-load latency is user-visible. Two consequences:

- `config` ships as a launcher **built-in**, never an external subcommand — an
  external would cost a second fetch-verify-cache round trip on top of the
  launcher's own.
- The resolver must not pay costs the bash avoided — though the bash pays more
  than an earlier draft claimed. `config-read-path.sh:19-23` sources
  `vcs-common.sh` (for `find_repo_root`) and `config-defaults.sh`, and `:75`
  `exec`s `config-read-value.sh`, which sources `config-common.sh` and calls
  `find_repo_root` twice; `config-read-all-paths.sh:5-7` records that each
  `config-read-value.sh` subprocess re-triggers VCS detection in its own process
  (11 detections total). So a `config path` call is two bash startups, five
  sourced files and two ancestor walks — more expensive than "20-30ms" from
  memory implies, which cuts in the migration's favour but means the band must be
  **measured**, not recalled. The Rust root discovery
  (`FileConfigStore::discover_root`) is a cheap ancestor walk with no subprocess
  and is depth-invariant, where the bash `find_repo_root` forks `dirname` per
  ancestor level — so the measurement's working directory is load-bearing (see
  the latency measurement note).
- `install_crypto_provider()` runs unconditionally in `main()` before argument
  parsing (`main.rs:95`), so a `config path` pays rustls provider installation
  for capability it never uses. It moves behind the external-resolution path so
  built-ins skip it.

The latency criterion is self-relative — bash p95 captured in the same run on the
same host — so no reference machine is needed.

**The bootstrap is a per-call cost, not a first-use cost.** What the cache holds
is the download; the cache-hit test at `bin/accelerator:178` *includes* a full
minisign verification, so the launcher is hashed on every invocation. Measured on
darwin-arm64 with the vendored shim, the cost is ≈3.75ms fixed plus ≈0.75ms per
megabyte of launcher — 11-23ms per call across a plausible 10-25MB range, before
the rest of the warm bootstrap (bash startup, two `uname` forks, a `sed` over
`plugin.json`, `probe_dir`'s write-chmod-exec-rm, the shim staging).

Skipping this warm re-verification behind a validity stamp would remove the term,
but is **rejected**: a stamp cheap enough to save the dominant cost must key on
metadata (size/mtime), which cannot detect a same-size, mtime-preserved swap of
the cached launcher — it reintroduces exactly the warm-path tamper detection the
author deliberately kept. A SHA-based stamp would have to re-hash the whole file,
which is the cost being avoided. So the verification term stays, and the budget is
made satisfiable by **measuring the shape that ships, not the un-batched
worst-case**.

**The gate measures the batched shape.** After Phase 5 the two worst-case skills
do not issue 17/13 separate `config path` calls — batching collapses each run of
consecutive same-file `config path <key>` calls into one `config paths` (or
`config paths --all`) invocation. Measuring the raw un-batched set at end of
Phase 4 would gate on ~17-23× the verification term for a shape the product never
exhibits post-Phase-5, and the plan's own arithmetic (11-23ms × 17 vs an
85-128ms allowance) shows that shape cannot pass. The measurement is therefore
built from the **post-Phase-5 batched invocation shape** — assembled at the
measurement point by invoking `config paths`/`config paths --all` (both exist from
Phase 2) plus the residual non-batchable calls — so it reflects what reaches
users. **Batching for `visualise` and `config/init` is consequently committed
Phase 5 work**, a named carve-out from the mechanical rewrite, not a conditional
lever. Because batching is now load-bearing rather than optional, the label
question it raises is a **Phase 5 blocker to resolve, not a deferral**:
`config paths --all` renders a `## Configured Paths` block, whereas the skills
inject labelled `**Plans directory**: <value>` lines, so the batched form must
either carry the same labels (a keyed/`--labels` form) or the block rendering must
be accepted as a deliberate prompt-content change — decided in Phase 5, before the
gate can be measured.

Because the batched shape moves the gate off the raw invocation count, the gate
is measured at the **end of Phase 5's batching carve-out and before the release
that carries the flip to users** (Migration Notes: call sites may coexist with
bash in `main` until the release, so this is still before user impact), rather
than as a Phase-4-exit entry precondition — the earlier framing measured a shape
that no longer ships.

**Levers, if the batched measurement still comes back bad**:

1. Binary size beyond Phase 0's `strip`/`lto`, bounded by the launcher-size
   regression check (Phase 7) — the verification term is linear in launcher size.
2. Reconsider the validity stamp only with a full-verify fallback that does not
   trust metadata — accepting it does not save the dominant cost, so this is a
   last resort with no expected benefit, recorded for completeness.

No cross-invocation reuse exists by design: each process re-runs
`discover_root`'s ancestor walk and re-parses both config files
(`config-adapters/src/store.rs:33-45,84-100`), so an un-batched 17-call skill load
performs ~34 reads and parses of the same two files — subsumed entirely by
batching on the skills where it matters, which is now committed Phase 5 work.

## Migration Notes

- Bash and `accelerator` call sites may coexist across commits and PRs behind an
  `allowed-tools` set covering both. A temporarily mixed state is expected.
- The one state that must never ship is a **released** half-migrated contract,
  which is why Phase 5 is a single phase rather than split per skill category.
- Rollback: phases 0-4 are additive and revert cleanly. Phase 5 is the first
  irreversible-in-practice step (skills stop working if the binary cannot be
  fetched), which is why 0165's artefact verification is an **entry precondition
  in Phase 5's own criteria**, not a later verification step. Phase independence
  is what makes placement load-bearing: a gate recorded against a downstream
  phase can be overtaken by a merge.
- Source installs — a clone, a fork, an untagged working tree — have no release
  to fetch and are served by Phase 0's `ACCELERATOR_LAUNCHER_BIN` override. The
  marketplace install path is versioned and therefore release-backed, so this is
  a contributor concern rather than a user-facing one.
- `.accelerator/tmp` is no longer created by config writes after Phase 1; `init`
  still creates it.

## References

- Work item: `meta/work/0167-config-command-and-invocation-contract-migration.md`
- Research: `meta/research/codebase/2026-07-19-0167-config-command-and-invocation-contract-migration.md`
- Source research: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- ADRs: ADR-0020, ADR-0021, ADR-0045, ADR-0047
- 0106's authoritative directive: `meta/plans/2026-06-11-0106-invoke-plugin-scripts-by-bare-path.md:164-166`
- Luminosity reference: `../luminosity/cli/launcher/src/{config_command,context_command}/`,
  `../luminosity/cli/config-adapters/src/store.rs:108-154`,
  `../luminosity/cli/launcher/tests/config.rs`
