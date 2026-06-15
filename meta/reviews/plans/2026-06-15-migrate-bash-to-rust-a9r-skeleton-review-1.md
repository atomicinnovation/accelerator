---
type: plan-review
id: "2026-06-15-migrate-bash-to-rust-a9r-skeleton-review-1"
title: "Plan Review: a9r Migration: Walking Skeleton + config-read Family"
date: "2026-06-15T15:33:17+00:00"
author: "Phil Helm"
producer: review-plan
status: complete
target: "plan:2026-06-15-migrate-bash-to-rust-a9r-skeleton"
reviewer: "Phil Helm"
verdict: "REVISE"
lenses: [architecture, code-quality, test-coverage, correctness, security, compatibility, portability, safety]
review_number: 1
review_pass: 2
tags: [bash, rust, a9r, cli, migration, visualiser, build-system, workspace]
last_updated: "2026-06-15T16:08:27+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

## Plan Review: a9r Migration: Walking Skeleton + config-read Family

**Verdict:** REVISE

This is a disciplined, risk-aware migration plan: it sequences a behaviour-free
restructure first, proves a cheap well-tested vertical behind a bash fallback,
defers the risky rename to last, and treats the existing test suite as a
cross-language parity contract. The architecture (functional-core `a9r-core`,
clap subcommand seam, retained fallback) is sound and the byte-for-byte traps are
unusually well-catalogued. However, two critical issues and a cluster of major
ones must be addressed before implementation: the parity gate as specified
**cannot actually prove byte-for-byte equivalence** (command-substitution capture
masks trailing-newline divergence — the dominant trap, feeding prompt injection),
and the binary **rename creates a cross-version download-asset skew** with no
dual-publish window. Compounding these, the new binary-resolution path widens
arbitrary-code-execution exposure to every skill load, the eager blocking
SessionStart hook lacks a timeout contract, and several subtle bash behaviours are
missing from the trap list.

### Cross-Cutting Themes

- **The parity gate does not bite where it matters most** (flagged by:
  test-coverage, correctness, safety) — Most `test-config.sh` assertions capture
  output via command substitution (`$(...)`), which strips trailing newlines, so a
  port emitting zero/one/many trailing newlines passes identically. Nothing asserts
  the `A9R_BIN` run actually executed (the count floor counts files, not modes), and
  guarded exclusions can make the gate vacuously green. Since this stdout is injected
  verbatim into prompts via the `!` preprocessor, a "green" gate can still corrupt
  rendered context. This is the spine of the plan's correctness claim and currently
  the weakest link.

- **The eager blocking SessionStart hook is under-specified** (flagged by:
  architecture, portability, safety) — A new fourth SessionStart hook synchronously
  downloads + SHA-verifies the binary before any skill loads, moving a network call
  (curl/wget, egress to github.com) onto every cold session's critical path. No
  explicit timeout/backgrounding contract is given; a slow/captive/hung network
  stalls session start rather than degrading fast. The three existing SessionStart
  hooks are local-only and fast.

- **Binary resolution widens the trust/execution surface** (flagged by: security,
  correctness, compatibility, safety, architecture) — `a9r_bin()` resolves
  env → team-committed config key (`visualiser.binary`) → cached binary and `exec`s
  it on nearly every skill load. The config key travels with a cloned/PR'd repo
  (auto-RCE vector); the plan's "pure resolution (no download)" drops the launcher's
  per-exec SHA re-verification (cache-tamper/TOCTOU); a present-but-buggy binary is
  not caught at runtime (fallback only triggers on *absence*); and resolution
  precedence is duplicated across three files.

- **Duplicated logic without a single source of truth** (flagged by: architecture,
  code-quality) — Subtle config-parsing semantics are duplicated across bash awk and
  Rust `a9r-core` (kept in lockstep only by the CI gate), *and* `a9r-core` introduces
  a second frontmatter/config implementation alongside the visualiser lib's existing
  `frontmatter`/`config` modules, inside one binary, with no stated ownership.

- **Transitional `--config` alias fights clap** (flagged by: compatibility,
  architecture, code-quality) — Accepting a bare `--config` as shorthand for
  `visualise --config` via "clap default subcommand or arg detection" is ambiguous;
  clap derive has no native default subcommand, and a top-level flag mixed with a
  subcommand enum risks parse conflicts with the `ConfigRead*` positional grammar.

- **Triplicated `../frontend/dist` literal** (flagged by: architecture,
  code-quality, portability, safety) — Hard-duplicated across `build.rs:5` and
  `assets.rs:9,71`, relative to the server crate manifest; constrains the Phase 6
  directory rename. All four lenses endorse the plan's safe default (rename package,
  keep `server/` dir) and several suggest hoisting to one shared const.

### Tradeoff Analysis

- **Security (re-verify SHA on every shim call) vs Performance/Architecture
  (single-digit-ms spawn)**: Security wants the hot-path shim to re-hash the cached
  binary against `checksums.json` before every `exec` (matching the launcher).
  Re-hashing on every config-read adds latency to the path the plan touts as
  *faster* than bash. Recommendation: rely on the eager-hook-verified cache as the
  trust anchor, reject the cache if mtime/hash changed since verification, and only
  full-re-hash when that cheap check fails — rather than hashing on every call or
  not at all.

- **Eager-blocking provisioning (reliability, binary ready before first use) vs
  lazy acquisition (no session-start latency)**: The shim already degrades to bash
  on a miss, which weakens the case for eager-blocking. Recommendation: a fast
  cache-valid fast-path as the first gate, a hard timeout, and consider
  non-blocking/background acquisition since fallback covers the cold path.

### Findings

#### Critical

- 🔴 **Test Coverage**: Parity gate cannot detect trailing-newline divergence — the dominant byte-for-byte trap
  **Location**: Phase 3: Cross-language parity gate / Testing Strategy
  Nearly every call site captures output via command substitution
  (`OUTPUT=$(... bash "$READ_PATH" ...)`) then `assert_eq`s it; command substitution
  strips all trailing newlines, so a port emitting zero/one/three trailing newlines
  passes identically. The research flags this as caveat 2, yet the only check is a
  single manual-verification bullet. This stdout is injected verbatim into prompts.

- 🔴 **Compatibility**: Binary rename creates a cross-version download-asset skew with no dual-publish window
  **Location**: Phase 6, Section 3: Distribution/paths cleanup (and Phase 5)
  The launcher derives the download URL and asset name from the *installed* plugin
  version (`${RELEASES_URL_BASE}/v${PLUGIN_VERSION}/accelerator-visualiser-...`).
  Phase 6 renames the artifact to `a9r-<platform>` but specifies no release shipping
  *both* names. A plugin requesting `a9r-<platform>` against a release that only
  published `accelerator-visualiser-<platform>` (or the reverse) gets a 404; the
  visualiser launch path has **no** fallback.

#### Major

