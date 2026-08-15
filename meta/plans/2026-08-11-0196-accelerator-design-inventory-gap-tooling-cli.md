---
type: plan
id: "2026-08-11-0196-accelerator-design-inventory-gap-tooling-cli"
title: "accelerator-design: Design Inventory and Gap Tooling CLI Implementation Plan"
date: "2026-08-11T15:52:23+00:00"
author: Toby Clemson
producer: create-plan
status: superseded
work_item_id: "work-item:0196"
parent: "work-item:0196"
derived_from: ["codebase-research:2026-08-11-0196-design-cli-implementation-surface"]
tags: [rust, design, cli, playwright, launcher, release-pipeline, tree-artifacts, sub-binary]
revision: "ed12be49cd87192e74090cb0af1eb109df19e8cb"
repository: "accelerator"
last_updated: "2026-08-11T16:16:42+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# accelerator-design: Design Inventory and Gap Tooling CLI Implementation Plan

> **Superseded on 2026-08-11.** Split into two plans against the same work item:
> `meta/plans/2026-08-11-0196-design-cli-migration.md` (the sub-binary, the five ported
> subcommands and the `run.sh` port) and
> `meta/plans/2026-08-11-0196-design-vendored-runtime-distribution.md` (tree artifacts, the
> release pipeline and the runtime swap).
>
> The split line is where the defects were. This plan was reviewed three times; each pass
> closed the previous pass's findings and introduced new criticals in the fix material — 7
> after pass 1, 8 after pass 2 — and every one landed in the tree-artifact,
> release-pipeline and runtime-swap phases. Those phases now sit behind a spike phase that
> settles four questions answered wrongly on paper twice. The migration phases carried no
> criticals in the last two passes and proceed on their own.
>
> Kept as the record the three-pass review targets:
> `meta/reviews/plans/2026-08-11-0196-accelerator-design-inventory-gap-tooling-cli-review-1.md`.
> **Do not implement from this document.**

## Overview

Migrate the design inventory and gap tooling into an `accelerator-design`
dispatched sub-binary, and vendor the Playwright runtime it drives so the
tooling stops depending on a system Node.js. The launcher gains the ability to
resolve directory-tree artifacts alongside the single-file sub-binaries it
already fetches; the release pipeline gains a build-time assembly step that
constructs the driver bundle and the browser from verified upstream inputs and
publishes them under the project's own signing key.

## Current State Analysis

Nine shell scripts back the two design skills, plus a Playwright launcher:

| Script | Lines | Disposition |
|---|---|---|
| `inventory-design/scripts/validate-source.sh` | 306 | → `design validate-source` |
| `inventory-design/scripts/resolve-auth.sh` | 68 | → `design resolve-auth` |
| `inventory-design/scripts/scrub-secrets.sh` | 47 | → `design scrub-secrets` |
| `inventory-design/scripts/notify-downgrade.sh` | 46 | → `design notify-downgrade` |
| `analyse-design-gaps/scripts/audit-cue-phrases.sh` | 105 | → `design audit-cue-phrases` |
| `inventory-design/scripts/playwright/run.sh` | 203 | → `design executor` |
| `inventory-design/scripts/inventory-metadata.sh` | 37 | deleted → `corpus metadata derive` |
| `analyse-design-gaps/scripts/gap-metadata.sh` | 37 | deleted → `corpus metadata derive` |
| `inventory-design/scripts/ensure-playwright.sh` | 367 | deleted, no replacement |
| `inventory-design/scripts/regenerate-notify-downgrade-fixtures.sh` | 18 | deleted with its fixtures |

The eleven `lib/*.js` modules and `run.js` are retained: they are the Playwright
automation itself and must run in Node.

Three constraints shape the sequencing:

**`scripts/test-design.sh` runs in CI, and carries more than the suites it
drives.** An earlier draft of this plan recorded the opposite, on the grounds that
the file is referenced by no mise task, no invoke task and no workflow. That is
true by name and irrelevant by mechanism: `run_shell_suites` globs `**/test-*.sh`
under the subtree and filters on the exec bit (`tasks/test/helpers.py:96-102`),
`test:integration:config` calls it for `scripts/`, and `mise run test:integration`
runs in CI (`.github/workflows/main.yml:91`). `test-design.sh` is `0755`, so it and
the four bash suites it drives run on every build. The plan's own floor arithmetic
— sixteen suites under `scripts/` — already implied it.

The consequence is a sequencing constraint rather than a footnote: of the file's 552
lines, roughly 200 are *inline* assertions over surfaces that **survive** this
migration. So it cannot be deleted wholesale (Phase 8 §1), and its per-script
blocks cannot simply be dropped by the phases that delete those scripts (Phase 2
§6, Phase 6 §8, Phase 7 §8).

By contrast the eleven `node --test` suites genuinely have no runner:
`scripts/playwright/package.json` declares no `scripts` block, and no glob reaches
them because discovery is `test-*.sh`. Phase 6 §6 adds one. The floors mechanism
(`tasks/test/integration.py:77-103`) globs no `skills/design/` subtree, so the only
floor touching design is `_EXPECTED_CONFIG_SUITES = 15`
(`tasks/test/integration.py:41`), one below today's sixteen.

**Registration forces skill rewiring into the same change.** The checklist at
`tasks/README.md:322-474` requires points 1, 2, 3, 4, 7 and 8 to land together,
and point 7 is the skill binding — a SKILL.md invoking `accelerator design`
through the `!` preprocessor with a `Bash(...)` rule whose token segment is
exactly `design`.

**Four resolver properties do not carry across to trees.** `fetcher.rs:147-150`
buffers the whole body and `cache::store` takes `&[u8]`; `TOTAL_TIMEOUT` is 300s
*per attempt*, sized in its own comment for "a multi-MB release binary". No
archive crate exists in the workspace (`cli/Cargo.toml:13-131`). `cache.rs:118-133`
renames files only. And nothing seals, reaps orphan temp trees, or writes a
sentinel.

## Desired End State

`accelerator design <subcommand>` serves both design skills; no
`skills/design/**/scripts/*.sh` survives except the retained JavaScript
automation. On a machine with no system Node.js, the inventory skill's
Playwright path fetches a driver bundle and a `chromium-headless-shell` tree
from the project's own release host, verifies both before extraction, seals
them, and drives a headless crawl. The release pipeline assembles both artifacts
in CI from inputs verified against their publishers' own signatures.

Verified by: `mise run` exits 0; `accelerator design --help` lists six
subcommands; `manifest.json` carries an `artifacts` map beside `binaries`;
`skills/design/` contains no `.sh` file; a container fixture with Node absent
from `PATH` completes a Playwright-driven inventory.

### Key Discoveries

- The manifest extension is additive by construction — `manifest.rs:1-3` states
  "Unknown additive fields are ignored", the schema has no top-level
  `additionalProperties: false`, and `manifest.rs:223-231` is a dedicated test
  feeding `"future_field": 42`. The gate rejects only strictly-greater versions
  (`manifest.rs:82-89`). No `SCHEMA_VERSION` bump, and no flag day.
- `cache::find`'s prefix scan (`cache.rs:51-73`) will *see* a directory in the
  same root and rejects it only because no `.minisig` sidecar exists. Tree
  entries need a distinct subdirectory, and a tree's sidecars must never be named
  `*.minisig`. `cache.rs:56` also aborts the whole scan on one non-UTF-8 entry,
  so new on-disk names stay ASCII.
- `cache.rs:1-6` records that "the checksum in the name lets a hit resolve
  offline". The single-file warm path never loads the manifest, and
  `load_manifest` (`resolve/mod.rs:116-135`) is two HTTPS GETs plus a signature
  verification, called only on a miss. A tree hit must hold the same property:
  each executor invocation is a fresh launcher process and a crawl makes 100–200
  of them.
- `parse_and_validate` requires `manifest.version` to equal `CARGO_PKG_VERSION`
  exactly, and `release_base_url()` pins the base to the `v{version}` tag, so a
  given launcher build only ever wants one release's artifacts. That is what
  makes a locally recorded version→digest pointer authoritative without a
  manifest fetch.
- ADR-0058 states the launcher's lock shares no on-disk contract with
  `scripts/atomic-common.sh` — the `owner.<nonce>` sentinel protocol is not
  reached for here.
- `FilenameTimestampFormat::CompactTime` (`cli/corpus/src/metadata.rs:5-11`)
  already renders `{Y}-{m}-{d}-{H}{M}{S}` with label `Timestamp For Filename`,
  digit-for-digit `inventory-metadata.sh:11`'s format. It is unreachable only
  because `corpus-cli/src/main.rs:77,82` hardcodes `DateTimeUnderscored`.
- `derive_at` builds its own `SystemClock` (`corpus-adapters/src/metadata.rs:235`),
  so there is no clock seam reachable through the compiled binary. A fixed-clock
  byte-for-byte assertion lives at the adapter level with a `FakeClock`.
- `lib/identity.js` and `lib/lock.js` have no production callers, and
  `identity.test.js:70-95` cross-validates against a `launcher-helpers.sh` that
  no longer exists, passing silently via `catch { return; }`.
- `test-run.sh:44-63` sources `run.sh` and asserts `start_time_of` under `LANG=C`
  equals it under `de_DE.UTF-8`. That guard covers the exact bug ADR-0058 names
  and must survive the port.
- `run.sh` exits 0 for daemon-side errors — `client.js:41,47` always resolves and
  error envelopes land on stdout — while launcher-level failures go to stderr
  with non-zero. `SKILL.md:142-143` discriminates on exactly that asymmetry.
- `run.sh`'s own envelopes are 3-key (`error`, `message`, `category`), unlike
  everything `errors.js:10` produces.
- `@actions/glob`'s `*` does not cross `/`, so tree archives must stay flat in
  `dist/release/` for `dist/release/accelerator-*` to keep matching.
- The launcher's redirect allowlist is `github.com` plus `*.githubusercontent.com`
  only (`fetcher.rs:17-18,31-33`).
- `cargo-pup`'s `allowed_only` rules reject grouped imports, so a `design` domain
  crate needs one single-item `use` per import (`cli/pup.ron:132-138`).

## What We're NOT Doing

- Restructuring the inventory report format, or moving the model-authored report
  body into the binary. No script produces it today.
- Byte-comparing screenshots. Assertions cover count, dimensions and
  non-emptiness.
- Shipping full Chromium. `chromium-headless-shell` is 177MB across 14 files
  against 297MB across 327, and the daemon launches headless
  (`lib/daemon.js:106`).
- Bundling `ffmpeg`. `browsers.json` marks it install-by-default but it serves
  video recording.
- A musl driver bundle. Playwright publishes none, and its Chromium builds are
  glibc-linked.
- Bespoke cache eviction. Tree artifacts live under the versioned plugin root and
  are pruned when Claude Code prunes old plugin versions. Content-addressed naming
  means an unchanged artifact *is* shared across plugin versions for anyone who
  points `ACCELERATOR_CACHE_DIR` at a stable location, but no eviction logic is
  added for that case.
- Keeping `scripts/test-design.sh`. It already runs in CI (Current State Analysis),
  so this is a removal rather than a non-wiring: the four bash suites die with the
  scripts they cover, and the fifteen assertion blocks that outlive them are re-homed
  first (Phase 8 §1).
- A formal legal review gate on the release.

## Implementation Approach

Eight phases. Each is independently **mergeable** — it leaves the tree green and no
call site pointing at a missing script — but they are not independent in *order*:

```
Phase 1 ──> Phase 3
Phase 2 ──> Phase 6 ──┐
Phase 4 ──────────────┼──> Phase 7 ──> Phase 8
Phase 5 ──────────────┘
```

The two cheap workstreams land first so the expensive ones are not blocked behind
review. The executor port is split from the driver swap so a behaviour-preserving
Rust port can merge without waiting on tree artifacts or the release pipeline.

Two sequencing constraints are load-bearing and easy to lose:

- **Phases 5 and 7 belong in the same release train.** Phase 4 adds `tar` and
  `flate2` to the launcher, whose size is a per-invocation cost for *every*
  sub-binary because `bin/accelerator:352-354` verifies it on each warm start; and
  Phase 5 begins assembling and publishing roughly 1.2GB per release — twice per
  stable cut (§7) — before any consumer exists. If Phase 7 slips, that cost accrues
  release after release for a dead capability. Either land them together, or gate
  assembly on `TREE_ARTIFACTS` being non-empty and populate it in Phase 7, so nothing
  unconsumed is published.
- **Phases 4 and 5 touch surfaces two sibling work items are editing concurrently.**
  The work item requires owners to sync before merging any change to
  `cli/launcher/src/launch/outbound/resolve/` or `tasks/release.py`, naming
  work-item:0195 (corpus) and work-item:0197 (collaboration), which register
  sub-binaries through the same checklist in the same window. Those are exactly the
  files three items edit: the manifest schema, `build_manifest`,
  `_assert_staged_manifest_is_current` and the registration registry. An unsequenced
  merge produces semantic conflicts — two different additive manifest shapes — that a
  textual merge will not catch.

Decisions taken during planning, so no phase carries an open question:

- **Layout**: `lib/*.js` is retargeted at `playwright-core` directly, matching
  what Microsoft's own bindings do. `playwright-loader.js` and its three fixture
  trees are retired.
- **Exit codes**: `scrub-secrets` and `audit-cue-phrases` split usage error onto
  exit 2, matching `validate-source` and `notify-downgrade` and the
  `kernel::Error::Refusal` mapping every other sub-binary uses.
- **Repair**: a launcher built-in under an `accelerator cache` namespace, with
  `verify`, `repair` and `ensure` verbs, so the namespace has room for later verbs
  rather than burning a bare token.
- **Tree addressing**: content-addressed, per platform, with a **generation**. A
  tree lives at `trees/<name>-<platform>-<sha256>-<gen>/`, and a pointer names the
  directory this launcher's release resolves to. The digest gives cross-version
  sharing (an unchanged artifact is one tree, which is what makes
  `ACCELERATOR_CACHE_DIR` more than cosmetic); the platform stops a shared cache root
  mixing incompatible trees; and the generation makes every rename target fresh by
  construction, which deletes the collision case entirely and lets a repair
  materialise *alongside* a tree a live daemon is reading. ADR-0060 says "addressed
  by release version and digest"; this satisfies its intent — a changed artifact
  materialises a new tree rather than mutating one — while moving the version out of
  the path and into a pointer. **ADR-0060 needs an amendment recording that.**
- **Tree integrity**: split by who reads it. A small `.sealed` attestation carries
  the archive digest, platform, release version, entry count and a digest of the
  file table; a separate `.files` table carries the per-entry
  `(path, mode, size, sha256)` rows. The hit path reads only the attestation, so its
  cost does not scale with the driver tree's ~490 files; `verify` and `repair` are
  the table's only readers. The archive digest alone attests provenance and nothing
  about the bytes on disk, since an extracted tree cannot be re-archived
  byte-identically — so the table is what makes corruption *detectable*. It does not
  make repair granular: repair is whole-tree, because the archive is the unit of
  signed provenance.
- **Artifact handoff**: the launcher exports paths for trees that are *already*
  sealed, and never fetches or fails on dispatch; cold materialisation is
  requested explicitly by `accelerator-design` through `accelerator cache ensure`.
  The launcher stays token-agnostic and the whole downgrade ordering stays inside
  the design binary, which is what ADR-0057 requires.
- **Open forks, closed**: the plan carries no either/or into implementation. The
  minisign signature form is established from `tasks/signing.py:24-43` before Step 4a
  starts and the streaming path chosen accordingly (Step 4a); tree materialisation is
  Unix-only by design per ADR-0057's matrix, stated in the module docs rather than
  hedged with `#[cfg(not(unix))]` arms (Step 4b); the mkdir lock backend is dropped
  outright, since no NFS requirement exists in this repo and ADR-0058 records nothing
  external depending on it (Phase 6 §1); `resolve_optional` is extracted to a shared
  crate with its tests (Phase 7 §5); and the two genuinely empirical questions — whether
  `${CLAUDE_PLUGIN_ROOT}` expands in a subagent Bash call (Phase 6 §7) and whether
  `playwright-core` writes into a sealed browsers root (Phase 7 §2) — are one-line
  checks run *before* their phase is scheduled, each with its branch's edit set stated.
- **Sealing**: files read-only, directories owner-writable. Unlinking an entry
  needs the write bit on its parent, so a fully read-only tree could not be
  removed by `remove_dir_all`, by a user's `rm -rf`, or by the plugin pruning that
  is this design's only eviction mechanism.
- **Downgrade vocabulary**: five reasons, stated in full in Phase 7 §6 and only
  summarised here. `executor-ping-failed`, `disk-floor-not-met` and
  `cache-unwritable` survive — the last two because a ~600MB peak first run makes
  both *more* likely, not less; `node-missing`, `node-too-old` and `bootstrap-failed`
  are dropped; `unsupported-platform` and `artifact-unavailable` are added. Phase 2
  ports today's set unchanged and Phase 7 performs the replacement, because
  `ensure-playwright.sh` still emits the old reasons until then.
- **Archive format**: `tar.gz`, flat in `dist/release/`, named
  `accelerator-{key}-{platform}.tar.gz`.
- **Trust anchors**: both key sets committed (`keys/nodejs-release.asc`,
  `keys/npm-registry.pem`) and both upstream pins committed
  (`tasks/vendor/pins.py`), under one refresh procedure, so no check ever validates
  a signature with a key from the channel that served it. ADR-0059 left the key's
  origin open; this fills the gap rather than deviating from it.
- **Release privilege**: the step that extracts untrusted upstream archives holds
  no credential. Verification needs `GH_TOKEN` for `gh attestation verify` and
  never extracts; assembly extracts outside the checkout and runs with no token.
  This extends the rule that already keeps the signing secret out of `prepare`.
- **Artifact registry**: `TREE_ARTIFACTS` in `tasks/shared/paths.py` beside
  `DISPATCHED_SUBBINARIES`, driving assembly, signing, manifest emission, upload
  and re-verification from one source — because the release path's existing design
  rule is that those lists "cannot derive from two values".
- **Crate shape**: `cli/design/` (domain), `cli/design-adapters/` (filesystem,
  process, clock), `cli/design-cli/` (the `accelerator-design` binary), matching
  the corpus/vcs/work precedent.

---

## Phase 1: `corpus metadata derive --filename-timestamp-format`

### Overview

Expose the existing `CompactTime` variant through the corpus CLI. Nothing in
`corpus` or `corpus-adapters` changes — the variant, its renderer and its label
already exist and already match the shell script's output.

### Changes Required

#### 1. CLI-local value enum

**File**: `cli/corpus-cli/src/cli.rs`
**Changes**: Add a `ValueEnum` mirror of `FilenameTimestampFormat` and a `From`
impl. The domain crate cannot derive `ValueEnum` — `cli/pup.ron:40-56` restricts
its imports to std, `kernel::Error` and `crate`.

```rust
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum FilenameTimestampFormatArg {
    DateTimeUnderscored,
    CompactTime,
}
```

Add `--filename-timestamp-format` to the `metadata derive` args, defaulting to
`date-time-underscored` so existing callers are unaffected.

#### 2. Unhardcode the format

**File**: `cli/corpus-cli/src/metadata.rs`, `cli/corpus-cli/src/main.rs`
**Changes**: Thread the argument through in place of the two hardcoded
`DateTimeUnderscored` constants at `main.rs:72-86`.

#### 3. Golden harness

**File**: `cli/corpus-cli/tests/metadata_goldens.rs`
**Changes**: `:31` hardcodes `["metadata","derive"]`. Parameterise on the arg
list and add a golden for the `compact-time` variant. The binary-level golden
pins *shape* only — `derive_at` builds its own `SystemClock`
(`corpus-adapters/src/metadata.rs:235`) so there is no clock seam through the
compiled binary.

#### 4. Where AC15's byte-for-byte claim already holds

**File**: `cli/corpus-adapters/src/metadata.rs`
**Changes**: None to the renderer, and **no `FakeClock` test** — a fake would be
the wrong instrument. `Clock::filename_timestamp` *is* the seam a `FakeClock`
replaces, so a fake returns a canned string and the renderer never executes; such
a test asserts plumbing and would pass through a changed format string.

The property AC15 wants is already pinned: `format_filename_timestamp`
(`cli/corpus-adapters/src/metadata.rs:69`) is a pure function, and the test at
`:301-312` fixes an instant to `"2026-07-13-090507"` — digit-for-digit
`inventory-metadata.sh:11`'s `date '+%Y-%m-%d-%H%M%S'`, both rendered host-local so
no timezone divergence arises. AC15 points at that test.

What is genuinely new, and therefore what the new test covers, is the
argument→variant mapping: a direct assertion on the
`From<FilenameTimestampFormatArg>` impl, so selecting `compact-time` cannot
silently resolve to `DateTimeUnderscored`. That mutation is exactly what a
shape-only golden would miss.

### Success Criteria

#### Automated Verification

- [ ] Failing test first for each change above — including the
      `From<FilenameTimestampFormatArg>` mapping assertion, which is the only
      genuinely new behaviour in this phase
- [ ] `mise run cli:check` exits 0
- [ ] `cargo nextest run -p accelerator-corpus` passes, including the new
      `compact-time` golden
- [ ] `cargo nextest run -p corpus-adapters` passes, with AC15's byte-for-byte
      claim pointed at the existing `format_filename_timestamp` test
      (`:301-312`) and a new assertion on the `From<FilenameTimestampFormatArg>`
      mapping
- [ ] `mise run check` exits 0

#### Manual Verification

- [ ] `accelerator corpus metadata derive --filename-timestamp-format compact-time`
      emits four lines whose `Timestamp For Filename` matches
      `inventory-metadata.sh`'s shape
- [ ] Omitting the flag reproduces today's output exactly

---

## Phase 2: The `design` sub-binary and its five non-Playwright subcommands

### Overview

Create the three design crates, port five scripts, and register the sub-binary.
Registration points 1, 2, 3, 4, 7 and 8 must land together, and point 7 is a
skill binding — so both SKILL.md files are rewired in this phase for the
subcommands it introduces.

### Changes Required

#### 1. Domain crate

**File**: `cli/design/` (new)
**Changes**: Pure logic with no I/O, organised by domain concept rather than one
module per deleted script. The scripts' file boundaries are an artefact of shell,
and the corpus crate this follows names domain nouns (`doc_type`, `record`, `slug`,
`typed_ref`, `linkage`, `cluster`) rather than activities.

