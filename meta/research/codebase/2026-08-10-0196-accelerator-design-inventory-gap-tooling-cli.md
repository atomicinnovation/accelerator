---
type: codebase-research
id: "2026-08-10-0196-accelerator-design-inventory-gap-tooling-cli"
title: "Research: accelerator-design — Design Inventory and Gap Tooling CLI"
date: "2026-08-10T00:43:47+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0196"
parent: "work-item:0196"
relates_to:
  - "codebase-research:2026-08-06-0195-accelerator-corpus-cli-implementation-surface"
  - "codebase-research:2026-08-08-0197-accelerator-collaboration-pr-helper-cli"
  - "codebase-research:2026-07-06-0165-multi-binary-distribution-release-pipeline"
  - "codebase-research:2026-08-02-0187-generalise-sub-binary-registration-surface"
topic: "Implementation surface for migrating design inventory/gap tooling into an accelerator-design sub-binary with a bundled Playwright driver"
tags: [research, codebase, design, playwright, cli, distribution, minisign, sub-binary]
revision: "155e0919a0cdc9d73074b919b79fdb92b9083c39"
repository: "accelerator"
last_updated: "2026-08-10T00:43:47+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: accelerator-design — Design Inventory and Gap Tooling CLI

**Date**: 2026-08-10T00:43:47+00:00
**Author**: Toby Clemson
**Git Commit**: 155e0919a0cdc9d73074b919b79fdb92b9083c39
**Branch**: workspace `ticket-management` (change `vpuoxkskyunt`)
**Repository**: accelerator

## Research Question

Educate a plan for work-item:0196 — migrate the design inventory/gap tooling
(`inventory-design`, `analyse-design-gaps`) into an `accelerator-design`
sub-binary, and bundle Microsoft's per-platform Playwright driver (Node.js +
`playwright-core`) as a distributed artifact fetched, verified, and cached via
an extension of the CLI's `manifest.json` + minisign mechanism.

## Summary

The migration splits cleanly into two halves of very unequal risk.

**The sub-binary half is well-trodden and low-risk.** Six sibling sub-binaries
have shipped through the same thirteen-point checklist; the launcher needs
**zero Rust changes** (dispatch is manifest-driven via clap's
`external_subcommand` catch-all); and 0187 already generalised the
registration surface. The design scripts themselves are small — nine
production scripts totalling well under a thousand lines, most of them pure
functions over strings and environment variables.

**The bundled-driver half conflicts with four accepted decisions and one
acceptance criterion is unachievable as written.** Two findings should change
the plan before it is written:

1. **AC2 and AC6 cannot pass.** They require the report artefact to be
   byte-identical between the shell and Rust invocations. The report is not
   produced by any script — it is **LLM-authored prose** (SKILL.md Step 9
   "Synthesise"), written over frontmatter carrying second-granularity
   timestamps, VCS revisions, ephemeral localhost ports, absolute home
   directory paths, and a `sequence` field derived from existing filesystem
   state, under a 5-minute wall-clock crawl bound. Two runs of the *current
   shell pipeline* are not byte-identical to each other. The work item's
   Assumption that the format is "fully deterministic — free of timestamps,
   absolute paths, or non-deterministic ordering" is false on all three named
   counts.

2. **Four conflicts with accepted ADRs**, the sharpest being ADR-0046's
   "fully static, dependency-free" decision and its explicit rejection of
   dynamically-linked fetched artifacts. Microsoft's driver bundle ships a
   **glibc-linked** Node; there is no musl variant. A new ADR is warranted
   before implementation — and its principal subject is ADR-0046, not
   ADR-0048 as the work item currently supposes.

Two further hard technical blockers are under-specified in the work item: the
retained `lib/*.js` requires a **`playwright` package layout that the
`playwright-core` driver bundle does not provide**, and the cache key derives
from `sha256(package-lock.json)` which disappears with the bundle (AC9
depends on it).

On the positive side, the work item's *motivation* is stronger than it
claims: the status quo (requiring system Node ≥20 plus `npm`/`npx`) is itself
an ADR-0046 violation — "the end user installs nothing beyond the plugin
itself; no toolchain, **runtime**, package manager, or `PATH` configuration"
— and ADR-0054 names "a browser-automation subdomain embedding Playwright"
by name as the motivating example for the separate-sub-binary model. 0196's
placement is not merely permitted; it is the case the ADR was argued from.

## Detailed Findings

### 1. The design scripts and a candidate subcommand mapping

AC1 makes recording the subcommand mapping a precondition of implementation.
Here is the complete production surface (test scripts excluded).

**`skills/design/inventory-design/scripts/`**