- 🟡 **Test Coverage**: No automated guard that the `A9R_BIN` run actually executed
  **Location**: Phase 3, Section 2
  The count floor (`_EXPECTED_CONFIG_SUITES = 16`) counts suite *files*, not modes.
  If the `A9R_BIN` step silently no-ops (typo, ordering, empty path falling through
  to bash), CI stays green while testing bash twice — the worst failure mode for a
  safety net.

- 🟡 **Test Coverage**: `run_sut` reroute must preserve heterogeneous redirection forms, not just trailing args
  **Location**: Phase 3, Sections 1–2
  Call sites are not uniform: bare command-substitution, inline redirections
  (`2>/dev/null`, `2>&1 1>/dev/null`), helper-form trailing `"$@"`
  (`assert_exit_code ... bash "$READ_REVIEW"`), and nested `bash -c`. A `run_sut`
  that `exec`s breaks the helper-form sites; a naive replace misses the `bash -c`
  form. The stderr-isolation tests are the most error-prone and most important.

- 🟡 **Test Coverage**: "config-read is already well-covered" is asserted, not measured
  **Location**: What We're NOT Doing / Phase 7 (Decision 6)
  The gate only catches divergence where an assertion already pins behaviour. Traps
  named in Key Discoveries (empty-vs-omitted default, one-layer quote strip,
  last-file-wins-from-second-file-only, UTF-8, CRLF) become whatever `a9r` does if
  no fixture exercises them. Apply the JIT-backfill discipline to slice 1 too.

- 🟡 **Test Coverage**: Excluding the SKILL.md grep-assertions under `A9R_BIN` is unjustified
  **Location**: Phase 3, Section 2 / Key Discoveries
  Those tests (L4398-4452) assert literal strings appear in SKILL.md files — they do
  not invoke the SUT and remain valid in `a9r` mode (skills still call the `.sh`
  shim). Only the genuinely sourced-function tests need the guard.

- 🟡 **Correctness**: Within-section first-match-wins precedence is not enumerated alongside last-file-wins
  **Location**: Key Discoveries / Phase 2 §1 `lookup()`
  `_read_from_file`'s awk `exit`s on the first matching subkey inside a section, so a
  duplicate key in one file resolves to the *first* occurrence, while across files
  the *last* file wins — two orthogonal axes. A HashMap/last-write-wins `lookup()`
  would invert same-file duplicate resolution silently.

- 🟡 **Correctness**: Empty-but-closed frontmatter is treated as not-found, distinct from unclosed
  **Location**: Phase 2 §1 `extract_frontmatter()`
  `[ -z "$fm" ] && return 1` (`config-read-value.sh:54`): a file containing exactly
  `---\n---` is silent not-found, whereas unclosed frontmatter returns 1 *with* a
  stderr warning. Conflating them would emit a spurious warning, violating
  clean-stderr-on-success.

- 🟡 **Correctness**: Unclosed-frontmatter warning gate uses a looser regex than the parser
  **Location**: Key Discoveries / Phase 2 §1
  The parser opens on `^---[[:space:]]*$` (exact); the warning gate re-reads with
  `head -1 | grep -q '^---'` (unanchored, fires on `---foo`/`----`). The two `^---`
  checks must be reproduced with their exact differing anchoring or stderr diverges.

- 🟡 **Correctness**: Unknown path key with no explicit default still performs a config lookup
  **Location**: Phase 2 §1 `read_path()` / Key Discoveries
  For an unknown key with no default, bash warns to stderr AND still execs
  `config-read-value.sh "paths.${key}" ""` — so a user who set `paths.unknownkey`
  gets that value, not empty. A `read_path` that returns empty immediately for
  non-table keys ignores a configured value bash honours.

- 🟡 **Correctness**: Concurrent acquisition / TOCTOU between hook, launcher, and shim is unaddressed
  **Location**: Phase 5 §3 / Phase 4 §1
  The eager hook may be mid-download (writing the cache) while a shim resolves and
  `exec`s the cache path. A shim that checks-then-execs a partially-written binary is
  a TOCTOU (ENOEXEC / wrong SHA), intermittently breaking a skill load. Require
  atomic rename into the final path.

- 🟡 **Architecture**: Frontmatter/config logic duplicated between `a9r-core` and the existing visualiser lib
  **Location**: Phase 2 §1–2 / Desired End State
  The visualiser lib already has `frontmatter`/`config` modules; `a9r-core` adds a
  parallel implementation, and the `a9r` binary depends on both. Two cohabiting
  implementations of the same domain concept drift; name `a9r-core` as the single
  owner or document why the visualiser's parser is intentionally separate.

- 🟡 **Architecture**: Hot-path config commands transitively link the entire axum/SPA visualiser lib
  **Location**: Phase 2 (a9r depends on visualiser lib) / Decision 1
  Every `config-read-*` invocation ships and loads a binary carrying the whole
  server dependency closure. This was an accepted Decision-1 consequence; ensure
  `a9r-core` carries **zero** dependency on the visualiser lib so the logic boundary
  stays clean and a later split stays possible.

- 🟡 **Architecture**: New eager blocking SessionStart hook adds a network-dependent step before every session
  **Location**: Phase 5 §3
  Specify an explicit short timeout and a fast cache-valid fast-path; consider lazy
  acquisition inside the shim since the shim already degrades to bash.

- 🟡 **Code Quality**: Duplicated config-parsing semantics across bash and Rust with only a CI gate to keep them in lockstep
  **Location**: What We're NOT Doing / Phase 2 §1 / Phase 4
  Two independent implementations of fragile parsing logic in different languages
  must be edited in lockstep for every future change, with only a prose "traps" list
  as spec. Codify the traps as a shared fixture table the parity suite iterates, and
  cross-reference both sites in comments.

- 🟡 **Code Quality**: Shim `_fallback` duplicates the full bash implementation in-file, risking drift
  **Location**: Phase 4 §2
  The plan offers "`_fallback()` function (or sibling `*-impl.sh`)" interchangeably.
  Commit to the sibling `*-impl.sh` so the entry script stays a uniform ~5-line shim,
  applied identically across all Phase 7 ports; confirm the `config-read-path.sh`
  `exec` chaining survives the wrapping.

- 🟡 **Code Quality**: Mechanical rewrite of 58 test call sites with conditional guards reduces readability
  **Location**: Phase 3 §2
  Sprinkling raw `if [ -z "${A9R_BIN:-}" ]` guards through a ~5992-line suite adds a
  cross-cutting "which mode am I in?" concern and silent skips. Centralise into a
  `skip_unless_bash_mode "reason"` helper that logs the SKIP so exclusions are
  accounted for in the summary.