`design` as a bounded context means **inspecting a running design surface, auditing
the documents produced from it, and knowing whether the runtime to do either is
available here** — three sub-domains, each a module directory rather than a prose
grouping, so a new module has an obvious home or an obvious rejection:

*Source acquisition* — what may be inspected, and with what credentials:

- `host.rs` — a `Host` value type owning canonicalisation (lowercase,
  bracket/zone-id/port/trailing-dot stripping, userinfo rejection) and its
  rejections.
- `host_reach.rs` — a `HostReach` classification: loopback, private, link-local,
  unspecified, or public.
- `source_location.rs` — parsing into a `Url { scheme, Host }` or a
  `RepositoryPath`, including `about:blank`, which `validate-source.sh:198`
  classifies as its own scheme and then accepts by falling through every rejection.
  A reimplementation of "the decision tree" would plausibly turn that accept into a
  rejection, so it is named here and pinned by a characterization test. Path
  locations are classified here; existence is checked in adapters.
- `access_policy.rs` — the verdict from a `SourceLocation` plus an `Allowances
  { internal, insecure_scheme }` value type. The two flags only ever travel together
  and are only meaningful as a pair, so passing them as bare booleans would make
  every call site read `evaluate(&location, true, false)`.
- `credentials.rs` — the header/form/none precedence and the
  partial-configuration refusal naming the missing variables.

*Document auditing* — what the produced documents must not contain:

- `leaked_credentials.rs` — literal-substring detection over a set of named
  values, reporting the *name* and never the value. `ACCELERATOR_BROWSER_AUTH_HEADER`
  holds a full `Name: value` pair (`auth-header.js:14-17` splits on the first
  colon), so the scan splits it and matches the value component too — otherwise an
  artefact rendering just the bearer token, the likely leakage shape, matches
  nothing.
- `cue_phrase_audit.rs` — H2 sectioning and the audit verdict, over a
  `CuePhraseMatcher` **port**. The patterns are EREs —
  `scripts/extract-work-items-cue-phrases.txt` documents itself as "one ERE
  alternative per non-comment line" and contains `users? need` and
  `[Ii]mplement [A-Z]` — and the domain crate's pup rule permits only std,
  `kernel::Error` and `crate`, exactly as `cli/corpus/Cargo.toml` depends on nothing
  but `kernel`. So the compiled regex is injected, mirroring how corpus injects its
  `IdScanner` (`corpus/src/work_item_id.rs:13-17`) for the same reason. The `const`
  pattern slice stays in the domain as the *source* the adapter compiles; without the
  port the module could not compile against its own lint rule, and the alternative —
  hand-rolling an ERE subset matcher in the domain — is complexity with no test
  story.

*Runtime capability* (`src/runtime/`) — whether a browser runtime is available, and
what to tell the user when it is not: `platform.rs` (Phase 7 §4), `executor/`
(Phase 6 §1) and `downgrade.rs`. An earlier draft called `downgrade.rs` homeless and
then added two more modules that belonged to neither stated sub-domain; naming the
third is what makes the layout predict where things go.

`downgrade.rs` **ports today's six-reason vocabulary as-is**, with the existing messages and
goldens. The replacement happens in Phase 7 §6, and it must not happen earlier:
`ensure-playwright.sh` survives until then and emits `node-missing`,
`node-too-old`, `disk-floor-not-met`, `cache-unwritable` and `bootstrap-failed`
(`:131,139,155,280,293,308,339,352`), which `SKILL.md:132` passes verbatim to
`notify-downgrade --reason`. Since every phase is independently mergeable, shipping
the new vocabulary here would make every real downgrade exit 2 with "unknown
--reason" — failing the graceful-degradation path on exactly the machines that need
it.

`credentials.rs` likewise ports `resolve-auth.sh` behaviour-for-behaviour, and this
is the place to record why despite a real defect: `makeAuthHeaderHandler` is
imported at `daemon.js:11` and never called, and its second required input
`ACCELERATOR_BROWSER_LOCATION_ORIGIN` is set nowhere in the repository, so the
header-auth path is doubly dead — while `SKILL.md:89-95,196` documents its origin
allowlist as security-critical. Users are therefore told to put real bearer tokens
into the environment of a browser-driving daemon for a feature that never applies
them, and an authenticated crawl silently produces an unauthenticated inventory.
Neither wiring it up (new feature work) nor retiring it (removing a documented
capability) belongs in a behaviour-preserving migration, so this plan preserves the
status quo and the defect becomes a **follow-up work item**, named in Phase 8 §5.

**Reachability classification is not transcribed.** `classify_internal`'s regexes
have real gaps that a verbatim port would preserve: they match `::ffff:127.0.0.1`
but not `::ffff:169.254.169.254` or `::ffff:10.0.0.1`, miss IPv6 unique-local
`fc00::/7`, carrier-grade NAT `100.64.0.0/10`, `0.0.0.0/8` beyond the exact string
`0.0.0.0`, and `0:0:0:0:0:0:0:1`; and the octal rejection `^0[0-9]+\.` inspects only
the *first* octet, so mixed encodings slip through. Since `validate-source` is the
SSRF boundary for a tool that then drives a headless browser at the supplied
location, each unlisted encoding is a route to a link-local metadata endpoint. So
`host_reach.rs` parses the canonical host with `std::net::IpAddr` and classifies via
`is_loopback` / `is_private` / `is_link_local` / `is_unspecified` / `is_multicast`,
treating any host that *looks* numeric but fails strict parsing as a rejection rather
than as a hostname. `Ipv4Addr::is_global` is unstable, so the reserved set is
enumerated by hand rather than gestured at: `0.0.0.0/8`, `100.64.0.0/10`,
`192.0.0.0/24`, `198.18.0.0/15`, `240.0.0.0/4`, IPv6 `fc00::/7`, and the transition
encodings that embed an IPv4 address — IPv4-mapped `::ffff:0:0/96`, 6to4 `2002::/16`,
Teredo `2001::/32` and NAT64 `64:ff9b::/96` — all unwrapped and re-classified on their
embedded address, so `2002:a9fe:a9fe::` cannot reach the metadata endpoint through an
encoding the regexes never contemplated. The matrix is a table-driven unit test.

Two limits are recorded in the module docs as taken positions rather than left
implicit. The check is **pre-resolution only** — a public hostname resolving to
`169.254.169.254` still passes, and nothing re-checks after DNS. And it covers **only
the initial location**: `validate-source` inspects the one argument the skill is
invoked with, while the daemon's `navigate` command takes an arbitrary `url` per
request and calls `page.goto(req.url)` with no classification at all
(`lib/daemon.js:165-167`), and `links` hands the agent a crawlable set whose
`same_origin` flag drives route following. So describing `validate-source` as "the SSRF
boundary" would be wrong — it is the front door, and the navigation surface is
unconstrained.

Plumbing the `AccessPolicy` verdict through the executor so `navigate` URLs are
classified by this same code is the right fix and the domain code will exist for it,
but it changes the behaviour of every crawl and belongs in its own change rather than
inside a migration. It is raised as a follow-up (Phase 8 §6) with the module docs
naming the gap, so nobody reads the hardened front door as covering the whole
surface.

One single-item `use` per import; `cli/pup.ron` gains a
`design_domain_imports_only_permitted` rule copied from the corpus precedent at
`cli/pup.ron:57-72`.

#### 2. Adapters crate

**File**: `cli/design-adapters/` (new)
**Changes**: Directory-existence checks for path locations, file reading for
`scrub-secrets` and `audit-cue-phrases`, environment reads for `resolve-auth`, and the
compiled regex behind `CuePhraseMatcher`.

The `cli/pup.ron` rule mirrors the **vcs-adapters** shape, and its scoping matters
from the start: that rule is `denied: ["^std::process"]` scoped to
`vcs_adapters::library` precisely because "the sibling subprocess module spawns by
design" (`pup.ron:140-163`). `design-adapters` will spawn — Phase 6 adds `process.rs`
for `/proc` and `ps` reads, daemon spawn and signalling — so the module split is
declared now: `design_adapters::filesystem` and `design_adapters::environment` carry
the no-spawn rule, `design_adapters::process` spawns by design. Landing the rule
crate-wide and weakening it two phases later would read to a future maintainer as a
rule nobody meant.

#### 3. Binary crate

**File**: `cli/design-cli/` (new)
**Changes**: Package `accelerator-design`, `[[bin]] name = "accelerator-design"`,
a mandatory `package.description`, inherited workspace version/edition/
rust-version/license/publish, and `[lints] workspace = true`.

Subcommands:

```
accelerator design validate-source <location> [--allow-internal] [--allow-insecure-scheme]
accelerator design resolve-auth
accelerator design scrub-secrets <file>
accelerator design notify-downgrade --reason <enum> [--from <mode>] [--to <mode>]
accelerator design audit-cue-phrases <file>
```

Exit codes: 0 accept, 1 domain rejection, 2 usage. This splits
`scrub-secrets.sh` and `audit-cue-phrases.sh`'s conflated `1` — a deliberate
behaviour change, so the repointed tests assert the new mapping explicitly.

Those three observable codes are what the plan intends, but they must not be reached
by inverting `kernel::Error`. Every other sub-binary maps `Refusal → 2` and
everything else → 1 (`corpus-cli:132`, `vcs-cli:77`, `migrate-cli:297`,
`launcher:206`, `collaboration-cli:232`), with `Refusal` documented at
`cli/kernel/src/lib.rs:16-19` as "a subcommand-scoped, caller-actionable refusal".
Making a *usage* error the `Refusal` while a *domain rejection* — the most
caller-actionable outcome this binary has — becomes a `Failed` sharing exit 1 with
genuine internal failures would leave a caller unable to tell "the tool worked and
refused your input" from "the tool broke".

So the rejection is modelled as a **domain verdict, not an error**: the domain returns
a verdict, the command layer renders it and returns exit 1 explicitly, and
`kernel::Error` keeps its documented meaning with `Refusal` carrying usage errors to
exit 2. The three classes map:

| Outcome | Mechanism | Exit |
|---|---|---|
| Accepted | `Verdict::Accepted` | 0 |
| Domain rejection | `Verdict::Rejected(reason)` | 1 |
| Usage error | `kernel::Error::Refusal` | 2 |

**The rule, not just the examples.** A usage error is a malformed *invocation* — an
unknown flag, a missing or excess argument, an argument the tool cannot interpret at
all. Anything the tool successfully evaluated and then rejected is a verdict. That
rule is what settles the cases an example list leaves open: `scrub-secrets` on a
nonexistent file is exit 2 (the argument cannot be interpreted as a file to scan),
while `validate-source` on a path that exists but is not a directory is exit 1 (it was
evaluated and rejected, matching `validate-source.sh:223-226`), and `notices` for an
artifact that is not materialised is exit 1 rather than 2.

**The carrier is named, because the corpus one cannot express exit 1.**
`cli/corpus-cli/src/outcome.rs` is two `String` fields and `corpus-cli/src/main.rs:145-149`
maps every `Ok(outcome)` to `ExitCode::SUCCESS`; no sub-binary in the workspace has a
successful-outcome path that exits non-zero. So `design-cli` returns
`Verdict<Reason> { Accepted { stdout }, Rejected { reason, stderr } }` and its `main`
matches on it, giving one render-and-exit function rather than the four bespoke
mappings a per-subcommand verdict enum would need. This is a new shape, not reuse of
an existing pattern, and the plan says so rather than describing it as
"corpus-cli-style".

**Canonical data stays in the domain crate as `const`s**, not `include_str!`'d into
the binary. Two path-relative files are involved:
`notify-downgrade-messages.json` (script-dir-relative at `notify-downgrade.sh:5`)
and `scripts/extract-work-items-cue-phrases.txt` (a four-level `..` climb at
`audit-cue-phrases.sh:32`). Baking them into the binary would leave the downgrade
enum and its message table in different crates coupled only by runtime string
equality — a new variant would compile cleanly and fail at lookup, and the compiler
could not prove the table exhaustive. It would also orphan the cue-phrase file's own
header claim to be canonical for both `extract-work-items` and this subcommand.

The workspace already has the pattern
(`cli/corpus/src/frontmatter_validation/schema.rs:277`): the canonical data is a
`const` in the domain crate — a `match reason { … }` returning `&'static str`, and a
`const` slice of cue-phrase patterns — and `include_str!` appears only inside a
`#[cfg(test)]` drift test asserting the on-disk file still agrees. Exhaustiveness
becomes a compile error and the shared-file contract becomes an executable
assertion.

Relatedly, `downgrade.rs` does **not** port `notify-downgrade.sh`'s runtime
bidi-override and printable-ASCII filters. Those guard text the shell reads from a
file at runtime; once the reason is a clap enum and the messages are compiled-in
`const`s, the only input they can see is the binary's own data, making the branch
unreachable by construction and testable only by mutating the shipped table. The
invariant is kept as a single test asserting every message in the table is printable
ASCII and free of bidi overrides.

#### 4. Registration

**Files**: `tasks/shared/paths.py`, `tasks/manifest.py`, `tasks/build.py`,
`cli/Cargo.toml`, `cli/Cargo.lock`, `.gitignore`,
`tests/integration/tasks/test_github.py`, `cli/deny.toml`

- `DISPATCHED_SUBBINARIES` gains `"design"` (point 1), with the registry pin,
  upload count and `_setup_release` fixture in `test_github.py` updated, plus
  the `_SUBBINARY_DESCRIPTIONS` entry at `:35-46` that KeyErrors without it.
- `_SUBBINARY_MANIFESTS` gains `"design": CLI_DIR / "design-cli/Cargo.toml"`
  (point 2) — required because `cli/design/` is the domain crate.
- `[workspace].members` gains all three crates; `Cargo.lock` regenerated and
  committed (point 4).
- `.gitignore` gains `bin/design-*` (point 5).
- `_CLI_RELEASE_BINARIES` gains `accelerator-design` (point 8).
- `cli/deny.toml` extended only if `mise run deny:check` reddens (point 13).

#### 5. Skill rewiring

**Files**: `skills/design/inventory-design/SKILL.md`,
`skills/design/analyse-design-gaps/SKILL.md`
**Changes**: Every call site for the five ported scripts becomes
`${CLAUDE_PLUGIN_ROOT}/bin/accelerator design <subcommand>`. `allowed-tools`
gains `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator design *)`. The existing
`scripts/*` rules stay for the phase, since `ensure-playwright.sh` and the
metadata scripts still live there.

The token segment must be exactly `design`, and any over-broad rule in the same
frontmatter disqualifies the whole file as a witness for
`lint:dispatch-coherence:check`.

#### 6. Script removal

**Files**: the five ported scripts, their three bash suites, and the by-name pins
**Changes**: Delete `validate-source.sh`, `resolve-auth.sh`, `scrub-secrets.sh`,
`notify-downgrade.sh`, `audit-cue-phrases.sh`, plus `test-validate-source.sh` and
`test-notify-downgrade.sh`.

`scripts/test-design.sh` exercises these scripts through **inline** assertions, not
only through delegated suites, and it runs in CI — so this phase must cut
`:169-281` (`validate-source`, including its delegation), `:282-315`
(`resolve-auth`), `:316-338` (`scrub-secrets`), `:368-425` and `:428-430`
(`audit-cue-phrases`) and `:547-551` (the `notify-downgrade` delegation). It must
also **rewrite** `:359-364`, which asserts that `analyse-design-gaps/SKILL.md`
invokes `audit-cue-phrases.sh` and that the script exists and is executable — three
assertions that break on the call-site rewrite even though they sit in a block
otherwise re-homed unchanged. They become assertions about the new subcommand. Leaving
them makes `test:integration:config` red on merge, so this phase is not
independently mergeable without the edit. Every assertion in those ranges must map
to a named Rust test in the migration checklist below, or be recorded as a
deliberate drop with a reason. The file itself, and the blocks unrelated to these
scripts, survive until Phase 8 §1.

Two by-name pins break and need lockstep edits:

- `tests/unit/tasks/test_exec_bits.py:275-278` — `_DUAL_USE_SCRIPTS` pins
  `validate-source.sh` by literal path.
- `scripts/test-skill-frontmatter-conformance.sh:102-103` — the `EMITTERS` array
  names both design SKILL.md files. This is a `_REQUIRED_CONFIG_SUITES` by-name
  gate (`tasks/test/integration.py:63`), so it runs unconditionally.

`EXPECTED_INJECTION_SKILLS = 42` (`tasks/lint/skill_permissions.py:48`) is an
equality, not a floor — but it moves only if a *skill* is added or removed, and
neither happens here.

#### 7. Documentation

**Files**: `docs-site/src/content/docs/design.md` (new),
`docs-site/astro.config.mjs`, `README.md`
**Changes**: Registration point 11 — a page for the sub-binary, the sidebar
entry, the Concepts list, and an `ACCELERATOR_DESIGN_BIN` override row.

### Success Criteria

#### Automated Verification

- [ ] Characterization tests written first, derived from the deleted bash suites
      through an enumerated migration checklist: every assertion in
      `test-design.sh:169-338` and `:368-430`, plus `test-validate-source.sh` and
      `test-notify-downgrade.sh`, maps to a named Rust test or is recorded as a
      deliberate drop with a reason. The checklist lands as a **committed artefact**
      in the phase's PR, not a scratch note, since it is the evidence nothing was
      lost; and each deliberate-drop row must name the replacement property or state
      explicitly that none survives, because "recorded with a reason" is otherwise
      self-certifying. `test-validate-source.sh` is the sharpest case: its assertions
      are over shell *functions* (`canonicalise_host`, `classify_internal`) with label
      vocabulary the `HostReach` model restructures, so many rows land in that bucket
      by construction — the `RFC1918` stderr wording (`test-design.sh:205-208`) is
      called out as a label that must survive the restructuring. AC1's "at least one
      success and one failure path per subcommand" is the floor, not the plan — `validate-source` alone
      pins the RFC1918 boundary at both edges *and* both just-outside cases
      (differentiated by stderr content), IPv6 zone-id/mapped/wildcard/bracketed
      forms, decimal/hex/octal IPv4 encodings, the `user:pass@127.0.0.1@evil.com`
      userinfo class, `about:blank` acceptance, and unknown-flag exit 2
- [ ] A table-driven `host_reach` test covers every **newly**-rejected encoding, since
      the migration checklist by construction only demands tests for behaviour the
      shell already had: `::ffff:169.254.169.254`, `::ffff:10.0.0.1`, `fd00::1`,
      `100.64.0.1`, `0.1.2.3`, `0:0:0:0:0:0:0:1`, `127.0.0.01`, `2002:a9fe:a9fe::`,
      `2001:0:...` and `64:ff9b::a9fe:a9fe`, plus a numeric-looking host that fails
      strict parsing being rejected rather than treated as a hostname
- [ ] The downgrade goldens are exhaustive by construction — the test iterates the
      reason enum, so a variant without a golden fails — replacing
      `test-notify-downgrade.sh`'s message-key/fixture set-equality check
- [ ] `mise run cli:check` exits 0 (including the new pup rules)
- [ ] `cargo nextest run -p accelerator-design -p design -p design-adapters`
      passes
- [ ] `mise run lint:dispatch-coherence:check` exits 0
- [ ] `mise run test:unit:build-system` passes with the updated registry pins
- [ ] `mise run test:integration:config` passes with the updated `EMITTERS` array
- [ ] `mise run deny:check` exits 0
- [ ] `mise run docs:check` exits 0
- [ ] `mise run` exits 0 end to end

#### Manual Verification

- [ ] `accelerator design validate-source https://example.com` exits 0;
      `http://example.com` exits 1; `--allow-insecure-scheme` flips it to 0
- [ ] `accelerator design validate-source 0x7f000001` exits 1 with the numeric
      IPv4 message, and no flag bypasses it
- [ ] `accelerator design scrub-secrets /nonexistent` exits **2**, not 1
- [ ] Each ported downgrade reason reproduces its existing golden fixture byte for
      byte — except `executor-ping-failed`, whose remediation text names `run.sh` and
      is rewritten in Phase 6 §5, the phase that deletes it
- [ ] Both design skills run end to end in a live session

---

## Phase 3: Retire the two metadata scripts

### Overview

Route both skills' frontmatter provenance through `corpus metadata derive` and
delete the scripts. Depends on Phase 1.

### Changes Required

#### 1. Skill call sites

**Files**: `skills/design/inventory-design/SKILL.md`,
`skills/design/analyse-design-gaps/SKILL.md`
**Changes**: Replace the `inventory-metadata.sh` / `gap-metadata.sh` invocations
with
`${CLAUDE_PLUGIN_ROOT}/bin/accelerator corpus metadata derive --filename-timestamp-format compact-time`.
`allowed-tools` gains `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator corpus *)`.

#### 2. Deletion

**Files**: `inventory-metadata.sh`, `gap-metadata.sh`
**Changes**: Deleted — but **not before** a characterization golden records what they
emit. Phase 1 pins the timestamp component byte-for-byte against a pure renderer, and
that is the only line covered: the claim that corpus "already emits the same four lines
with the same labels in the same order" is verified nowhere automatically, since the
binary-level golden pins shape only and the label/order equivalence sits under manual
verification. So the four-line output for a fixed clock and repo fixture is recorded as
a golden first, and `corpus metadata derive --filename-timestamp-format compact-time`
is asserted to reproduce its labels and ordering, with the five divergences below as
the only permitted differences. AC15 is then read as byte-for-byte on the timestamp and
label/order-equivalent on the rest, which is what is actually true.

Neither script has a bash suite, so no floor moves.

### Divergences to accept explicitly

Five places where the bash scripts and corpus genuinely differ, all improvements
or immaterial to frontmatter provenance:

- `jj log -r @` snapshots the working copy; `jj_revision` never does.
- The bash branches gate on `command -v jj` / `git`; corpus needs neither.
- `git rev-parse HEAD` on an unborn HEAD aborts the whole bash script under
  `set -e`; corpus omits one line and exits 0.
- Outside a repository the bash script exits 1 (the trailing
  `[ -n "$REPO_NAME" ] && echo` is its last command); corpus exits 0.
- Neither reads its two timestamps atomically.

The last two are the only observable changes for a skill, and both are strictly
better: a skill invoked outside a repository no longer sees a non-zero exit for
what is not an error.

### Success Criteria

#### Automated Verification

- [ ] Failing test first: the four-line characterization golden lands before the
      scripts are deleted