| Script | Lines | Surface | Exits |
|---|---|---|---|
| `validate-source.sh` | 306 | `[--allow-internal] [--allow-insecure-scheme] <location>` | 0 ok, 1 rejected, 2 usage |
| `resolve-auth.sh` | 68 | no args; reads `ACCELERATOR_BROWSER_*`; prints `header\|form\|none` | 0, 1 on partial form vars |
| `scrub-secrets.sh` | 47 | `<file-path>`; names offending env var, never its value | 0 clean, 1 leak/usage |
| `inventory-metadata.sh` | 37 | no args; prints key-value frontmatter lines | 0 |
| `notify-downgrade.sh` | 46 | `--reason <enum> [--from <mode>] [--to <mode>]` | 0, 1 unknown reason, 2 usage |
| `ensure-playwright.sh` | — | first-run bootstrap; the script 0196 replaces | 10–15 (see below) |
| `playwright/run.sh` | — | the retained executor | 0, 3 `playwright-not-installed` |

`--from`/`--to` on `notify-downgrade.sh` are accepted for forward-compatibility
and explicitly **not used in message selection**
(`notify-downgrade.sh:20-21`) — a quirk to preserve or consciously drop.

`validate-source.sh` is dual-purpose: `test-validate-source.sh:17` **sources**
it to unit-test `canonicalise_host`, `is_localhost_default`, and
`classify_internal` without invoking `main`. Those three functions are the
real domain logic and port naturally to pure Rust functions.

**`skills/design/analyse-design-gaps/scripts/`**

| Script | Lines | Surface | Exits |
|---|---|---|---|
| `audit-cue-phrases.sh` | 105 | `<file-path>`; per-H2 cue-phrase compliance | 0 pass, 1 fail |
| `gap-metadata.sh` | 37 | no args; prints key-value frontmatter lines | 0 |

`audit-cue-phrases.sh` reads its pattern list from
`scripts/extract-work-items-cue-phrases.txt` — a **shared** file owned by the
work-items skill, not the design skills. Case sensitivity is deliberately
mixed: the first three patterns are case-insensitive, the fourth
(`implement [A-Z]`) case-sensitive so "implement Foo" matches and "implement
foo" does not (`audit-cue-phrases.sh:12-14`).

**Candidate mapping** (two noun groups mirroring the two skills):

```
accelerator design inventory validate-source <location> [--allow-internal] [--allow-insecure-scheme]
accelerator design inventory resolve-auth
accelerator design inventory scrub-secrets <file>
accelerator design inventory metadata derive
accelerator design inventory notify-downgrade --reason <enum>
accelerator design gaps audit-cue-phrases <file>
accelerator design gaps metadata derive
```

Two mapping decisions the plan must make explicitly:

- **The two metadata scripts are near-duplicates of each other and of
  `accelerator corpus metadata derive`.** They differ only in the filename
  timestamp format (`%Y-%m-%d-%H%M%S` for inventories vs `%Y-%m-%d` for
  gaps). 0195 deliberately left them behind, noting `metadata` had only one
  verb "in this item's scope". Folding them into `corpus metadata derive`
  behind a doc-type flag would remove a genuine duplication but reaches into
  a crate 0195 just finished. Recommend deciding this in planning rather than
  discovering it mid-implementation.
- **`ensure-playwright.sh` has no natural subcommand home** — under the
  bundled-driver design its job (install Node + Playwright + Chromium) is
  largely absorbed by the fetch-verify-cache mechanism, leaving only the
  Chromium install. Candidate: `accelerator design ensure-runtime`.

### 2. AC2/AC6: the byte-identical determinism assumption is false

This is the highest-value finding and it invalidates two acceptance criteria.

**The report is not script-generated.** `inventory-design/SKILL.md:202-210`
Step 9 instructs the model to "**Synthesise** — Compile agent findings into
the five inventory categories"; Step 11 (`:219-268`) has the model hand-
substitute frontmatter fields into a template. The only scripts in the
pipeline are validators and metadata helpers. `analyse-design-gaps/SKILL.md:86-115`
is the same shape ("Write Gap Prose"). There is nothing for a Rust
implementation to be byte-identical *to*.

**Even the mechanical parts are non-deterministic.** Evidence from committed
artefacts:

| Source of drift | Evidence |
|---|---|
| Second-granularity timestamps | `inventory-metadata.sh:10-11` — feeds `date`, `last_updated`, **and the directory name / `id`** |
| VCS revision | `inventory-metadata.sh:23,27`; committed artefacts show both 40-char and 12-char forms |
| Ephemeral ports in frontmatter *and* prose | `source_location: "http://127.0.0.1:52339/"`; `:51771` and `:54844` in other runs |
| Absolute home paths | gap frontmatter `current_inventory: "/Users/tobyclemson/…/workspaces/build-system/…"`, mandated as absolute by SKILL.md `:164-167` |
| Repository name | `inventory-metadata.sh:13-22` resolves jj secondary-workspace indirection — differs by workspace |
| Filesystem-state dependence | `sequence` scans existing inventories (Step 7); `status` is mutated to `superseded` by later runs |
| Wall-clock bound | 50-route cap, **5-minute** wall clock, 50 MB screenshot budget (SKILL.md `:179-186`) — machine-speed dependent, so a Rust launcher with different startup cost changes which routes are reached |
| Parallel agent merge order | SKILL.md `:161-177` spawns locator + analyser agents in parallel; merge order unpinned |
| Screenshots | headless Chromium PNGs — font rasterisation and timing dependent |

