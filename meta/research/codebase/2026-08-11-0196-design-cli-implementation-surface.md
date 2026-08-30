---
type: "codebase-research"
id: "2026-08-11-0196-design-cli-implementation-surface"
title: "Research: accelerator-design implementation surface for planning 0196"
date: "2026-08-11T11:39:14+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0196"
parent: "work-item:0196"
topic: "Implementation surface for the accelerator-design sub-binary, tree artifacts, and the release-pipeline assembly step"
tags: ["research", "codebase", "design", "cli", "playwright", "launcher", "release-pipeline", "tree-artifacts"]
revision: "14c96c74fc3821dbc37e68a4df9be6939fc8ea41"
repository: "accelerator"
last_updated: "2026-08-11T11:39:14+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: accelerator-design implementation surface for planning 0196

**Date**: 2026-08-11 11:39 UTC
**Author**: Toby Clemson
**Git Commit**: `14c96c74fc3821dbc37e68a4df9be6939fc8ea41`
**Branch**: workspace `ticket-management` (working commit, not pushed — references below are local `file:line`)
**Repository**: accelerator

## Research Question

What does the codebase actually look like where work-item:0196 lands, so an
implementation plan can be written against evidence rather than against the
work item's prose?

## Summary

The work item is plan-ready on the **decision** axis — ADR-0057 through
ADR-0060 close every conflict the previous research raised. It is not yet
plan-ready on the **sequencing** axis, and six findings below change what a
plan should say.

The item is six loosely-coupled workstreams wearing one story's clothes. Two
of them (the sub-binary itself; the `corpus metadata derive` flag) are small
and independently mergeable today. Two (launcher tree artifacts; release
assembly) are shared infrastructure that three sibling items also touch, and
carry all the genuine risk. One (the `run.sh` → Rust port) depends on the
tree-artifact work landing first. One (skill rewiring and deletion) depends on
everything.

The single biggest correction to the item's own account: **the design shell
test suites are not wired into CI at all**. `scripts/test-design.sh` — the only
thing that runs four of the design bash suites — is referenced by no mise task,
no invoke task and no workflow. The eleven `node --test` suites under
`scripts/playwright/` have no runner anywhere. AC4's "affected suite floors
decremented in lockstep" turns out to be one number, and it is already sitting
exactly at its floor.

Two acceptance criteria describe behaviour that **does not exist today** rather
than behaviour to be preserved: AC11 (musl downgrade) has no musl detection to
port — `ensure-playwright.sh`'s platform guard matches `linux-musl` happily —
and the degrade-vs-hard-fail distinction AC11/AC12 assume lives entirely in
`SKILL.md` prose, not in any script.

## Detailed Findings

### 1. The subcommand mapping — AC1's precondition, ready to record

AC1 makes recording this mapping in Drafting Notes a precondition of
implementation. Here is the evidence to record it from. Nine shell scripts
exist across the two skills, plus the Playwright launcher:

| Script | Lines | Args / flags | stdout | Exit codes |
|---|---|---|---|---|
| `inventory-design/scripts/validate-source.sh` | 306 | `<location>`, `--allow-internal`, `--allow-insecure-scheme`, `--` | *(nothing)* | 0 accept / 1 reject / 2 usage |
| `inventory-design/scripts/resolve-auth.sh` | 68 | none (4 env vars) | one word: `header`\|`form`\|`none` | 0 / 1 partial-config |
| `inventory-design/scripts/scrub-secrets.sh` | 47 | `<file>` | *(nothing)* | 0 clean / **1 conflated** |
| `inventory-design/scripts/notify-downgrade.sh` | 46 | `--reason <enum>`, `--from`/`--to` (discarded) | one ASCII line | 0 / 1 bidi / 2 usage |
| `inventory-design/scripts/ensure-playwright.sh` | 367 | none (7 env vars) | progress preamble | 0,1,2,**10–15** |
| `inventory-design/scripts/inventory-metadata.sh` | 37 | none | 4 labelled lines | 0 (see §7) |
| `inventory-design/scripts/regenerate-notify-downgrade-fixtures.sh` | 18 | none | `Written: …` per key | 0 |
| `analyse-design-gaps/scripts/audit-cue-phrases.sh` | 105 | `<file>` | *(nothing)* | 0 pass / **1 conflated** |
| `analyse-design-gaps/scripts/gap-metadata.sh` | 37 | none | 4 labelled lines | 0 |
| `inventory-design/scripts/playwright/run.sh` | 203 | `"$@"` forwarded verbatim | one JSON line | 0,1,2,3 |

A defensible subcommand set, with the reasoning:

```
accelerator design validate-source <location> [--allow-internal] [--allow-insecure-scheme]
accelerator design resolve-auth
accelerator design scrub-secrets <file>
accelerator design notify-downgrade --reason <enum> [--from <mode>] [--to <mode>]
accelerator design audit-cue-phrases <file>
accelerator design executor <command> [json-args]      # replaces playwright/run.sh
accelerator design repair                              # AC14, if not launcher-owned
```

Four scripts are **not** in it, each for a different reason:

- `inventory-metadata.sh` and `gap-metadata.sh` are deleted, not ported —
  AC15 routes both to `corpus metadata derive` (§7).
- `ensure-playwright.sh` is deleted with no replacement subcommand — its whole
  job (npm ci + `npx playwright install chromium`) is what ADR-0059 moves to
  build time. What survives of it is the *downgrade-reason vocabulary*, which
  moves into the executor path and the platform guard.
- `regenerate-notify-downgrade-fixtures.sh` is a maintainer dev tool invoked
  by no SKILL.md. It regenerates
  `evals/fixtures/notify-downgrade/*.expected.txt`. Either keep it as a script
  (it drives the new binary instead) or fold it into a `--regenerate` test
  affordance. Worth an explicit decision; it is currently invisible.

**Two exit-code conflations to resolve deliberately.** `scrub-secrets.sh` and
`audit-cue-phrases.sh` both return `1` for usage error, missing file, *and* a
genuine detection (`scrub-secrets.sh:17,22,44`; `audit-cue-phrases.sh:23,28,36,102`).
`validate-source.sh` and `notify-downgrade.sh` already separate usage (2) from
domain failure (1). A `kernel::Error::Refusal` → exit 2 mapping would split the
conflated pair — which is a *behaviour change* the repointed suites will catch.
Decide it in the plan rather than at the keyboard.

**Three path-relative data files** must survive relocation into a binary:
`notify-downgrade-messages.json` (script-dir-relative,
`notify-downgrade.sh:5`), the playwright `package.json`/`package-lock.json`
(hashed into the cache namespace, `ensure-playwright.sh:48-49,59`), and
`scripts/extract-work-items-cue-phrases.txt` (a four-level `..` climb,
`audit-cue-phrases.sh:32`). The first and third want `include_str!`; the second
disappears with the lockhash namespace.

### 2. The design test suites are not in CI — AC4's floor story is one number

`scripts/test-design.sh` (~550 lines) is the sole entrypoint for four bash
suites under `skills/design/**`:

- `:278` → `inventory-design/scripts/test-validate-source.sh`
- `:487` → `inventory-design/scripts/test-ensure-playwright.sh`
- `:543` → `inventory-design/scripts/playwright/test-run.sh`
- `:548` → `inventory-design/scripts/test-notify-downgrade.sh`

It is referenced **only from `meta/` history documents** — not `mise.toml`, not
any `tasks/` invoke task, not `.github/workflows/main.yml`. The eleven
`node --test` suites (`scripts/playwright/lib/*.test.js` plus `test-run.js`)
have no runner at all: `scripts/playwright/package.json` declares no `scripts`
block, and nothing invokes `node --test`.

The floors mechanism (`tasks/test/integration.py:77-103`, `_require_suite_floor`)
globs subtrees for executable `test-*.sh`. **No guard globs `skills/design/`.**
The only floor touching design is `_EXPECTED_CONFIG_SUITES = 15`
(`tasks/test/integration.py:41`), because `scripts/test-design.sh` is one of
16 discovered suites under `scripts/`.

Consequences for AC4:

- Deleting the four `skills/design` suites needs **no** floor change.
- Deleting `scripts/test-design.sh` takes `scripts/` 16 → 15, which is exactly
  the floor, so CI would still pass without a decrement — with zero headroom.
  The honest move is 15 → 14 with a comment in the established style.
- The node suites carry no floor to decrement.

Two by-name pins **will** break and need lockstep edits:

- `tests/unit/tasks/test_exec_bits.py:275-278` — `_DUAL_USE_SCRIPTS` pins
  `skills/design/inventory-design/scripts/validate-source.sh` by literal path.
- `scripts/test-skill-frontmatter-conformance.sh:102-103` — the `EMITTERS`
  array names both design `SKILL.md` files. This script is the
  `_REQUIRED_CONFIG_SUITES` by-name gate (`tasks/test/integration.py:63`), so it
  runs unconditionally.

Also: `EXPECTED_INJECTION_SKILLS = 42` (`tasks/lint/skill_permissions.py:48`)
is an **equality**, not a floor. Both design skills inject context, so it moves
only if the *skills* go — script deletion alone leaves it untouched.