- 🟡 **Security**: Team-committed config key becomes an automatic load-time arbitrary-binary-execution vector
  **Location**: Phase 4 §1 (`scripts/a9r-resolve.sh`)
  `a9r_bin()` resolves `visualiser.binary` from team-committed `.accelerator/config.md`
  and the shim `exec`s the result on nearly every skill load. A malicious `config.md`
  (`visualiser: { binary: ./evil }`) achieves auto-RCE on a merely-cloned repo.
  Restrict the config-key branch on the hot path to gitignored `config.local.md`, or
  exclude it from the automatic shim path; require SHA verification before exec.

- 🟡 **Security**: Shim resolution skips per-invocation SHA re-verification of the cached binary
  **Location**: Phase 4 §1 + Phase 5 §1
  `a9r_bin()` is "pure resolution (no download)" and resolves the cached binary by an
  executable check; the launcher by contrast re-computes SHA before exec. A binary
  tampered after acquisition executes unverified on every skill load.

- 🟡 **Security**: New eager binary download omits SLSA provenance verification for the a9r artifact
  **Location**: Phase 5 §3
  The integrity root for an auto-executed, blocking-at-session-start binary is a
  `checksums.json` in the same trust domain as the artifact. Verify SLSA provenance
  where available and treat `ACCELERATOR_VISUALISER_RELEASES_URL` overrides as
  untrusted (HTTPS-pinned) on the eager path.

- 🟡 **Compatibility**: Transitional bare `--config` alias collides with clap subcommand parsing
  **Location**: Phase 6 §1
  clap derive has no native default subcommand; mixing a top-level `--config` with a
  `Subcommand` enum risks conflicts with the `ConfigReadValue { key, default }`
  positional grammar. An old launcher calling `"$BIN" --config "$CFG"` against a new
  `a9r` would fail to start the server. Prefer a symlink/wrapper that prepends
  `visualise`, or explicit argv pre-processing with a test asserting equivalence.

- 🟡 **Compatibility**: `checksums.json` `a9r` keys before a release ships the asset — visualiser path has no fallback
  **Location**: Phase 5 §2 / Phase 6 §3
  The launcher `die`s on the all-zeros sentinel and on version drift. The "degrades
  silently" guarantee holds only for config-read; the visualiser launch path has no
  bash fallback. Sequence the `a9r` checksum key to land in lockstep with (not
  before) a release publishing the asset.

- 🟡 **Compatibility**: Launcher fake-binary tests invoke the old `--config` form
  **Location**: Phase 6 §1
  `test-launch-server.sh` drives a `make_fake_visualiser` fake invoked as
  `"$BIN" --config` against an asset literally named `accelerator-visualiser-...`.
  Phase 6's changes list omits updating these fakes/fixtures; the suite will break
  and the `visualiser.binary`/env-override contract it guards is exactly what must
  keep working.

- 🟡 **Portability**: Eager blocking SessionStart hook adds a network + tool dependency to every fresh session
  **Location**: Phase 5 §3
  Moves a github.com download (curl/wget) onto every cold session, including
  air-gapped/proxied/tool-less hosts. Specify tight connect/total timeouts, confirm
  fast short-circuit on offline/proxy/missing-tool, and document the egress + downloader
  prerequisite.

- 🟡 **Portability**: New shim, resolve, and provision shell must observe the bash 3.2 floor — never stated
  **Location**: Phase 4 §1 / Phase 5 §3
  New shell on macOS bash 3.2. A bash-4 construct (assoc array for the platform map,
  `${var,,}` case-folding) passes on CI's bash 5.x but fails on 3.2. State the floor
  in success criteria; lower-case via `tr`, not `${var,,}`.

- 🟡 **Safety**: "Every phase ends green on `mise run check`" does not cover the parity gate
  **Location**: Implementation Approach + each phase's Success Criteria
  `mise run check` runs format + lint only — *not* tests. The parity gate (Phase 3)
  and acquisition tests (Phase 5), the real correctness guarantees, live under the
  `test`/`test:integration` aggregates reached only by the bare `mise run` default.
  State the merge gate per phase as the full `mise run` (or the specific test tasks),
  and block merges on the twice-run parity suite.

- 🟡 **Safety**: Blocking SessionStart download lacks an explicit timeout/backgrounding contract
  **Location**: Phase 5 §3
  `download_to` uses `curl --retry 3` with no `--max-time`/`--connect-timeout`. A
  slow-but-alive network or captive portal stalls every session start before fallback
  triggers. Give a hard wall-clock timeout, guarantee exit 0 on timeout, add a
  timeout-degrades case to the acquisition tests.

- 🟡 **Safety**: "Independently mergeable" is only true for the forward sequence
  **Location**: Implementation Approach + Phase 4 vs Phases 2–3
  Phase 4 shims behave correctly only because Phases 2–3 produced + proved the
  binary; fallback triggers on *absence*, not on a present-but-buggy binary. Reframe
  as "sequentially mergeable, forward-only" and add the invariant that the shim must
  never route to a binary whose SHA is not in the verified manifest.

#### Minor

- 🔵 **Correctness**: Legacy-layout guard vs usage-error check ordering differs between the two subcommands
  **Location**: Phase 2 §2
  `config-read-value` runs the legacy assert before arg validation; `config-read-path`
  validates the key first then execs value. Preserve each subcommand's distinct
  ordering and test the legacy-layout + empty-key combination per subcommand.

- 🔵 **Correctness**: The legacy-override probe re-reads config via `config-read-value` — recursion must be replicated in-process
  **Location**: Key Discoveries / Phase 2 §1 `read_path`
  The warning is conditional on a non-empty *resolved* `paths.<legacy>` value (not
  just a grep match). Replicate the two-stage gate; add fixtures for match-but-empty
  and match-and-set.

- 🔵 **Correctness**: `repo_root` walk never tests a marker at the filesystem root
  **Location**: Phase 2 §1 `repo_root()`
  `find_repo_root` loops `while [ "$dir" != "/" ]`, so a repo at `/` is never matched.
  A Rust `Path::ancestors()` walk that includes `/` would diverge. Match bash exactly
  or consciously accept and pin with a test.

- 🔵 **Code Quality**: Triplicated `../frontend/dist` literal left in place rather than centralised
  **Location**: Phase 6 §2
  While the crate is being touched, hoist the path into a single const referenced by
  all three sites, removing the hand-sync requirement at low cost.

- 🔵 **Code Quality**: `a9r-core` module decomposition is listed but not bounded; risk of a god-module
  **Location**: Phase 2 §1
  The signatures example places everything in `config.rs`; Phase 7 absorbs
  `config-common.sh` (template resolution, array parsing, display-path). State the
  intended module split up front (`repo`, `files`, `frontmatter`, `lookup`,
  `defaults`, later `template`).

- 🔵 **Code Quality**: Exit-code and stream policy spread across binary match arms without a single mapping
  **Location**: Phase 2 §2
  Centralise `ConfigError` → exit-code in one function with a comment that not-found
  deliberately exits 0, so the surprising-but-required behaviour lives at one site.