Review-2 flagged exactly this as a minor testability finding
(`0196-…-review-2.md:151-159`) and recommended "confirm or relax the
byte-identical determinism assumption". It was closed by *asserting* the
assumption rather than checking it.

**Recommended replacement**, using a pattern the codebase already has:
`evals/fixtures/notify-downgrade/*.expected.txt` are single-line golden files
regenerated by `regenerate-notify-downgrade-fixtures.sh` and checked by
`test-notify-downgrade.sh`. Genuinely byte-comparable surfaces are the
`notify-downgrade` message set, `validate-source.sh` exit codes and stderr
across the URL/host matrix, `resolve-auth.sh` mode and precedence warnings,
`audit-cue-phrases.sh` failure reports, `scrub-secrets.sh` behaviour, the
daemon's JSON protocol envelopes, and the `ACCELERATOR_DOWNGRADE_REASON`
enum/exit-code table. For an end-to-end criterion, a **normalised**
comparison (redacting date/revision/id/port/absolute paths, sorting
frontmatter keys, excluding `screenshots/`) is the strongest honest oracle —
and still cannot cover the prose body. Both metadata scripts need a clock and
VCS injection seam before even their own output can be pinned.

### 3. Conflicts with accepted ADRs

**Conflict 1 — static/musl (strongest).** ADR-0046:83-86 decides "zero-setup,
**fully static, dependency-free** binaries … Linux targets are built against
**musl** for full static linking"; ADR-0046:117-120 rejects option 4 because
"dynamic linking reintroduces a dependency on the host's libc and system
libraries — fragile across Linux distributions … defeating the 'runs anywhere
with no setup' guarantee". Microsoft's driver bundle ships glibc-linked Node.
On a musl host the launcher runs and `accelerator design` does not. The ADR's
subject is "the CLI" and Node is not the CLI — but the *mechanism* is
identical and the *reason* transfers unchanged.

**Conflict 2 — "verifiably built by us".** ADR-0046:53-54 lists as a decision
driver: "a fetched binary must be verifiable, **and verifiably built by us**,
before it is executed." Re-signing changes the key's meaning from "we built
these bytes" to "we took custody of these bytes", and the launcher cannot
tell the difference — `keys.verifies()` returns a bool. Two aggravations:
the signing secret is reachable by the **unapproved prerelease job** (any
merged commit is signed and published with no human gate), so the upstream
URL/pin becomes signing-authority-equivalent; and SLSA attestation
(`actions/attest-build-provenance`) asserts "this workflow built this
artifact", which would become a provenance falsehood for a Microsoft-built
binary.