- [ ] `mise run test:integration:config` passes (skill frontmatter conformance)
- [ ] `mise run lint:dispatch-coherence:check` exits 0
- [ ] No `.sh` remains in `analyse-design-gaps/scripts/`
- [ ] `mise run` exits 0

#### Manual Verification

- [ ] Both skills produce frontmatter with a correctly shaped filename timestamp,
      revision, and repository name
- [ ] Running the inventory skill outside a repository does not fail

---

## Phase 4: Launcher tree artifacts

### Overview

Teach the resolver to fetch, verify, extract and seal directory-tree artifacts,
and add the repair path that replaces the self-healing trees are exempt from.
Tested against a synthetic tarball — no design consumer exists yet.

Three internally staged steps, each of which should compile and test green on
its own.

### Step 4a: Manifest `artifacts` map and streaming fetch

#### 1. Manifest shape

**File**: `cli/launcher/src/launch/outbound/resolve/manifest.rs`
**Changes**: A new `artifacts: BTreeMap<String, ArtifactEntry>` beside
`binaries`, `#[serde(default)]`. `SUPPORTED_SCHEMA_VERSION` stays `1`. The
all-zeros sentinel digest carries over for platforms where an artifact is
deliberately absent, reusing `bare_sha256`'s existing handling.

```rust
#[derive(Debug, Deserialize)]
pub struct ArtifactEntry {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub platforms: BTreeMap<String, ArtifactPlatformEntry>,
}

#[derive(Debug, Deserialize)]
pub struct ArtifactPlatformEntry {
    pub sha256: String,
    pub signature: String,
    pub archive_size: u64,
    pub uncompressed_size: u64,
    pub entry_count: u64,
}
```

A tree needs three sizes that a single-file binary does not, and they are three
different magnitudes: `archive_size` bounds the download (~120MB compressed),
`uncompressed_size` and `entry_count` bound the extraction (~294MB across hundreds of
files), and the free-space precheck needs `archive_size + uncompressed_size` because
both exist on disk at once. One field serving all three would be wrong by 2–3× for at
least one consumer whichever quantity it held.

So `binaries` keeps `PlatformEntry` genuinely untouched, and artifacts get their own
entry type. The three sizes are **required, not `#[serde(default)]`** — a defaulted 0
would silently disable the download cap and the decompression-bomb ceiling, which is
the failure mode a default exists to avoid. Additivity is unaffected: an older
launcher never reads `artifacts` at all, and a newer one reading a manifest without
the key gets an empty map.

The asset-name convention is `accelerator-{key}-{platform}.tar.gz`, mirroring the
single-file `accelerator-{token}-{platform}` rule pinned in one commented place at
`resolve/mod.rs:144-147`. Phase 4 builds the consumer and Phase 5 the producer, in
separate changes, so the convention is pinned in one artefact both sides read:
`tests/fixtures/manifest.example.json` gains an `artifacts` block in this phase,
asserted from `manifest.rs`'s golden test here and from
`tests/unit/tasks/test_manifest_contract.py` in Phase 5. Without that, a key-name
or extension disagreement surfaces only in Phase 7's container fixture, after both
halves have merged.

#### 2. Streaming download

**File**: `cli/launcher/src/launch/outbound/resolve/fetcher.rs`
**Changes**: `try_get` currently ends in `response.bytes().map(|body| body.to_vec())`
(`:147-150`) — the body buffered, transiently twice. Add a
`get_to_writer(&self, url: &str, sink: &mut impl Write)` that copies from the
response reader, leaving `get` as a thin wrapper for the existing small-asset
callers.

**The sink must be owned inside the retry loop.** `get` retries up to
`MAX_ATTEMPTS` (3), and today each attempt is safe only because `try_get` returns a
fresh `Vec<u8>` — a failed attempt leaves nothing behind. Writing into a
caller-provided sink breaks that invariant: an attempt that fails partway has
already written bytes, and the next appends the full body after them. The sha256
would catch the result, so nothing unverified is extracted, but the retry loop could
never succeed — a transient blip on a 294MB transfer would become a permanent,
unrecoverable failure presenting as a checksum mismatch. So the streaming path
creates and truncates the temp file at the start of *each* attempt (or `set_len(0)`
plus seek to 0) and resets the incremental digest state with it.

**The deadline is a throughput floor, not a number picked once.** `TOTAL_TIMEOUT`'s
300s per attempt was sized for a multi-MB binary. It governs the *compressed
archive*, whereas the ~294MB figure is the uncompressed tree — so the value is
derived from Phase 5's measured archive sizes, expressed as "sized for X MB at ≥N
KB/s sustained", and recorded in the constant's doc comment with its reasoning as
the existing one does. Make it a per-request override via
`RequestBuilder::timeout()` rather than a second `Fetcher`: each `Fetcher` builds a
`reqwest::blocking::Client` (installing the rustls provider and a background runtime
thread), and `FetchVerifyCacheResolver::new` already constructs one on *every*
invocation including warm hits — so it is also constructed lazily, and a warm
resolution builds none at all.

**A stalled transfer must fail fast.** `fetcher.rs:12-14` records that blocking
reqwest has no idle timeout and that the total deadline is "the only bound on a
slow-but-progressing transfer". Enlarging that deadline widens the window in which a
connection stalled at byte one is indistinguishable from a slow one — three times
over, inside a tool call with no progress output and no cancel. The copy loop
therefore enforces a progress floor (abort if fewer than N bytes arrive in M seconds),
so the large deadline bounds legitimate slow transfers while stalls fail quickly. Both
numbers go in the doc comment.

The **mechanism** must be named, because a plain byte-counting check between reads
cannot fire while a read is blocked — which is the stall case. Blocking reqwest exposes
neither an idle timeout nor the socket, so the floor needs either a watchdog thread
that drops the response to interrupt the blocked read, or a custom `Read` wrapper over
a socket with `SO_RCVTIMEO` set. State which. And the test fixture must **stop
sending** rather than trickle, or it exercises the slow path and passes without ever
testing a stall.

**Signature verification needs a named streaming mechanism.** sha256 streams
trivially, but `TrustedKeys::verifies(&self, data: &[u8], signature: &str)`
(`keys.rs:62`) is a contiguous-slice API, and incremental Ed25519 verification is
only possible in minisign's *prehashed* mode. `tasks/signing.py:24-43` signs with a
plain `minisign -S` and no `-H`, so the form must be established before this step
starts rather than assumed: confirm what `minisign -S` emits, and if it is not
prehashed either add `-H` for tree artifacts — checking the vendored `cli/verify`
shim and `minisign-verify` both accept it — or state plainly that the archive is
buffered for verification and bound the peak. Left unstated, an implementer reads a
294MB temp file back into a `Vec<u8>`, giving the launcher a peak RSS an order of
magnitude above anything it does today, in exactly the memory-limited containers
AC6 and AC11 use. A test asserts the release pipeline's signatures are in the
expected form, so a signing-flag change fails loudly rather than degrading to a
full buffer.

The download is capped at `archive_size` from the artifact's platform entry;
`uncompressed_size` and `entry_count` bound the extraction in Step 4b step 4.

### Step 4b: Extraction, sealing, atomic rename, attestation, pointer

#### 1. Archive dependency

**Files**: `cli/Cargo.toml`, `cli/launcher/Cargo.toml`, `cli/deny.toml`,
`tests/integration/deny/test_launcher_feature_graph.py`
**Changes**: Add `tar` and `flate2` as workspace-pinned dependencies with
justification comments. `tar` is pinned **exactly**, not caret-bound: it is pre-1.0,
and its entry classification, PAX/GNU long-name handling and symlink semantics are
precisely what the extraction allowlist sits on top of, so a patch bump could shift
the trust boundary without a pin-edit review. `cli/Cargo.toml`'s stated discipline is
to exact-pin crates whose behaviour the workspace depends on (`clap`, `reqwest`,
`rustls`, `minisign-verify`, `serde-saphyr`, `jj-lib`) and caret-bound only those
documented as behaviour-stable. It also gets `default-features = false`, since the
default `xattr` feature adds a transitive edge that mode masking makes pointless.

`flate2` is pinned explicitly to its pure-Rust backend:

```toml
flate2 = { version = "1", default-features = false, features = ["rust_backend"] }
```

That pin is load-bearing, not stylistic. `flate2`'s alternative backends (`zlib`,
`zlib-ng`, `zlib-rs`) pull `libz-sys`/`zlib-ng-sys`, which need a C toolchain and
would break the fully-static musl cross-build ADR-0046 depends on. Because Cargo
unifies features across the workspace, a *future* crate enabling a C backend would
pull it into the launcher silently — so `libz-sys`, `zlib-ng-sys` and `zlib-sys`
join `_ABSENT` in `tests/integration/deny/test_launcher_feature_graph.py:24-31`,
which already parametrises `test_banned_or_native_crate_is_absent` over that tuple
for exactly this class of regression.

`cli/Cargo.toml:149-151` documents launcher binary size as a per-invocation latency
term, because `bin/accelerator:352-354` minisign-verifies the whole launcher on
every warm start. "Reconsider if it exceeds a few hundred KB" is not a gate — it has
no number and no outcome — and it also weights the wrong axis. Work-item:0186
measured shim exec plus verify of a 7.6MB launcher at ~6.8ms, with minisign alone at
~2.3ms for 8MB: roughly **0.3ms/MB**, so a few hundred KB is ~0.1ms, plausibly below
the measurement noise floor. So the budget is expressed in time and converted to a
ceiling — but the slope is **measured, not back-derived**. The 2.3ms figure comes from
0186's pre-change Context table, a 20-run bash loop that 0186's own Validation Results
declare "not method-comparable" to its interleaved medians; and the post-change
composition table's ~6.8ms bundles shim process startup with the read, so attributing
all of it to size gives 0.3ms/MB while attributing half gives ~0.45ms/MB and a ceiling
nearer 2MB. Deriving a marginal per-MB cost from one point that includes fixed costs is
not sound. So the slope is obtained directly, with 0186's method: verify two padded
launchers of known differing size on the same host and take the difference. The 1ms
budget then converts to a real ceiling.

Two notes on that gate. `tar` plus `flate2`/`miniz_oxide` realistically add a few
hundred KB, so a multi-MB ceiling is a weak tripwire — the assertion is on the measured
delta plus a small margin, not on the headroom. And the ceiling is an **absolute
per-target size** checked against the cross-compiled artefacts in the release lane,
recorded beside the other pins in `tasks/shared/paths.py` with its derivation in the
comment, because a ratio gate would need a stored pre-Phase-4 baseline that
`mise run test:*` has nowhere to keep and cross-compiled binaries that only exist after
`build.cli_cross_compile`.

The backend's real consequence is the other direction: `miniz_oxide` (the pure-Rust
default) inflates materially slower than a zlib-ng build, and the cold path inflates
~294MB. So record decompression throughput over a real archive alongside the size
figure, and if `rust_backend` proves unacceptably slow the resolution is a faster
pure-Rust backend (`zlib-rs`, if it can be shown to need no C toolchain), never a
`*-sys` crate.

#### 2. Tree materialisation

**File**: `cli/launcher/src/launch/outbound/resolve/tree.rs` (new)
**Changes**:

Layout — a dedicated subdirectory so `cache::find`'s prefix scan
(`cache.rs:51-73`) never sees a tree, content-addressed so an unchanged artifact is
one tree however many plugin versions want it, per-platform so a shared cache root
cannot mix incompatible trees, and generation-suffixed so a rename target is always
fresh:

```
<cache_root>/trees/<name>-<platform>-<sha256>-<gen>/        the sealed tree
<cache_root>/trees/<name>-<platform>-<sha256>-<gen>.sealed  the attestation
<cache_root>/trees/<name>-<platform>-<sha256>-<gen>.files   the per-entry table
<cache_root>/trees/<name>-<platform>-<version>.ref          the pointer
```

All names are ASCII (`cache.rs:56` aborts the scan on one non-UTF-8 entry) and none
is named `*.minisig`. The **attestation** is small and fixed-size — archive digest,
platform, release version, entry count, and a digest of the table — and is the only
sidecar the hit path opens. The **table** carries one
`(path, mode, size, sha256)` row per entry (or a link target, for a symlink) and is
read only by `verify` and `repair`; keeping it out of the attestation is what stops
the hit path's cost scaling with the driver tree's ~490 files. The **pointer** names
a directory rather than a digest, which is what lets a repair swap one generation for
another atomically.

The attestation and the pointer each carry a `format_version`, and the tree directory
name carries a layout version alongside the generation. Extraction and sealing policy
— the entry-type allowlist, mode masking, the `0444`/`0555` seal, the table's own shape
— is launcher-version-specific and is *not* covered by the archive digest, yet content
addressing means a newer launcher routinely adopts an older launcher's tree from a
shared cache root. Without a layout version a policy fix would be silently inherited
rather than applied, and `verify` would pass because it checks against the older
table. The same "unknown additive fields ignored, higher version refused" discipline
`manifest.rs` already documents applies, and the ADR-0060 amendment records that
cross-version tree *adoption* — not just digest addressing — is the real deviation.

The generation is the load-bearing addition. Because every materialisation picks a
fresh one, `rename(2)` never lands on an existing target — so there is no
already-present branch to get right, no need to distinguish a concurrent winner from
a crash leftover at rename time, and a repair can build a complete replacement
beside a tree a live daemon is still reading.

`trees/` is created `0700`. The cache root, every generation directory and every
sidecar must be owned by the effective uid and be neither group- nor
world-writable; anything failing that is treated as absent rather than trusted.
ADR-0060's threat model assumes the cache lives under the user's own home directory,
and `ACCELERATOR_CACHE_DIR` — which this plan actively recommends — can break that
assumption, so it is enforced rather than assumed and documented as requiring a
private, user-owned path.

**`locate`** (the hit path, on every dispatch):

1. Read `<name>-<platform>-<version>.ref`. Absent or unparseable → miss.
2. Reject the name unless it matches `<name>-<platform>-<64 lowercase hex>-<gen>`
   exactly and resolves to a direct child of `trees/`. The pointer is unsigned local
   state whose contents become a path, so it is validated before it is joined.
3. `stat` the directory: present, a directory, correctly owned, not
   group/world-writable. Otherwise miss — a tree removed by a partial `rm -rf` or an
   interrupted prune leaves its tiny sidecars behind, and returning a dead path would
   surface as an opaque Node error instead of a re-materialisation.
4. Read the attestation; its digest must equal the digest in the directory name.

Two small reads and two stats. No network, no manifest, and the table untouched.

**`materialise`** (the cold path — reached only from `cache ensure` and `repair`),
under the per-`(name, platform)` single-flight lock:

1. Load the manifest; the entry names digest `D`, `archive_size` and
   `uncompressed_size`.
2. **Reuse scan**: any `trees/<name>-<platform>-D-*` whose attestation is valid and
   whose directory passes the step-3 checks → publish the pointer at it and return,
   **with no download**. This is what makes an unchanged artifact across two plugin
   versions a genuine hit rather than a refetch, and it is asserted by a zero-fetch
   criterion rather than by a directory-layout one.
3. Free-space precheck against `archive_size + uncompressed_size` plus a margin, for
   every tree about to be materialised. A shortfall emits `disk-floor-not-met` before
   a single byte is fetched.
4. Stream the archive to `trees/.tmp-<gen>.archive`, truncating the file and
   resetting the incremental digest at the start of each attempt, under Step 4a's
   deadline and progress floor.
5. Verify sha256 and minisign over the archive. On failure, remove the temp archive
   and return the cause — nothing has been extracted.
6. Extract into `trees/.tmp-<gen>/` under the entry rules in step 4 below,
   **computing each entry's sha256 inline as it is written**, so the table costs no
   second pass over ~294MB.
7. Seal bottom-up: `0444` for files, `0555` for files the archive marks executable,
   directories left owner-writable. Symlinks are walked with `symlink_metadata` and
   their permissions left alone — `set_permissions` follows a link and would re-mode
   the target — and recorded in the table by link target rather than by digest.
8. Write `.tmp-<gen>.files`, then `.tmp-<gen>.sealed` carrying its digest.
9. `rename(2)` the temp directory into place, then the two sidecars. Fresh by
   construction, so no collision case arises.
10. Publish the pointer atomically, last. Until then the generation is invisible to
    `locate` and reclaimable by the reaper, so a crash at any earlier step leaves
    only garbage rather than a half-trusted tree.

**Single-flight**: one lock directory per `(name, platform)` under `trees/`, reusing
the PID-owner staleness discipline `bin/accelerator:317-345` implements — but not its
waiter budget, which resets on every live-owner observation and so waits unbounded.
Here the wait carries an explicit deadline derived from the fetch deadline plus an
extraction allowance, and the loser waits on the **lock**, never on the pointer: a
winner that fails writes no pointer, so a pointer-waiter would hang forever. On
acquiring the lock the loser re-runs `locate` and materialises only if still needed;
on deadline expiry it emits `artifact-unavailable` rather than blocking a crawl. The
lock is released by a `Drop` guard on every path.

Without this, two cold invocations each stream ~294MB, hash it, verify it, extract it
and seal it — ~588MB of transfer and ~1.2GB of transient disk, one copy of which is
then discarded. `cache::store` needs no such guard at ~8MB; at this size the
duplication is the dominant cost of a first run.

Orphan reaping: `cache.rs:130` removes a single temp file on a failed rename. Here
the residues are larger and more varied — a partial temp archive, a partial temp
tree, and a fully-materialised generation no pointer references (left by a crash
between steps 9 and 10, or superseded by a repair). `reap_orphans` reclaims all
three. Liveness is gated on the owning pid **and its start time** — the executor
already needs that probe, and a bare pid check would spare an orphan forever once the
pid recycled — with an age backstop beyond the fetch-plus-extract deadline so nothing
leaks permanently, and a skip for any generation a live process still holds. It runs
from `materialise` and from `cache prune`, never from `locate`, which stays a query
with no side effects.

#### 3. Documented divergence

**Files**: `cli/launcher/src/launch/outbound/resolve/mod.rs`,
`cli/launcher/src/launch/core.rs`, `cli/pup.ron`
**Changes**: ADR-0060 calls the two integrity models "a documented difference
rather than an oversight", which means it must actually be documented in
`resolve/`. Extend the module doc comment to state both models and which applies
where.

Trees are **not** routed through `ResolveBinary::resolve` (`mod.rs:180-233`) —
that method's per-exec re-verify is precisely what they are exempt from, and its
contract is name → executable path for `exec`. But refusing that port leaves the
second artifact class with no port at all: `resolve/tree.rs` would be an *outbound
adapter* module called directly from `main.rs` and from the `cache` built-in, while
`launch::core` holds both existing driven ports (`ResolveBinary`, `ExecBinary`), the
error taxonomy and the `run_external` use case — and `cli/pup.ron` pins
`accelerator::launch::core` to std/kernel/self imports. The launcher's one enforced
architectural rule would then cover one of its two resolution paths.

So `launch::core` declares **three narrow ports**, not two broad ones:

- `LocateSealedTree` — pure lookup, no network, returns `Option<TreePath>`. This is
  the only one the dispatch path may call.
- `MaterialiseTree` — network plus filesystem, called only by `ensure` and `repair`.
- `VerifyTree` — a read-only walk returning a per-entry discrepancy report.

A single `ResolveArtifactTree` meaning "find-or-materialise" would put the forbidden
behaviour one argument away from the warm path, when the whole design rests on
dispatch never fetching; and a `VerifyArtifactTree` meaning "walk, and repair" would
put a query and a destructive mutation behind one abstraction, blunting the very seam
the ports exist to provide. With the split, `repair = verify → materialise → repoint →
reap` is a **use case in `launch::core`** over the three ports, mirroring how
`run_external` sits over `ResolveBinary` + `ExecBinary` — so the interesting decision
(what to do when verification fails) sits in front of the adapter rather than inside
it.

Tree-specific `ResolutionError` variants — extraction, path-escape, seal, attestation,
pointer — replace folding everything into `Cache { path, detail }`. Each states its
`Refusal`/`Failed` mapping explicitly, because `swallow_under_fail_safe`
(`launch/core.rs:218-224`) swallows only `Failed`, so the choice silently decides
whether a crawl degrades or hard-fails under `--fail-safe`. Since the pup rule pins
`launch::core` to std, `kernel::Error` and self, the discrepancy report and the
attestation are plain core-owned types with serde living in the adapter.

Which variant each maps to is not cosmetic: `swallow_under_fail_safe`
(`launch/core.rs:218-224`) swallows only `Failed`, so the choice silently decides
whether a crawl degrades or hard-fails under `--fail-safe`. Every new variant states
its mapping explicitly.

`tree.rs` is split along its natural seams — layout and attestation, verified download,
safe extraction, sealing — rather than being one module owning seven
responsibilities, because `cache repair` needs several of them independently.
Following `cache.rs`'s convention, the sealing and permission helpers carry
`#[cfg(not(unix))]` no-op arms so the launcher still type-checks off Unix, or the
module doc states that tree materialisation is Unix-only by design; Windows is
outside ADR-0057's matrix either way, so this is about keeping the neighbouring
module's discipline rather than about supporting Windows.

#### 4. A test trust root, so the container fixtures can verify anything

**Files**: `cli/launcher/build.rs`, `cli/launcher/Cargo.toml`,
`cli/launcher/src/launch/outbound/resolve/keys.rs`, `keys/accelerator-test.pub`
(new), `tasks/build.py`
**Changes**: AC6 and AC12 rest on container fixtures that build artifacts in the same
run and serve them from a `MockServer` — but nothing can sign those artifacts.
`build.rs:28-45` copies `keys/accelerator-release.pub` into `OUT_DIR`
unconditionally and `keys.rs:12` `include_str!`s it, with no env override, no feature
and no path indirection, so a compiled launcher accepts only artifacts signed with
the real release secret, which no test can hold. `ACCELERATOR_RELEASE_BASE_URL`
answers *where* the manifest comes from, never *who signed it*.
`cli/launcher/tests/resolution.rs` sidesteps this by constructing `TrustedKeys`
in-process, which a container running the real binary cannot do.

`keys.rs` already documents itself as "verify-any-of over a small key set, so
rotation has an overlap window", so the seam exists. A non-default
`test-trust-root` cargo feature makes `build.rs` embed a **second** key from
`ACCELERATOR_TEST_PUBLIC_KEY_FILE` alongside the production one, with
`rerun-if-env-changed` on both and a `cargo:warning` when it fires. The production
key is always embedded, so the feature widens the trusted set for a test build rather
than substituting it, and the production verification path is still the one under
test.