- 🔵 **Code Quality**: Transitional bare `--config` alias adds CLI complexity with no stated removal trigger
  **Location**: Phase 2 §2 / Phase 6 §1
  Prefer clap's native default-subcommand mechanism over manual arg detection; record
  a concrete removal trigger tied to the bash-fallback deletion milestone.

- 🔵 **Architecture**: Triplicated `../frontend/dist` literal constrains the directory rename
  **Location**: Key Discoveries / Phase 6 §2
  Prefer the plan's safe default (keep `server/` dir, rename only the package)
  explicitly rather than offering the directory rename as an equal option; add a
  cross-reference or automated check.

- 🔵 **Architecture**: Transitional bare-`--config` alias mixes two CLI grammars in one parser
  **Location**: Phase 6 §1 / Decision 2
  Prefer a symlink/wrapper that always prepends `visualise` over teaching clap two
  grammars; record the intended removal milestone.

- 🔵 **Architecture**: Binary-resolution logic split across two new helpers risks divergence from the launcher's tri-precedence
  **Location**: Phase 4 §1 / Phase 5 §1
  Three places encode "how to find the binary" (`a9r-resolve.sh`, `acquire_binary`,
  the launcher). Have `a9r-resolve.sh` own precedence as the single source; both
  shim and launcher build on it.

- 🔵 **Test Coverage**: `a9r-core` unit tests omit stderr-warning content and exit-code mapping assertions
  **Location**: Phase 2 §1 / Testing Strategy
  Add explicit `#[test]`s asserting exact warning substrings (`.claude/accelerator.md`,
  `/accelerator:migrate`, "Warning") and the precise exit code per branch.

- 🔵 **Test Coverage**: `config-read-template` tab-delimited output parity is asserted but the harness strips it
  **Location**: Phase 7
  For structured `<source>\t<path>` output, add a raw-byte differential assertion
  (`cmp` of bash vs `a9r` output files) rather than command-substitution equality.

- 🔵 **Correctness**: Trailing-newline parity is partly masked by command-substitution capture
  **Location**: Performance Considerations / Migration Notes
  (Reinforces the critical Test-Coverage finding.) Add at least one parity assertion
  comparing raw bytes so the single-trailing-newline contract is actually enforced.

- 🔵 **Security**: Resolved binary path is not validated as a regular, owner-controlled file before exec
  **Location**: Phase 4 §2
  Carry the launcher's `! -L` symlink rejection and regular-file/executable checks
  into `a9r_bin()`'s cache-resolution branch; reject world-writable locations.

- 🔵 **Security**: Transitional symlink/alias widens what the SHA-verified `bin/` directory contains
  **Location**: Phase 6 §1
  Prefer an in-binary clap alias over a filesystem symlink in `bin/`; if a symlink is
  unavoidable, ensure resolution hash-verifies the real target before exec.

- 🔵 **Compatibility**: Legacy `ACCELERATOR_VISUALISER_BIN` override may be repurposed by the config-read shim
  **Location**: Phase 5 §1
  A user who set the env var to a pre-rename visualiser-only binary will have the
  shim `exec` `config-read-path` against a binary with no such subcommand. Probe the
  resolved binary for subcommand support and fall back to bash if absent.

- 🔵 **Compatibility**: clap `Option<String>` may not distinguish explicit-empty from omitted default
  **Location**: Phase 2 §2 / Key Discoveries
  The bash `[ -n "${2:-}" ]` guard hinges on this distinction. Add a parity test for
  `config-read-path <key> ''` vs `config-read-path <key>` and ensure the clap
  declaration preserves it.

- 🔵 **Compatibility**: Confirm every `a9r` invocation is reached via a `scripts/*`-matched shim
  **Location**: Phase 4 §2 / What We're NOT Doing
  A bare `a9r` path in a SKILL body would not match the existing
  `Bash(.../scripts/*)` globs. Note that all in-scope invocations go via the shim and
  that the new SessionStart hook is compatible with the v2.1.144 hooks.json schema.

- 🔵 **Portability**: Factored `acquire_binary` must preserve the sha256/curl-vs-wget fallbacks
  **Location**: Phase 5 §1
  Make preserving the `sha256_of` (sha256sum→shasum) and `download_to` (curl→wget,
  127 if neither) fallbacks an explicit success criterion; treat 127 as clean
  degrade-to-fallback.

- 🔵 **Portability**: Platform coverage is replicated, not widened (no Windows/WSL; Linux musl-only)
  **Location**: Phase 5 §3 / Current State
  Confirm the new hook degrades silently to bash on the unsupported-platform/arch
  branch (returns 0, emits nothing) rather than surfacing the launcher's `die_json`,
  and note Windows/WSL + musl-only as a conscious scope boundary in What We're NOT Doing.

- 🔵 **Portability**: Optional `server/` dir rename risks the triplicated `../frontend/dist` literals
  **Location**: Phase 6 §2
  Keep the `server/` dir name (rename only the package), or add a Phase 6 success
  criterion asserting the release build embeds and serves the SPA before merging.

- 🔵 **Safety**: In-place shim rewrite of two hot scripts has no documented rollback beyond git revert
  **Location**: Phase 4 §2
  Prefer the sibling `*-impl.sh` extraction so the original is preserved verbatim; add
  a success criterion asserting the path→value exec chain applies the defaults table
  exactly once in both modes.

- 🔵 **Safety**: Lockfile relocation + version-coherence repointing inside the "no behaviour change" phase alters the version source of truth
  **Location**: Phase 1 §3 / §5
  Add a Phase 1 check that `server/Cargo.toml` no longer carries a literal `version`
  and that `validate_version_coherence` fails *closed* if the workspace key is
  missing — verify the fail-closed path, not just the happy path.

- 🔵 **Safety**: Parity-gate enforcement depends on test-discovery invariants the reroute could silently weaken
  **Location**: Phase 3 §2
  Make "gate bites" automated: assert a minimum number of *executed* (not skipped)
  assertions per mode, and add a count floor for a9r-mode assertions.

### Strengths

- ✅ Excellent risk sequencing: behaviour-free restructure (Phase 1) → test-first
  `a9r-core` (Phase 2) → parity contract before any shim (Phase 3) → cheap vertical
  proven behind fallback → risky rename/fold deferred last (Phase 6). Each phase is
  scoped for small blast radius.
- ✅ The functional-core/imperative-shell split is explicit and correct: `a9r-core`
  is pure logic, `ReadOutcome` carries stdout + stderr warnings so the binary
  controls streams, and Rust `#[test]`s precede the implementation.
- ✅ The bash fallback is retained end-to-end and nothing it depends on is deleted —
  logic is *duplicated* into `a9r-core`, not moved — so a failed/offline/tampered
  binary degrades to known-correct behaviour for config-read.