**Conflict 3 — per-exec re-verification is unaffordable.** 0164 froze
"re-verify the signature before every exec, **including cache hits**", tested
by "a cached signed binary mutated on disk is refused on re-invocation with
no exec". 0196's AC7 restates it. 0186 measured hashing at ≈900 MB/s, so a
full re-hash of a ~117MB bundle is **~130ms of pure hashing on every
dispatch** — against a post-0186 warm-path budget of ~41ms that 0169's
≈38.6ms threshold already over-subscribes. Note also that **rename-by-inode**,
the load-time integrity guarantee for single files ("the kernel holds the
verified inode open across `execve`"), has **no directory-tree equivalent**.
0196 must either weaken a frozen invariant or design a new integrity model
(verified unpack + directory seal + immutable-by-permissions cache).

**Conflict 4 — the manifest is frozen and single-file-shaped.** 0165 states
"Not designing the manifest schema — it is frozen (`schema_version: 1`)". The
trap the work item's Open Question does not name: the schema gate is
**strictly-higher-rejects**, and `manifest.json` is a *single global document
every deployed launcher consumes*. Bumping to `schema_version: 2` makes every
shipped launcher reject the manifest outright — for the visualiser and every
other sub-binary too. **The extension must be strictly additive under
`schema_version: 1`**, `#[serde(default)]`-shaped. This belongs in the plan as
a hard constraint, not an implementation-time decision.

**Not a conflict:** ADR-0053 explicitly enumerates "wrappers around external
binaries" as a legitimate outbound adapter, so a subprocess Playwright
executor is architecturally unremarkable. ADR-0045 puts deterministic
procedural logic squarely on the CLI side. ADR-0048's Node passage is
accurately characterised by the work item, but the negative half of the claim
is an argument from silence — ADR-0048 never contemplates distribution, and
scopes itself to "only the split and the role each toolchain plays".

One residual tension worth recording: 0196 has the **CLI delegating down to
shell, which delegates to Node**, inverting ADR-0048's permitted shell role
("the wrapper does little more than resolve and delegate to the CLI") and
running against its stated direction that "shell's footprint is intended to
shrink toward thin wrappers".

### 4. The fetch-verify-cache mechanism and what a tree breaks

`FetchVerifyCacheResolver` (`cli/launcher/src/launch/outbound/resolve/mod.rs:52`)
implements the `ResolveBinary` port. The manifest types
(`manifest.rs:26-46`) are:

```rust
pub struct Manifest { schema_version: u64, version: String,
                      binaries: BTreeMap<String, BinaryEntry> }
pub struct BinaryEntry { description: String,
                         platforms: BTreeMap<String, PlatformEntry> }
pub struct PlatformEntry { sha256: String, signature: String }
```

There is **no notion of an artifact kind anywhere**. Single-file assumptions,
each of which needs an answer:

| Assumption | Site |
|---|---|
| One asset, no extension, no format field | `mod.rs:147` `format!("accelerator-{name}-{platform}")` |
| Whole asset buffered in RAM | `Fetcher::get -> Vec<u8>` (`fetcher.rs:109,147-150`); peak RSS ≈ 2× artefact |
| Digest/signature over one file's bytes | `verifier.rs:29-50` |
| Cache entry is a **file** named `{name}-{version}-{sha256}` | `cache.rs:29-31`; `find()` requires a sibling `.minisig` (`cache.rs:51-73`) |
| Atomic publish is one `fs::rename` | `cache.rs:118-133`; a tree needs dir-staging and hits `ENOTEMPTY` |
| Modes are `0o600` write then `0o755` on one file | `cache.rs:136-159` — a tree needs 0755 dirs, 0755 node, 0644 JS |
| Re-verify reads one file | `mod.rs:90-109` |
| `resolve()` returns a directly-`exec`'d `PathBuf` | `core.rs:227-235`, `exec.rs:15-22` — a tree needs both an entrypoint and a root |

**There is no archive/zip/tar extraction anywhere in `cli/`.** The only
archive code in the repo is producer-side and write-only
(`tasks/build.py:627`, `tarfile.open(archive, "w:gz")` for debug archives,
explicitly never fetched by the launcher). Extraction is entirely new code.

Other constraints: fetch has `MAX_ATTEMPTS = 3`, `TOTAL_TIMEOUT = 300s` per
attempt with **no resumable/ranged download** (each retry restarts from
zero); the bootstrap caps at `--max-filesize 268435456` (256 MiB); and the
cache **never evicts** — 0164 explicitly dropped luminosity's retained-
versions cap and mtime eviction because "the cache is naturally bounded" by
version scoping. At 117MB per plugin version that assumption is falsified,
and the cache lives inside `${CLAUDE_PLUGIN_ROOT}` with **no XDG fallback**
(an XDG-resident binary would break the `allowed-tools` glob).

A first-run `accelerator design` fetching ~117MB + ~150MB Chromium will hold
the bootstrap lock far longer than its ~30s timeout budget, so a concurrent
second invocation hits a **spurious lock-timeout failure** — a concrete,
reachable bug. `acquire_lock` also has a known unbounded-spin arm (0190).

**Dev overrides bypass the fetch entirely.** `ACCELERATOR_<SUB>_BIN` is
consulted before any fetch and returns the path unverified
(`outbound/mod.rs:21-47`); the var name is *derived*, not listed
(`core.rs:268-293`), so `ACCELERATOR_DESIGN_BIN` works with no Rust change.

### 5. Release pipeline extension points

The pipeline is prepare → sign → attest → finalise (`tasks/release.py`).
`manifest.json` is written by `tasks/manifest.py` and signed via the
`minisign` CLI (pinned `0.12` in `mise.toml:35`), with the secret from
`ACCELERATOR_RELEASE_SECRET_KEY` scoped to Sign steps only. Four seams break
for a non-crate artifact:

- **`DISPATCHED_SUBBINARIES`** (`tasks/shared/paths.py:29-36`, currently
  `("visualiser","vcs","work","corpus","collaboration","migrate")`) — the
  single registry; `design` would be the seventh.
- **`build.cli_cross_compile`** stages from `cli/target/{triple}/release/{bin}`.
  A fetched Microsoft artifact has no cargo build — it needs a new
  fetch-upstream-and-stage task with its own pin and integrity check *of the
  upstream download*.
- **`_read_description`** (`manifest.py:71-77`) reads `package.description`
  from a crate `Cargo.toml` and **raises if absent**. A downloaded tarball has
  none.
- **`sign_staged_binaries`** signs an explicit expected set (`signing.py:60-79`),
  deliberately "not a directory scan … so a partial cross-compile fails closed".

Also: `_assert_magic_bytes`/`_assert_static_elf` are fail-closed per-file
binary assertions and an archive is neither Mach-O nor ELF; SLSA attest globs
are asserted by `test_workflows.py:207` to cover every path from
`_release_uploads()`, so an artefact staged as
`dist/release/playwright-driver-*.tar.gz` matches **nothing** and reds that
test; and CI re-downloads and re-verifies every asset before publishing, so
4×117MB adds ~1GB of transfer per release — twice for a stable release.

`SKILL_EXEMPT_SUBBINARIES` (`paths.py:38`, currently empty) is the existing
escape hatch for a registered artifact with no SKILL.md binding — precisely
the shape of a runtime dependency consumed by another binary rather than
dispatched as a subcommand. Worth evaluating as the driver bundle's home.

### 6. Registration mechanics

Naming for the new triple: domain crate `cli/design/` (package `design`),
adapters `cli/design-adapters/`, binary crate `cli/design-cli/` with package
**and** `[[bin]]` name `accelerator-design`. The token `design` collides with
neither `BUILTIN_SUBCOMMANDS` (`version`, `config`, `help`) nor
`RESERVED_TOKENS` (`verify`, `launcher`), and matches `^[a-z][a-z0-9-]*$` so
`ACCELERATOR_DESIGN_BIN` derives cleanly.

Because the domain crate occupies `cli/design/`, **`_SUBBINARY_MANIFESTS`
(`manifest.py:55-64`) is mandatory** — the default path resolution
`CLI_DIR / name / "Cargo.toml"` would find the domain crate, which has no
`description`.

Per-sub-binary edits still required after 0187: `DISPATCHED_SUBBINARIES`;
`_SUBBINARY_MANIFESTS`; the crate `Cargo.toml`; `cli/Cargo.toml` members plus
regenerated `Cargo.lock` (clippy runs `--locked`); `.gitignore` `bin/design-*`
(**[author] — nothing catches a miss**); the skill binding; `_CLI_RELEASE_BINARIES`
(the prefixed name); `test_github.py`'s `_SUBBINARY_DESCRIPTIONS` and the
**tuple-named registry-pin test, which must be renamed**; docs. Points 1, 2,
3, 4, 7 and 8 must land in one commit.

Now generic (no edit needed): dispatch coherence, SKILL.md parsing, debug
archives, SLSA glob coverage (`dist/release/accelerator-*` already matches),
and the upload-count assertion (now a derived expression).

**The dispatch-coherence guard's sharp edge:** a skill is disqualified *as a
whole* as a binding witness if it declares bare `Bash`, any rule covering the
bare launcher, or any wildcarded token segment — even if it also carries a
correct rule. And only the **first non-blank line of a fenced block** is
recognised, so every rewritten invocation must lead its own single-purpose
block.

**Do not skip the dev-override wiring.** 0195 discovered this during
implementation, not planning: a `design_bin=True` flag in
`tasks/test/helpers.py`'s `accelerator_env()`, plus `--bin accelerator-design`
in `tasks/build.py`'s `cli_dev`, or dev/test invocations attempt a real GitHub
fetch.

### 7. CI suites and floor decrements

Verified directly: `tasks/test/integration.py` has tasks for `config`
(rooted at `scripts/`), `decisions`, `hooks`, `github`, `work`, and
`integrations`. **There is no task rooted at `skills/design`.**

Consequences:

- `scripts/test-design.sh` and `scripts/test-metadata-helpers.sh` **are**
  discovered (they live under `scripts/`) and count toward
  `_EXPECTED_CONFIG_SUITES = 15`.
- The four design-local suites — `test-ensure-playwright.sh`,
  `test-notify-downgrade.sh`, `test-validate-source.sh`, and
  `playwright/test-run.sh` — are **not run by CI at all today**. The plan must
  decide whether to wire a design lane in (the 0047 Phase 0 precedent) or
  record the gap explicitly.
- `scripts/test-metadata-helpers.sh` now contains **exactly the two design
  helpers** (confirmed at `:21-24`) — 0195 removed its third entry and left
  the floor alone. 0196 removes the last two, deletes the file, and
  decrements `_EXPECTED_CONFIG_SUITES` 15 → 14.
- `scripts/test-design.sh` asserts the old `allowed-tools` glob (`:154-155`),
  the canonical-key guard (`:40-42`), the executor deny-list (`:121-123`) and
  the owner-PID removal grep — **every one invalidated by the migration**.

Current floors: config 15, work 5, integrations 32, hooks 1, decisions 0,
github 0. The convention when a group empties is to **zero the floor, not
delete the task**.

### 8. The Playwright executor: two unaddressed technical blockers

**The owner-PID hazard is gone.** `run.sh` no longer passes `--owner-pid`; the
watcher was deleted outright and a source-level grep guard added. A Rust
parent can launch the executor freely without the daemon dying when it exits.

**Its replacement does constrain the launcher.** Liveness now keys on process
**start-time** (`run.sh:106-121`), and the daemon writes that time under
`LANG=C` (`lib/state.js`). `run.sh:27-34` warns inline: without matching the
locale "every reuse check fails and the launcher respawns the daemon between
commands — losing page state". A Rust launcher **must force `LANG=C`/`LC_ALL=C`**
on both the child and any start-time probe, or reintroduce silent page-state
loss. Reimplementing natively means replicating `/proc/<pid>/stat` field-20
arithmetic on Linux and `ps -o lstart=` on Darwin, with the ±1s tolerance.

**Blocker A — `playwright` vs `playwright-core`.** `run.sh:94-97` hard-checks
`$NS_ROOT/node_modules/playwright/package.json` and exits 3 otherwise;
`playwright-loader.js:16-17` records that this layout "is owned by
ensure-playwright.sh — keep in sync"; `:48-57` prefers
`pkg.exports['.'].import` and **throws** rather than falling back (the fix for
the 0072 CJS bug). Microsoft's driver bundle ships **`playwright-core`**, not
`playwright`, and does not provide that layout. Every `chromium` destructure
in the retained `lib/*.js` depends on it. This is the concrete technical shape
of AC10 and it is under-specified.

**Blocker B — the cache key disappears.** `NS_ROOT` is
`$CACHE_ROOT/<sha256(package-lock.json)[0:8]>` (`run.sh:84-92`), and
`ensure-playwright.sh` writes a sentinel of
`lockhash`/`node_version`/`playwright_version`/`completed_at`. With a bundled
driver there is no lockfile. **AC9's idempotency criterion depends on
replacing this key derivation.**

Also relevant: `run.sh:77` re-enters the Rust CLI
(`${ACCELERATOR_BIN:-$PLUGIN_ROOT/bin/accelerator} config path tmp`), so
`accelerator design` → bash → `accelerator` nesting occurs and the
`ACCELERATOR_BIN` override must be propagated. `run.sh` also depends on `jq`,
`flock` (with `mkdir` fallback), `nohup`/`disown`, and `sha256sum`/`shasum`.

Removing the system-Node prerequisite makes downgrade reasons **10
(`node-missing`) and 11 (`node-too-old`) unreachable**, which must be
reconciled against the committed golden fixtures.

### 9. Skill call sites and the target idiom

`inventory-design/SKILL.md` currently declares:

```
  - Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator config *)
  - Bash(${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/playwright/*)
```

with invocations at `:55` (validate-source), `:83` (resolve-auth), `:121`
(ensure-playwright), `:139` (`run.sh ping`), `:216` (inventory-metadata),
`:273` (scrub-secrets), `:299` (`run.sh daemon-stop`).
`analyse-design-gaps/SKILL.md` declares the analogous pair and invokes
`audit-cue-phrases.sh` and `gap-metadata.sh`.

The migrated idiom to follow (`skills/decisions/create-adr/SKILL.md:9-11`):

```
  - Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator config *)
  - Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator corpus adr *)
  - Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator corpus metadata derive)
```

— i.e. wildcard the noun when a skill invokes several verbs, exact-match when
it invokes exactly one; a per-skill judgement call, not a blanket rule.

**The executor path is hard-coded in four coupled places** that must move in
lockstep: `scripts/config-read-browser-executor.sh` (which fails loudly via
`test -x` rather than emitting a stale path), the `inventory-design`
`allowed-tools` glob, and the bodies of `agents/browser-locator.md` and
`agents/browser-analyser.md` (both preload `accelerator:browser-executor` and
carry a guard forbidding `which`/`find` fallback). `scripts/test-design.sh`
asserts several of these.

### 10. Evals need repointing, and carry pre-existing bit rot

Both skills have `evals/{evals.json,benchmark.json}`. Eval prompts name shell
scripts literally — `resolve-auth.sh` (`:108,94`), `validate-source.sh`
(`:121,134,167,181`), `ensure-playwright.sh` "exits 14" (`:196`), `run.sh
navigate`/`ping` (`:202,210,215-216`) — and eval 14 (`:149-151`) greps
`agents/browser-analyser.md` for the literal H2 `## run.sh evaluate Payload
Allowlist`. All break on migration.

Pre-existing rot to fix while repointing: eval prompts still reference the
**pre-migration-0004** path `meta/design-inventories/…` rather than
`meta/research/design-inventories/` (`inventory-design/evals/evals.json:25,82`;
`analyse-design-gaps/evals/evals.json:7,70,83,96`), and step numbering has
drifted from the SKILL. `benchmark.json` is a recorded run log, so its stored
pass rates are invalidated and need a re-run.

### 11. The sibling migration recipe

From 0195 (corpus, done) and 0197 (collaboration, planned), the common
ordered sequence: characterize the bash in research → inventory what already
exists in Rust → walk the thirteen-point checklist in research → scaffold
crates + register + bind a skill in one phase → port domain logic behind
narrow ports → CLI command layer returning `Outcome { stdout, stderr }` →
three test tiers → rewrite skill call sites → delete bash + suites →
decrement floors → docs phase → closing sweep.

0195 sliced **vertically by noun group** (5 phases); 0197 sliced
**horizontally by layer** (8 phases). 0196 resembles 0195 — two independent
noun groups with independent caller sets — so vertical slicing with
registration folded into Phase 1 is the better fit.

"Independently mergeable" means each phase is complete and green **on top of
what precedes it**, not that phases can land in any order. Intermediate
phases assert `mise run check` / `cli:check`; only the final phase asserts the
bare `mise run`.

Characterization gotchas worth inheriting: goldens are **pre-captured, not
live-compared**, and must run **unconditionally** — not behind the
`bash-parity` feature gate, which 0195 warns is easy to copy-paste by habit
and silently disable. Exit codes are **not** exempt from parity even where
clap's usage text is. Don't port bash-only defensive code. A bash "branch" is
often several English failure modes sharing one exit code, so a target-Rust
enumeration is legitimate re-architecture, not a parity gap — say so
explicitly.

## Code References

- `cli/launcher/src/launch/outbound/resolve/mod.rs:52,90-109,137-177,147` — resolver, re-verify, fetch flow, asset name
- `cli/launcher/src/launch/outbound/resolve/manifest.rs:13,26-46` — schema version and manifest types
- `cli/launcher/src/launch/outbound/resolve/cache.rs:29-31,51-73,118-159` — cache naming, hit detection, atomic store
- `cli/launcher/src/launch/outbound/resolve/fetcher.rs:10-18,109,147-150` — timeouts, buffering
- `cli/launcher/src/launch/outbound/mod.rs:21-47` — `ACCELERATOR_<SUB>_BIN` override
- `cli/launcher/src/launch/core.rs:268-293` — override var derivation
- `cli/launcher/src/launch/inbound/cli.rs:26-28` — `external_subcommand` catch-all
- `tasks/shared/paths.py:29-36,38` — `DISPATCHED_SUBBINARIES`, `SKILL_EXEMPT_SUBBINARIES`
- `tasks/manifest.py:23,26-45,55-64,71-77` — schema, types, `_SUBBINARY_MANIFESTS`, description read
- `tasks/signing.py:24-43,60-79` — minisign invocation, explicit expected set
- `tasks/build.py:36-44,627` — `_CLI_RELEASE_BINARIES`, the only archive code
- `tasks/test/integration.py:41-74,343-396` — floors and suite roots
- `tasks/README.md:322-474` — the thirteen-point registration checklist
- `skills/design/inventory-design/SKILL.md:13-14,55,83,121,139,202-210,216,273,299` — allowed-tools and call sites
- `skills/design/inventory-design/PROTOCOL.md:548-566` — downgrade enum/exit-code table
- `skills/design/inventory-design/scripts/playwright/run.sh:27-34,77,84-97,106-121` — locale contract, CLI re-entry, cache key, layout check
- `skills/design/inventory-design/scripts/playwright/lib/playwright-loader.js:10-17,48-57` — trust boundary, layout contract, fail-loud resolution
- `skills/decisions/create-adr/SKILL.md:9-11` — the target `allowed-tools` idiom
- `scripts/test-metadata-helpers.sh:21-24` — the two remaining design helpers

## Architecture Insights

- **Dispatch is data, not code.** The launcher has no token allowlist; adding
  a sub-binary is entirely a Python build-system and repo-metadata change.
  This is why 0196's *sub-binary* half is low-risk and its *distribution* half
  is not — the latter touches the one place where the shape of an artifact is
  actually encoded.
- **The trust model's atoms are files.** sha256-over-bytes, detached
  signature, digest-in-filename, single `rename`, and rename-by-inode at exec
  all compose into a coherent story *because the unit is one file*. A tree
  breaks each of them independently, which is why "reuse the same trust story"
  is more aspiration than plan until the integrity model for trees is designed.
- **Version-scoping substituted for cache management.** 0164 dropped eviction
  because plugin-version-scoped caches are self-bounding at a few MB. That
  reasoning is sound at 2MB and unsound at 117MB; the dropped policy is
  exactly the policy now needed.
- **Fail-closed is the house style** — explicit expected sets over directory
  scans, raising rather than skipping when a checker is unavailable, loud
  failure over stale paths. Any new pipeline step should match it.
- **Determinism is achieved by injection, not by luck.** The codebase's
  byte-comparable surfaces are the ones with clock/VCS seams and captured
  goldens; the design reports have neither, which is precisely why they are
  the wrong target for a byte-identical criterion.

## Historical Context

- `meta/work/0173-…` (abandoned) — split into 0195/0196/0197 on a scope-lens
  review finding; its review-1 drew the boundary this item inherits.
- `meta/plans/2026-08-06-0195-…` — the nearest structural model (vertical
  noun-group slicing); explicitly hands the two design metadata helpers to a
  later item and records the dev-override wiring gap it discovered late.
- `meta/plans/2026-08-08-0197-…` — the concurrent sibling; shares
  `tasks/test/integration.py` and the registration surface, so merge
  contention is expected. Its review-1 records the `LANG=C` locale bug class
  "this codebase has hit before with the Playwright launcher".
- `meta/plans/2026-08-02-0187-…` — generalised the registration surface; what
  is now automatic versus still manual is set out in §6.
- `meta/plans/2026-07-06-0165-…` — froze the manifest schema and built the
  signing pipeline; its non-goals bound what a re-signing step may assume.
- `meta/plans/2026-08-02-0186-…` — the warm-path latency budget that a
  per-exec tree re-verify would blow.
- `meta/plans/2026-05-19-inventory-design-and-browser-agent-fixes.md` —
  removed the owner-PID watcher, added the `links` command and the
  `browser-executor` skill.
- `meta/research/codebase/2026-05-18-0072-playwright-daemon-cjs-import-bug.md`
  — the CJS/ESM bug whose fix created the `playwright`-layout contract that
  Blocker A collides with; also documents that browser integration tests skip
  by default in CI, which is how that bug shipped.
- ADRs 0045, 0046, 0048, 0053, 0054 — analysed in §3.

## Related Research

- `meta/research/codebase/2026-08-06-0195-accelerator-corpus-cli-implementation-surface.md`
- `meta/research/codebase/2026-08-08-0197-accelerator-collaboration-pr-helper-cli.md`
- `meta/research/codebase/2026-07-06-0165-multi-binary-distribution-release-pipeline.md`
- `meta/research/codebase/2026-08-02-0187-generalise-sub-binary-registration-surface.md`
- `meta/notes/2026-05-19-playwright-daemon-owner-pid-ephemeral-shell.md`
- `meta/notes/2026-05-19-browser-agents-self-discover-playwright-executor.md`

## Open Questions

Carried from the work item, now sharpened:

1. **Manifest shape** — must be **additive under `schema_version: 1`** (a bump
   is a flag-day for every deployed launcher). Does the driver bundle belong
   in `binaries` with a new optional `BinaryEntry` field, or in a parallel
   top-level section? `SKILL_EXEMPT_SUBBINARIES` suggests the registry already
   anticipates non-dispatched artifacts.
2. **Versioning/compatibility** — subsumed by Blocker A: how does a
   `playwright-core` driver bundle satisfy the `node_modules/playwright/`
   layout the retained `lib/*.js` requires? A shim layout, a loader change, or
   retargeting `lib/*.js` at `playwright-core`?

Newly surfaced, and each needs an answer before or during planning:

3. **Do AC2 and AC6 get rewritten?** They cannot pass as stated (§2). This is
   the single most plan-shaping question.
4. **What replaces per-exec re-verification for a tree** (§3, Conflict 3), and
   is the resulting model acceptable against AC7's "no unverified binary is
   executed"?
5. **Is a new ADR written first?** Recommended, amending ADR-0046, covering
   the musl/static carve-out, the meaning of re-signing third-party bytes, and
   the tree integrity model.
6. **What replaces the `sha256(package-lock.json)` cache key** (AC9)?
7. **Does the driver bundle get SLSA-attested?** Attesting Microsoft-built
   bytes under our build provenance is a provenance falsehood; the attest
   globs currently cover `dist/release/accelerator-*` by pattern.
8. **Do the two metadata helpers become `design` verbs or fold into
   `corpus metadata derive`?**
9. **Is a `design` CI lane wired in** (four suites currently unrun), or is the
   gap recorded and accepted?
10. **Cache growth and lock-timeout budgets** — both are deliberate defaults
    (0164/0186) that a 117MB artifact invalidates; changing them should be
    explicit reviewed items, not incidental edits.
11. **Musl-host behaviour** — what does `accelerator design` do on Alpine? Fail
    with a clear message, or is the platform matrix narrowed for this token?