Three guards keep it out of a release, because a feature that weakens the trust root
is only acceptable if it cannot ship:

1. The feature is absent from `[features] default`, and a build-system test asserts
   `build.cli_cross_compile` passes no `--features` flag.
2. `keys/accelerator-test.pub`'s minisign comment line is the fixed marker
   `ACCELERATOR TEST KEY — NEVER SHIP`. Because an embedded key is a string constant
   in the binary, the release pipeline asserts that byte sequence appears in **none**
   of the four cross-compiled launchers — a mechanical check, not a convention.
3. The same assertion runs over the committed `bin/` shims, so a locally-built
   launcher with the feature on cannot be committed either.

The container task builds its launcher with `--features test-trust-root` and signs
its synthetic artifacts with the matching throwaway secret, which is generated per
run and never committed.

### Step 4c: `accelerator cache` built-in

#### 1. Command surface

**Files**: `cli/launcher/src/launch/inbound/cli.rs`,
`cli/launcher/src/launch/core.rs`, `cli/launcher/src/main.rs`,
`tasks/shared/dispatch_coherence.py`
**Changes**:

```
accelerator cache verify [<name>]   walk sealed trees against their file tables
accelerator cache repair [<name>]   re-materialise any tree that fails verify
accelerator cache ensure <name>     materialise a tree if it is not already
accelerator cache prune             reclaim unreferenced generations and orphans
```

`verify` walks each pointed-at generation against its `.files` table using
`symlink_metadata`, and **hashes every regular file**. There is deliberately no
stat-and-escalate shortcut: a substitution that preserves size and mode is exactly
the case the table exists to catch, and an escalation predicate keyed on size or mode
never fires for it — the digests would never be read on the only path that reads
them. ADR-0060 measures a full hash of the whole set at roughly 120ms on the
reference host, which is affordable on a command a user runs deliberately and never
runs on the hit path. The stat pass survives only as a cheap pre-check for missing
and unexpected entries. `verify` reports per-entry discrepancies — missing, extra,
size, mode, digest, link target — rather than a bare pass/fail, so the output
diagnoses as well as detects.

`verify` is **offline by construction**: `<name>` is validated against a compiled-in
artifact-name set (the Rust mirror of `TREE_ARTIFACTS`, held to it by a drift test),
not against the manifest. Validating against the manifest would make a diagnostic
that inspects local disk require two HTTPS GETs and a signature verification, so it
would be unavailable exactly when a user reaches for it — offline, air-gapped, or
with the release host down. Default-deny still holds, and no path is ever constructed
from an unrecognised token.

`repair` verifies, then **materialises a new generation** for each failing artifact
and swaps the pointer to it. Because generations are distinct directories, the
replacement is built alongside the tree in use: a live daemon keeps every inode it has
already opened *and* every file it opens later — locale packs, `.pak` resources,
`icudtl.dat`, lazily-`require`d modules — which the single-file `exec` inode argument
does not cover. Nothing is unlinked before a verified replacement exists, so a repair
whose refetch fails leaves the working tree exactly as it was rather than destroying
the only copy. The superseded generation is left for `prune`.

`repair --force` skips verification and re-materialises unconditionally. It is the
only recovery for a tree that is internally consistent but *wrong* — assembled for the
wrong architecture, or missing a component — which `verify` cannot detect by
construction, since such a tree matches its own table perfectly. Without it, a user
following the remediation string in a failure envelope gets a successful no-op and no
diagnosis.

`ensure` is the cold-path entry point `accelerator-design` calls when the launcher
exported no path for a tree it needs. It materialises and prints the resolved path, or
fails with a structured cause the caller maps to a downgrade reason. It exists so the
launcher never has to know which design subcommands need a runtime (see Phase 7).

`prune` reclaims every generation no pointer references and no live process holds,
plus orphan temps. It is what bounds growth for anyone who takes the documented
`ACCELERATOR_CACHE_DIR` escape, since that location sits outside the plugin tree and
so outside the only eviction this design otherwise has: content addressing means an
unchanged artifact is reused rather than duplicated, but each pin bump still
materialises a fresh tree and nothing else would ever remove the old one.

`<name>` is validated against the compiled-in artifact set for every verb, and the
canonicalised target must be a direct child of `trees/` before any removal.

`BUILTIN_SUBCOMMANDS` (`dispatch_coherence.py:41`) gains `"cache"` (registration
point 10). A test pins that set against the clap `Command` enum, so the two
cannot drift. `cache` becomes permanently unavailable as a dispatch token.

### Success Criteria

#### Automated Verification

- [ ] Failing tests first. The signature and end-to-end resolution cases follow
      `tests/resolution.rs:41-199` with its `MockServer` and real keypair — but the
      extraction, sealing, attestation, pointer and reaper tests exercise
      `resolve/tree.rs` **directly with no signing step**, so they cannot inherit that
      file's `skip_if_no_minisign!` guard (`:189-199`), which returns `Ok(())` with
      only an `eprintln!` and would report green on any machine without `minisign` on
      `PATH`
- [ ] A corrupt archive is rejected **before** anything is extracted — the test
      asserts the trees directory is empty after the failure
- [ ] A tarball is rejected for each of: a `../` entry, an escaping symlink, a
      hardlink whose target escapes, an absolute path, a symlink-then-traverse
      chain, a FIFO or device entry, a tree exceeding `uncompressed_size`, and an
      entry count over `entry_count`
- [ ] A setuid archive member is materialised without its setuid bit, and an
      archive member marked executable keeps only its executable bit
- [ ] A streaming fetch whose first attempt fails after N bytes succeeds on retry,
      rather than producing a concatenated archive that can never verify
- [ ] A stalled transfer (no bytes for longer than the progress floor) fails fast
      rather than waiting out the full deadline three times
- [ ] Exactly **one** archive fetch occurs when two cold resolutions of the same
      tree race, asserted against the `MockServer`'s request count
- [ ] The launcher binary size delta is within the stated per-target ceiling
      (~3.3MB, derived from work-item:0186's ~0.3ms/MB verify rate and a 1ms
      budget), asserted per target rather than recorded
- [ ] A second resolution of the same tree issues **zero** HTTP requests,
      asserted against the `MockServer`'s request count
- [ ] A resolution with the release host unreachable still succeeds on a populated
      cache
- [ ] Two concurrent cold resolutions of the same tree issue **exactly one** archive
      fetch, asserted against the `MockServer`'s request count, and neither observes
      a partial tree
- [ ] A winner that fails mid-materialisation releases the lock, and the loser makes
      progress rather than waiting on a pointer that will never appear
- [ ] A crash at each of steps 4 through 10 leaves only reclaimable garbage: no
      pointer is published, `locate` reports a miss, and the reaper removes the
      residue
- [ ] A pointer naming a directory that does not exist, is not a direct child of
      `trees/`, is not 64-hex, or is not owned by the effective uid is treated as a
      miss rather than exported
- [ ] A sealed tree is removable by `remove_dir_all` without an intervening chmod;
      an archive member marked executable is still executable after sealing; and a
      symlink's target is not re-moded by the seal walk
- [ ] `cache verify` detects each of a deleted file, a truncated file, a **same-size
      same-mode** content substitution, a mode change, a changed symlink target, and
      an unexpected extra entry
- [ ] `cache verify` succeeds with the release host unreachable
- [ ] A truncated tree and a corrupted tree are each returned to a working state by
      `accelerator cache repair`, which materialises a **new generation** and swaps
      the pointer rather than removing the old tree first
- [ ] A repair whose refetch fails leaves the previous tree in place and still
      resolvable
- [ ] A repair run while a process holds files open in the old generation does not
      unlink them, and that process can still open further files from it
- [ ] `repair --force` re-materialises a tree that passes `verify`
- [ ] Every `cache` verb refuses an unrecognised `<name>` without touching the
      filesystem
- [ ] Two release versions naming the same digest share **one** generation
      directory and two pointers, and the second version issues **zero** archive
      fetches
- [ ] Two platforms sharing one cache root each resolve their own tree
- [ ] A cache root that is group- or world-writable, or not owned by the effective
      uid, is refused rather than trusted
- [ ] The reaper removes a temp archive, a temp tree, and an unreferenced
      generation whose owning pid is dead; spares all three while it is live; and
      spares nothing indefinitely once the age backstop passes, including after pid
      reuse
- [ ] `cache prune` reclaims an unreferenced generation and leaves the pointed-at
      one
- [ ] `manifest.example.json` with an added `artifacts` key still parses, and a
      manifest *without* `artifacts` still resolves single-file binaries
- [ ] `mise run cli:check` exits 0
- [ ] `mise run deny:check` exits 0, and `libz-sys`/`zlib-ng-sys`/`zlib-sys` are
      absent from the launcher feature graph
- [ ] `test-trust-root` is absent from the launcher's default features, and
      `build.cli_cross_compile` passes no `--features` flag
- [ ] The `ACCELERATOR TEST KEY` marker appears in none of the four
      cross-compiled launchers, nor in any committed `bin/` shim
- [ ] A launcher built with `--features test-trust-root` verifies an artifact
      signed with the test key **and** one signed with the production key
- [ ] A warm executor invocation satisfies `after ≤ 1.1 × before` against a
      pre-Phase-4 launcher on the same host, measured with work-item:0186's method
      (50 interleaved samples in one process, order alternated) rather than a
      bash loop, which 0186 records as not method-comparable
- [ ] `mise run` exits 0

#### Manual Verification

- [ ] Inflating the browser archive completes within a stated ceiling on the
      reference host — a threshold, not a recorded observation; if `rust_backend`
      misses it the escalation is a faster **pure-Rust** backend (`zlib-rs`, if it can
      be shown to need no C toolchain), never a `*-sys` crate
- [ ] Files in a materialised tree are not writable by the owning user without an
      explicit chmod, and the tree as a whole is still removable
- [ ] `accelerator cache verify` on a clean cache reports every tree as sealed
      and matching

---

## Phase 5: Release-pipeline assembly

### Overview

Assemble the driver bundle and the browser in CI from verified upstream inputs,
and publish them on the existing manifest and minisign path. Nothing in `tasks/`
exists to reuse for the *inputs*: there is no HTTP helper, no GPG code, and no npm
signature or SLSA verification. AC13 is three new implementations.

The *output* side reuses the existing path but is not free either. Every list on
that path is derived from `DISPATCHED_SUBBINARIES` by design rather than from a
directory scan, so tree artifacts are invisible to signing, upload and
pre-publish re-verification until each is given an explicit arm (§5). Getting that
wrong publishes a signed manifest promising assets that do not exist, which is
unrecallable.

### Changes Required

#### 1. Pin the vendored version

**File**: `skills/design/inventory-design/scripts/playwright/package.json`
**Changes**: `~1.55.1` becomes the exact version. This makes the fetched package,
the API `lib/*.js` was written against, and the derived Chromium revision one
choice rather than three that can drift. AC10's guard reads this file.

#### 2. Upstream input verification

**Files**: `tasks/vendor/verify.py` (new), `tasks/vendor/pins.py` (new),
`keys/nodejs-release.asc` (new), `keys/npm-registry.pem` (new), `mise.toml`,
`RELEASING.md`
**Changes**: Three verifications, each failing the release rather than the user's
run. Each needs a trust anchor that does not arrive over the channel it is
verifying, and that is the part ADR-0059 leaves open: it establishes that the
sha512 integrity is fixity rather than provenance "because it comes from registry
metadata fetched over TLS", but never says where the key validating the *signature*
comes from. Fetching that key from the registry too would reproduce the same
problem one level up, so both key sets are committed.

- **`playwright-core`** — fetch from `registry.npmjs.org`, verify the registry
  signature against `keys/npm-registry.pem`, and verify the SLSA provenance
  attestation. That check is only as strong as its predicate:
  `gh attestation verify` without `--owner`/`--repo` accepts an attestation from
  any builder, so the expected source repository, the expected workflow identity,
  and a subject digest bound to the fetched tarball are all asserted explicitly,
  and any mismatch fails the release. `gh attestation verify` appears today only
  as a manual step in `RELEASING.md:271-281`; this makes it a pipeline step.
- **Node runtime** — fetch `SHASUMS256.txt` and its `.asc` from `nodejs.org/dist`,
  verify the GPG signature, then verify the tarball's digest against the signed
  manifest. The version is not chosen independently: ADR-0059 has it mirror the
  pairing upstream ships, so it is derived from the vendored driver's pairing and
  guarded like the Chromium revision (§4).

  The verification must not trust `gpg`'s exit code, which is **0** for a
  well-formed signature from a key merely present in the keyring and carrying no
  trust — it prints only `WARNING: This key is not certified` to stderr. So:
  `gpg --no-default-keyring --keyring` against the committed key, with `--status-fd`
  parsed.

  `VALIDSIG` alone is not the predicate, though, and this is the adjacent trap:
  GnuPG emits `VALIDSIG` for cryptographically valid signatures made by **expired**
  and **revoked** keys too — those cases replace `GOODSIG` with `EXPKEYSIG` or
  `REVKEYSIG` rather than suppressing `VALIDSIG`. A `VALIDSIG`-plus-fingerprint check
  would therefore accept a `SHASUMS256.txt` signed by a Node release key that has since
  been revoked, which is the single case where rotation matters most. So the check
  requires `GOODSIG` **and** explicitly rejects `EXPKEYSIG`, `REVKEYSIG`, `EXPSIG` and
  `NO_PUBKEY`, and compares the allowlist against `VALIDSIG`'s **primary-key**
  fingerprint field rather than only the signing subkey's. `gpg` joins the pinned tooling in `mise.toml`, since its
  presence and version on the `macos-latest` runner are otherwise incidental, and
  its absence must fail the release loudly rather than skip the check. The pinning
  route needs checking rather than assuming: `minisign` is pinned as a direct
  GitHub-release binary (`mise.toml:32-35`, `ubi:jedisct1/minisign`), and GnuPG is
  not distributed that way — if no satisfactory pin exists, pin the *behaviour*
  instead with a preflight that asserts a known-good signature verifies and a
  known-bad one does not, so a host `gpg` that cannot be pinned is at least
  proven functional before the release depends on it.
- **Chromium** — pinned, not verified, per ADR-0059. The revision is read from the
  vendored `playwright-core`'s `browsers.json` and cross-checked against
  `pins.CHROMIUM_REVISION`; the bytes are checked against a committed
  `pins.CHROMIUM_SHA256` per platform. That committed constant is what makes
  ADR-0059's "makes the bytes reviewable" true — a digest derived from whatever the
  CDN served this release attests our own output rather than the input, and is
  trust-on-first-use on every cut. Committing it converts that into one reviewed
  moment. It bounds blast radius; it does not establish provenance, and the
  module's docstring says so plainly rather than implying otherwise.

**One refresh procedure** covers both key sets and both pins, documented in
`RELEASING.md`, because they fail the same way — stale blocks releases, and
carelessly refreshed is the verification's weakest point, which is ADR-0059's own
recorded consequence. It requires that a new key or hash be obtained from a channel
independent of the one it will verify, landed in the same PR as the
`playwright-core` pin bump that motivated it, and reviewed as a change to a trust
anchor rather than a routine version bump. A Playwright upgrade is therefore one PR
touching the pin, four Chromium hashes, the eight `ASSEMBLED_SHA256` entries §8
introduces (two artifacts × four platforms), and any key that rotated with it.

The procedure is documentation, so it is backed by two mechanical guards, because a
committed anchor is only as strong as the review that gates it and this repository has
no CODEOWNERS file — a change to `keys/**` or `tasks/vendor/pins.py` is reviewed
exactly like a version bump today. First, a build-system test asserts the keys present
in `keys/nodejs-release.asc` are exactly the fingerprints in the committed allowlist and
that each is unexpired, so the two halves of the Node anchor cannot diverge silently.
Second, a CODEOWNERS entry (or an equivalent CI guard) covers `keys/**` and
`tasks/vendor/pins.py`, so a trust-anchor change cannot merge on a routine path.

The assembled digests are the one anchor whose value cannot be obtained
independently — they are computed from our own deterministic assembly of inputs the
other anchors have already verified, so they attest reproducibility rather than
provenance. The procedure records that distinction, and requires them to be
regenerated by a clean assembly on a machine that fetched the upstream inputs fresh,
never copied from a reuse path.

`requests` is added to the build-system dependency group, since `tasks/` has no
HTTP client and every existing fetch delegates to `npm`/`cargo`/`rustup`/`gh`.

#### 3. Assembly

**Files**: `tasks/vendor/assemble.py` (new), `tasks/build.py`,
`.github/workflows/main.yml`
**Changes**: Two tasks, not one, and — this is the part an earlier draft got wrong —
**two workflow steps**, not one:

- `vendor.verify_upstream_inputs` downloads and verifies, and **never extracts**. It
  needs `GH_TOKEN` for `gh attestation verify`.
- `build.assemble_tree_artifacts` extracts and assembles, and runs with **no**
  `GH_TOKEN`.

Wiring both into `release_prepare` would have made the split imaginary: `Prepare
stable release` (`.github/workflows/main.yml:604-607`) is a single step running
`mise run release:prepare` with `GH_TOKEN` in its `env`, so two invoke tasks inside
it share one environment. Assembly therefore gets its own mise task and its own
workflow step, invoked outside `release:prepare` — which is also what makes the
scoping assertable, since the existing attest-block tests inspect workflow shape and
cannot see inside an invoke call graph.

The split matters because assembly extracts an npm tarball and the Chromium zip,
and ADR-0059 records Chromium's custody as TLS-only with no signature. Today the
`Prepare` steps carry `GH_TOKEN` in a job holding `contents: write` and
`attestations: write`, upstream of the step holding
`ACCELERATOR_RELEASE_SECRET_KEY` — so a path-traversal entry could overwrite a
`tasks/*.py` module that the later Sign step imports. Extraction therefore lands in
a staging directory **outside the checkout**, only the finished archives are copied
into `dist/release/`, and the same entry rules the launcher applies (Step 4b step 4,
plus the entry-type allowlist, absolute-path and hardlink rejection, mode masking,
and size and count caps) apply CI-side too. This extends the rule the plan already
follows for the signing secret: the step that handles untrusted input holds no
credential.

**What a step boundary does and does not buy**, stated plainly rather than
overclaimed. It removes `GH_TOKEN`, and `persist-credentials: false` on the release
job's checkout removes the app token `actions/checkout` otherwise writes into
`.git/config`. It does **not** remove the job-wide values: `id-token: write` and
`attestations: write` mean `ACTIONS_ID_TOKEN_REQUEST_URL`/`_TOKEN` and
`ACTIONS_RUNTIME_TOKEN` are present in every step of the job regardless of its `env`.
So an extraction escape still reaches enough to mint an OIDC token for a fraudulent
attestation.

Two things bound that residue. §8's committed `ASSEMBLED_SHA256` means tampered bytes
cannot reach the signing step at all — the attacker's path to a *signed* artifact is
closed independently of the token question — so what remains is token theft rather
than artifact substitution. And the extraction rules above are what stop the escape
happening in the first place.

Full isolation would mean a separate job with `permissions: {}`, passing ~1.2GB of
archives between jobs as workflow artifacts. That is deliberately not taken here: the
release job's own comment (`main.yml:600-603`) requires the prepare/sign/finalise
sequence to stay in one job for version monotonicity, the transfer cost is
substantial, and the digest pin already closes the outcome that matters. It is
recorded as the escalation if the residual is later judged unacceptable.

`build.assemble_tree_artifacts` produces, per platform:

```
dist/release/accelerator-driver-<platform>.tar.gz
dist/release/accelerator-browser-<platform>.tar.gz
```

Flat in `dist/release/` — `@actions/glob`'s `*` does not cross `/`, so a nested
staging tree would silently miss `dist/release/accelerator-*` and fail
`test_attest_globs_cover_every_published_asset`
(`tests/unit/tasks/test_workflows.py:207-221`).

The driver tree contains the Node binary and `playwright-core`. The browser tree
contains `chromium-headless-shell` only; `ffmpeg` is excluded.

Assembly also **decides whether the trees contain symlinks at all**, and records the
answer, because the launcher's extraction allowlist admits in-root symlinks — the
trickiest branch in the extractor, since defeating a symlink-then-traverse chain needs
each entry resolved against the real root as it is created. Since we produce the
archives, that branch may be unnecessary: if assembly emits no symlink, a CI-side
assertion pins that and Step 4b narrows its allowlist to regular files and directories
only, retiring the hardest-to-review code in the extractor rather than maintaining it
for a capability nothing exercises.

Both tasks are wired into `prerelease_prepare` (`tasks/release.py:117-129`) and
`release_prepare` (`:144-160`), verification then assembly, **after**
`build.cli_cross_compile` and **before** `build.create_debug_archives`. They go in
`prepare`, never `sign` — `_sign` (`:86-100`) is the only function holding the
secret, and `.github/workflows/main.yml:494-499` scopes
`ACCELERATOR_RELEASE_SECRET_KEY` to Sign steps deliberately. No npm, nodejs.org or
CDN fetch ever happens inside `_sign`.

#### 4. Version guards

**File**: `tasks/vendor/assemble.py`
**Changes**: The assembly fails the release if the fetched `playwright-core` is
not the exact version `package.json` declares, or if the fetched Chromium
revision is not the one that package's `browsers.json` names. Per ADR-0059 the
pairing is structural, so this guards the construction rather than testing
compatibility after the fact.

#### 5. The publish path: registry, signing, manifest, upload, re-verify

**Files**: `tasks/shared/paths.py`, `tasks/signing.py`, `tasks/manifest.py`,
`tasks/github.py`
**Changes**: Every list on the publish path is derived from an explicit registry
rather than from a directory scan, and each is derived from the *same* one —
`upload_and_verify_release` (`tasks/github.py:335-337`) records why: the "every
asset uploaded" and "every asset re-verified before `--draft=false`" lists "cannot
derive from two values". Tree artifacts need the same treatment, so they start with
a registry.

`tasks/shared/paths.py` gains `TREE_ARTIFACTS: tuple[str, ...] = ("driver",
"browser")` beside `DISPATCHED_SUBBINARIES` (`:29`), plus a
`tree_artifact_asset_path(name, platform)` mirroring `subbinary_asset_path`
(`:79`). Assembly, signing, manifest emission, upload and re-verification all
derive from it, so adding or retiring an artifact is one edit rather than a hunt
across five files.