- ✅ The byte-for-byte traps are unusually well-catalogued (empty-vs-omitted default,
  not-found-as-exit-0, last-file-wins, one-layer quote strip, string-prefix matching,
  no realpath), and the `A9R_FORCE_BASH` escape hatch keeps the fallback path
  continuously exercised in CI.
- ✅ Reuses the existing, hardened acquisition pipeline (tri-precedence resolution,
  SHA-256, all-zeros sentinel, version-drift rejection, bounded curl) and the
  existing cross-compile + checksums + version-coherence machinery rather than
  inventing new ones; consolidates version to one `[workspace.package]` source.
- ✅ The `../frontend/dist` triplication trap is explicitly acknowledged and the
  directory rename is gated on lockstep updates, with the safe default of keeping the
  `server/` dir.
- ✅ The parity-gate verification step (deliberately break `a9r` output, confirm the
  a9r-mode run fails) is a conscious test of the test.

### Recommended Changes

1. **Add raw-byte differential parity assertions** (addresses: Parity gate cannot
   detect trailing-newline divergence; Trailing-newline partly masked;
   `config-read-template` tab-delimited parity). For each ported command, add at
   least one assertion that captures raw bytes (write to files and `cmp`, or
   `| xxd`) rather than command substitution, so trailing-newline and separator
   divergence actually fail the gate.

2. **Make the parity gate non-vacuous** (addresses: No automated guard the `A9R_BIN`
   run executed; Enforcement depends on discovery invariants). Have `test-config.sh`
   emit a mode banner, assert `A9R_BIN` is non-empty *and* executable in the a9r run
   (fail loud, never degrade), and add a count floor on *executed* a9r-mode
   assertions.

3. **Specify the binary-resolution security contract** (addresses: Team-committed
   config key auto-RCE; Shim skips SHA re-verification; Regular-file/symlink
   validation; Resolution split across three helpers; Shim routes to present-but-buggy
   binary). Make `a9r-resolve.sh` the single owner of precedence; restrict the
   `visualiser.binary` config branch on the hot path to gitignored local config (or
   exclude it); reject symlinked/world-writable cache entries; and require the shim to
   route only to a binary whose SHA is in the verified manifest (cheap mtime/hash
   guard against the eager-hook-verified cache, full re-hash on mismatch).

4. **Define the SessionStart hook resilience contract** (addresses: Eager blocking
   hook network step ×3; Lacks timeout; curl/wget + sha256 fallbacks; Platform
   coverage; TOCTOU). Add hard connect/total timeouts, guarantee exit 0 (silent
   fallback) on timeout/offline/missing-downloader/unsupported-platform, preserve the
   `sha256_of`/`download_to` fallbacks as a success criterion, use atomic rename into
   the cache path, and add a fast cache-valid fast-path. Reconsider eager-blocking vs
   lazy/background.

5. **Define the binary-rename transition release** (addresses: Cross-version asset
   skew; `checksums.json` keys before release; Launcher fake-binary tests). Specify a
   release that publishes *both* `accelerator-visualiser-<platform>` and
   `a9r-<platform>` (both in `checksums.json`), have the launcher try the new name
   then the old, sequence the `a9r` checksum key in lockstep with the asset, and add
   the `test-launch-server.sh` fake/fixture updates to Phase 6's changes list.

6. **Complete the byte-for-byte trap list** (addresses: Within-section first-match;
   Empty-but-closed frontmatter; Looser warning-gate regex; Unknown key still reads
   config; Legacy-override probe recursion; Check ordering; `repo_root` at `/`;
   empty-vs-omitted via clap). Enumerate these in Phase 2/Testing Strategy as named
   fixtures, codified as a shared table both the bash suite and Rust `#[test]`s
   iterate, and audit `test-config.sh` for each before Phase 3 lands.

7. **Resolve the duplication ownership and module boundaries** (addresses:
   Frontmatter duplicated `a9r-core` vs visualiser lib; Hot-path links visualiser
   lib; bash↔Rust lockstep; `a9r-core` god-module). Name `a9r-core` the single owner
   of shared parsing (or document the separation), guarantee `a9r-core` has zero
   dependency on the visualiser lib, and state the intended `a9r-core` module split.