### 3. Launcher tree artifacts — what exists, and the four things that genuinely don't

The resolver is `cli/launcher/src/launch/outbound/resolve/`. Reusable verbatim:

| Asset | Location |
|---|---|
| Trust root (single key holder, as ADR-0060 requires) | `keys.rs:55-57` `TrustedKeys::embedded()` |
| Signature verify over any `&[u8]` | `verifier.rs:29-50`, `:57-67` |
| Manifest fetch + verify + schema gate | `mod.rs:116-135` (already `pub`) |
| Schema/version gates — **need no change** | `manifest.rs:77-110` |
| Cache root selection + write/exec probe | `cache_root.rs:48-64`, `:88-99` |
| Temp-name discipline (`.tmp-` + pid + `AtomicU64`) | `cache.rs:16,37-39` |
| Error → exit-code plumbing (exhaustive match) | `core.rs:167-193`, `main.rs:200-209` |
| Test scaffolding (MockServer, real minisign keygen) | `tests/common/mod.rs`, `tests/resolution.rs:41-199` |

ADR-0060's "additive under `schema_version: 1`" is confirmed by construction:
`manifest.rs:1-3` states "Unknown additive fields are ignored", the schema has
no top-level `additionalProperties: false`, and there is a dedicated test at
`manifest.rs:223-231` feeding `"future_field": 42`. The gate rejects only
strictly-greater versions (`manifest.rs:82-89`). No flag day, as the ADR says.

**Genuinely new, in rough order of risk:**

1. **Streaming download.** `fetcher.rs:147-150` does
   `response.bytes().map(|body| body.to_vec())` — the whole body buffered,
   twice, transiently. `cache::store` takes `bytes: &[u8]`. A 294MB fetch is
   not a tuning problem here, it is a redesign of the fetch signature. Worse:
   `TOTAL_TIMEOUT = 300s` **per attempt** (`fetcher.rs:15`), sized in its own
   comment for "a multi-MB release binary". 294MB inside one attempt needs
   ~1MB/s sustained.
2. **Archive extraction.** No `tar`, `zip`, `flate2`, `zstd` or `xz` crate
   exists anywhere in the workspace (`cli/Cargo.toml:13-131`). A new dependency
   means: workspace pin with justification, `cli/deny.toml` review, four-target
   cross-compile including musl-static, **and a launcher binary-size
   increase** — which is a per-invocation latency term, because `bin/accelerator:352-354`
   minisign-verifies the whole launcher on every warm start and
   `cli/Cargo.toml:149-151` documents size as exactly that.
3. **Directory atomic rename.** `cache.rs:118-133` is file-only.
   `rename(2)` over an existing non-empty directory is `ENOTEMPTY`. The
   "single syscall" property holds only because the version+digest name makes
   the target fresh — the pre-existing-target case needs an explicit decision.
4. **Recursive read-only sealing, the sentinel, temp-tree cleanup, and
   concurrency.** `cache.rs:130` removes a single temp file; a failed
   extraction leaves a temp *tree* and nothing reaps orphans. Two concurrent
   300MB extractions are a real cost concern. Note ADR-0058 explicitly says the
   launcher's lock shares **no** on-disk contract with `scripts/atomic-common.sh`
   — do not reach for the `owner.<nonce>` sentinel protocol here.

**Seams to edit**, with the one trap: `cache::find`'s prefix scan
(`cache.rs:51-73`) will *see* a directory named `{name}-{version}-{sha}` in the
same root and only rejects it because no `.minisig` sidecar exists. Use a
distinct subdirectory or stem, and never name the tree's sentinel `*.minisig`.
Also `cache.rs:56` aborts the entire scan on one non-UTF-8 entry — keep new
on-disk names ASCII.

The repair path (AC14) needs a new built-in `Command` variant
(`inbound/cli.rs:16-29`), a `dispatch` arm, its own composition in `main.rs`,
**and** an entry in `BUILTIN_SUBCOMMANDS` (`tasks/shared/dispatch_coherence.py:41`)
— which makes that name unavailable as a dispatch token forever.

Do **not** route trees through `ResolveBinary::resolve` (`mod.rs:180-233`):
that method's per-exec re-verify is precisely what ADR-0060 exempts them from,
and its contract is name → executable path for `exec`.

### 4. The release pipeline — where assembly plugs in, and four guards that will trip