The single source has to cross the language boundary, though, or it stops at `tasks/`.
The Rust side encodes the same names in two places — the launcher's compiled-in set
that validates `cache` verbs offline, and `accelerator-design`'s `ensure` call sites —
so a **drift test** pins the Rust set against `TREE_ARTIFACTS` and both against the
`artifacts` keys in `manifest.example.json`, in the same shape as the
`BUILTIN_SUBCOMMANDS` ↔ clap `Command` pin registration point 10 already requires.
Without it, retiring an artifact yields a launcher exporting a variable nothing
publishes, or a design binary requesting a name the manifest no longer carries —
failures that surface at runtime on a user's machine, since trees are exempt from the
per-exec re-verification that would otherwise catch a mismatch.

Four arms follow, and none is optional:

1. **Signing** — `sign_staged_binaries` (`tasks/signing.py:60-79`) builds an
   explicit expected list from the launcher plus `_subbinary_signing_targets()`
   and raises on any missing member, deliberately never scanning a directory. A
   `_tree_artifact_signing_targets()` arm joins it, so a partial assembly fails
   closed exactly as a partial cross-compile does.
2. **Manifest** — `collect_artifact_entries()` mirrors `collect_entries`
   (`tasks/manifest.py:80-107`) and a second key joins `build_manifest`
   (`:110-129`). It emits more than `collect_entries` does: alongside `sha256` and
   the inline signature it records `archive_size`, `uncompressed_size` and
   `entry_count`, all three measured during assembly rather than restated, so
   producer and consumer cannot disagree about the bounds the launcher enforces.
   **Do not bump `SCHEMA_VERSION`** (`:23`). Ordering is not free
   here: `collect_entries` slurps the pre-produced `.minisig` contents as the
   inline signature, so collection must follow signing — which `_sign`
   (`tasks/release.py:84-99`) already sequences correctly, and the artifact arms
   slot into those same two calls.
3. **Upload** — `_release_uploads` (`tasks/github.py:231-248`) assembles launcher,
   manifest, debug archives and `_subbinary_uploads`; a `_tree_artifact_uploads`
   arm joins it, each archive with its `.minisig` sidecar. The existing `missing`
   check (`:339-343`) then fails loudly on an unassembled artifact before a single
   upload starts.
4. **Re-verification** — `_subbinary_reverifies` (`:287-315`) reads
   `manifest["binaries"][name]` and re-downloads each asset to check its sha256
   and inline signature. A `_tree_artifact_reverifies` arm reads
   `manifest["artifacts"][name]` and does the same, so the `--draft=false`
   transition (`:356`) waits on the tree archives too.

Without all four, the release publishes a *signed* manifest naming artifacts that
were never signed, never uploaded and never re-verified —
`_assert_staged_manifest_is_current`'s own docstring names that outcome as one that
"cannot be recalled". Every user on that version would 404 on their first design
run, and the fix would be a whole new release.

#### 6. The five guards that will trip

**Files**: `tasks/release.py`, `tests/unit/tasks/test_manifest_contract.py`,
`cli/launcher/tests/fixtures/manifest.schema.json`, `.github/workflows/main.yml`
**Changes**:

1. `_assert_staged_manifest_is_current` (`tasks/release.py:57-83`) compares only
   `set(staged["binaries"])` against `DISPATCHED_SUBBINARIES`. Without an
   artifact equivalent a stale artifact manifest passes silently — add the
   parallel arm against `TREE_ARTIFACTS`.
2. `test_attest_globs_cover_every_published_asset` — satisfied by the flat
   naming above, but assert it rather than assume it.
3. `test_every_attest_block_declares_the_same_subjects` (`:198-204`) — all three
   blocks (`main.yml:502-508`, `:615-621`, `:639-645`) must stay identical.
4. `tests/unit/tasks/test_manifest_contract.py:30-48` iterates `binaries` only;
   add a parallel arm for `artifacts`, asserting the same
   `accelerator-{key}-{platform}.tar.gz` convention Phase 4 pinned in
   `manifest.example.json`, so producer and consumer are held to one fixture.
5. `cli/launcher/tests/fixtures/manifest.schema.json` describes itself as "the
   signed distribution contract between the release signer and the
   launcher/bootstrap readers", and its top-level `required` is
   `["schema_version", "version", "binaries"]` with no `artifacts` property and no
   artifact `$defs`. It gains both — an `artifactEntry` and an
   `artifactPlatformEntry` carrying the three required sizes — or the one document a
   third party would read to understand the wire format describes a shape the
   producer no longer emits. `test_schema_platform_enum_matches_the_alias_set` reads
   only `$defs.binaryEntry` today, so it is extended to assert the artifact side's
   platform-alias enum equals `ALIASES` too; otherwise the guard that exists to stop
   platform tables drifting would not cover the new axis.

A fifth guard turns out **not** to trip: `_assert_no_leaked_artifacts`
(`tasks/release.py:40-54`) matches markers against `git status --porcelain -uall`,
and `/dist/` is gitignored (`.gitignore:23`), so the archives are invisible to it.
Worth recording so nobody spends time on it.

#### 7. Release-job capacity

**File**: `.github/workflows/main.yml`
**Changes**: The `release` job runs the whole pipeline **twice** in one job —
stable, then the post-stable pre.0 cut (`:604-650`) — so roughly 2.4GB of
assembly and upload per stable release, on a `macos-latest` runner with no
`timeout-minutes` and no disk guard. `dist/release/` is never cleaned between the
two (`tasks/release.py:60-62`), and `--clobber` on retry
(`tasks/github.py:318-319`) re-uploads the lot.

Add a `timeout-minutes` to the release job and a disk-space assertion before
assembly. Hosting capacity itself is confirmed and assumed.