8. **Fix the per-phase merge gate and mergeability framing** (addresses: "green on
   `mise run check`" misses tests; "Independently mergeable" only forward).
   State the gate as the full `mise run` (or specific `test:integration:*` tasks) and
   reframe as "sequentially mergeable, forward-only".

9. **Commit to one shim shape and the `--config` alias mechanism** (addresses: Shim
   `_fallback` in-file vs sibling; Test-site guard readability; bare `--config`
   collides with clap; Removal trigger). Choose sibling `*-impl.sh` shims uniformly,
   a `skip_unless_bash_mode` helper for guards, and a symlink/wrapper (or explicit
   argv pre-processing with an equivalence test) over teaching clap two grammars;
   record a removal milestone.

10. **Centralise low-cost hardening** (addresses: Triplicated `../frontend/dist`;
    Exit-code mapping; fail-closed coherence; `allowed-tools`/hooks schema). Hoist
    `../frontend/dist` to one const, centralise `ConfigError` → exit-code, verify the
    fail-closed coherence path, and note that all `a9r` invocations go via
    `scripts/*` shims and the hook fits the v2.1.144 schema.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: Architecturally sound at its core: correctly identifies the
skill→script boundary as an already-clean CLI/IPC contract, sequences the risky
rename behind a proven cheap vertical, and uses a parity gate plus bash fallback to
keep phases independently mergeable. Dominant concerns: a deliberate heavy coupling
(single binary bundling axum + SPA behind hot-path config reads), duplicated
frontmatter/config logic between `a9r-core` and the existing visualiser lib, the
`a9r` binary depending on the full visualiser lib for non-visualiser commands, and a
new network-dependent SessionStart hook (well-mitigated by silent fallback).

**Strengths**: Walking-skeleton-first sequencing isolates structural from logic
risk; bash-fallback shim + twice-run gate give genuine graceful degradation;
functional-core/imperative-shell split with `ReadOutcome` is explicit and correct;
byte-for-byte CLI contract preserved as the cross-language invariant; version
coherence consolidated to one source.

**Findings**:
- 🟡 (high) Frontmatter/config logic duplicated between `a9r-core` and the visualiser lib (Phase 2 §1–2 / Desired End State).
- 🟡 (high) Hot-path config commands transitively link the entire axum/SPA visualiser lib (Phase 2 / Decision 1).
- 🟡 (medium) New eager blocking SessionStart hook adds a network-dependent step before every session (Phase 5 §3).
- 🔵 (high) Triplicated `../frontend/dist` literal constrains the directory rename (Key Discoveries / Phase 6 §2).
- 🔵 (medium) Transitional bare-`--config` alias mixes two CLI grammars in one parser (Phase 6 §1 / Decision 2).
- 🔵 (medium) Binary-resolution logic split across two helpers risks divergence from the launcher's tri-precedence (Phase 4 §1 / Phase 5 §1).

### Code Quality

**Summary**: Unusually disciplined for a migration — good risk sequencing, every
phase independently mergeable, a clean clap-subcommand seam lifted from the existing
thin-binary/fat-lib layout. Dominant liability: deliberate long-lived duplication of
subtle config-parsing semantics across bash awk and Rust, guarded only by a CI
parity gate with no shared spec beyond a prose trap list. The shim design, the
`run_sut` retrofit, and the triplicated `frontend/dist` literals carry maintainability
risk the plan should make explicit.

**Strengths**: Excellent risk sequencing (structural → test-first → parity contract);
clap `#[derive(Subcommand)]` is the right pattern; `ReadOutcome` keeps `a9r-core`
pure; traps enumerated explicitly; the parity-gate "does it bite?" check.

**Findings**:
- 🔴/🟡 (high) Duplicated config-parsing semantics across bash and Rust with only a CI gate to keep them in lockstep (What We're NOT Doing / Phase 2 §1 / Phase 4).
- 🟡 (medium) Shim `_fallback` duplicates the full bash implementation in-file, risking drift (Phase 4 §2).
- 🟡 (medium) Mechanical rewrite of 58 test call sites with conditional guards reduces readability (Phase 3 §2).
- 🔵 (medium) Triplicated `../frontend/dist` literal left in place rather than centralised (Phase 6 §2).
- 🔵 (medium) `a9r-core` module decomposition listed but not bounded; risk of a god-module (Phase 2 §1).
- 🔵 (medium) Transitional bare `--config` alias adds CLI complexity with no removal trigger (Phase 2 §2 / Phase 6 §1).
- 🔵 (low) Exit-code and stream policy spread across binary match arms without a single mapping table (Phase 2 §2).

(Note: the lens emitted its lead finding at major severity; the body text carried a 🔴 glyph. Treated as **major** for aggregation.)

### Test Coverage

**Summary**: The parity-gate strategy is sound in shape — reusing the black-box
`test-config.sh` against both backends directly tests the contract that matters, and
the TDD-first `a9r-core` tests are well-enumerated. But the gate does NOT prove
byte-for-byte parity: most assertions flow through command substitution (strips
trailing newlines — the dominant trap), there is no automated differential check, no
guard that the a9r-mode run executed, and the SKILL.md grep-exclusion is unjustified.

**Strengths**: Test-first discipline with concrete enumerated cases; reusing the
existing suite as the cross-language net; twice-run CI; `A9R_FORCE_BASH` exercises
the fallback branch; dedicated acquisition-error test suite.

**Findings**:
- 🔴 (high) Parity gate cannot detect trailing-newline divergence — the dominant byte-for-byte trap (Phase 3 / Testing Strategy).
- 🟡 (high) No automated guard that the a9r-mode run actually executed; count floor is blind to it (Phase 3 §2).
- 🟡 (medium) Excluding the SKILL.md grep-assertions under `A9R_BIN` is unjustified (Phase 3 §2 / Key Discoveries).
- 🟡 (high) `run_sut` reroute must preserve heterogeneous redirection forms, not just trailing args (Phase 3 §1–2).
- 🟡 (medium) "config-read is already well-covered" is asserted, not measured (What We're NOT Doing / Phase 7).
- 🔵 (medium) `a9r-core` unit tests omit stderr-warning content and exit-code mapping assertions (Phase 2 §1).
- 🔵 (medium) `config-read-template` tab-delimited output parity asserted but the harness strips it (Phase 7).

### Correctness

**Summary**: Strong correctness awareness — most byte-for-byte traps enumerated, the
existing suite (run twice) is the right gate. But several subtle bash behaviours are
under-specified or missing (within-section first-match vs last-file-wins,
empty-but-closed frontmatter, the looser warning-gate regex, unknown-key-still-reads-config,
the recursive legacy-override probe), and the new acquisition path introduces TOCTOU
/ concurrent-download races. The gate catches divergence only for inputs the suite
already exercises.

**Strengths**: Existing suite as parity contract with a deliberately-broken-output
check; highest-risk traps identified; bash fallback retained; TDD ordering with an
explicit edge-case list.

**Findings**:
- 🟡 (high) Within-section first-match-wins precedence not enumerated alongside last-file-wins (Key Discoveries / Phase 2 §1).
- 🟡 (high) Empty-but-closed frontmatter treated as not-found, distinct from unclosed (Phase 2 §1).
- 🟡 (high) Unclosed-frontmatter warning gate uses a looser regex than the parser (Key Discoveries / Phase 2 §1).
- 🟡 (medium) Unknown path key with no explicit default still performs a config lookup (Phase 2 §1).
- 🟡 (medium) Concurrent acquisition / TOCTOU between hook, launcher, and shim unaddressed (Phase 5 §3 / Phase 4 §1).
- 🔵 (medium) Legacy-layout guard vs usage-error check ordering differs between the two subcommands (Phase 2 §2).
- 🔵 (medium) Legacy-override probe re-reads config via `config-read-value` — recursion must be replicated in-process (Phase 2 §1).
- 🔵 (low) `repo_root` walk never tests a marker at the filesystem root (Phase 2 §1).
- 🔵 (low) Trailing-newline parity partly masked by command-substitution capture (Performance / Migration Notes).

### Security

**Summary**: Extends an existing unverified-binary escape hatch (env var +
team-committed config key) from a user-triggered, use-time launch to an automatic,
load-time gate on every skill load — substantially enlarging the
arbitrary-binary-execution blast radius. The download path is reused soundly, but the
shim resolution appears to skip the per-invocation SHA re-verification the launcher
performs, and SLSA provenance is not addressed for the new artifact.

**Strengths**: Retains layered download-integrity controls (SHA-256, sentinel,
version-drift, bounded curl) with planned acquisition tests; preserves
metacharacter-safe string-prefix matching; fail-safe degradation; reuses existing
`allowed-tools` globs via the shim.

**Findings**:
- 🟡 (high) Team-committed config key becomes an automatic load-time arbitrary-binary-execution vector (Phase 4 §1).
- 🟡 (medium) Shim resolution skips per-invocation SHA re-verification of the cached binary (Phase 4 §1 / Phase 5 §1).
- 🟡 (medium) New eager binary download omits SLSA provenance verification for the a9r artifact (Phase 5 §3).
- 🔵 (medium) Rust port becomes a new prompt-injection output channel; clean-stderr must be enforced, not assumed (Phase 2 §2 / Phase 7).
- 🔵 (medium) Resolved binary path is not validated as a regular, owner-controlled file before exec (Phase 4 §2).
- 🔵 (low) Transitional symlink/alias widens what the SHA-verified `bin/` directory contains (Phase 6 §1).

### Compatibility

**Summary**: Unusually compatibility-conscious for a rename-plus-rewrite — preserves
the CLI contract via the twice-run gate, keeps user config knobs working, retains the
bash fallback, centralises the version. Principal risk: cross-version skew in the
download/release contract (asset name + URL derived from installed version) with no
dual-publish window. Secondary: the bare `--config` alias collides with clap, and the
launcher fake-binary tests use the old `--config` form.

**Strengths**: The parity gate is the right mechanism to prove the CLI contract;
user config knobs preserved; bash fallback retained; version consolidated; Phase 1 is
a pure no-behaviour-change restructure.

**Findings**:
- 🔴 (high) Binary rename creates a cross-version download-asset skew with no dual-publish window (Phase 6 §3 / Phase 5).
- 🟡 (high) Transitional bare `--config` alias collides with clap subcommand parsing (Phase 6 §1).
- 🟡 (medium) `checksums.json` `a9r` keys before a release ships the asset — visualiser path has no fallback (Phase 5 §2 / Phase 6 §3).
- 🟡 (medium) Launcher fake-binary tests invoke the old `--config` form (Phase 6 §1).
- 🔵 (medium) Legacy `ACCELERATOR_VISUALISER_BIN` override may be repurposed by the config-read shim (Phase 5 §1).
- 🔵 (high) clap `Option<String>` may not distinguish explicit-empty from omitted default (Phase 2 §2 / Key Discoveries).
- 🔵 (medium) Confirm every `a9r` invocation is reached via a `scripts/*`-matched shim (Phase 4 §2).

### Portability

**Summary**: Broadly portability-aware — reuses the proven platform-detection and
download/verify pipeline, retains the bash fallback, cross-compiles the same four
targets. Main risks are net-new: a blocking SessionStart hook adds a network + tool
dependency to every fresh session; platform detection is replicated without widening
coverage (no Windows/WSL, Linux musl-only); and the new shim/hook shell must stay on
the bash 3.2 floor, which the plan only implicitly assumes.

**Strengths**: Reuses the launch-server.sh acquisition pipeline via a shared
`acquire_binary`; bash fallback covers unprovisionable platforms; same four
cross-compile targets folded into existing checksums/coherence; user knobs preserved.

**Findings**:
- 🟡 (high) Eager blocking SessionStart hook adds a network + tool dependency to every fresh session start (Phase 5 §3).
- 🟡 (medium) New shim/resolve/provision shell must observe the bash 3.2 floor but the plan never states it (Phase 4 §1 / Phase 5 §3).
- 🔵 (medium) Factored `acquire_binary` must preserve the sha256/curl-vs-wget portability fallbacks (Phase 5 §1).
- 🔵 (medium) Platform coverage is replicated, not widened: no Windows/WSL, Linux musl-only (Phase 5 §3).
- 🔵 (high) Optional `server/` dir rename risks the triplicated `../frontend/dist` literals (Phase 6 §2).

### Safety

**Summary**: Unusually safety-conscious — bash fallback retained end-to-end, nothing
the fallback depends on deleted, parity gate as the correctness contract, risky
rename deferred last. Two material gaps: the "every phase green on `mise run check`,
independently mergeable" claim is overstated (the gate lives in the test aggregates
`mise run check` does NOT run, and phases have hard forward ordering), and the
blocking SessionStart hook needs an explicit timeout + fail-open contract.

**Strengths**: Fallback retained and nothing it depends on deleted; risky rename
sequenced last; `frontend/dist` trap acknowledged and gated; `A9R_FORCE_BASH` keeps
the fallback exercised; Phase 1 isolates structural risk with a "no source-logic
changes" gate; hook is a no-op-until-ready addition.

**Findings**:
- 🟡 (high) "Every phase ends green on `mise run check`" does not cover the parity gate — the real guarantee runs only under the full default task (Implementation Approach + each phase's Success Criteria).
- 🟡 (high) Blocking SessionStart binary download lacks an explicit timeout/backgrounding contract (Phase 5 §3).
- 🟡 (medium) "Independently mergeable" is only true for the forward sequence (Implementation Approach + Phase 4 vs Phases 2–3).
- 🔵 (medium) In-place shim rewrite of two hot production scripts has no documented rollback beyond git revert (Phase 4 §2).
- 🔵 (medium) Lockfile relocation + version-coherence repointing conflated into the "no behaviour change" phase (Phase 1 §3 / §5).
- 🔵 (low) Parity-gate enforcement depends on test-discovery invariants the reroute could silently weaken (Phase 3 §2).

## Re-Review (Pass 2) — 2026-06-15T16:08:27+00:00

**Verdict:** REVISE

The revision resolved the great majority of pass-1 findings — including one of
the two criticals — and several lenses now cite the former problem areas as
strengths (raw-byte differential, the trap list, the sibling-`*-impl.sh` shim
shape, the centralised resolution helper, the corrected merge gate). The verdict
remains REVISE for two reasons: (1) a **newly-surfaced critical** correctness bug
that pass 1 missed — found-with-empty-value is not distinguished from not-found in
the `lookup` spec — and (2) the now-more-specific plan exposed a **next layer** of
mechanism gaps (the twice-run harness and executed-assertion floor are referenced
but unspecified; `checksums.json` is keyed by platform only so it cannot express
the dual-asset transition; `embed-dist` feature-unification pulls the frontend stub
into the "lightweight" `a9r` build; `timeout(1)` is absent on macOS). A few of
these (twice-run wiring, `timeout(1)`, executed floor) are direct consequences of
pass-1 edits that introduced a requirement without pinning its mechanism.

### Previously Identified Issues

- 🔴 **Test Coverage**: Trailing-newline parity gap — **Resolved** (raw-byte `cmp`
  differential added; now cited as a strength).
- 🔴 **Compatibility**: Binary-rename asset skew — **Partially resolved** (dual-asset
  release defined; but the `checksums.json` schema and the version-drift `die`
  interaction are newly flagged — see New Issues).
- 🟡 **Test Coverage**: No guard the a9r run executed — **Partially resolved** (intent
  added: fail-loud, banner, floor; but the floor/twice-run *mechanism* is
  unspecified — see New Issues).
- 🟡 **Test Coverage**: `run_sut` redirection forms — **Resolved** (all four forms
  enumerated; a residual minor on normalising inline stderr checks remains).
- 🟡 **Test Coverage**: SKILL.md grep-assertion exclusion — **Resolved** (un-excluded).
- 🟡 **Test Coverage**: "well-covered" asserted not measured — **Resolved** (pre-Phase-3
  trap audit now required).
- 🟡 **Correctness**: within-file first-match, empty-but-closed frontmatter, loose
  warning gate, unknown-key-reads-config — **Resolved** (all enumerated; cited as
  strengths).
- 🟡 **Correctness**: acquisition TOCTOU — **Resolved** (atomic rename; residual
  cheap-guard TOCTOU downgraded to minor).
- 🟡 **Architecture**: frontmatter dup `a9r-core` vs visualiser lib — **Partially
  resolved** (ownership/zero-dep stated; the actual consolidate-or-document decision
  is still deferred to implementation).
- 🟡 **Architecture**: hot-path links the visualiser lib — **Partially resolved**
  (Decision-1 restated; but `embed-dist` feature-unification is a newly-exposed
  contradiction — see New Issues).
- 🟡 **Architecture**: eager blocking hook — **Resolved** (timeout + fast-path +
  fail-open contract).
- 🟡 **Code Quality**: bash/Rust lockstep duplication — **Partially resolved** (shared
  fixture table added; but three coexisting parsers and the retained-bash drift have
  no mechanical guard — see New Issues).
- 🟡 **Code Quality**: shim `_fallback` in-file — **Resolved** (sibling `*-impl.sh`;
  cited as a strength).
- 🟡 **Code Quality**: 58-site guard readability — **Resolved** (`skip_unless_bash_mode`
  helper).
- 🟡 **Security**: team config-key auto-RCE on hot path — **Resolved for the hot path**
  (restricted to gitignored local); **still present on the user-triggered
  `visualise` path** — see New Issues.
- 🟡 **Security**: shim skips SHA re-verify — **Resolved** (SHA-in-manifest gate;
  residual minor on the mtime/size cheap-guard).
- 🟡 **Security**: SLSA provenance omitted — **Partially resolved** (added, but
  conditional; portability flags it as a heavy host dependency — tradeoff to settle).
- 🟡 **Compatibility**: clap `--config` alias collision — **Resolved** (symlink/wrapper
  preferred; residual minor on argv pre-processing).
- 🟡 **Compatibility**: checksums key before release / no-fallback visualiser path —
  **Resolved** (sequenced; no-fallback noted).
- 🟡 **Compatibility**: launcher fake-binary tests — **Resolved** (added to Phase 6 §3).
- 🟡 **Portability**: eager hook network dependency — **Partially resolved** (timeout +
  fail-open; but `timeout(1)` portability and unsupported-platform enumeration are
  new — see New Issues).
- 🟡 **Portability**: bash 3.2 floor in new shell — **Resolved** (stated; `tr` not
  `${var,,}`).
- 🟡 **Safety**: "mise run check" merge gate — **Resolved** (corrected to full `mise run`).
- 🟡 **Safety**: blocking download timeout — **Partially resolved** (in-script timeout
  added; but the Claude Code hook-timeout regime is not validated — see New Issues).
- 🟡 **Safety**: "independently mergeable" — **Resolved** (reframed forward-only).
- 🔵 (various minors) — addressed transitively (exit-code mapping centralised,
  clean-stderr enforced, `frontend/dist` const hoist, fail-closed coherence,
  repo-root-at-`/`, two-stage legacy probe).

### New Issues Introduced

- 🔴 **Correctness**: **Found-with-empty-value is not distinguished from not-found**
  in the `lookup` spec (Phase 2 §1). A key present but set empty (`key:`) returns
  found-empty in bash and **suppresses the default**; an `Option`-based Rust port
  returning `None` for both cases would wrongly emit the default, and a local empty
  value must override a team non-empty value. Latent in pass 1; surfaced now. Needs
  a found/not-found flag distinct from the value + fixtures.
- 🟡 **Test Coverage**: The **twice-run mechanism is unspecified** — the only cited
  harness (`run_shell_suites`) runs each suite once with no env-variance (Phase 3 §4),
  and the **executed-assertion floor + mode banner have no specified mechanism**
  (Phase 3 §1; the cited pytest only covers the unrelated discovery floor). Both are
  consequences of pass-1 requirements added without pinning the wiring.
- 🟡 **Architecture / Code Quality**: `a9r` depends on the visualiser lib whose
  **default `embed-dist` feature** hard-requires `frontend/dist/index.html`; Cargo
  feature-unification means a default `build:a9r:dev`/`lint:a9r` pulls the frontend
  stub — contradicting Phase 2 §3's "no stub needed". Needs explicit
  `default-features = false` + an opt-in `embed-dist` only for the visualise build.
- 🟡 **Compatibility**: **`checksums.json` is keyed by `<platform>` only** — one SHA
  slot per platform, so the dual-asset transition ("list both names") cannot be
  expressed; an old launcher falling back to the old asset name would verify it
  against the `a9r` SHA and fail. Needs a manifest schema change (Phase 5 §2).
- 🟡 **Compatibility**: **Version-drift `die` vs dual-asset transition** — an old
  launcher reading a new release's manifest hits the `MANIFEST_VERSION !=
  PLUGIN_VERSION` `die` before the asset-name fallback; the skew story needs the
  plugin/release version-coupling spelled out (Phase 5 §2 / Phase 6 §1).
- 🟡 **Security**: **Team-committed `visualiser.binary` still auto-executes on the
  `visualise` launch path** (Phase 6 §1) — the hot-path restriction does not cover
  the user-triggered launch, leaving a one-step in-repo RCE on a hostile clone. Also:
  **env-var override (`ACCELERATOR_VISUALISER_BIN`) runs unverified on the hot path**,
  and the eager hook can reach the existing `ACCELERATOR_VISUALISER_INSECURE_DOWNLOAD`
  / mirror escape hatches.
- 🟡 **Portability**: **`timeout(1)` is not on stock macOS** (Phase 5 §1) — bound the
  network calls with `curl --max-time`/`--connect-timeout` + `wget --timeout`, not the
  external `timeout` binary. Plus: **unsupported platforms (Windows/WSL, non-musl,
  other arches) are not enumerated** for the now-universal hook, and **SLSA
  verification is a heavy host dependency** the portability lens wants made
  best-effort.
- 🟡 **Safety**: The in-script timeout sits inside **Claude Code's own SessionStart
  hook-timeout regime**, which is not validated against v2.1.144; a runtime kill
  mid-download could bypass in-script cleanup. Plus a **hot-path blast-radius**
  concern: a repeatedly-tripping cheap guard could full-re-hash the binary on every
  skill load.

### Assessment

The plan is materially stronger and the pass-1 criticals are essentially closed.
The remaining work splits cleanly: **one substantive correctness fix** (found-empty
vs not-found — fix before implementation), **a handful of mechanism specifications
my own pass-1 edits implied but did not pin** (twice-run harness wiring, executed
floor, `timeout(1)` → curl flags, `embed-dist` `default-features = false`, the
`checksums.json` schema for dual assets), and **the `visualise`-path team-key RCE**
which should get the same gitignored-local restriction as the hot path. The rest
(SLSA mandatory-vs-best-effort, unsupported-platform enumeration, hook-timeout
validation, parser-consolidation decision) are bounded decisions to record rather
than open design problems. A third targeted edit pass on those items — especially
the new critical — would bring this to APPROVE; further full-lens passes risk
diminishing returns (each round exposes the next implementation-detail layer).