Sequencing: a new `build.assemble_tree_artifacts` belongs **after**
`build.cli_cross_compile` and **before** `build.create_debug_archives`, in both
`prerelease_prepare` (`tasks/release.py:117-129`) and `release_prepare`
(`:144-160`). It must be in `prepare`, not `sign` — `_sign`
(`tasks/release.py:86-100`) is the only function holding the secret, and
`.github/workflows/main.yml:494-499` scopes `ACCELERATOR_RELEASE_SECRET_KEY` to
Sign steps deliberately. Do not put npm/nodejs.org/CDN fetches inside it.

Manifest side: add `collect_artifact_entries()` mirroring
`tasks/manifest.py:80-107` and a second key in `build_manifest`
(`:110-129`). **Do not bump `SCHEMA_VERSION`** (`tasks/manifest.py:23`).

**Four guards that will trip:**

1. `_assert_staged_manifest_is_current` (`tasks/release.py:57-83`) compares
   only `set(staged["binaries"])` against `DISPATCHED_SUBBINARIES`. Without an
   artifact equivalent, a stale artifact manifest passes silently.
2. `test_attest_globs_cover_every_published_asset`
   (`tests/unit/tasks/test_workflows.py:207-221`) imports
   `tasks.github._release_uploads()` and asserts every path matches an
   `attest-build-provenance` `subject-path` glob. Globs are `@actions/glob`
   semantics — **`*` does not cross `/`**. Keep archives **flat** in
   `dist/release/` (e.g. `accelerator-driver-<platform>.tar.zst`) so the
   existing `dist/release/accelerator-*` already matches. A nested staging tree
   silently misses.
3. `test_every_attest_block_declares_the_same_subjects` (`:198-204`) — all
   three blocks (`main.yml:502-508`, `:615-621`, `:639-645`) must stay
   identical.
4. `tests/unit/tasks/test_manifest_contract.py:30-48` iterates `binaries` only;
   artifacts get no coverage unless a parallel arm is added.

**Two operational facts the item does not record.** The `release` job runs the
whole pipeline **twice** in one job — stable, then the post-stable pre.0 cut
(`main.yml:604-650`) — so ~2.4GB of assembly and upload per stable release,
with **no `timeout-minutes` and no disk guard** on that job, on a
`macos-latest` runner. And `dist/release/` is never cleaned between the two
(`tasks/release.py:60-62`). `--clobber` on retry (`tasks/github.py:318-319`)
re-uploads the lot.

**Nothing to reuse for the upstream verification.** There is no HTTP helper in
`tasks/` at all — every existing fetch delegates to `npm`/`cargo`/`rustup`/`gh`,
and `requests`/`httpx` are not dependencies. No GPG/OpenPGP code exists. No
npm-registry-signature or SLSA verification exists (`gh attestation verify`
appears only as a manual user step in `RELEASING.md:271-281`). AC13 is three
new verification implementations, not three new calls.

**The blocker is still live.** The work item's Dependencies list "confirmation
that the release-artifact hosting infrastructure can accommodate both
per-platform tree artifacts" as a blocker. I found no evidence anywhere in the
repo that this has been confirmed, and there is no size guard, disk guard or
recorded artifact-size measurement to confirm it against. Current release is
62 assets; total size is not recorded anywhere. This gates workstream D.

One more constraint the item does not name: the launcher's redirect
**allowlist is `github.com` + `*.githubusercontent.com` only**
(`fetcher.rs:17-18,31-33`). Hosting the trees anywhere else changes the
launcher's trust surface.

### 5. The `run.sh` → Rust port — a precise spec, and five behaviours to decide about

The behavioural spec is now fully mapped. The load-bearing parts:

- **Start-time identity**, `run.sh:13-42`. Linux: `sed -E 's/.*\) //'`
  (greedy, so a `comm` with parens is safe) → `awk '{print $20}'` → `btime +
  ticks/hz`, with `getconf CLK_TCK` failure fatal. Darwin: `LANG=C LC_ALL=C ps
  -p <pid> -o lstart=` → `tr -s ' '` → `date -j -f "%a %b %d %H:%M:%S %Y"`
  parsed in **local** time. Tolerance ±1s (`run.sh:52-59`), empty expected ⇒
  match. The daemon's mirror is `lib/state.js:40-61`. Both agree because all
  three read local time consistently. Fixture-pinned:
  `lib/__fixtures__/proc-stat-linux.txt` → `1700145620`.
- **Locale regression guard**: `test-run.sh:44-63` sources `run.sh` and asserts
  `start_time_of` under `LANG=C` equals it under `de_DE.UTF-8`. This test must
  survive the port in some form — it is the guard on the exact bug ADR-0058
  names.