One failure path also changes character at this payload size. `download_and_verify`
(`tasks/github.py:140-145`) converts a `subprocess.TimeoutExpired` into an
`AssetVerificationError`, which preserves the draft — but the two re-verify helpers
tree artifacts actually reach do not: `_reverify_via_shim` (`:192-193`) and
`_reverify_subbinary` (`:206`) call `download_release_asset` bare, and its
`timeout=120` (`:111`) is sized for a 7.6MB launcher. A 177MB archive plausibly
exceeds it, raising `TimeoutExpired`, which is not an `AssetVerificationError` and
so lands in `upload_and_verify_release`'s `except Exception` arm — running `gh
release delete <tag> --cleanup-tag --yes` (`:359-365`) *after* `_publish` has
already committed, tagged and pushed the version bump. A transient download hiccup
would burn a version number and leave the repository and the release host
inconsistent, under the `accelerator-release` concurrency lock.

So: size `download_release_asset`'s timeout to the expected asset rather than a flat
120s, and wrap both re-verify helpers so a transport failure becomes an
`AssetVerificationError`. That routes it to the draft-preserving arm with the forensic
alert that already exists, and `--clobber` (`:318-319`) means a preserved draft can be
re-driven to green.

`TimeoutExpired` is not the only newly-reachable path, though, so the narrowing is by
**default** rather than by enumeration: every failure inside the upload/re-verify
envelope preserves the draft, and the delete arm is reserved for an explicit,
enumerated set of pre-upload failures. At ~2.4GB per stable cut,
`OSError: No space left on device` from a re-verify download or from `compute_sha256`,
a hung `gh release upload` (which has neither timeout nor retry), and a
`CalledProcessError` from a transport blip are all now plausible — and each would
otherwise delete the tag after `_publish` has already pushed the version bump. Bounded
retry with backoff wraps `_upload_clobber`, the disk assertion covers the whole job
rather than only pre-assembly (assembly plus upload staging plus re-verify temp
downloads, across both passes), and the newly-added `timeout-minutes` is itself
recorded as an abort cause that runs no cleanup arm, so it is sized with headroom and
`--clobber` is documented as its recovery.

#### 8. Reuse across cuts, and a functional gate

**Files**: `tasks/vendor/assemble.py`, `.github/workflows/main.yml`
**Changes**: Two problems that only appear once assembly is in the pipeline.

**Every release becomes dependent on three third-party hosts.** Assembly is wired
into both `prerelease_prepare` and `release_prepare`, so every cut fetches from
`registry.npmjs.org`, `nodejs.org/dist` and `cdn.playwright.dev` — yet all three
inputs are now pinned by exact version and hash, so the produced bytes are identical
release after release. As written, an npm outage, a key rotation or a yanked version
makes the pipeline unreleasable, including for an urgent fix to something entirely
unrelated: a large new single point of failure in front of the one mechanism that
ships fixes to users.

So assembly becomes **deterministic and digest-pinned**, and reuse is authenticated
rather than merely cached.

**Deterministic assembly.** `assemble_tree_artifacts` normalises everything that
would otherwise vary between runs: entries emitted in sorted order, mtimes, uid, gid
and owner names fixed to constants, modes masked to the same `0755`/`0644` the
launcher enforces, and gzip written without an embedded timestamp. Assembling the
same pin triple twice must produce byte-identical archives, asserted by a test that
assembles twice and compares digests. This is worth doing on its own merits — it
makes a release auditable by anyone who can run the same pins — but it is also the
precondition for everything below, because an unreproducible archive cannot be
pinned.

**A committed expected digest.** `pins.py` gains `ASSEMBLED_SHA256`, one digest per
artifact per platform, committed and reviewed under the same trust-anchor refresh
procedure as the keys and the upstream pins (§2). Every archive that reaches the
signing step — freshly assembled or reused — is checked against it, and a mismatch
fails the release. Without this the digest check is self-referential: "matching
digest" computed from whatever is on disk proves only that the bytes are the bytes.

**Reuse is our own signed asset, not a cache blob.** When the pin triple is
unchanged, the reuse source is the **previous release's published artifact**,
re-downloaded and verified with sha256 plus minisign against the embedded public key
— the identical check the launcher performs on a user's machine — and then against
`ASSEMBLED_SHA256`. That keeps the chain of custody inside our own signature rather
than extending trust to a mutable store: a CI cache is writable from other workflows
on the default branch, is evictable after a quiet week, and shares a per-repository
budget with the toolchain caches this repo already depends on, so a poisoned or
partially-restored entry would be signed with `ACCELERATOR_RELEASE_SECRET_KEY` and
published with none of §2's npm, SLSA, GPG or Chromium-hash gates re-running — the
plan's own "cannot be recalled" outcome, reached by accident rather than by attack.
Any mismatch, any absent asset, and any pin movement falls back to a full cold
assembly, so the reuse path can only ever be an optimisation.

The bytes therefore reach a user having been verified against upstream once, at a
reviewed moment recorded in `pins.py`, and authenticated to our own key on every
reuse after that. The same mechanism removes the duplicated work in the release job's
double pass (§7): the post-stable pre.0 cut reuses the stable pass's archives by
digest instead of re-assembling identical bytes.

**Nothing ever executes what was built.** Every other gate in this phase is about
provenance and shape: upstream signatures, version and hash guards, glob coverage,
manifest arms, and a `.minisig` the CLI-side verifier accepts. A brand-new step
composing four platforms from three upstreams can produce a correctly-signed,
correctly-hashed, structurally-wrong tree — wrong architecture, missing `NOTICES/`,
a layout `playwright-core` cannot resolve — and it would pass everything, reach
every user of that release, never self-heal (trees are exempt from per-exec
re-verification), and be faithfully re-fetched by `cache repair`, which trusts the
same manifest. Recovery would be a new release for every affected user.

So assembly ends with a host-platform smoke check: unpack the driver and browser for
the runner's own platform, execute the Node binary and the headless shell with
`--version`, and assert `NOTICES/` is populated. It runs on **reused** archives as
well as freshly assembled ones — a reuse path that skipped it would be the one route
by which an unexecuted artifact reaches a release.

It cannot cover the cross-compiled platforms, so the three non-host artifacts get a
structural check instead: the expected file set, plus the ELF/Mach-O header and
architecture of the Node binary and the headless shell for the target they claim to
be. That catches a wrong-architecture or truncated assembly without executing it,
which is the failure mode that would otherwise reach every Linux user of a release,
never self-heal, and be faithfully re-fetched by `cache repair`. Between them the two
checks are the only gates distinguishing "signed" from "works", and both are nearly
free on the macOS runner.

#### 9. Redistribution notices

**File**: `tasks/vendor/assemble.py`
**Changes**: Each artifact carries the notices for what it contains — Node and
its bundled dependencies, `playwright-core`, and Chromium's credits — assembled
into a `NOTICES/` directory at the tree root. Phase 7 adds the subcommand that
surfaces them, so a user reaches them without unpacking the artifact by hand.

An automated assertion covers it here rather than only the manual check: the produced
tree contains `NOTICES/` with an entry per expected component, driven from the same
component list the assembly uses. AC16's notices are the plan's stated substitute for
a legal review gate, so an assembly refactor that silently drops a component must
fail rather than ship.

### Success Criteria

#### Automated Verification

- [ ] Failing tests first for each verification, using recorded upstream fixtures
      rather than live network calls. Committing the keys makes Node/GPG fully
      offline-verifiable, so it is tested for real rather than mocked; the SLSA
      check contacts a transparency log, so its runner is injected and both
      branches asserted — and the plan records that the attestation's *content* is
      not verified in tests
- [ ] A tampered `SHASUMS256.txt` signature fails the release
- [ ] A `SHASUMS256.txt` signed by a well-formed key absent from the committed
      fingerprint allowlist fails the release, **even though `gpg` exits 0**
- [ ] An absent `gpg` fails the release rather than silently skipping the check
- [ ] A `SHASUMS256.txt` signed by a revoked key fails the release, and one signed by
      an expired key fails the release, even though both yield `VALIDSIG`
- [ ] The npm/SLSA path fails closed in each degraded mode — attestation bundle
      absent, transparency log unreachable, `gh attestation verify` unavailable — since
      only mismatch cases were covered before, and these are the modes most likely to
      be made advisory under release pressure
- [ ] The committed Node keyring and the committed fingerprint allowlist describe the
      same key set, each unexpired
- [ ] A `playwright-core` tarball failing its registry signature fails the release
- [ ] An attestation whose source repository or workflow identity differs from the
      pinned predicate fails the release
- [ ] An attestation whose subject digest does not match the fetched tarball fails
      the release
- [ ] A `playwright-core` version other than `package.json`'s pin fails the
      release
- [ ] A Chromium revision other than `browsers.json`'s fails the release
- [ ] Chromium bytes whose sha256 differs from `pins.CHROMIUM_SHA256` fail the
      release
- [ ] A Node version other than the vendored driver's pairing fails the release
- [ ] Assembly is its own workflow step, and that step's `env` contains no
      `GH_TOKEN` — asserted by a workflow test alongside the existing attest-block
      assertions, which is only possible because it is a step rather than a task
      nested inside `release:prepare`
- [ ] The release job's checkout sets `persist-credentials: false`
- [ ] Extraction happens outside the checkout, and a tarball with a `../` entry, an
      escaping symlink, a hardlink, an absolute path or a setuid bit is rejected
      CI-side by the same rules the launcher applies
- [ ] The assembled, signed, manifest-listed, uploaded and re-verified sets are
      pinned against each other by one test, so an artifact cannot appear in some
      and not others
- [ ] An unassembled artifact fails the **signing** step, not the upload step
- [ ] A tree archive with no `.minisig` fails `collect_artifact_entries`
- [ ] `_assert_staged_manifest_is_current` rejects a manifest whose `artifacts`
      keys differ from `TREE_ARTIFACTS`
- [ ] An artifact platform entry missing any of `archive_size`,
      `uncompressed_size` or `entry_count` fails to parse, rather than defaulting
      to 0 and disabling the cap it feeds
- [ ] The emitted sizes match the assembled archive and its extracted tree, so
      producer and consumer agree on the bounds the launcher enforces
- [ ] `manifest.schema.json` validates a manifest carrying `artifacts`, and its
      artifact platform-alias enum equals `ALIASES`
- [ ] A simulated download timeout during tree re-verification preserves the draft
      and emits the forensic alert, rather than deleting the release and its tag
- [ ] `mise run test:unit:build-system` passes, including the new manifest
      contract and attest-glob arms
- [ ] `mise run build-system:check` exits 0
- [ ] Every produced archive matches `dist/release/accelerator-*`
- [ ] Assembling the same pin triple twice produces **byte-identical** archives
- [ ] Every archive reaching the signing step matches `pins.ASSEMBLED_SHA256`,
      whether freshly assembled or reused; a mismatch fails the release
- [ ] An unchanged pin triple reuses the previous release's published artifact and
      performs **no** upstream fetch; moving any one pin re-runs the
      fetch-and-verify path
- [ ] A reused artifact failing its minisign check, its sha256, or the committed
      digest falls back to a full cold assembly rather than being signed
- [ ] The second (pre.0) pass reuses the stable pass's archives rather than
      re-assembling them
- [ ] The host-platform smoke check runs on reused archives as well as freshly
      assembled ones, and fails the release on a tree whose Node binary or headless
      shell will not execute, or whose `NOTICES/` is empty
- [ ] The structural check fails a cross-compiled artifact whose Node binary or
      headless shell has the wrong architecture or object format for its target
- [ ] The produced tree contains a `NOTICES/` entry per expected component, driven
      from the assembly's own component list
- [ ] An end-to-end round trip: a synthetic tree assembled through the real
      assembly path, a manifest emitted through the real `build_manifest`, signed
      with a test key, resolved by the launcher's tree resolver — so the two halves
      of the artifact contract are verified together rather than only by hand
- [ ] `mise run` exits 0

#### Manual Verification

- [ ] A full local dry-run assembly produces both artifacts for one platform, and
      their measured sizes are recorded and fed back into Step 4a's fetch deadline
- [ ] Each produced `.tar.gz` has a `.minisig` that the CLI-side verifier accepts
- [ ] The upload list and the re-verify list, printed for one platform, each
      contain both tree archives and their sidecars
- [ ] `manifest.json` renders `artifacts` beside `binaries` with a launcher built
      before this phase still resolving single-file binaries from it
- [ ] The `NOTICES/` directory in each artifact contains all three licence sets

---

## Phase 6: `run.sh` → Rust, behaviour-preserving

### Overview

Reproduce the Playwright launcher in Rust so the chain is CLI → Node with no
shell in between (ADR-0058). This phase changes *nothing* about where the
runtime comes from: it still resolves the existing lockhash namespace and still
requires system Node. That keeps it independent of Phases 4 and 5 and makes it a
pure, characterization-testable port.

### Changes Required

#### 1. The executor subcommand

**Files**: `cli/design/src/executor/` (new), `cli/design-adapters/src/process.rs`,
`cli/design-cli/src/executor.rs`
**Changes**: `accelerator design executor <command> [json-args]`, forwarding args to
`run.js`.

**The logic goes in the domain crate, behind ports.** This is the most
regression-prone code in the plan — ADR-0058 warns these 203 lines "encode
hard-won fixes … each regresses silently if the port misses it" — and ADR-0053 puts
no business logic in the command layer. Placing the reuse decision, the tolerance
comparison, the state interpretation, the lock policy, the poll deadline and the
envelope/exit-code taxonomy in `design-cli` would put the riskiest code where it is
hardest to exercise, exempt from the domain-purity pup rule. It would also make AC2
unsatisfiable, since AC2 requires the volatile inputs to be "supplied through
injected ports so the output is deterministic by construction rather than by
normalisation".

So `cli/design/src/executor/` holds pure functions over injected ports:

- `Clock` — now, and the poll deadline arithmetic.
- `ProcessProbe` — liveness, and the start time for a pid, as
  `ObservedStartTime::{Known(u64), Unavailable}` so the container case where `/proc`
  is unreadable is a value the domain matches on rather than an adapter-side panic.
- `StateStore` — returns a domain value, not raw JSON: `RecordedDaemon { pid,
  start_time: RecordedStartTime }` where
  `RecordedStartTime::{Probe(u64), Wallclock(u64), AbsentOrUnparseable}`. The design
  crate cannot parse JSON (its pup rule permits only std, `kernel::Error` and
  `crate`), so the parse happens in the adapter — but §3's decisions turn on parse
  *outcomes*, and a port whose return type is unstated cannot carry them.
- `Lock` — acquire, and release.
- `Spawner` — start the daemon. Signalling is a separate `ProcessControl` port;
  bundling creation with signalling made one port do two unrelated things.
- `RunClient` — run the client and return an exit outcome. This is the seam the
  previous draft omitted, and it covers the behaviour most visible to consumers:
  `SKILL.md:142-143` discriminates on the launcher-versus-daemon exit asymmetry, and
  without a port that asymmetry can only be tested end to end. The adapter's terminal
  implementation is `CommandExt::exec`.
- `FreeSpace` — bytes available at a path, so `disk-floor-not-met` is a unit test
  over an injected value plus a zero-request assertion rather than a test that fills
  the runner's disk or needs a loopback filesystem.

`design-adapters` implements them (`/proc` and `ps` parsing, filesystem, spawn), and
`design-cli` is argument parsing plus composition — the same shape
`work`/`work-adapters`/`work-cli` already uses. The reuse verdict then becomes a pure
function of (recorded pid, recorded start time, observed liveness, observed start
time), which is what makes the six characterization behaviours below deterministic
rather than dependent on real processes and real elapsed time.

It must be **total** over those inputs, including the cases `run.sh` never named:

| Recorded | Observed | Verdict |
|---|---|---|
| `Probe(r)` | `Known(o)`, `\|r-o\| ≤ 1` | reuse |
| `Probe(r)` | `Known(o)`, otherwise | stale → recover |
| `Probe(_)` | `Unavailable` | reuse on liveness alone |
| `Wallclock(_)` | any | reuse on liveness alone |
| `AbsentOrUnparseable` | any | stale → recover |

The `Unavailable` row matters because `/proc` being unreadable defeats the *reader*
exactly as it defeats the writer, and `run.sh:55` answers that case with a mismatch —
which would respawn the daemon on every command and lose page state in precisely the
containers AC6 and AC11 exercise. A record with no `start_time_source` key at all, as
written by a daemon that predates this change and survived a plugin upgrade, is read
as `Probe`: `ps lstart` is itself derived from `p_starttime` and agrees to the second.
"Accepted once" is replaced by these rules, because a fresh launcher process per
invocation has nowhere to record that the once was spent.

**Reserved internal tokens are rejected, not merely unnamed.** §3 claims
`design executor daemon` "is not reachable" because "the Rust surface does not expose
it", but verbatim forwarding *is* exposure: `run.js:18` dispatches on
`args[0] === 'daemon'` and `:20` takes the state dir from
`ACCELERATOR_PLAYWRIGHT_STATE_DIR`, which the executor sets on every path. A single
stray `accelerator design executor daemon` would start a second foreground daemon
that binds a fresh port, overwrites `server-info.json` and `server.pid` for the live
one, orphans it mid-crawl, and never returns — the failure `run.sh:191`'s
kill-on-timeout exists to prevent. So argument validation rejects `daemon` and any
other internal `run.js` subcommand by an explicit allowlist of forwardable commands,
with a test.

Behaviour ported from `run.sh`:

- **Start-time identity** (`run.sh:13-42`). Linux: read `/proc/<pid>/stat`, take
  everything after the last `)` (greedy, so a `comm` containing parens is safe),
  field 20, plus `/proc/stat`'s `btime`, divided by `CLK_TCK` with **truncating
  integer division** — `run.sh:25`'s shell arithmetic and `state.js:49`'s
  `Math.floor` both truncate, and floating-point or `(btime * hz + ticks) / hz`
  differs by up to a second, consuming the entire ±1s budget the tolerance exists to
  provide for whole-second-boundary drift. Fixture-pinned against
  `lib/__fixtures__/proc-stat-linux.txt` → `1700145620`, plus a second fixture whose
  tick count does not divide evenly.

  Darwin: **not** `ps -p <pid> -o lstart=`. That emits a wall-clock string with no
  offset, which both `run.sh:38` and `state.js:57` resolve in local time — so
  converting it needs the UTC offset *at that instant*, and the workspace's `time`
  crate is featured `["parsing"]` only (`cli/Cargo.toml:25`), with `local-offset`
  being a separate feature whose `current_local_offset()` returns
  `IndeterminateOffset` in a multi-threaded Unix process. Worse, during the repeated
  hour of a DST fall-back the string maps to two distinct instants, and the port
  would have to resolve that ambiguity identically to V8's parser or be wrong by up
  to 3600s against a ±1s tolerance.

  So Rust reads `sysctl` `KERN_PROC_PID`'s `p_starttime`, which is epoch-based —
  requiring a `libc` workspace dependency, which joins `cli/Cargo.toml` with its
  `deny.toml` and pup implications recorded. **`state.js` stops computing the start
  time at all.** An earlier draft had both sides read `p_starttime`, but Node has no
  `sysctl` binding and `sysctl(8)` prints `kern.proc.pid.<n>` as an opaque struct
  dump with no parseable epoch field, so the JS half had no mechanism — and the
  fallback it would have taken is `Math.floor(Date.now()/1000)`, i.e. the weakest
  path, for every macOS daemon on the primary development platform.

  The dependency inverts instead: the **launcher** spawns the daemon, so it knows the
  pid at fork time and observes the start time with the same probe it will later use
  to check it. It writes the identity record itself once the daemon reports ready.
  `state.js` writes only what it genuinely owns — the port and the readiness facts —
  and the cross-language agreement requirement disappears rather than being tested.
  ADR-0058 names this contract as the port's principal silent-regression risk, and
  one owner is a better answer than two implementations plus an equality assertion.
- **±1s tolerance** (`run.sh:52-59`) for a present, numeric expected value.
- **Double-checked reuse short-circuit** around the lock (`run.sh:101-121` and
  the re-check at `:139-160`) — but see §3: the pre-lock check becomes read-only.
- **Lock**: one implementation, not two. The `flock`-or-`mkdir` dichotomy exists
  because the `flock(1)` **binary** is absent on macOS — a constraint that vanishes
  in Rust, where `flock(2)`/`fcntl` is available on every supported target, and
  ADR-0058 already records that nothing external depends on the mkdir form. So
  `ACCELERATOR_LOCK_FORCE_MKDIR` and the mkdir backend are dropped unless an NFS
  requirement is found, in which case the plan records it; the `owner.<nonce>`
  sentinel protocol is not used either way.

  **The lock is released at launcher exit**, by a `Drop` guard, on every path. This
  resolves a contradiction in the previous draft, which claimed both "held for the
  daemon's lifetime where `run.sh` inherits the flock FD into the child" *and* "a
  `Drop` guard releases it on every path" — the negation of each other. The two
  backends genuinely differ today: `run.sh:126` leaks FD 9 into the daemon
  (confirmed by `test-run.sh:160-163`, which records that it "holds the launcher.lock
  FD across its shutdown, so the next launcher invocation is blocked until then"),
  while `run.sh:152,202` explicitly `rmdir`s the mkdir lock before `exec`. One guard
  cannot reproduce both, and Rust opens files `O_CLOEXEC` by default, so an
  unremarked port would silently drop the inheritance anyway. Releasing at exit is
  the more defensible of the two: holding it for the daemon's lifetime means a
  stale-start-time recovery while the daemon still lives falsely reports
  `another-launcher-running`. **This is a deliberate behaviour change**, so it moves
  to §3 and the contention test asserts the new semantics.
- **State dir** at `<repo>/<config path tmp>/inventory-design-playwright`, mode
  0700, with the `accelerator config path tmp` shell-out replaced by an
  in-process `config` call.
- **Daemon spawn** and the 30s start poll with kill-on-timeout (`run.sh:163-194`).
  Three properties come from shell primitives the port removes, and each must be
  reproduced explicitly rather than inherited. `nohup … &` plus `disown` makes the
  daemon SIGHUP-immune and reparented — so the spawn uses `setsid` (or a double
  fork), or a Ctrl-C in an interactive session kills the daemon mid-crawl.
  `>>"$BOOTSTRAP_LOG" 2>&1` redirects the daemon's stdio away from the caller's — so
  the spawn redirects to the bootstrap log rather than inheriting. And the final
  `exec node run.js "$@"` means the client's exit status, stdout, stderr and
  signal-death *are* the launcher's, with no forwarding logic — so the client path
  either uses `CommandExt::exec` or propagates exit status and signal death (128+n)
  explicitly. `SKILL.md:142-143`'s error discrimination and the launcher-vs-daemon
  exit-code asymmetry both depend on that, and all three regress silently, which
  ADR-0058 names as the port's principal risk.
- **Environment handed to the child**: `ACCELERATOR_PLAYWRIGHT_STATE_DIR`,
  `NODE_PATH`, `ACCELERATOR_PLAYWRIGHT_NS_ROOT`.
- **Three behaviours an earlier draft's inventory missed**, each observable and each
  the kind of thing ADR-0058 warns regresses silently. `run.sh:71-75` emits a
  `no-repo` 3-key envelope on stderr and exits **2** when `find_repo_root` finds
  nothing — a distinct usage outcome the state-dir bullet does not cover.
  `run.sh:159` removes `server-stopped.json` before every spawn, so a stale stop
  reason from a previous daemon's idle or wall-clock shutdown is not left for a later
  reader (`test-run.sh:224` reads it) — without it a failed start looks like a
  completed shutdown to anyone diagnosing a daemon that never came up. And
  `run.sh:163-165` truncates the bootstrap log and `chmod 0600`s it before the daemon
  writes, which is what keeps the timeout envelope's "Check $BOOTSTRAP_LOG" pointing
  at *this* attempt's output, under a `umask 077` the Rust binary does not inherit.
  The inventory is derived by a line-by-line pass over `run.sh`, the way Phase 2 §6
  derives its cut list from `test-design.sh`'s ranges, rather than from memory.

The port removes runtime dependencies on `jq`, `flock`, `sha256sum`/`shasum`,
`nohup`, `sed`/`awk`/`tr`/`date`, and the bash 3.2 floor on this path.

#### 2. Behaviours preserved deliberately

Four asymmetries the port keeps, each because a consumer depends on it:

- **Daemon-side errors exit 0** with the envelope on **stdout**; launcher-level
  failures go to **stderr** with a non-zero exit. `SKILL.md:142-143` discriminates
  on exactly that asymmetry, so collapsing it breaks the skill.
- **Launcher envelopes stay 3-key** (`error`, `message`, `category`) — no
  `protocol`, no `retryable`, unlike everything `errors.js:10` produces. AC2's
  byte-identical assertion pins this.
- **Exit codes 0, 1, 2, 3** keep their current meanings *in this phase*. Phase 7
  redefines exit 3 (`playwright-not-installed`) into an `artifact-unavailable`
  downgrade, which also leaves `PROTOCOL.md:555-566`'s exit-code table describing a
  contract nothing implements — so that table moves with the vocabulary in Phase 7
  §6.

#### 3. Behaviours corrected deliberately

Four, each recorded so the change reads as taken rather than accidental:

- **An absent or unparseable `start_time` is now a mismatch, not a match.**
  `run.sh:108,142` extract `.start_time` via `jq`, swallow failure with `|| true`,
  and `start_time_matches` returns 0 for an empty expected value (`:54`) — so any
  live PID is accepted. Native JSON parsing removes one *cause* of an empty value (a
  missing `jq`) but not the branch: a `server-info.json` written without a
  `start_time` key — by an older daemon, by a truncated write, or by any future
  `state.js` change — still bypasses the PID-recycle guard entirely, which is
  precisely the state an interrupted or partially-migrated daemon leaves behind. So
  absent-or-unparseable is treated as stale state and triggers recovery. The ±1s
  tolerance applies only to a present, numeric value.
- **The lock releases at launcher exit on every path**, by a `Drop` guard — see §1.
  This both fixes the mkdir leak `test-run.sh:178-182` calls "a pre-existing run.sh
  quirk" and deliberately drops the flock FD inheritance into the daemon, unifying
  two backends that behave differently today.
- **The pre-lock reuse check is read-only.** `run.sh:106-121`'s first check runs
  *before* the lock and ends with an unconditional `rm -f "$INFO" "$PID_FILE"` when
  the daemon looks dead. But `state.js:63-66` writes those two files as separate
  atomic renames, so a launcher reading between them — or racing a daemon that has
  just written both — judges the state stale and deletes a **live** daemon's state
  files outside any lock. Two launchers starting concurrently can therefore orphan a
  healthy daemon and spawn a second, losing page state mid-crawl: the exact symptom
  the start-time contract exists to prevent. So the pre-lock check short-circuits on
  a hit or falls through, and the stale-state deletion happens only under the lock,
  where `run.sh:156` already does it.
- **Internal `run.js` subcommands are rejected by argument validation.**
  `run.sh daemon` is an unfiltered internal subcommand that starts a second
  foreground daemon which never returns. Not exposing it on the Rust surface is not
  enough, because §1 forwards arguments to `run.js` and `run.js:18` dispatches on
  `args[0]` — so the executor validates against an explicit allowlist of forwardable
  commands. A command added to `daemon.js` later is unreachable until the allowlist
  moves with it, which the `PROTOCOL.md` ↔ `daemon.js` sync assertion re-homed into
  the node suites (Phase 8 §1) is extended to cover.

Three more, in the retained JS.

**`state.js` stops computing the start time.** `:40-61` currently probes it and
`:60` falls back to `Math.floor(Date.now()/1000)` on any failure — reached on any
platform that is neither `linux` nor `darwin`, and on Linux whenever `/proc` is
unreadable or `execSync('getconf CLK_TCK')` fails, which is common in minimal,
distroless and `hidepid`-hardened containers. An earlier draft kept the fallback and
recorded its provenance in a `start_time_source` field. §1's inversion makes that
unnecessary: the launcher spawns the daemon, so it observes the start time itself with
the probe it will later use to check it, and writes the identity record once the
daemon reports ready. `state.js` writes the port and readiness facts it owns, the
`getconf` shell-out goes with the probe, and the two implementations can no longer
disagree because there is only one. `start_time_source` survives as a field the Rust
writer sets, since a probe that returns `Unavailable` on the *writing* side still has
to be recorded as such for the verdict table above to read it.

**The daemon gains a request token.** It serves JSON commands on `127.0.0.1` with an
OS-assigned port and no authentication (`daemon.js:286-338`), and loopback binding is
not a uid boundary — any local process, including anything the model runs, can drive a
browser that may hold the user's authenticated session, screenshot it, and `evaluate`
in its context. This lands here rather than as a follow-up because the writer, the
reader, the `StateStore` port and the client path are all already being rewritten in
this phase: the launcher generates a random token, records it in the already-`0700`
`server-info.json`, and `client.js` sends it as a header that `daemon.js` requires.
Deferring would mean migrating that file's format and both its readers twice.

**`makeAuthHeaderHandler` stays in place** — imported at `daemon.js:11` and never
called, with `ACCELERATOR_BROWSER_LOCATION_ORIGIN` set nowhere. Wiring it up is new
feature work and deleting it removes a capability `SKILL.md:89-95,196` documents as
security-critical, so neither belongs in a behaviour-preserving port and the defect is
raised as a follow-up (Phase 8 §5). But the *documentation* is corrected in the phases
that rewrite it: both SKILL.md files and `resolve-auth`'s help text state that the
header-auth path is currently inert and that `ACCELERATOR_BROWSER_AUTH_HEADER` should
not be given a live credential until the follow-up lands. Leaving a known-false
security claim in a file this plan is editing anyway is not defensible.

#### 4. The locale regression guard

**File**: `cli/design-adapters/tests/start_time.rs`
**Changes**: `test-run.sh:44-63` sources `run.sh` and asserts `start_time_of`
under `LANG=C` equals it under `de_DE.UTF-8`. That guard covers the exact bug
ADR-0058 names and must survive in some form. The Rust equivalent asserts the
computed start-time is identical with `LANG`/`LC_ALL` set to each of `C`,
`de_DE.UTF-8` and unset, and additionally asserts agreement with the value
`lib/state.js` writes for the same process.

It must also cover an axis the shell version never had. Reading a kernel epoch value
(§1) is what removes the locale *and* timezone hazards, but the guard should prove
that rather than assume it: assert agreement across `TZ` ∈ {unset, `UTC`, a
half-hour-offset zone such as `Asia/Kolkata`, and a DST-observing zone at a
fall-back boundary}. A `ps`-parsing implementation would fail those cases; a
`p_starttime` one cannot. The Linux branch additionally pins the `CLK_TCK` source
inside a static musl binary, where `getconf` may be unavailable.

#### 5. Dead JS removal

**Files**: `lib/identity.js`, `lib/lock.js` and their tests, `lib/daemon.js`,
`PROTOCOL.md`, `skills/design/inventory-design/SKILL.md`
**Changes**: `identity.js` and `lock.js` have no production callers — they appear
only in their own tests, and `identity.test.js:70-95` and `:99-118`
cross-validate against a `launcher-helpers.sh` that no longer exists, passing
silently via `catch { return; }`. Both modules and both suites are deleted.

Three further defects get an explicit disposition rather than a mention, so they are
neither silently skipped nor fixed ad hoc mid-phase:

- `daemon.js:341-347` opens `/dev/null` and does nothing with the fd — **deleted**.
- `PROTOCOL.md:317` and `:603` are stale against `path-guard.js:74` and
  `daemon.js:341` — **corrected**.
- `makeAuthHeaderHandler` is imported at `daemon.js:11` and never called —
  **left in place**, with the dead-path defect raised as the follow-up work item
  Phase 2 §1 names. Wiring it up is new feature work and deleting it removes a
  capability `SKILL.md:89-95,196` documents as security-critical; neither belongs in
  a behaviour-preserving port. The unused *import* is not removed either, because
  removing it would make the dead path harder to find later.

`SKILL.md` is also in scope here, and was missing from the previous draft's file
list: Step 5 (`:139`) invokes `scripts/playwright/run.sh ping`, which this phase
deletes. The step is repointed at `accelerator design executor ping`. Migration Notes
claims call sites are rewired in the phase that deletes what they call, and this is
one of the two places that was not true.

`executor-ping-failed`'s message is also rewritten **here**, not in Phase 7. It
currently reads "Run `run.sh ping` manually to diagnose", naming a file this phase
deletes — so between Phase 6 and Phase 7 the plugin would ship a diagnostic whose
remediation cannot be followed, and Phase 2's byte-for-byte pin on that message
actively prevents fixing it early. That pin is therefore relaxed to the messages whose
text genuinely survives unchanged. `daemon.js`'s `chromium-not-found` message has the
same defect — "Run ensure-playwright.sh to reinstall" — and is rewritten in Phase 7
§2, alongside the path change it already needs.

#### 6. Node suite runner

**Files**: `skills/design/inventory-design/scripts/playwright/package.json`,
`skills/design/inventory-design/scripts/playwright/test-run.js`,
`skills/design/inventory-design/scripts/playwright/lib/daemon.test.js`,
`tasks/test/unit.py`, `mise.toml`
**Changes**: The retained suites have no runner anywhere. Add a
`test:unit:design-automation` task running `node --test` over them, wired into the
aggregate unit-test task, so AC1 and AC2 have CI-observable meaning.

**Scope**: `lib/*.test.js` **and** `test-run.js`, which lives at the `playwright/`
root and a `lib/`-only glob would silently exclude — while being the suite that
actually exercises the daemon end to end.

**Floor**: 8 discovered suites — 7 under `lib/` after Phase 6 deletes
`identity.test.js` and `lock.test.js` and Phase 7 deletes
`playwright-loader.test.js`, plus `test-run.js`. Asserted in the established style
(`_EXPECTED_CONFIG_SUITES` and friends), with the three deletions recorded in the
floor's comment, and the discovered list passed to `node --test` explicitly.

**A file-count floor is not sufficient here, and this is the important part.** Two
retained suites gate themselves on the bootstrap layout Phase 7 deletes:

- `test-run.js:93-94` computes
  `playwrightInstalled = existsSync(ACCELERATOR_PLAYWRIGHT_CACHE || ~/.cache/accelerator/playwright)`
  and carries `skip: !playwrightInstalled` on **all sixteen** of its tests.
- `daemon.test.js:15-27` resolves its namespace from the `package-lock.json`
  lockhash and requires `node_modules/playwright/index.js` to exist, returning `null`
  otherwise, which `:72` turns into an early `return`.

Phase 7 §8 deletes `package-lock.json` and the whole
`${ACCELERATOR_PLAYWRIGHT_CACHE}` namespace, so both gates become permanently false
— and `daemon.test.js` breaks twice over, because the retarget to `playwright-core`
means `node_modules/playwright/index.js` would not exist even if the namespace did.
These are precisely the suites asserting the envelope shapes AC2 pins: the
`bootstrap` category, `screenshot-output-root-unset`,
`screenshot-path-outside-output-root`, the protocol round-trip, daemon reuse, and the
idle and wall-clock shutdown envelopes. A count floor counts *files*, so all of this
would pass while executing nothing.

So Phase 7 repoints both gates at the resolved tree paths — the driver tree and
`ACCELERATOR_DESIGN_BROWSER_EXECUTABLE` — and `daemon.test.js`'s entry-point probe at
`playwright-core`, matching §1's retarget. And the task asserts the **executed** test
count, not just the discovered file count: a run whose skipped count is non-zero, or
whose total falls below a recorded floor, fails. That is the assertion that would have
caught this class of silent loss, and the floors mechanism alone would not.

This is also the home for the four assertions re-homed from `test-design.sh`
(Phase 8 §1): the executor-source deny-list greps, the `PROTOCOL.md` ↔ `daemon.js`
sync, `BLOCKING_OPS` containing `links`, and the `ownerPid` guard.

#### 7. Executor-path retirement

**Files**: `skills/config/browser-executor/`, `scripts/config-read-browser-executor.sh`,
`agents/browser-locator.md`, `agents/browser-analyser.md`,
`tasks/lint/call_site_migration.py`, `tests/unit/tasks/test_call_site_migration.py`
**Changes**: The `browser-executor` skill, `config-read-browser-executor.sh`, and
the `{browser-executor-script}` convention exist solely to resolve `run.sh`'s
absolute path. With `run.sh` gone they retire: both agents call
`accelerator design executor` directly at their ~40 call sites.

**One precondition must be checked before committing to that.** No agent in
`agents/` references `${CLAUDE_PLUGIN_ROOT}` or invokes `accelerator` today — the
established pattern is preload-a-skill-that-injects-resolved-values (`paths` →
`{work}`, `browser-executor` → `{browser-executor-script}`), and both agents carry an
explicit preload guard with a verbatim user-facing failure message that retires with
it. If `${CLAUDE_PLUGIN_ROOT}` is not expanded inside a subagent's Bash tool
environment, all ~40 rewritten call sites resolve to `/bin/accelerator` and every
browser agent breaks, with no guard left to produce a diagnosable message. So confirm
the expansion with a one-line manual check **before this phase is scheduled**, since
the two branches have different edit sets and discovering that mid-implementation
means rework:

| | Retire branch | Keep-a-preload-skill branch |
|---|---|---|
| `EXPECTED_INJECTION_SKILLS` | decremented | unchanged |
| `call_site_migration.py` allowlist | entry removed, fixture replaced | entry retargeted at the new script |
| `releases-and-compatibility.md:41-44` | rationale restated | unchanged |
| Manual criterion | agents work without `{browser-executor-script}` | agents work via the slimmed preload |

The keep branch retires only the `run.sh`-specific resolution and leaves a minimal
skill injecting the launcher path.

Two documentation surfaces go with it, and neither is covered by Phase 8's sweep
(scoped to "references to a deleted design script"):
`docs-site/src/content/docs/reference/agents.md`, and
`docs-site/src/content/docs/releases-and-compatibility.md:41-44`, which cites
`browser-executor` as one of the two mechanisms justifying the documented **minimum
Claude Code v2.1.144**. That rationale is restated against the mechanisms that
remain rather than left pointing at a retired skill.

`call_site_migration.py:8,33` allowlists that script as a retained shell call
site — the entry goes with it, and `test_call_site_migration.py:35` uses it as a
fixture, so that fixture is replaced.

Removing the `browser-executor` skill moves `EXPECTED_INJECTION_SKILLS`
(`tasks/lint/skill_permissions.py:48`) — it is an equality, not a floor.

#### 8. Deletion

**Files**: `run.sh`, `test-run.sh`
**Changes**: Deleted, along with `scripts/test-design.sh:542-546` (the `test-run.sh`
delegation) and `:442-485` (the `browser-executor` skill assertions this phase
retires). `:518-531`'s `browser-locator` links contract keeps its substance but
loses its `{browser-executor-script}` clause. Since `test-design.sh` runs in CI,
omitting these edits leaves `test:integration:config` red on merge.

### Success Criteria

#### Automated Verification

- [ ] Characterization tests written first for cold start, warm reuse, stale-PID
      recovery, PID-recycle rejection, lock contention and daemon-start timeout —
      derived from **`run.sh`'s source**, not from `test-run.sh`, which covers none
      of them: it contains structural and shellcheck checks, the `start_time_of`
      locale comparison, a ping/daemon-stop/links block and a
      survives-shell-exit smoke test, and everything from line 65 onward self-SKIPs
      without a real Playwright install, which CI does not have. Each is made
      deterministic by an injected port rather than by real time or real processes:
      PID-recycle via a `ProcessProbe` returning a fabricated start-time for a live
      pid, daemon-start timeout via an injected `Clock` and a `Spawner` that never
      signals ready, contention via two `Lock` holders — no sleeps
- [ ] The reuse verdict is unit-tested as a pure function over (recorded pid,
      recorded start time, observed liveness, observed start time), including the
      absent-`start_time` case now treated as a mismatch
- [ ] The start-time probe agrees across `C`, `de_DE.UTF-8` and an unset locale,
      **and** across `TZ` ∈ {unset, `UTC`, a half-hour offset, a DST fall-back
      boundary}
- [ ] The start-time probe agrees with `lib/state.js`'s value for the same process,
      with both reading the same kernel source
- [ ] `proc-stat-linux.txt` yields `1700145620`, and a fixture whose tick count does
      not divide evenly by `CLK_TCK` yields the truncated value
- [ ] `accelerator design executor daemon` is rejected by argument validation
- [ ] The daemon survives a SIGHUP to the launcher's process group, and the client
      path propagates a non-zero exit status and a signal death unchanged
- [ ] A `server-info.json` with `start_time_source: wallclock` is accepted once
      without being held to the ±1s tolerance
- [ ] Concurrent executor invocations produce exactly one daemon and the loser
      reports `another-launcher-running`, asserted for the single lock backend
- [ ] The reuse verdict table is exhaustive: a test per row, including
      `Probe`+`Unavailable` reusing on liveness alone and `AbsentOrUnparseable`
      recovering
- [ ] A `server-info.json` with no `start_time_source` key is read as `Probe`
- [ ] The launcher writes the identity record; `state.js` writes no start time, and
      no `getconf` shell-out remains on that path
- [ ] A daemon request without the token is refused, and `client.js` supplies it
- [ ] Invoking outside a repository emits the `no-repo` envelope on stderr and exits 2
- [ ] A stale `server-stopped.json` is removed before a spawn, so a failed start is
      not reported as a completed shutdown
- [ ] The bootstrap log is truncated and `0600` before the daemon writes, under a
      `umask 077`
- [ ] A warm executor invocation is no slower than today's `run.sh` path, measured
      with work-item:0186's interleaved-sample method — the per-invocation cost of the
      path a crawl takes 100–200 times, which no criterion measured before
- [ ] Launcher envelopes are byte-identical 3-key JSON on stderr, and daemon
      envelopes reach stdout at exit 0
- [ ] `mise run test:unit:design-automation` passes over the retained suites, with
      **zero skipped tests** and an executed count at or above its floor — a
      wholesale skip must fail, which a discovered-file floor alone would not catch
- [ ] The task's glob covers `test-run.js` as well as `lib/*.test.js`
- [ ] `mise run cli:check` and `mise run scripts:check` exit 0
- [ ] `mise run` exits 0

#### Manual Verification

- [ ] A live inventory crawl completes with page state preserved across
      consecutive executor commands (the identity contract holding)
- [ ] Both browser agents work end to end without `{browser-executor-script}`
- [ ] Running two executor commands concurrently produces one daemon, and the
      loser reports `another-launcher-running`

---

## Phase 7: Swap onto the bundled driver and browser

### Overview

Point the executor at launcher-resolved tree artifacts, retarget the automation
at `playwright-core`, and delete the on-machine install. Depends on Phases 4, 5
and 6.

### Changes Required

#### 1. Retarget the automation

**Files**: `lib/daemon.js`, `lib/playwright-loader.js` and its three fixture trees,
`lib/playwright-loader.test.js`
**Changes**: The assembled bundle ships `playwright-core`, not `playwright`.
`playwright-loader.js:23-67` requires `<nsRoot>/node_modules/playwright/package.json`
and deliberately throws when `exports['.']` is an object whose `.import` is not a
string (`:53-56`) — the fix for the 0072 CJS-shim bug.

`daemon.js` uses only `chromium.launch({headless:true})` (`:106`) and
`chromium.executablePath()` (`:121`), both present in `playwright-core`. So
`daemon.js` imports `playwright-core` directly, matching what Microsoft's own
bindings do, and `playwright-loader.js`, its test and its three
`fake-playwright*` fixture trees are deleted.

The 0072 regression it guarded does not recur: the bug was the loader selecting
a CJS shim entry from a `playwright` package whose `exports` map it
misinterpreted, and there is no longer a loader making that selection. A test
asserts `chromium` is a defined export of the resolved module, which is the
property 0072 actually cared about.

#### 2. Passing the browser path, and the `chromium-not-found` diagnostic

**File**: `lib/daemon.js`
**Changes**: `daemon.js:106` calls `chromium.launch({ headless: true })` with **no
`executablePath`**, so `playwright-core` resolves from its own browser registry —
exactly the mechanism both the bundled tree and the `design.browser_path` hatch must
override. Without an explicit argument the path would be resolved in Rust and then
ignored in JS, and **AC12 could not pass**. So `daemon.js` reads the resolved path
from `ACCELERATOR_DESIGN_BROWSER_EXECUTABLE` and passes it:
`chromium.launch({ headless: true, executablePath })`.

**The `ping` handler must read the same variable, not `executablePath()`.** An
earlier draft asserted that `cr.executablePath()` at `:121` "reports that same
resolved path". It does not: `BrowserType.executablePath()` is computed from
`playwright-core`'s **browser registry** — the `PLAYWRIGHT_BROWSERS_PATH` layout or
its default — and neither takes nor reflects a per-launch `executablePath` option.
With the bundled sealed tree, and a fortiori under the hatch pointing at a distro
Chromium, that registry path does not exist, so `daemon.js:123`'s
`promises.access(execPath)` throws and `ping` returns `chromium-not-found`.

That is not a cosmetic error. `ping` is the readiness probe `SKILL.md` Step 5 runs,
and its failure is the `executor-ping-failed` downgrade — so **every crawl would
degrade to the code-only crawler on exactly the machines the bundled artifacts exist
to serve**, and AC6 and AC12 would both fail, after Phases 4 and 5 have shipped
~1.2GB per release to support them. The handler therefore `access()`es and reports
the launch path. If the registry path is wanted as a secondary diagnostic it may be
reported alongside, but it never decides the outcome.

The diagnostic's own text changes too: `:120-125` reports against the **full
Chromium** path while this ships `chromium-headless-shell`, and its message says
"Run ensure-playwright.sh to reinstall", naming a script this phase deletes. It is
rewritten to name `accelerator cache repair` — the remediation that now applies.

Passing `executablePath` explicitly also resolves the sealed-tree layout risk rather
than merely mitigating it: supplying the path is what makes `playwright-core` skip
registry resolution and its validation entirely, so the browsers root of a
`0444`/`0555` tree is never consulted or written. Confirm that empirically against
the pinned `playwright-core` version — a test asserting a launch succeeds against a
read-only browsers root is the cheapest form — and if any path still writes there,
place the marker outside the tree rather than unsealing it.

#### 3. Tree resolution

**Files**: `cli/design-cli/src/executor.rs`, `cli/launcher/src/main.rs`,
`cli/launcher/src/launch/core.rs`
**Changes**: The embedded signing key keeps exactly one holder (ADR-0060), so the
launcher owns materialisation — but it must not own the *decision*, because
ADR-0057 puts the ordering and the downgrade vocabulary in the design binary. The
split is by cost:

- **Warm, on every dispatch**: for each tree `locate` resolves, the launcher exports
  `ACCELERATOR_TREE_<NAME>` — a generic name derived from the pointer files present
  on disk, not a `DESIGN`-prefixed one, so the launcher enumerates rather than knows,
  and a second tree consumer inherits the convention rather than a design-shaped
  variable. That is `locate`'s two small reads and two stats per tree (Step 4b),
  issues no network request, and has no failure mode: a tree that is absent,
  unpointed, unparseable, or failing its ownership check simply yields no variable.
  The launcher learns nothing about design's subcommands, and no dispatch path can
  fail because of a tree.

  The variables are **always set or explicitly cleared**, never merely left alone, so
  an inherited or injected value from the surrounding environment can never be
  mistaken for one the launcher resolved.
- **Cold, only when needed**: `accelerator-design` calls
  `accelerator cache ensure <name>` at the point in its own ordering where it has
  established that it needs the runtime. That is the only place a ~294MB fetch can
  be triggered, so `validate-source`, `resolve-auth`, `scrub-secrets`,
  `notify-downgrade` and `audit-cue-phrases` never touch the network, and
  `notices` reads whatever is already materialised.

An absent variable is therefore the normal state rather than an error: it means
"not materialised yet", and the executor decides whether to `ensure`, downgrade,
or proceed. That is also what makes the `ACCELERATOR_DESIGN_BIN` dev override work
— it bypasses the launcher's resolve path entirely, so the variables are never set
and the executor reaches `ensure` exactly as it would on a cold cache.

**The `ensure` contract**, since this is a machine-consumed interface between two
separately-built executables rather than a human-facing command:

- **Discovery.** `accelerator-design` must locate the launcher to invoke it, and
  `argv[0]` is its own content-addressed cache path. The launcher exports
  `ACCELERATOR_LAUNCHER_BIN` (its own resolved shim path) alongside the tree
  variables; its absence is itself a diagnosable cause, not a panic. This closes the
  dev-override case too: `ACCELERATOR_DESIGN_BIN` bypasses the resolve path, so the
  variable is unset and the executor reports `artifact-unavailable` with a cause
  naming why, rather than failing opaquely.
- **Envelope.** `ensure` emits a golden-pinned structured envelope with an enumerated
  cause set mapped 1:1 onto downgrade reasons — unreachable host, signature mismatch,
  digest mismatch, disk shortfall, unwritable cache root, platform unsupported,
  artifact absent from the manifest. The executor maps causes, never parses prose.
- **Version skew.** Against a launcher predating Phase 4, `cache` is not a built-in
  and is treated as a dispatch token, producing an `AssetNotFound` for
  `accelerator-cache-<platform>` — a distribution error that would surface instead of
  a downgrade. So an unrecognised cause, a non-zero exit with no parseable envelope,
  and a resolution error all map to `artifact-unavailable`.

Collapsing every cause into `artifact-unavailable` unconditionally would leave a 3am
failure with no diagnosis, which is why the cause set exists; mapping *unknown* causes
there is the fallback, not the default.

**A failed materialisation is sticky for the session.** A crawl makes 100–200
executor invocations, and with no negative caching a persistent failure — a full
disk, a read-only plugin root, a flapping link, a 404 for one platform — would
produce a fresh full-size attempt, times three fetch retries, on *every one* of them.
A single crawl on a failing machine could attempt tens of gigabytes and repeatedly
fill the user's disk with partial archives. This risk did not exist for
megabyte-scale single-file sub-binaries. So the first `artifact-unavailable` downgrade
suppresses re-attempts and the remaining invocations take the code-only path
immediately.

The marker lives in the executor's own state directory — the `0700` directory under
the repo's config tmp path that Phase 6 already establishes — **not** beside `trees/`.
Two of the failure causes it exists to damp are a full disk and an unwritable cache
root, so a marker written into the cache root could not be created in exactly the
cases that recur; the state dir is writable when the cache root is not. It records the
artifact name, the cause and a timestamp, and it is cleared by any successful `ensure`
and by `cache repair`, so the documented remediation is also the reset. Its TTL is
stated explicitly and derived from the crawl bound: a crawl is bounded at five
minutes, so a TTL of that order suppresses within-crawl retries without stranding the
next crawl after a user frees disk space or reconnects.

Tree-related failure envelopes also carry a remediation string naming
`accelerator cache repair <name>`. ADR-0060 accepts as a known negative that a
truncated tree "surfaces as a confusing runtime failure until the repair path is
run" — but self-healing needed no discovery, whereas this needs the user to already
know a command exists that the failure never mentions. Naming it in the failure is
what makes AC14's recovery reachable in practice rather than only documented.

The executor sets `NODE_PATH` and passes the browser executable path through to
`daemon.js` from the resolved trees, replacing the lockhash namespace. The layout
precondition that today exits 3 `playwright-not-installed` becomes an
`artifact-unavailable` downgrade rather than a hard failure, since the artifacts
are now fetchable.

#### 4. Failure ordering and the platform probe

**Files**: `cli/design/src/platform.rs` (new),
`cli/design-adapters/src/platform.rs` (new), `cli/design-cli/src/executor.rs`
**Changes**: ADR-0057 requires the runtime check to come **before**
`design.browser_path` is consulted, because the hatch substitutes the browser and
never the runtime. A musl host must reach the code-only downgrade, not a
browser-path error. Nothing enforces any such ordering today because neither
check exists.

Order: platform supported? → runtime available? → browser resolvable (bundled,
then `design.browser_path`)? Each failure emits its downgrade reason, and the
default and hybrid crawler modes fall back to the code-only crawler. An explicit
`--crawler runtime` request hard-fails.

The platform check needs a mechanism that exists nowhere in the codebase today.
`HOST_PLATFORM` (`resolve/mod.rs:21-28`) is a compile-time constant reading
`linux-x64` on Alpine and Debian alike — `TARGETS` builds Linux against
`*-unknown-linux-musl` precisely so one binary runs on every libc — and the
manifest's platform axis carries no libc dimension. Nothing in the existing
resolution path can tell the two apart, so without a probe an Alpine host fetches
~294MB of glibc-linked artifacts, seals them, and dies at `execve` with a bare
ENOENT from the absent dynamic loader: the hard failure AC11 exists to prevent, at
maximum cost.

The probe is a filesystem question with a pure answer. `cli/design/src/platform.rs`
classifies a `LibcFlavour` from the set of loader paths present; the adapter
supplies that set by globbing `/lib/ld-musl-*.so.1`, `/lib64/ld-linux-*.so.2` and
`/lib/ld-linux-*.so.*` (the last for 32-bit and non-multilib layouts). Neither
present is itself an answer — `unsupported-platform`, since no glibc loader means
no glibc driver can run. The classification is unit-tested over an injected
directory listing for the musl, glibc and neither shapes; the Alpine container
fixture confirms the wiring but cannot on its own distinguish "detected musl" from
"failed for some other reason", which is why the unit test carries the property.

The probe runs **before** any artifact resolution, so an unsupported host
downgrades at zero network cost.

#### 5. `design.browser_path`

**Files**: `cli/config/src/catalogue.rs`, `scripts/config-defaults.sh`,
`cli/launcher/tests/fixtures/dump/dump.golden`, `docs-site/…/design.md`
**Changes**: Add to `EXTRA_KEYS` (`catalogue.rs:121-133`) — no default,
presence-only, exactly like `visualiser.editor`. That costs the catalogue entry,
a mirror at `scripts/config-defaults.sh:208-220`, a row in the dump golden, and
docs. It does **not** touch `assert_eq!(count, 55)` at `catalogue.rs:267` or the
Rust↔bash drift test, which does not extract `EXTRA_KEYS`. A catalogue *default*
would cost a new group, an entry in `default_for`'s hardcoded group loop
(`:230`), a `dump::assemble` arm, two extra drift-test loops and the count bump —
so `EXTRA_KEYS` is the route.

The `ACCELERATOR_DESIGN_BROWSER_PATH` env override is **not** a config-layer
concern: `config-adapters` reads exactly one env var and `store.rs:195-205`
documents that as the rule. `cli/visualiser/server/src/compose.rs:216-252`
(`resolve_optional`) is the exact env-beats-config shape, whitespace collapse
included — but it is **extracted into a shared crate with its tests** rather than
copied verbatim. Copying logic while leaving its tests at the original site is how
two copies drift, and this precedence is the mechanism AC12 rests on; verifying it
only through a container fixture would leave the edges untested. If extraction proves
disproportionate, the fallback is explicit precedence tests at the new site over env
set/unset × config set/unset × whitespace-only, so a mutation in either copy fails
locally.

#### 6. Downgrade vocabulary

**Files**: `cli/design/src/downgrade.rs`,
`skills/design/inventory-design/evals/fixtures/notify-downgrade/*`,
`skills/design/inventory-design/evals/evals.json`,
`skills/design/inventory-design/evals/benchmark.json`,
`skills/design/inventory-design/PROTOCOL.md`,
`skills/design/inventory-design/SKILL.md`
**Changes**: Keep `executor-ping-failed`; drop `node-missing`, `node-too-old` and
`bootstrap-failed`; add `unsupported-platform` (AC11's musl case) and
`artifact-unavailable`. The messages and their golden fixtures are rewritten to
match, and the fixtures become Rust goldens beside the subcommand — exhaustive by
construction, iterating the reason enum so a variant without a golden fails, which
replaces the message-key/fixture set-equality check `test-notify-downgrade.sh`
enforced.

**`disk-floor-not-met` and `cache-unwritable` are retained**, contrary to the
previous draft's "conditions that can no longer arise". Both still arise and are now
*more* likely: a first run needs headroom for a ~294MB archive **plus** its extracted
copy — ~600MB peak, more with both trees — and the cache root's unwritability is
already modelled as `CacheRootUnavailable` in the launcher. Today
`ensure-playwright.sh` refuses up front with a named reason; dropping these would
mean a disk-full condition surfaces mid-extraction as a generic
`artifact-unavailable`, having already consumed the remaining free space. So free space is
checked *before* a fetch starts against `archive_size + uncompressed_size` summed
over every tree about to be materialised — not against the archive size alone, which
would under-reserve roughly threefold and let the check pass on a machine that then
fills mid-extraction, which is the exact condition this reason exists to catch. A
partial temp tree is removed eagerly on failure rather than left to the reaper.

Three consumers beyond `downgrade.rs` and the fixtures were missing from the previous
draft's file list, and each names retired reasons by string:

- `evals/evals.json` — eval 20, `executor-bootstrap-failure-fallback`, expects
  "the literal `bootstrap-failed` downgrade message". Retargeted onto
  `artifact-unavailable`, which now covers its scenario, rather than deleted.
- `evals/benchmark.json` — six occurrences, updated in step.
- `PROTOCOL.md:555-566` — a table mapping every retired reason to an exit code, and
  the document Phase 6 §2 defers here because exit 3 is redefined in this phase. It
  is the executor's published contract, so leaving it describing a vocabulary
  nothing emits is worse than the original drift Phase 6 §5 already fixes.

`notify-downgrade-messages.json` is **deleted** here, with the `#[cfg(test)]` drift
test Phase 2 §3 pins it by. Its content moved into the domain crate's `const` table
at that point; once the vocabulary is rewritten there is no on-disk file left to
drift against, and keeping one would mean maintaining a second copy of a table the
compiler already makes exhaustive.

`scripts/test-design.sh:154-155` asserts the `inventory-design` `allowed-tools`
entry `Bash(${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/*)` — the
residual rule this section drops — so it is rewritten in step, not left to Phase 8.

`SKILL.md` Steps 4–6 (`:117-133`) also change here: they invoke
`ensure-playwright.sh` and parse its `ACCELERATOR_DOWNGRADE_REASON=` stderr line.
With bootstrapping moved to build time there is no bootstrap step to run, so Step 4
is replaced by the executor's own ordering and the reason is read from the executor's
envelope. This is the second of the two places Migration Notes' "rewired in the phase
that deletes what they call" claim was not met. The residual
`Bash(${CLAUDE_PLUGIN_ROOT}/skills/design/**/scripts/*)` `allowed-tools` rules, kept
alive by Phase 2 §5 while these scripts existed, are dropped here too.

`regenerate-notify-downgrade-fixtures.sh` — a maintainer dev tool invoked by no
SKILL.md — is deleted with them; regeneration is a test affordance on the Rust
goldens.

#### 7. `design notices`

**File**: `cli/design-cli/src/notices.rs`
**Changes**: `accelerator design notices [--artifact driver|browser]` prints the
paths of the `NOTICES/` directories Phase 5 assembles into each tree, and lists
the components covered. This is what makes AC16's "reachable by a user without
unpacking the artifact by hand" true; it lands here rather than in Phase 5
because the trees it reads do not exist on a user's machine until this phase.

#### 8. Deletion

**Files**: `ensure-playwright.sh`, `test-ensure-playwright.sh`, `package-lock.json`,
`regenerate-notify-downgrade-fixtures.sh`
**Changes**: Deleted, along with `scripts/test-design.sh:486-490` (the
`test-ensure-playwright.sh` delegation) — which runs in CI, so omitting the edit
leaves `test:integration:config` red on merge. With them go the lockhash namespace
under
`${ACCELERATOR_PLAYWRIGHT_CACHE:-$HOME/.cache/accelerator/playwright}`, the
sentinel idempotency contract, the disk floor, the node-version floor and the
sweep.

### Success Criteria

#### Automated Verification

- [ ] Failing tests first for the failure-ordering state machine, at unit level
      over injected platform, runtime and browser resolution, so the ADR-0057
      ordering is pinned in a fast test rather than only in a container
- [ ] The libc classification returns musl, glibc and unsupported for the three
      loader-set shapes, over an injected directory listing
- [ ] An unsupported platform downgrades without issuing any HTTP request
- [ ] A non-executor design subcommand performs no tree resolution and no fetch on
      an empty cache
- [ ] With no tree variables set (the `ACCELERATOR_DESIGN_BIN` override path), the
      executor reaches `cache ensure` rather than failing
- [ ] `ensure`'s distinct failure causes map to distinct downgrade reasons
- [ ] A container fixture with Node absent from `PATH` fetches both artifacts,
      launches the headless shell, and emits the envelopes Phase 6 pinned (AC6)
- [ ] A musl/Alpine container fixture emits `unsupported-platform` and completes
      via the code-only crawler with a non-error exit — and does so with
      `design.browser_path` both set and unset (AC11)
- [ ] On a glibc host with the bundled browser unavailable and
      `design.browser_path` pointing at a system Chromium, the runtime crawler
      runs against that executable (AC12)
- [ ] `--crawler runtime` hard-fails on an unsupported platform
- [ ] Each artifact downloads at most once per platform per version (AC9)
- [ ] `chromium` is a defined export of the module `daemon.js` resolves
- [ ] `daemon.js` launches with an explicit `executablePath`, and the value it
      receives is the one Rust resolved — asserted for both the bundled tree and the
      `design.browser_path` hatch, since AC12 depends on it
- [ ] `ping` succeeds when `playwright-core`'s registry path does not exist,
      proving the handler checks the launch path rather than `executablePath()` —
      the regression that would silently degrade every crawl to code-only
- [ ] A launch succeeds against a read-only browsers root, proving an explicit
      `executablePath` bypasses registry validation and writes
- [ ] `resolve_optional`'s precedence is tested over env set/unset × config
      set/unset × whitespace-only, at whichever site owns it
- [ ] `design notices` has a success path and a failure path, including
      `--artifact`, over a fixture tree — it is one of the seven recorded
      subcommands, so AC1 applies to it
- [ ] A persistent materialisation failure produces **one** fetch attempt per
      session, not one per executor invocation
- [ ] A free-space shortfall emits `disk-floor-not-met` before any fetch starts, and
      an unwritable cache root emits `cache-unwritable`
- [ ] Tree-failure envelopes name `accelerator cache repair <name>`
- [ ] The retired reasons appear nowhere in `evals.json`, `benchmark.json` or
      `PROTOCOL.md`, and eval 20 passes against `artifact-unavailable`
- [ ] `mise run test:unit:design-automation` passes with the loader suite removed
- [ ] `mise run cli:check` exits 0
- [ ] `mise run` exits 0

#### Manual Verification

- [ ] A full inventory crawl on a machine with no system Node produces the same
      artefacts as one on a machine with Node installed
- [ ] First-run download completes within a stated wall-clock ceiling at the stated
      minimum throughput (the same floor Step 4a's deadline encodes), with host and
      connection recorded — a pass/fail bound, not an observation
- [ ] `accelerator design notices` reaches all three licence sets
- [ ] Deleting one file from a sealed tree, then running
      `accelerator cache repair`, restores a working crawl

---

## Phase 8: Removal sweep

### Overview

The residue: floors, by-name pins, documentation, and the four places
`tasks/README.md` has drifted.

### Changes Required

#### 1. Floors and the `test-design.sh` teardown

**Files**: `scripts/test-design.sh`,
`scripts/test-skill-frontmatter-conformance.sh`,
`skills/design/inventory-design/scripts/playwright/lib/*.test.js`,
`tasks/test/integration.py`
**Changes**: `test-design.sh` is not only a runner. Its blocks split three ways,
and only the first group dies with what it covers:

| Block | Lines | Disposition |
|---|---|---|
| `validate-source` behavioural + delegated suite | 169-281 | dies with the script (Phase 2) |
| `resolve-auth` behavioural | 282-315 | dies with the script (Phase 2) |
| `scrub-secrets` behavioural | 316-338 | dies with the script (Phase 2) |
| `audit-cue-phrases` behavioural | 368-425, 428-430 | dies with the script (Phase 2) |
| delegated `test-notify-downgrade.sh` | 547-551 | dies with the script (Phase 2) |
| `browser-executor` preloaded skill | 442-485 | dies with the skill (Phase 6) |
| delegated `test-run.sh` | 542-546 | dies with the launcher (Phase 6) |
| delegated `test-ensure-playwright.sh` | 486-490 | dies with the script (Phase 7) |
| `init` path keys + `DIR_COUNT` marker | 12-29 | **re-home** → frontmatter conformance |
| `configure` paths table | 31-38 | **re-home** → frontmatter conformance |
| canonical `research_design_*` call-site guard | 40-44 | **re-home** → frontmatter conformance |
| docs list `design-inventories/`, `design-gaps/` | 46-57 | **re-home** → frontmatter conformance |
| browser agents exist; `tools:` is exactly `Bash` | 59-108 | **re-home** → frontmatter conformance |
| `browser-analyser` body forbids `fetch`/`eval`/… | 109-119 | **re-home** → frontmatter conformance |
| `.mcp.json` does not exist | 131-136 | **re-home** → frontmatter conformance |
| both skills' structure | 138-168, 350-358, 365-367 | **re-home** → frontmatter conformance |
| `analyse-design-gaps` skill-instructions hook | 426-427 | **re-home** → frontmatter conformance |
| `audit-cue-phrases.sh` call site, existence, exec bit | 359-364 | **rewrite in Phase 2** → asserts the new subcommand |
| `inventory-design` `allowed-tools` `scripts/*` glob | 154-155 | **rewrite in Phase 7** → the rule it asserts is dropped there |
| both skills' `evals.json`/`benchmark.json` validity | 339-349, 431-441 | **re-home** → frontmatter conformance |
| `evaluate-payload-rejected`/`mcp__playwright__` absent from executor source | 121-129 | **re-home** → node suites |
| `PROTOCOL.md` ↔ `daemon.js` command and env-var sync | 491-510 | **re-home** → node suites |
| `links` is in `BLOCKING_OPS` | 511-517 | **re-home** → node suites |
| `ownerPid`/`--owner-pid`/`OWNER_POLL_MS` never return | 532-541 | **re-home** → node suites |
| `browser-locator` links contract | 518-531 | **re-home**, minus the `{browser-executor-script}` clause Phase 6 retires |

The target is chosen by what each assertion is *about*. Skill, agent and docs
structure goes to `scripts/test-skill-frontmatter-conformance.sh`, already a
`_REQUIRED_CONFIG_SUITES` by-name gate (`tasks/test/integration.py:63`) and so run
unconditionally. Anything asserting a property of the retained JavaScript goes to
the `test:unit:design-automation` suites Phase 6 §6 introduces, beside the code it
constrains.

Three are worth naming because losing them would be silent. The
`evaluate-payload-rejected` and `mcp__playwright__` guards constrain `lib/` and
`run.js` — both **retained** — so they are unrelated to this migration and need
only a new home. The `ownerPid` guard encodes a resolved incident (see
`meta/notes/2026-05-19-playwright-daemon-owner-pid-ephemeral-shell.md`). And the
canonical `research_design_*` guard greps the very SKILL.md and agent files this
plan rewrites, making it more valuable during this work than before it.

Only once every re-home has landed is `scripts/test-design.sh` deleted. That takes
`scripts/` from 16 discovered suites to 15, and `_EXPECTED_CONFIG_SUITES`
(`tasks/test/integration.py:41`) is **already 15** — so it stays at 15, no edit,
which is the floor doing its job at full strength. Its docstring (`:77-90`) says the
floor exists to catch "an exec bit dropped … or a suite renamed off the
`test-*.sh` convention"; every unit of headroom is one suite that can leave CI
silently, so headroom is the blind spot rather than the safety margin. AC4's
lockstep requirement is satisfied here by leaving one number alone and asserting it
still matches.

No `skills/design/` floor exists to decrement. The new `test:unit:design-automation`
task gains a floor of its own in the established style, because Phase 6 deletes two
of its suites and Phase 7 a third — without a count assertion a renamed suite would
remove the AC1/AC2 coverage the task exists to provide while it still exits 0.

#### 2. Documentation

**Files**: 14 `docs-site/src/content/docs/` pages, `README.md`, `CHANGELOG.md`,
`.claude-plugin/plugin.json`, `docs-site/src/content/docs/reference/agents.md`,
`docs-site/src/content/docs/releases-and-compatibility.md`
**Changes**: Every reference to a deleted design script is repointed at its
subcommand. `plugin.json:11` declares the `Node >= 20` requirement this work removes —
it goes.

Two behaviour changes need an explicit note rather than a silent repoint.
`validate-source`'s reachability rewrite (Phase 2 §1) **narrows** the accepted set: 
`::ffff:10.0.0.1`, `::ffff:169.254.169.254`, `fc00::/7` and `100.64.0.0/10` addresses,
`0:0:0:0:0:0:0:1`, IPv4 forms with a non-first octal octet, and the 6to4/Teredo/NAT64
encodings all now exit 1 without `--allow-internal`, where several passed before. A
maintainer inventorying an app on a CGNAT or unique-local address would otherwise read
that as their setup breaking rather than the tool's classification tightening, so
`CHANGELOG.md` and the new design page name the classes and name `--allow-internal` as
the route. And `scrub-secrets`/`audit-cue-phrases` now split usage error onto exit 2.

`releases-and-compatibility.md:41-44` cites `browser-executor` as one of two mechanisms
justifying the documented minimum Claude Code **v2.1.144**; Phase 6 §7 restates that
rationale against the mechanisms that remain, and this sweep verifies no page still
describes the retired resolver.

#### 3. ADR and work-item amendments

**Files**: `meta/decisions/ADR-0060-launcher-resolved-tree-artifacts.md`,
`meta/work/0196-accelerator-design-inventory-gap-tooling-cli.md`
**Changes**: The plan states twice that ADR-0060 needs an amendment and no phase owned
it, so it lands here. ADR-0060 says tree entries are "addressed by release version and
digest"; Phase 4 addresses them by digest, platform and generation, with the version in
a pointer. The amendment records three things the ADR does not currently contemplate:
that addressing is content-based with a per-release pointer; that this introduces
cross-version tree **adoption**, which is why the layout carries a format version; and
that the pointer is a deliberately-unsigned local indirection whose compromise sits
inside the same-uid threat model the ADR already accepts.

work-item:0196's Requirements bullet restating the version+digest addressing is
corrected in step, so the work item and the ADR do not describe a scheme the code does
not implement. Per ADR immutability, an accepted ADR is amended by superseding note
rather than edited in place — `/accelerator:review-adr` is the route.

#### 4. Registration checklist drift

**File**: `tasks/README.md`
**Changes**: The checklist has drifted in four places, and
`tests/unit/tasks/test_registration_docs.py` enforces its shape, so fixing it is
part of the work:

- The `assert len(uploads) == 22` count no longer exists — it is derived.
- `_setup_release` now loops the registry rather than being single-token.
- "the visualiser is the worked example" is stale; all six tokens have entries.
- Point 7 no longer describes only `!`-preprocessor commands — fenced blocks in
  numbered steps count too (`dispatch_coherence.py:9-13,95`).

Add the undocumented per-token edit the research found: `_SUBBINARY_DESCRIPTIONS`
(`tests/integration/tasks/test_github.py:35-46`) KeyErrors without an entry.

#### 5. Final state assertion

**File**: `tests/unit/tasks/test_call_site_migration.py`
**Changes**: Assert `skills/design/` contains no `.sh` file, so a future
reintroduction is caught.

#### 6. Follow-up work items

**Changes**: Three defects surfaced during this work are deliberately **not** fixed
here, because each is a behaviour change rather than a migration, and this plan's
premise is behaviour preservation. Raise them so they are carried rather than lost:

- **The header-auth path is dead.** `makeAuthHeaderHandler` is imported at
  `daemon.js:11` and never called, and `ACCELERATOR_BROWSER_LOCATION_ORIGIN` is set
  nowhere — while `SKILL.md:89-95,196` documents its origin allowlist as
  security-critical. Users are told to place real bearer tokens in a browser-driving
  daemon's environment for a feature that never applies them, and an authenticated
  crawl silently produces an unauthenticated inventory. The follow-up decides between
  wiring it up (with an origin-allowlist test) and retiring `resolve-auth`, the
  `ACCELERATOR_BROWSER_*` variables and their scrub rules together.
- **The daemon has no request authentication.** It serves JSON commands on
  `127.0.0.1` with an OS-assigned port and no auth (`daemon.js:286-338`); loopback
  binding is not a uid boundary, so any local process can drive a browser that may
  hold the user's authenticated session. A random per-daemon token recorded in the
  already-`0700` `server-info.json` and required as a header would close it cheaply.
- **Navigation URLs are unclassified.** `validate-source` hardens the initial
  location, but `daemon.js:165-167` calls `page.goto(req.url)` on whatever each request
  supplies, so an attacker-influenced page or a redirect can steer a crawl at an
  internal endpoint. The follow-up plumbs the `AccessPolicy` verdict — including
  `--allow-internal` — through the executor into `navigate` and the `links`
  same-origin decision, reusing the `host_reach` code this plan writes.
- **The abandoned legacy Playwright cache.** `cache prune` (Step 4c) reclaims
  unreferenced tree generations, but the pre-migration
  `${ACCELERATOR_PLAYWRIGHT_CACHE:-$HOME/.cache/accelerator/playwright}/<sha8>`
  namespaces — one per historical lockfile hash, each holding a full Chromium — sit
  outside anything this plan manages, and the sweep `ensure-playwright.sh` performed
  dies with it. Migration Notes names the path and states it is safe to delete; the
  follow-up decides whether `prune` should also report or remove it, which is a
  destructive action against a path this tooling no longer owns.

### Success Criteria

#### Automated Verification

- [ ] Failing test first: every re-homed assertion from the §1 table is present in
      its new suite **and shown to fail when its property is broken**, verified
      *before* `test-design.sh` is deleted
- [ ] `mise run test:integration:config` passes with `_EXPECTED_CONFIG_SUITES`
      unchanged at 15, and `scripts/test-design.sh` is not among the discovered
      suites — `_require_suite_floor` is an at-least check
      (`tasks/test/integration.py:77-103`), so "exactly 15" is not assertable through
      it without a new equality guard, and the absence check is what this phase
      actually needs
- [ ] `mise run test:unit:design-automation` passes at its own floor
- [ ] `mise run test:unit:build-system` passes, including
      `test_registration_docs.py`
- [ ] `mise run lint:scripts:exec-bits:check` exits 0
- [ ] `mise run docs:check` exits 0
- [ ] No `.sh` file remains under `skills/design/`
- [ ] `mise run` exits 0

#### Manual Verification

- [ ] The docs site builds and every design page's links resolve
- [ ] A fresh plugin install with no system Node completes an inventory run

---

## Testing Strategy

### Unit Tests

- Domain logic in `cli/design/` with no I/O: host canonicalisation across the
  numeric-encoding rejections, the internal-range classification matrix, the auth
  precedence table, the bidi and non-ASCII message filters, and H2 sectioning
  against the cue-phrase patterns.
- Tree materialisation in `cli/launcher/` against synthetic tarballs: rejection
  before extraction, the entry-type allowlist's full rejection set, attestation and
  table round-trip, a crash injected at each of steps 4-10, single-flight with a
  failing winner, pointer validation, `verify`'s detection of each corruption shape,
  and repair's new-generation swap against a live reader.
- Platform classification in `cli/design/` over injected loader-path sets, so
  AC11's musl case is pinned without a container.
- Upstream verification in `tasks/` against recorded fixtures, never live network.
  Node/GPG is verified for real against the committed key; only the SLSA
  transparency-log call is injected, and its two branches are asserted rather than
  its content.

### Integration Tests

- Repointed suites derived from the four deleted bash suites, invoking the binary
  rather than the scripts, with the exit-code split asserted explicitly.
- End-to-end resolution against a `MockServer` and a real minisign keypair,
  following `cli/launcher/tests/resolution.rs:41-199`.
- Container fixtures: Node-absent glibc (AC6), musl/Alpine (AC11), and
  bundled-browser-unavailable with `design.browser_path` set (AC12). These are a
  `test:integration:design-containers` task modelled on the repo's only container
  precedent, `docker_visual_command` (`tasks/test/e2e.py:34-125`), which specifies
  image, platform, mounts, networking and a docker-reachability preflight — the
  previous draft asserted the fixtures as automated verification while naming no
  image, no task, no CI job and no artifact source. Concretely: `debian:12-slim` with
  Node absent from `PATH` for AC6, `alpine:3` for AC11, and `debian:12-slim` with a
  distro Chromium for AC12; `--platform linux/amd64` pinned so the musl case is not
  masked by emulation differences.

  The artifacts under test are **built in the same run and served from a
  `MockServer`**, with `ACCELERATOR_CACHE_DIR` pointed inside the container (a
  container-private path, not a host bind-mount, so the ownership checks in Step 4b
  hold and two platforms cannot share a root). They
  cannot come from the real release host: `artifacts` entries will not exist there
  until a release built *after* this work merges, so a fixture pointed at production
  would either skip or fail for the wrong reason. The launcher and
  `accelerator-design` are cross-compiled for the container's target as part of the
  task, musl for AC11 — the launcher with `--features test-trust-root` (Step 4b §4),
  since a stock build trusts only the real release key and could not verify anything
  the task signs.

  **CI lane**: the task runs in its own job, not in the aggregate `mise run`, matching
  how `test:e2e:visualiser:docker` is deliberately outside `check` and the default
  `test` roll-ups. That keeps Docker off the critical path for developers without it,
  and the job carries a `docker info` preflight that **fails** rather than skips, so an
  absent daemon is visible instead of turning AC6/AC11/AC12 into silent no-ops. A
  workflow test pins the job's existence, the way `test_workflows.py` already pins
  workflow shape.

### Manual Testing Steps

1. Run both design skills end to end in a live session after Phase 2.
2. Run an inventory crawl of a multi-route app after Phase 6 and confirm page
   state survives consecutive executor commands.
3. Repeat on a machine with no system Node after Phase 7.
4. Corrupt a file in a sealed tree, confirm the failure mode, then repair it.
5. Time a warm executor invocation before and after Phase 4 and confirm no
   regression against work-item:0186's bootstrap target.

## Performance Considerations

Two budgets are load-bearing and both are addressed by construction rather than
by tuning.

**The warm path.** Work-item:0186 took warm bootstrap from 125ms to ~30ms.
Per-exec re-verification of a 294MB artifact set would spend 16–33 seconds per
crawl re-hashing immutable bytes and put every invocation six times over that
target, which is why ADR-0060 exempts trees. The hit path is therefore two small
local reads plus two stats — a pointer and a fixed-size attestation — and loads no
manifest, which also keeps a populated cache working offline. A manifest-comparing
hit would instead cost two HTTPS GETs plus a signature verification on each of a
crawl's 100–200 invocations; Phase 4 asserts zero HTTP requests on a hit rather than
measuring its way to that conclusion.

The per-entry file table is deliberately **not** on that path. Folding it into the
attestation would put ~490 rows through a parser on every dispatch — and because the
launcher exports tree paths on every dispatch, that cost would be charged to
`accelerator vcs guard` (a PreToolUse hook) and every SessionStart hook, not only to
design. Splitting the two sidecars keeps the hit path's cost independent of how many
files an artifact contains, so a future larger artifact does not silently regress a
budget this plan polices to 1ms elsewhere. `verify` and `repair` are the table's only
readers and neither is on a hot path.

**Launcher binary size.** `bin/accelerator:352-354` minisign-verifies the whole
launcher on every warm start, so the `tar` + `flate2` addition is a per-invocation
latency term, not just disk — and it is charged to every sub-binary and every hook, not
only to design. Step 4b §1 derives the budget and the asserted ceiling, including why
the per-MB slope is measured rather than back-derived from work-item:0186's
non-method-comparable figure; it is not restated here.

Every performance criterion in this plan is a gate with a stated method or threshold,
and the warm-path gates reuse 0186's interleaved-sample method rather than inventing
one — Phase 4 for the launcher-size term, Phase 6 for the ported executor, Phase 7 for
the tree-export path. Measuring only Phase 4 would have left the two configurations
where the cost is actually paid unmeasured.

**First run.** ~294MB per platform. On the default cache root — inside the
versioned plugin tree — a plugin upgrade discards it, and this plugin pre-releases
often. `ACCELERATOR_CACHE_DIR` is the escape, and content-addressed naming is what
makes it actually work: the driver and browser change only when the pinned
`playwright-core` changes, so an upgrade that leaves the pin alone resolves the
same digest and hits. Version-keyed directory names would have made the escape
hatch cosmetic.

**The release job.** It runs the whole pipeline twice per stable release, so
roughly 2.4GB of assembly and upload, on a `macos-latest` runner with no
`timeout-minutes` and no disk guard, and `dist/release/` is not cleaned between
the two passes. Phase 5 adds both guards, and removes the duplication itself: with
the pins fixed, the second pass reuses the first's archives by digest instead of
re-assembling identical bytes, and an unchanged pin triple across releases skips the
upstream fetch entirely.

## Migration Notes

Existing installs carry a populated
`${ACCELERATOR_PLAYWRIGHT_CACHE:-$HOME/.cache/accelerator/playwright}/<sha8>`
namespace that nothing will read after Phase 7. It lives outside the plugin tree
so plugin pruning will not reclaim it. Phase 7's documentation names the path and
states it is safe to delete; no automated removal is added, consistent with not
building destructive-op UX where the filesystem makes recovery trivial.

Skills and agents are rewired inside the phases that delete what they call, so no
intermediate state has a call site pointing at a missing script. Two places did not
meet that claim in an earlier draft and now do: `inventory-design/SKILL.md`'s Step 5
`run.sh ping` call is repointed in Phase 6 §5, and its Steps 4–6
`ensure-playwright.sh` bootstrap-and-downgrade protocol is replaced in Phase 7 §6,
which also drops the residual `scripts/*` `allowed-tools` rules. The same applies to
user-facing remediation text: `executor-ping-failed` stops naming `run.sh` in the
phase that deletes `run.sh`, and `chromium-not-found` stops naming
`ensure-playwright.sh` in the phase that deletes it.

## References

- Original work item: `meta/work/0196-accelerator-design-inventory-gap-tooling-cli.md`
- Research: `meta/research/codebase/2026-08-11-0196-design-cli-implementation-surface.md`
- Prior research: `meta/research/codebase/2026-08-10-0196-accelerator-design-inventory-gap-tooling-cli.md`
- ADR-0057 (browser automation as a glibc-only capability), ADR-0058 (shell-free
  CLI-to-Node delegation), ADR-0059 (build-time assembly of vendored browser
  artifacts), ADR-0060 (launcher-resolved tree artifacts) — **ADR-0060 needs an
  amendment**: it states tree entries are addressed by release version and digest,
  and Phase 4 addresses them by digest alone with a per-release pointer. ADR-0059
  needs none — committing the npm key fills a gap it left open, and committing the
  Chromium hash is what makes its "reviewable bytes" claim true.
- Sub-binary template: `meta/plans/2026-08-06-0195-accelerator-corpus-adr-metadata-frontmatter-linkage-cli.md`
- Release-pipeline template: `meta/plans/2026-07-06-0165-multi-binary-distribution-and-release-pipeline.md`
- Registration checklist: `tasks/README.md:322-474`