- **`state.js:60` falls back to `Math.floor(Date.now()/1000)` silently** on any
  failure. That value can exceed the ±1s tolerance, making the daemon
  permanently unreusable rather than failing loudly. Worth fixing in the port,
  or at least knowing about.

**Five existing behaviours a faithful port must decide about, not merely copy:**

1. **`run.sh` exits 0 for daemon-side errors.** `client.js:41,47` always
   resolves; error envelopes land on **stdout** at exit 0. Launcher-level
   failures go to **stderr** with non-zero. `SKILL.md:142-143` discriminates on
   exactly that asymmetry. Collapsing it breaks the skill.
2. **`run.sh`'s own envelopes are 3-key** (`error`, `message`, `category`) —
   no `protocol`, no `retryable`, unlike everything `errors.js:10` produces.
   AC2's byte-identical envelope assertion pins this asymmetry.
3. **Missing `jq` silently disables PID-identity verification.**
   `run.sh:108,142` extract `.start_time` via `jq`; failure is swallowed by
   `|| true`, and `start_time_matches` returns 0 for an empty expected value
   (`run.sh:54`). Any live PID is then accepted. The port removes `jq`, so this
   branch simply disappears — a silent behaviour improvement worth recording.
4. **The flock FD is inherited by the daemon** (`run.sh:126` + `169-171`), so
   the launcher lock is held for the daemon's whole lifetime. The mkdir lock
   dir leaks on `exec` paths (`test-run.sh:178-182` calls it "a pre-existing
   run.sh quirk") and there is no stale-lock recovery.
5. **`run.sh daemon` is a reachable footgun** — the internal subcommand is not
   filtered, and starts a second foreground daemon that never returns.

**Dead code that should not be ported.** `lib/identity.js` and `lib/lock.js`
have **no production callers** — grep confirms they appear only in their own
tests. Worse, `identity.test.js:70-95` and `:99-118` cross-validate against
`launcher-helpers.sh`, which no longer exists, and pass silently via
`catch { return; }`. `makeAuthHeaderHandler` is imported at `daemon.js:11` and
never called. `daemon.js:341-347` opens `/dev/null` and does nothing with the
fd. `PROTOCOL.md:317` and `:603` are stale against `path-guard.js:74` and
`daemon.js:341`.

### 6. The layout Open Question — evidence for the choice

`playwright-loader.js:23-67` requires `<nsRoot>/node_modules/playwright/package.json`,
then selects an entry: `pkg.exports['.'].import` if it is a string, else
`pkg.main`, else `index.mjs` — and **throws** if `exports['.']` is an object
whose `.import` is not a string (`:53-56`), deliberately, because falling back
to `pkg.main` loads the CJS shim and leaves `chromium` undefined. That throw
is the fix for the 0072 bug and is fixture-pinned three ways
(`lib/__fixtures__/fake-playwright*/`).

The assembled bundle ships `playwright-core`, not `playwright`. Three routes:

- **(a) Shim layout** — synthesise a `node_modules/playwright/package.json` +
  entry re-exporting `playwright-core`. Zero JS changes, but it fabricates a
  package tree and the shim's own `exports` shape has to satisfy the loader's
  three-branch selection.
- **(b) Loader change** — teach `playwright-loader.js` to resolve
  `playwright-core` when `playwright` is absent. Smallest diff, and the loader
  already has a fixture-driven test harness for exactly this decision tree.
- **(c) Retarget `lib/*.js` at `playwright-core`** — what Microsoft's own
  bindings do (their driver bundle ships `playwright-core` and they load
  `playwright-core/cli.js`).

`daemon.js` uses only `chromium.launch({headless:true})` (`:106`) and
`chromium.executablePath()` (`:121`), both present in `playwright-core`. On the
evidence, **(b) is the smallest correct move and (c) is the most honest**;
(a) fabricates a lie about the tree's contents to satisfy a loader we control.

Related, and needing a decision in the same breath: `daemon.js:120-125` resolves
`executablePath()` and reports `chromium-not-found` against the **full
Chromium** path, while the item ships `chromium-headless-shell`. That
diagnostic needs revisiting, as the item notes.

### 7. `corpus metadata derive --filename-timestamp-format` — AC15 is nearly free, with one catch

Everything needed already exists end to end. `FilenameTimestampFormat::CompactTime`
(`cli/corpus/src/metadata.rs:5-11`) renders `{Y}-{m}-{d}-{H}{M}{S}`
(`corpus-adapters/src/metadata.rs:86-88`) with label `Timestamp For Filename`
(`:94-100`) — digit-for-digit `inventory-metadata.sh:11`'s
`'+%Y-%m-%d-%H%M%S'`, both host-local. Labels, order, separator and the two
conditional-line predicates all coincide with `render` (`:249-261`). No
`corpus`/`corpus-adapters`/`pup.ron` change is needed; the variant is currently
unreachable only because `main.rs:77,82` hardcodes `DateTimeUnderscored`.

The change is four files: a CLI-local `ValueEnum` mirror in
`cli/corpus-cli/src/cli.rs` (the domain crate cannot derive `ValueEnum` —
`cli/pup.ron:40-56` restricts its imports), a `From` impl, the two hardcoded
constants in `main.rs:72-86`, and the golden harness in
`tests/metadata_goldens.rs:31` which hardcodes `["metadata","derive"]`.

**The catch for "byte-for-byte for a fixed clock":** `derive_at` constructs its
own `SystemClock` internally (`corpus-adapters/src/metadata.rs:235`). **There
is no clock seam reachable through the compiled binary**, and
`Hermetic::apply` does not set `TZ`. A fixed-clock byte-for-byte assertion can
only live at the `derive`/`render` level with a `FakeClock`
(`corpus-adapters/tests/metadata.rs:20-38`). The binary-level golden can pin
shape only. AC15 as written ("reproduces … byte-for-byte for a fixed clock and
VCS fixture") is satisfiable — but at the adapter level, not the binary level.
Say so in the plan.

**Five fixture divergences** where bash and corpus genuinely differ:
`jj log -r @` snapshots the working copy while `jj_revision` never does;
the bash branches gate on `command -v jj`/`git` while corpus needs neither;
`git rev-parse HEAD` on an unborn HEAD aborts the whole script under `set -e`
while corpus omits one line and exits 0; **outside a repository the bash script
exits 1** (the trailing `[ -n "$REPO_NAME" ] && echo` is the last command)
while corpus exits 0; and neither reads its two timestamps atomically.

### 8. Config, registration, and the mechanical surface

**`design.browser_path`** is cheapest in `EXTRA_KEYS`
(`cli/config/src/catalogue.rs:121-133`) — no default, presence-only, exactly
like `visualiser.editor`. That costs: the catalogue entry, a mirror in
`scripts/config-defaults.sh:208-220`, a row in
`cli/launcher/tests/fixtures/dump/dump.golden`, and docs. It does **not**
touch the `assert_eq!(count, 55)` at `catalogue.rs:267` or the Rust↔bash drift
test (which does not extract `EXTRA_KEYS`). A catalogue *default* would cost a
new group, an entry in `default_for`'s hardcoded group loop (`:230`), a
`dump::assemble` arm, the bash pair, two extra loops in the drift test, and the
count bump. Take the cheap route.

The env override is **not** a config-layer concern. `config-adapters` reads
exactly one env var, and `store.rs:195-205` documents that as the rule. Copy
`cli/visualiser/server/src/compose.rs:216-252` (`resolve_optional`) verbatim —
it is the exact env-var-beats-config-key shape, whitespace collapse included.

**Sub-binary registration.** The launcher needs **no code change** — any
unknown subcommand hits `#[command(external_subcommand)]`
(`cli/launcher/src/launch/inbound/cli.rs:26-28`). The thirteen-point checklist
is `tasks/README.md:322-474`, guarded by
`tests/unit/tasks/test_registration_docs.py`. Points 1, 2, 3, 4, 7 and 8 must
land in one change (`README:449-452`). The two gates that catch you early are
`lint:dispatch-coherence:check` (token ↔ skill binding — the `Bash(...)` rule's
token segment must be **exactly** `design`, and any over-broad rule in the same
frontmatter disqualifies the whole file as a witness) and
`test_registration_docs.py`. Everything tagged `[release]` or `[author]` does not.

**The checklist itself has drifted in four places** — the `== 22` upload count
no longer exists (it is derived), `_setup_release` now loops the registry, the
"visualiser is the worked example" framing is stale (all six tokens have
entries), and point 7 no longer describes only `!`-preprocessor commands
(fenced blocks in numbered steps count too, `dispatch_coherence.py:9-13,95`).
An undocumented per-token edit also exists: `_SUBBINARY_DESCRIPTIONS`
(`tests/integration/tasks/test_github.py:35-46`) KeyErrors without it. Fixing
the README is part of the work, because the guard test enforces its shape.

**The pup gotcha is live**: `allowed_only` rules reject grouped imports
(cargo-pup resolves `use a::{b, c}` to an empty module name), so a `design`
domain crate needs one single-item `use` per import. `cli/pup.ron:132-138`
states it; `cli/pup.ron:57-107` has the corpus/vcs/work precedent to copy.

### 9. Skill rewiring and the browser-executor retirement

`allowed-tools` today:

- `inventory-design/SKILL.md:11-14` — `config *`, `scripts/*`, `scripts/playwright/*`
- `analyse-design-gaps/SKILL.md:10-12` — `config *`, `scripts/*`
- `skills/config/browser-executor/SKILL.md:9` — `scripts/config-read-browser-executor.sh`

The executor-path coupling to collapse: `scripts/config-read-browser-executor.sh:17`
hard-codes the `run.sh` path; `agents/browser-locator.md:9` and
`agents/browser-analyser.md:9` preload `accelerator:browser-executor` and use
`{browser-executor-script}` at ~40 call sites between them. Also
`tasks/lint/call_site_migration.py:8,33` allowlists that script as a retained
shell call site — the allowlist entry goes with it, and
`tests/unit/tasks/test_call_site_migration.py:35` uses it as a fixture.

Fourteen docs-site pages plus root `README.md` and `CHANGELOG.md` reference the
design scripts. `.claude-plugin/plugin.json:11` declares the `Node >= 20`
requirement that this work removes.

## Code References

- `skills/design/inventory-design/scripts/playwright/run.sh:13-42` — start-time identity, both platforms
- `skills/design/inventory-design/scripts/playwright/run.sh:52-59` — ±1s tolerance, empty-expected short-circuit
- `skills/design/inventory-design/scripts/playwright/run.sh:101-121` — pre-lock reuse short-circuit
- `skills/design/inventory-design/scripts/playwright/run.sh:125-137` — flock/mkdir lock dichotomy
- `skills/design/inventory-design/scripts/playwright/run.sh:163-194` — daemon spawn, 30s poll, kill-on-timeout
- `skills/design/inventory-design/scripts/playwright/lib/state.js:40-61` — the daemon's mirror of the identity computation
- `skills/design/inventory-design/scripts/playwright/lib/playwright-loader.js:23-67` — entry selection and the deliberate throw
- `skills/design/inventory-design/scripts/playwright/lib/daemon.js:101-137` — headless launch, `chromium-not-found`
- `skills/design/inventory-design/scripts/ensure-playwright.sh:128-158` — the downgrade-reason emission sites
- `cli/launcher/src/launch/outbound/resolve/manifest.rs:26-46` — `Manifest`/`BinaryEntry`, the shape `artifacts` sits beside
- `cli/launcher/src/launch/outbound/resolve/manifest.rs:77-110` — the two-pass schema gate (needs no change)
- `cli/launcher/src/launch/outbound/resolve/mod.rs:137-177` — `fetch_verify_store`, the model for tree resolution
- `cli/launcher/src/launch/outbound/resolve/mod.rs:180-233` — the per-exec re-verify trees are exempt from
- `cli/launcher/src/launch/outbound/resolve/fetcher.rs:147-150` — the whole-body buffering that must become streaming
- `cli/launcher/src/launch/outbound/resolve/cache.rs:51-73` — the prefix scan that will see a sibling directory
- `tasks/release.py:117-160` — the two prepare tasks assembly plugs into
- `tasks/manifest.py:110-129` — `build_manifest`, the dict gaining `artifacts`
- `tasks/release.py:57-83` — the staged-manifest currency guard that needs an artifact arm
- `tests/unit/tasks/test_workflows.py:198-221` — the two attest-glob guards
- `tasks/test/integration.py:41,77-103` — `_EXPECTED_CONFIG_SUITES` and the floor enforcer
- `tasks/shared/dispatch_coherence.py:84-112` — the token ↔ skill binding detector
- `cli/corpus-adapters/src/metadata.rs:86-88,94-100,249-261` — `CompactTime`, its label, and `render`
- `cli/corpus-adapters/src/metadata.rs:235` — `derive_at` building its own clock (no binary-level seam)
- `cli/visualiser/server/src/compose.rs:216-252` — the env-beats-config idiom to copy
- `cli/pup.ron:40-56,132-138` — the domain import rule and the grouped-import gotcha

## Architecture Insights

**Three artifact classes will coexist in one resolver.** Single-file
sub-binaries (per-exec re-verify, `{name}-{version}-{sha256}` naming,
self-healing, ETXTBSY-safe in-place replacement) and tree artifacts
(verify-archive-once, atomic directory rename, read-only seal plus adjacent
sentinel, no per-exec re-verify, explicit repair). ADR-0060 calls the
divergence "a documented difference rather than an oversight" — which means it
must actually be documented in `resolve/`, not merely true.

**The failure ordering is load-bearing.** ADR-0057 requires runtime (glibc
Node) availability to be checked **before** `design.browser_path` is consulted,
because the hatch substitutes the browser and never the runtime. A musl host
must reach the code-only downgrade, not a browser-path error. Nothing in the
current code enforces any such ordering because neither check exists.

**Policy lives in prompts, not scripts, and the item moves some of it.** The
crawler-mode decision, the degrade-vs-hard-fail branch, `sequence` computation,
the whole report body, and the supersede sweep are all `SKILL.md` instructions
with no script behind them. The binary owns exit codes, the four metadata
lines (moving to corpus), single-word/single-line stdout, and the
`ACCELERATOR_DOWNGRADE_REASON=` stderr line. AC2's scoping — "the
model-authored report is not in scope" — is correct and now evidenced. But
AC11/AC12 push *new* policy into the binary, which is a scope addition rather
than a port.

**`ensure-playwright.sh` disappearing takes more than an install step with it.**
The lockhash namespace, the sentinel idempotency contract, the disk floor, the
node-version floor, the sweep, and four of the six downgrade reasons
(`node-missing`, `node-too-old`, `disk-floor-not-met`, `cache-unwritable`,
`bootstrap-failed`) are all its. Under ADR-0059 most become meaningless — but
`notify-downgrade-messages.json`'s six keys and their six golden fixtures are
a shipped contract that needs an explicit disposition.

## Historical Context

- `meta/decisions/ADR-0057…ADR-0060` (all accepted, 2026-08-10/11) — the four
  decisions that made this item plan-ready. ADR-0060 is the newest and
  supersedes the manifest-shape and per-exec questions ADR-0059 left open.
- `meta/research/codebase/2026-08-10-0196-…md` — the prior research that
  falsified the determinism assumption and drove AC2/AC6's retargeting.
- `meta/plans/2026-08-06-0195-…md` — the corpus sub-binary plan, executed and
  validated. The closest available template for workstreams A and B.
- `meta/plans/2026-07-06-0165-…md` — the last plan to touch `tasks/release.py`,
  manifest emission and minisign. The template for workstream D.
- `meta/reviews/work/0196-…-review-2.md` — records which findings stay open by
  explicit reviewer choice (licensing-as-assumption, launcher-infrastructure
  bundling, kind-as-story).
- `meta/notes/2026-05-19-playwright-daemon-owner-pid-ephemeral-shell.md` — the
  owner-pid design that was removed; `scripts/test-design.sh:511-543` still
  greps to assert `ownerPid`/`--owner-pid`/`OWNER_POLL_MS` never return.

## Related Research

- `meta/research/codebase/2026-08-06-0195-accelerator-corpus-cli-implementation-surface.md`
- `meta/research/codebase/2026-08-08-0197-accelerator-collaboration-pr-helper-cli.md`
- `meta/research/codebase/2026-07-06-0165-multi-binary-distribution-release-pipeline.md`
- `meta/research/codebase/2026-08-02-0187-generalise-sub-binary-registration-surface.md`
- `meta/research/codebase/2026-05-18-0072-playwright-daemon-cjs-import-bug.md`

## Open Questions

1. **Should this be one plan?** Six workstreams, of which two are independently
   mergeable today (`corpus metadata derive` + metadata-script deletion; the
   non-Playwright design subcommands) and two are shared infrastructure that
   0195/0197 also touch. A single plan means one very large PR on
   `resolve/` and `tasks/release.py` — precisely the surface the item's own
   Coordination note asks siblings to sync on.
2. **Is the hosting-capacity blocker confirmed?** Nothing in the repo records
   it, and there is no size measurement to confirm against. It gates the
   release-assembly workstream.
3. **What happens to the six `notify-downgrade` reasons** once
   `ensure-playwright.sh` is gone? Four of them describe conditions that can no
   longer arise. The messages JSON and its six golden fixtures are a shipped
   contract needing an explicit disposition.
4. **Layout**: route (b) loader change vs (c) retarget `lib/*.js` at
   `playwright-core`. Evidence favours (c) as what upstream does, (b) as the
   smaller diff. Route (a) shim is workable but fabricates a package tree.
5. **Exit-code conflation** in `scrub-secrets` and `audit-cue-phrases`: preserve
   the conflated `1`, or split usage into `2` as a deliberate behaviour change?
6. **`regenerate-notify-downgrade-fixtures.sh`** — port, keep as a script
   driving the new binary, or delete with the fixtures?
7. **Do the design test suites get wired into CI as part of this work**, or is
   their absence accepted and the suites deleted outright? Repointing suites
   that never run gives AC1 and AC2 no CI-observable meaning.
