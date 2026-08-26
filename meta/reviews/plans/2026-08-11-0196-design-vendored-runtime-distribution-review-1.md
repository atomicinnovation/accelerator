---
type: plan-review
id: "2026-08-11-0196-design-vendored-runtime-distribution-review-1"
title: "Plan Review: accelerator-design: Vendored Runtime Distribution"
date: "2026-08-17T13:19:24+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "work-item:0196"
target: "plan:2026-08-11-0196-design-vendored-runtime-distribution"
relates_to: ["plan-review:2026-08-11-0196-accelerator-design-inventory-gap-tooling-cli-review-1", "plan-review:2026-08-11-0196-design-cli-migration-review-1"]
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, correctness, security, test-coverage, safety, portability, performance, code-quality]
review_number: 1
review_pass: 3
tags: [rust, launcher, release-pipeline, supply-chain, tree-artifacts, playwright, distribution]
last_updated: "2026-08-17T15:29:53+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: accelerator-design: Vendored Runtime Distribution

**Verdict:** REVISE

This is an exceptionally rigorous plan — every lens opened by saying so, and the strengths are real: the three narrow ports that keep fetching out of reach of the dispatch path by type rather than discipline, the generation-based rename discipline that eliminates the collision branch entirely, the correctly-diagnosed retry-append corruption trap, the `GOODSIG`-not-`VALIDSIG` predicate, and the refusal to back-derive a per-MB slope from a non-method-comparable figure. The problems are not sloppiness; they are the residue of a plan rewritten against a moved tree, where several mechanisms were settled in one section and specified differently in another. Eight critical findings cluster into three groups: the CI job topology that Phase 2's strongest gate requires cannot be expressed in GitHub Actions; the signed attestation that anchors the entire exempt-from-re-verification cache cannot be verified as specified and would not bind what it needs to bind even if it could; and Phase 3's edit set never repoints the executor at the vendored Node it spends ~1.2GB per release shipping, so the plan's own Desired End State is unreachable.

### Cross-Cutting Themes

Six issues were flagged independently by three or more lenses. These are where the plan's internal contradictions concentrate, and they deserve attention before any of the single-lens findings.

- **The smoke-check job topology is not expressible** (architecture, correctness, safety, security) — a separate `permissions: {}` job cannot gate a step inside the job that produces its inputs, and the fallback silently drops the only gate distinguishing "signed" from "works".
- **The attestation's trust anchor is broken three ways** (security ×2, correctness, test-coverage) — no producer emits a signature over anything the launcher retains; `locate`'s enumerated algorithm never verifies one; the signature binds no artifact identity, platform or version; and no criterion would catch its removal.
- **The `flock` lease has no acquisition site and contradicts ADR-0061 on placement** (architecture, correctness, safety ×2, portability) — every algorithm in the plan omits it, `locate` is declared pure, `ensure` runs in a process that exits, and the ADR puts the file beside the generation while the plan puts it inside.
- **`ACCELERATOR_LAUNCHER_BIN` is prohibited and then exported** (architecture, correctness, security, code-quality) — Key Discoveries names it as unsafe; Phase 3 §3 exports exactly it.
- **Tree-variable export is specified two contradictory ways** (architecture, performance, code-quality) — disk enumeration versus the compiled-in set, with different costs, different coupling, and an "always set or explicitly cleared" invariant that only the second reading can satisfy.
- **Load-bearing thresholds are deferred with no value and no owner** (test-coverage, performance, safety) — the fetch deadline, the inflate ceiling, the first-run wall clock, `sign_file`'s timeout and `download_release_asset`'s timeout are all criteria whose pass condition is set after the phase that gates them.

### Tradeoff Analysis

Two places where lenses reach opposite conclusions from the same text.

- **Cache-root strictness: security vs portability.** Security wants the hit path *stricter* — `locate` uses `stat` (which follows symlinks), never checks the pointer file's own ownership, and never checks `trees/` or the cache root, so a symlinked generation satisfies every check as written. Portability wants it *looser* — RHEL-family `umask 002`, container bind mounts with mapped uids, and NFS-squashed homes all fail an effective-uid-plus-not-group-writable test through no fault of the user, on exactly the relocated `ACCELERATOR_CACHE_DIR` the plan recommends. **Recommendation**: root the strict check at the `trees/` subdirectory the launcher itself creates `0700` (satisfying portability, since the launcher controls that mode), and harden the *mechanism* there — `symlink_metadata`/`O_NOFOLLOW`, pointer-file ownership, per-component resolution — rather than widening the ancestor set. Give the refusal a message naming the exact `chmod`.
- **Symlink admission in the extractor: code-quality vs portability.** Phase 2 §3 offers to retire the symlink branch — the hardest-to-review code in the extractor — if assembly emits none. Portability warns that macOS Chromium ships a `.framework` with `Versions/Current` symlinks, and flattening them breaks the bundle layout the upstream code signature records, which on arm64 macOS is an execution failure rather than a cosmetic change. **Recommendation**: settle this empirically per platform *before* Phase 1 fixes the allowlist, and expect to keep the branch for darwin.

### Findings

#### Critical

- 🔴 **Architecture / Correctness / Safety / Security**: The smoke-check gate's job topology cannot be built
  **Location**: Phase 2 §8: Reuse across cuts, and a functional gate
  A separate job with `permissions: {}` must `needs:` the job producing its artifacts, so it cannot gate a publish step inside that same job — and `main.yml:607-612` requires prepare/sign/finalise to stay in one job for version monotonicity. The described gating is a cycle, so the realistic outcome is either the plan's own weaker fallback or executing upstream Node and Chromium binaries in the job that later holds `ACCELERATOR_RELEASE_SECRET_KEY`, which §3's own rule forbids. Compounding it, §3 rejected job isolation partly because moving ~1.2GB between jobs is "substantial", while §8 accepts that same transfer.

- 🔴 **Security**: No producer emits a signature the launcher can verify from a retained artifact
  **Location**: Phase 1 Step 1b §2 — "The attestation is signed"
  The attestation is specified as "the manifest's minisign signature over the archive digest", but that signature is over the **archive file's bytes** (`tasks/signing.py:24-43` signs `-m <file>`, `tasks/manifest.py:81-108` slurps the `.minisig`), and the launcher deletes the archive after extraction. Phase 2 §5's four arms produce no small release-key-signed statement a 64-hex digest could be checked against, so the consumer needs a producer artifact the producer never emits.

- 🔴 **Correctness**: `locate`'s enumerated algorithm performs no signature verification at all
  **Location**: Phase 1 Step 1b §2 — `locate` steps 1-4
  The prose calls the signed attestation "the hit path's only cryptographic anchor"; the four-step algorithm immediately below checks only that the attestation's digest equals the digest in its own directory name, and the cost line reads "Two small reads and two stats." The enumerated form is what an implementer will code, and it is exactly the self-referential state ADR-0061 exists to eliminate — on the path taken by all 100–200 dispatches per crawl.

- 🔴 **Security**: The signature binds no identity, platform or version, and the pointer is unsigned
  **Location**: Phase 1 Step 1b §2 — layout and the `.ref` pointer
  Signing only the archive digest leaves artifact identity, platform and release version in unsigned local state. Any process able to write `trees/` can repoint a `.ref` at another artifact's, another platform's, or an **older release version's** generation whose attestation signature is entirely valid, and `locate` accepts it — silent rollback to a known-vulnerable vendored Chromium, with no network, no manifest and no re-hash on the path that would notice.

- 🔴 **Test coverage**: Deleting the signature check would leave every Phase 1 criterion green
  **Location**: Phase 1 Success Criteria (Automated Verification)
  Applying the mutation lens: remove signature verification from `locate` entirely and every listed criterion still passes. Nothing tests an untrusted-keypair attestation, and nothing tests a `.files` table mutated after sealing — the table is `verify`'s oracle, so an unanchored table makes every tree-side detection criterion vacuous.

- 🔴 **Correctness / Code quality**: The vendored Node binary is never wired in — the executor still spawns bare `node`
  **Location**: Phase 3 §3: Tree resolution
  `const NODE: &str = "node"` (`cli/design-cli/src/executor.rs:28`) is used verbatim as `program: PathBuf::from(NODE)` in both `DaemonSpawner` (`:163`) and `ExecClient` (`:174`) — a `PATH` lookup. Phase 3 §3 edits only the environment vector at `:139-156`, and the plan's Current State Analysis asserts that vector is "the single place a resolved browser path is threaded", which is wrong by one. AC6's Node-absent container fixture fails at `ENOENT` after ~294MB has been fetched, sealed and verified.

- 🔴 **Correctness**: Deleting `playwright-loader.js` breaks module resolution — `NODE_PATH` does not apply to ESM
  **Location**: Phase 3 §1: Retarget the automation
  `package.json` declares `"type": "module"` and `daemon.js` is ESM. Node's ESM resolver ignores `NODE_PATH` entirely — it is CommonJS-only — which is precisely why the loader builds an absolute `pathToFileURL(...)` and imports that (`playwright-loader.js:63-66`). A bare `import 'playwright-core'` resolves by walking `node_modules` upward from the plugin tree, not into the sealed driver tree, so every crawl fails with `ERR_MODULE_NOT_FOUND`.

- 🔴 **Architecture**: The lease's placement contradicts ADR-0061 and would make `verify` report every healthy tree as corrupt
  **Location**: Phase 1 Step 1b §2 — the in-use signal
  The plan puts the lease file "inside each generation"; ADR-0061 decides the opposite — a sidecar *beside* it, because the seal would otherwise make it read-only for the dispatches that must take it and `verify` would report it as an entry absent from the `.files` table. The layout block lists no lease at all, and a Step 1c criterion asserts `verify` detects "an unexpected extra entry". Three sections encode incompatible assumptions about one file, and the composite outcome turns `repair` into a loop.

- 🔴 **Safety**: The `gh release delete --cleanup-tag` arm can leave the marketplace advertising a deleted tag
  **Location**: Phase 2 §7: Release-job capacity and the failure envelope
  By the time that envelope runs, `_publish` has already committed, tagged and pushed — and the pushed commit carries `marketplace.update_version`'s edit setting `.claude-plugin/marketplace.json`'s `source.ref` to `vX`. Deleting the tag leaves `main` advertising a ref that no longer exists, breaking installs and `/plugin update` for every user. The plan's stated mitigation ("reserved for pre-upload failures") does not hold: the `missing` check sits outside the `try`, so after the narrowing there are no pre-upload failures inside the envelope at all.

- 🔴 **Test coverage**: Four headline acceptance criteria rest on container fixtures that have no artifact source, no CI job and no file in Changes Required
  **Location**: Phase 3 Success Criteria (AC6, AC9, AC11, AC12); Testing Strategy → Integration Tests
  The plan concedes the mechanism is unresolved ("the artifact-serving component and its binding must be named"). Phase 3 lists no `main.yml` edit, no invoke task and no image definition, and the repo has exactly one container lane today. AC6 as written also needs artifacts that exist only after a signed real release, so the test cannot run pre-merge on the change that introduces it.

#### Major

- 🟡 **Architecture / Correctness / Safety**: The `flock` lease is never acquired by any specified algorithm
  **Location**: Phase 1 Step 1b §2; Phase 3 §3 (the `ensure` contract)
  `locate`'s four steps and `materialise`'s ten steps both omit it, and `LocateSealedTree` is declared "pure lookup" — so the acquisition, a kernel-visible side effect plus a deliberately-leaked FD, has no owner. On the `ensure` path it is worse: `ensure` is a short-lived process whose descriptor closes on exit, so a first-run or dev-override daemon runs against a tree nothing holds. Steps 9→10 are additionally reclaimable by a concurrent `prune` running under a different single-flight key.

- 🟡 **Architecture / Correctness / Security / Code quality**: `ACCELERATOR_LAUNCHER_BIN` is flagged as unsafe to export and then exported
  **Location**: Key Discoveries vs Phase 3 §3 (Discovery)
  `derive_override_var("launcher")` produces exactly that string, and `launcher` is in `RESERVED_TOKENS`. One namespace now carries "an unverified path the user vouches for" and "a path the launcher resolved", inherited by every descendant of every dispatch.

- 🟡 **Architecture / Performance / Code quality**: Tree-variable export is specified as both a directory scan and a compiled-in lookup
  **Location**: Phase 3 §3 vs Performance Considerations
  Under the disk-enumeration reading the per-dispatch cost is O(releases-ever-installed) against a shared cache root, unvalidated on-disk filenames enter the environment of every sub-binary, and the "always set or explicitly cleared" invariant is unachievable — a tree with no pointer yields no name to clear, so an injected `ACCELERATOR_TREE_BROWSER` survives exactly in the cold-cache case the clearing defends.

- 🟡 **Correctness**: §3 both forbids and performs the `release_prepare` wiring
  **Location**: Phase 2 §3: Assembly
  The section opens with "Wiring both into `release_prepare` would make the split imaginary" and closes with "Both tasks are wired into `prerelease_prepare` … and `release_prepare`". Implementing the second puts the extracting task inside the step whose `env` carries `GH_TOKEN`, defeating the rationale and failing the plan's own workflow test.

- 🟡 **Correctness**: `get_to_writer(&mut impl Write)` cannot express the per-attempt truncation the same paragraph requires
  **Location**: Phase 1 Step 1a §2: Streaming download
  Truncation needs `File`/`Seek`, and the incremental digest is caller state the fetcher cannot reset — so an implementer following the signature gets exactly the append bug the paragraph diagnoses.

- 🟡 **Correctness / Test coverage**: The layout version is defined but enforced nowhere
  **Location**: Phase 1 Step 1b §2
  `locate`'s name grammar has no layout-version field, and the reuse scan admits any `trees/<name>-<platform>-D-*` with no version comparison — so a launcher shipping a policy fix silently adopts the pre-fix tree, and `verify` passes because it checks the old table. No criterion covers older-layout re-materialisation or higher-version refusal.

- 🟡 **Architecture**: Single-flight uses the pid discipline the plan rejects two paragraphs later
  **Location**: Phase 1 Step 1b §2
  Single-flight reuses `bin/accelerator`'s PID-owner staleness discipline while the lease uses `flock` precisely because a pid gate "is a repeat of a documented failure". The weaker mechanism sits on the more critical path, and a crashed winner's sentinel blocks every cold materialisation behind a heuristic whose expiry triggers a session-wide downgrade.

- 🟡 **Correctness / Performance**: The sticky failure marker conflates a transient lock timeout with a persistent failure
  **Location**: Phase 3 §3; Phase 1 Step 1b §2 (single-flight)
  A loser that times out while the winner is legitimately downloading writes the marker, so invocations 3–200 take the code-only path for the rest of the crawl even though materialisation succeeded. On a slow link the timeout is the *likely* outcome, so the first crawl on the machines the artifacts exist to serve degrades permanently.

- 🟡 **Test coverage**: The marker's clearing and TTL paths are untested, and a stuck marker permanently degrades a repaired machine
  **Location**: Phase 3 Success Criteria
  Only the suppression direction is asserted. Nothing tests that a successful `ensure` clears it, that `cache repair` clears it (so the remediation string in the envelope is actually effective), or that the TTL expires. Suppression is the safe direction to get wrong; clearing is not.

- 🟡 **Correctness / Performance / Safety**: `prune` cannot reclaim anything in the shared-cache mode it exists for
  **Location**: Phase 1 Step 1c §1; ADR-0063
  Pointers are `<name>-<platform>-<version>.ref`, one per release version, and nothing removes stale ones — so every superseded generation stays referenced and `prune`'s predicate never fires. A launcher also cannot distinguish a pointer left by an uninstalled older version from a live sibling install.

- 🟡 **Architecture / Portability / Test coverage / Security**: `ASSEMBLED_SHA256` couples every release to reproduction factors the refresh procedure does not cover
  **Location**: Phase 2 §8
  The digest depends on the DEFLATE encoder and level, tar PAX/GNU header choices, `requests`/Python versions, umask, readdir order and locale — none named as refresh triggers, and the refresh machine is not required to match the release runner. A runner image bump or a normalisation refactor makes the project unreleasable, which is the failure mode §8 exists to remove. The determinism test (assemble twice, same process, same host, same second) is invariant to every one of those factors.

- 🟡 **Safety / Performance**: The buffer-versus-prehash decision is unresolved and changes peak RSS by an order of magnitude
  **Location**: Phase 1 Step 1a §2
  Left open, the default outcome is reading a ~120MB archive into a `Vec<u8>` inside exactly the memory-limited containers AC6 and AC11 target, where the likely result is an OOM kill mid-materialisation rather than a diagnosable downgrade.

- 🟡 **Portability**: The platform boundary omits the two failure modes that dominate on real glibc hosts
  **Location**: Phase 3 §4
  Neither the glibc **version** floor (CentOS 7, Debian 10, Ubuntu 18.04 pass both observations and then fail with `GLIBC_2.xx not found`) nor Chromium's shared-library set (`libnss3`, `libatk`, `libgbm`, `libasound2` — the reason upstream ships `--with-deps`, and a negative consequence ADR-0057 records explicitly) is probed. Both produce the ~294MB-then-opaque-failure outcome the probe exists to prevent.

- 🟡 **Correctness**: The platform classifier has no branch for an absent, unreadable or static `/bin/sh`
  **Location**: Phase 3 §4
  Distroless and scratch images — which is what the AC6/AC11 fixtures are — have no `/bin/sh`; busybox-static on a glibc host has no `PT_INTERP`. None of the six enumerated test shapes covers them, and the observation type has no "unobservable" representation.

- 🟡 **Portability**: Three of four published platforms are never executed before publication
  **Location**: Phase 2 §8
  The release job runs on `macos-latest` (arm64), so only `darwin-arm64` is smoke-executed; `darwin-x64`, `linux-x64` and `linux-arm64` — the targets the whole vendoring exercise exists for — get only a header-and-architecture check.

- 🟡 **Portability**: Zip-to-tar assembly may silently drop Unix modes and symlinks
  **Location**: Phase 2 §3
  Python's `zipfile` does not apply `external_attr` permission bits on extract and materialises symlinks as regular files. A headless shell that lost its executable bit passes the structural check, sha256, minisign and `ASSEMBLED_SHA256`, is sealed at `0444`, and fails at `execve` with `EACCES` — unrecoverable without a new release.

- 🟡 **Security**: A repo-tracked config key executes an attacker-named binary, with the fix deferred
  **Location**: Phase 3 §5; Removal sweep §5
  `design.browser_path` in `EXTRA_KEYS` is readable from the **team** level — `.accelerator/config.md`, repo-tracked — and Phase 3 §2 passes it into `chromium.launch({ executablePath })`. Opening an untrusted repository and running the inventory skill executes a binary that repository chose. The plan identifies this precisely and ships the executing behaviour while deferring the mitigation to an unraised follow-up.

- 🟡 **Security**: An offline pinned keyring cannot observe upstream revocation
  **Location**: Phase 2 §2
  GnuPG emits `REVKEYSIG` only when the *local* keyring carries the revocation certificate, so a key revoked upstream after our snapshot yields `GOODSIG`. The criterion "a `SHASUMS256.txt` signed by a revoked key fails the release" passes in tests and never fires in production — the single case the plan says rotation matters most for.

- 🟡 **Security**: The trust-anchor review gate has no enforcement
  **Location**: Phase 2 §2
  The keyring↔allowlist guard detects an inconsistent edit but not a wholesale substitution of both in one PR — the same self-referential weakness the plan correctly diagnoses elsewhere — and a CODEOWNERS file enforces nothing without branch protection requiring code-owner review.

- 🟡 **Security**: The `ASSEMBLED_SHA256`-closes-substitution claim shares a failure domain with the threat
  **Location**: Phase 2 §3
  `pins.py` and its enforcement call site live in the working checkout — exactly what a path-traversal escape targets, as the plan itself says when motivating out-of-checkout staging. An escape defeats the digest gate in the same run, before the Sign step, so the accepted residual is larger than recorded.

- 🟡 **Security**: The npm registry signature is not bound to the downloaded bytes
  **Location**: Phase 2 §2
  The registry signature covers a packument metadata string, not the tarball. The SLSA subject digest is bound to the fetched bytes but the registry check is not, so without an explicit sha512-against-signed-`integrity` step the defence-in-depth collapses onto SLSA alone.

- 🟡 **Test coverage**: The SLSA criteria test our parsing, not the predicate
  **Location**: Phase 2 Success Criteria
  With an injected runner, dropping `--owner`/`--repo` from the real invocation would leave every SLSA criterion green while removing the check the plan says the whole thing is "only as strong as".

- 🟡 **Security**: The pinned runtime ages silently with all monitoring deferred
  **Location**: Removal sweep §5
  A full browser engine and Node runtime ship to every user, pinned by hash, with §8's reuse skipping re-verification entirely while pins hold. `cargo-deny` covers Rust crates only, trees are exempt from re-verification, and the only remediation path is a human noticing.

- 🟡 **Test coverage / Performance**: Five thresholds are stated as criteria with no value and no owning phase
  **Location**: Phase 1/2/3 Manual Verification; Step 1a §2; Phase 2 §5 arm 1 and §7
  The fetch deadline, the inflate ceiling, the first-run wall clock, `sign_file`'s timeout and `download_release_asset`'s timeout. A criterion whose bound is undetermined cannot fail; the two timeout re-derivations are precisely the ones that, left inherited, produce the release-burning `TimeoutExpired` path §7 spends a section on.

- 🟡 **Test coverage**: The warm-path criterion rejects two unsound gate shapes and substitutes nothing
  **Location**: Phase 1 Success Criteria
  No margin, no sample count, no statistic, no pass rule — and it sits under Automated Verification while requiring a pre-Phase-1 binary no `mise run test:*` task can produce. Given 0205 recorded a 1.28 ratio, the plan should also say what happens if a regression is measured.

- 🟡 **Architecture / Performance**: Phase 1 and Phase 2 are declared independent but Phase 1's constants come from Phase 2
  **Location**: Implementation Approach vs Step 1a §2 and Step 1b §1
  The fetch deadline, the decompression-bomb realism and the throughput figure all derive from Phase 2's measurements. Either Phase 1 merges with placeholders nothing forces anyone to revisit, or the stated parallelism is illusory.

- 🟡 **Test coverage**: Phase 2's most consequential gates are only exercisable inside the release job
  **Location**: Phase 2 §8 and Success Criteria
  Reuse, `ASSEMBLED_SHA256`, smoke and structural checks are asserted mostly as *workflow shape* — pinning the YAML without proving the gate fires — with no negative test anywhere. First exercise is inside the concurrency lock after the version bump has been pushed.

- 🟡 **Test coverage**: Crash injection, single-flight and lease inheritance name no test seam
  **Location**: Phase 1 Success Criteria
  Three ports at whole-operation granularity cannot express "fail between step 9 and step 10", so implementers will hand-construct the post-crash states (testing the reaper, not the sequencing) or synchronise with sleeps — in a repo with documented flake history in exactly this territory. `PROBE_ATTEMPTS` is also `thread_local!`, so `probes_during` is blind to threads a concurrency test spawns.

- 🟡 **Test coverage**: The `skip_if_no_minisign!` guard is lifted from the wrong half of the suite
  **Location**: Phase 1 Success Criteria; Testing Strategy
  The unsigned tree tests are correctly exempted, but the signature and end-to-end tests — everything exercising the attestation — are directed to keep the guard, which returns `Ok(())` with only an `eprintln!`. `minisign` is pinned in `mise.toml:35`, so making absence a hard failure costs CI nothing.

- 🟡 **Test coverage**: The stale-reference criterion has no mechanical guard, in a file that already rotted undetected
  **Location**: Phase 3 §6
  The plan records that eleven of `benchmark.json`'s stale references are stale *today* because the sibling missed the file, and that nothing in CI inspects its content — then makes the fix a one-shot state assertion with no guard introduced.

- 🟡 **Test coverage / Portability**: The binary-size gate runs only in the release lane
  **Location**: Phase 1 Step 1b §1
  Cross-compiled artefacts exist only in the `prerelease`/`release` jobs, so the ceiling cannot fail on the PR that adds `tar`+`flate2` — it fails at release time, blocking a cut rather than a change. The feature-graph absence guard has the mirror problem: `cargo tree` runs for the host triple only, so a target-specific C-backend edge would not appear.

- 🟡 **Safety**: A job timeout's documented `--clobber` recovery is unreachable
  **Location**: Phase 2 §7
  `release_prepare` begins with `git.pull` then `version.bump`, so re-running after a timeout bumps to the *next* version against the already-pushed commit; `--clobber` only helps if `release:finalise` is re-invoked against the same staged `dist/release/`, which no workflow entry point offers.

- 🟡 **Safety / Performance**: `locate` runs on every tool call against a user-relocatable path with no bound
  **Location**: Phase 3 §3; Performance Considerations
  The cost is charged to `accelerator vcs guard`, a PreToolUse hook. A `stat` against a hard-mounted NFS path blocks uninterruptibly in the kernel, so a cache root that becomes unresponsive wedges every Claude Code tool call — a full-session outage from a configuration the documentation encourages.

- 🟡 **Performance**: The release job's dominant new cost is serial transfer, which §8's reuse does not touch
  **Location**: Phase 2 §7 and §8
  `upload_and_verify_release` uploads and re-verifies in serial per-asset subprocess loops: ~480MB up and ~480MB back down per pass, doubled. §8 removes re-assembly CPU only, and as written may *add* a download for the in-job second pass whose bytes already sit in an uncleaned `dist/release/`.

- 🟡 **Code quality**: The `tree.rs` decomposition is asserted but never specified
  **Location**: Phase 1 Step 1b §2/§3/§4
  Every **File** header names one file while the prose promises four seams. Followed literally, one module owns layout, pointer validation, attestation, the table, download, the allowlist, mode masking, sealing, locking, the lease and reaping — and `repair`, which the plan says needs several independently, has nothing clean to reuse.

- 🟡 **Code quality**: The `Refusal`/`Failed` mapping is called load-bearing and then deferred
  **Location**: Phase 1 Step 1b §3
  Whether a path-escape rejection hard-fails a crawl or is silently swallowed under `--fail-safe` becomes an implementation accident. The two tests pinning the existing mapping are hand-maintained `vec![]` literals with no exhaustiveness link, so an omitted variant compiles, passes and ships unclassified.

- 🟡 **Code quality**: Instructions to record reasoning in doc comments will produce exactly the stale-prone comments CLAUDE.md forbids
  **Location**: Throughout (at least seven sites)
  The sections carrying those instructions express their rationale almost entirely through ADR, work-item and phase citations, plus host-specific measurements. A transcribed 29.92ms bootstrap figure or a `per work-item:0214 SQ-2` reference is stale the moment the host or the numbering changes.

- 🟡 **Code quality**: The `resolve_optional` extraction target is unresolvable as written
  **Location**: Phase 3 §5
  It returns `Result<_, ComposeError>`, which wraps a visualiser-only `PatternError`, and the plan simultaneously rules out the natural home by quoting `config-adapters`' one-env-var rule. The implementer must invent a crate (carrying the library-crate registration checklist), rework the error type and retarget the visualiser's callers — none of it budgeted.

- 🟡 **Code quality**: Three time-dependent behaviours have no clock seam
  **Location**: Phase 1 Step 1b §2; Phase 3 §3
  The reaper's age backstop, the waiter's deadline and the sticky marker's TTL. The codebase already has the pattern (`design::executor::ports::Clock`, `corpus::metadata::Clock`); without it the tests must sleep or back-date, and the marker decision cannot be unit-tested in the domain crate under its pup rule at all.

- 🟡 **Code quality**: The GPG check has no seam between invoking `gpg` and classifying its output
  **Location**: Phase 2 §2
  The revoked-key and expired-key negatives — the cases a naive `VALIDSIG` check gets wrong — become tests requiring crafted keyrings and a particular host GnuPG, which is the same shape as the `skip_if_no_minisign!` trap the plan rightly criticises.

- 🟡 **Architecture**: Two adjacent paragraphs say opposite things about the `ACCELERATOR_DESIGN_BIN` path
  **Location**: Phase 3 §3
  One says the override reaches `ensure` exactly as a cold cache would; the other says it reports `artifact-unavailable`. Under the second reading no developer can exercise tree materialisation without a full release cycle — which is also the configuration the container fixtures are most likely built in.

- 🟡 **Architecture**: The launcher has no channel to export environment to the exec'd child
  **Location**: Phase 3 §3
  `ExecBinary::exec(&self, program, args)` takes no environment and `run_external` composes exactly resolve + exec. The default implementation becomes `std::env::set_var` inside an outbound adapter — a hidden global mutation on a path described as a pure query, untestable in-process without env races.

#### Minor

- 🔵 **Security**: `locate` uses `stat` (follows symlinks), never checks the pointer file's ownership, and never checks `trees/` or the cache root — see the tradeoff analysis above.
- 🔵 **Security**: Extraction rules omit the mechanics tar CVEs turn on — per-component `openat`/`O_NOFOLLOW`, explicit discard of archive uid/gid/mtime/xattrs, entry-name length and charset policy. Duplicate-path and PAX long-name cases appear in the Testing Strategy but not in the rules.
- 🔵 **Security / Safety**: The sticky marker lives inside the repository being inventoried, which for this skill is routinely untrusted, with no symlink or ownership validation — a cheap way for a repo to suppress its own design findings.
- 🔵 **Architecture / Code quality**: The extraction allowlist is duplicated in Rust and Python with nothing binding the two; a shared adversarial-tarball fixture corpus both suites iterate would close it.
- 🔵 **Correctness**: The generation suffix has no specified generator, and step 9 asserts no collision case can arise — `rename(2)` onto a non-empty directory is `ENOTEMPTY`, and this repo has already been bitten by pid reuse in cache paths.
- 🔵 **Correctness**: `bare_sha256` is an inherent method on `PlatformEntry`, not reusable by `ArtifactPlatformEntry` as stated, and the sentinel-digest case conflicts with the three-sizes-never-defaulted rule.
- 🔵 **Correctness**: `repair`'s disposal of the superseded generation is specified two ways (`… → reap` in §3, "left for `prune`" in §1c) — and under the `prune` reading the one criterion exercising the lease passes vacuously.
- 🔵 **Correctness**: Count and citation drift — `benchmark.json` has 21 enumerated references, not fifteen; `ResolutionError` has fifteen variants, not sixteen; and `cli/Cargo.toml`, `cli/pup.ron` and `tasks/test/integration.py` line numbers are off by 1–20 throughout.
- 🔵 **Performance**: The `cache verify` budget quotes ADR-0060's *full Chromium* row (297MB/327 files); the shipped set is ~118ms combined, and neither figure transfers to an x86_64 host without SHA extensions or a cold page cache — which matters because `repair` runs `verify` first.
- 🔵 **Performance**: The version-keyed pointer forces two HTTPS GETs plus a signature verification after every plugin upgrade even when the reuse scan will find the tree already sealed — and fails outright offline, a weaker property than `cache.rs:1-6` documents for single-file binaries.
- 🔵 **Performance**: First run is fully serialised across the two artifacts, each in its own launcher process building its own `Fetcher`, rustls provider and TLS handshake, when the two are entirely independent.
- 🔵 **Performance**: The inflate-backend concern is likely mis-weighted — ~294MB through `miniz_oxide` is one to two seconds against a ~120MB download and ~294MB of writes — and its "ceiling" has no number.
- 🔵 **Performance / Test coverage**: The container lane materialises real ~294MB artifacts per run with no time budget, potentially under aarch64 emulation, and work-item:0208 may adopt it as an every-build job.
- 🔵 **Portability**: `flock` semantics are not uniform across NFS, SMB, FUSE and overlayfs; the plan states no behaviour for `ENOLCK`/`EOPNOTSUPP` on a relocated cache root.
- 🔵 **Portability**: The redirect allowlist is compiled-in `github.com` + `*.githubusercontent.com`, so `ACCELERATOR_RELEASE_BASE_URL` does not survive a redirecting internal proxy — a mirror hatch that fails exactly where ~1.2GB per release makes one necessary.
- 🔵 **Portability**: No pre-seed or air-gapped provisioning route for ephemeral environments (CI agents, devcontainers, Codespaces), despite `cache verify` being deliberately offline-capable.
- 🔵 **Portability**: The container lane's roll-up membership is unstated; a hard-failing Docker preflight inside the default `mise run` would make Docker Desktop mandatory for every contributor, against the `test:e2e:visualiser:docker` precedent.
- 🔵 **Portability**: Two cross-platform mechanisms are left as alternatives — the progress floor (only the watchdog is genuinely portable; `SO_RCVTIMEO` is not reachable through blocking reqwest over rustls) and the `#[cfg(not(unix))]` discipline, where both neighbouring modules keep marker arms.
- 🔵 **Safety**: `prune`'s legacy-cache report emits a ready-to-paste removal command for a path derived from an env var nothing else validates after Phase 3.
- 🔵 **Safety**: ADR-0063's ~14-day orphan sweep leaves a prerelease-tracking user accumulating hundreds of MB per upgrade with no signal, no ceiling and no first-party reclamation on the default root.
- 🔵 **Test coverage**: The "each unexpired" keyring assertion is a clock-dependent unit test that will spontaneously redden CI for every contributor on the day a key expires.
- 🔵 **Test coverage**: New workflow-shape assertions should join the two existing `*_rejects_known_bad_shapes` parametrisations, or a renamed step makes the `GH_TOKEN`-absence and `permissions: {}` guards vacuous.
- 🔵 **Test coverage**: "Failing tests first" as the opening bullet of each phase is unfalsifiable at validation time; naming the specific first red test per phase would evidence the loop.
- 🔵 **Architecture**: The `ensure` subprocess's position relative to `design-cli`'s `launcher.lock` (`executor.rs:160`) is unstated — invoked inside it, a first-run download blocks concurrent design invocations behind a lock reporting `another-launcher-running`.
- 🔵 **Architecture / Code quality**: The Phase 3 failure-ordering state machine has no named domain owner and will accrete in `design-cli/src/executor.rs`, contradicting that crate's own "nothing here decides anything" invariant; the new `design-adapters` platform module also falls outside the `design_adapters_read_in_process` pup rule.
- 🔵 **Code quality**: `tasks/vendor/verify.py` holds three unrelated trust protocols and `assemble.py` seven concerns; the sibling operations sit in different namespaces (`vendor.*` and `build.*`), and registering a `vendor` collection in `tasks/__init__.py` is in no Files list.

#### Suggestions

- 🔵 **Performance**: `reqwest = "=0.12.28"` exposes `ClientBuilder::read_timeout` — a genuine idle bound — which may make the hand-rolled watchdog unnecessary; also note `TOTAL_TIMEOUT` is per attempt and `MAX_ATTEMPTS` is 3, so an enlarged deadline triples.
- 🔵 **Test coverage**: Promote the rewritten-`.files`-row case from the Testing Strategy into a criterion — it pins the attestation↔table binding the whole layout depends on.
- 🔵 **Safety**: `_assert_staged_manifest_is_current`'s new arm should assert the full `(artifact, platform)` cross-product, not key-set equality — a manifest with three of four platforms passes as written.
- 🔵 **Architecture**: Lift entry classification into `launch::core` as a pure function over a described entry, making the eight-case rejection matrix a table-driven unit test and reducing the adapter to read-classify-write; add a pup rule for `resolve::tree` while there.
- 🔵 **Architecture**: Have the dispatch composition path accept only `&impl LocateSealedTree` so the wrong port cannot be threaded into it.
- 🔵 **Portability**: Parametrise the feature-graph guard over all four target triples rather than the host.
- 🔵 **Security**: Add a PR-triggered `permissions: {}` job that reproduces `ASSEMBLED_SHA256` on pin changes, so the reviewer compares two machine-produced values rather than trusting one laptop.

### Strengths

- ✅ The three narrow ports (`LocateSealedTree` / `MaterialiseTree` / `VerifyTree`) keep the forbidden warm-path fetch out of reach by type rather than discipline, and `repair` as a use case over them mirrors `run_external` over `ResolveBinary` + `ExecBinary`.
- ✅ Refusing to route trees through `ResolveBinary::resolve` is correctly reasoned — that port's contract is name → executable path handed to `Command::exec`, and its per-exec re-verify is exactly what trees are exempt from.
- ✅ Content-addressed generations eliminate the rename-collision branch entirely: no already-present case, no winner-versus-leftover disambiguation, and a repair can build a complete replacement beside a tree a live daemon is reading.
- ✅ The retry-into-a-caller-owned-sink corruption trap is diagnosed correctly and non-obviously — a transient blip would otherwise become a permanent, unrecoverable checksum mismatch.
- ✅ The GPG predicate is specified at exactly the right paranoia level: not the exit code, not `VALIDSIG` alone, but `GOODSIG` plus explicit rejection of `EXPKEYSIG`/`REVKEYSIG`/`EXPSIG`/`NO_PUBKEY`, compared against the primary-key fingerprint.
- ✅ The binary-size budget refuses to back-derive a marginal slope from work-item:0186's composite point and specifies a direct two-point measurement under 0205's method — and the code agrees that the warm bootstrap makes exactly one O(size) pass over the launcher.
- ✅ The `flate2` pure-Rust backend pin is identified as load-bearing for the static musl cross-build, and the feature-unification hole is closed by adding `libz-sys`/`zlib-ng-sys`/`zlib-sys` to the existing `_ABSENT` tuple.
- ✅ The platform classifier is a pure function over injected observations, unit-tested across six host shapes including macOS, with the Linux gate applied at compile time — container-free portability testing done properly, and musl-first ordering correctly beats a present `gcompat` loader.
- ✅ Per-entry sha256 is computed inline during extraction, so the `.files` table costs no second pass over ~294MB; the decompression-bomb ceiling is enforced against running totals, and the three sizes are required rather than defaulted precisely because a defaulted 0 fails open.
- ✅ The single-flight lock is justified quantitatively (~588MB of transfer and ~1.2GB of transient disk avoided) and explicitly contrasted with `cache::store`, which needs none at ~8MB.
- ✅ Both unit-lane floor movements and the config suite floor are identified with correct arithmetic, and the case floor must be read off the TAP summary rather than guessed.
- ✅ The `skip_if_no_minisign!` false-green hazard is identified and forbidden for the new unsigned tests; `Route::Stall` is correctly required to stop sending rather than trickle.
- ✅ Drift tests are two-sided by construction throughout — `BUILTIN_SUBCOMMANDS` ↔ clap, `TREE_ARTIFACTS` ↔ the Rust set ↔ `manifest.example.json`, downgrade variants ↔ goldens with orphan detection.
- ✅ The publish path is derived from one registry across signing, manifest, upload and re-verification, with an explicit criterion that an unassembled artifact fails at *signing* rather than after a signed manifest is published.
- ✅ Reuse across cuts is authenticated against our own published signature plus a committed digest rather than an evictable, cross-workflow-writable CI cache, with the accidental-poisoning path reasoned out explicitly.
- ✅ `ACCELERATOR_CACHE_DIR` is escalated from "a longer-lived location" to trust-relevant documentation, and the plan removes shell rather than adding it, shrinking the bash 3.2 surface.

### Recommended Changes

1. **Resolve the CI job topology once, at plan level** (addresses: the smoke-gate critical, the `release_prepare` contradiction, the `ASSEMBLED_SHA256`-independence claim, three-of-four-platforms-unexecuted)
   Move verification, assembly and the smoke check into an upstream job with `permissions: {}` that publishes the archives as workflow artifacts; have `release` `needs:` it, download them, and check against `ASSEMBLED_SHA256` before signing. That satisfies §3's "the step handling untrusted input holds no credential" more strongly than a step boundary, preserves version monotonicity inside `release`, makes the smoke gate a plain dependency, and lets the smoke job be a four-runner matrix. Then delete the `release_prepare` wiring sentence.

2. **Specify the producer side of the attestation, and make it bind the tuple** (addresses: all three attestation criticals plus the test-coverage critical)
   Add to Phase 2 §5 a per-artifact-per-platform signed statement — artifact name, platform, release version, archive sha256, layout version, sizes, table digest — signed with `minisign -S`, published, uploaded and re-verified alongside the `.tar.gz`. Store it as the `.sealed` file. Add signature verification as an explicit numbered step in `locate`, restate the cost line, and have `locate` check every field against what it is resolving for. Add criteria for: untrusted-keypair attestation is a miss; a rewritten `.files` row is detected; a pointer naming another artifact, platform or older version is refused.

3. **Add the Node program retarget and an absolute-path ESM import to Phase 3** (addresses: both Phase 3 criticals)
   Name `DaemonSpawner.program` and `ExecClient.program` in §3's edit set, derived from the driver tree, and retire the `NODE` constant so a bare-`node` spawn is unreachable. Keep an absolute-path import mechanism — a slimmed loader taking the driver tree root, or `import()` of a `pathToFileURL` — rather than relying on `NODE_PATH`, which ESM ignores. Consider collapsing program + environment into one resolved-runtime value so there is genuinely one threading site.

4. **Settle the lease: placement, acquisition site, and degraded filesystems** (addresses: the lease critical and its four major dependents)
   Adopt ADR-0061's sidecar placement and add it to the layout block. Name the acquisition explicitly — held across `materialise` steps 9–10 before the rename, and taken on the path that will exec a consumer — and either widen `LocateSealedTree` to admit the effect or split it into a distinct port. Make `ensure` return or re-establish the lease so the cold and dev-override paths are covered. Require `reap_orphans`/`prune` to hold the same single-flight key. State the `ENOLCK`/`EOPNOTSUPP` fallback.

5. **Remove the tag-deleting arm rather than narrowing it** (addresses: the marketplace-ref critical, the unreachable `--clobber` recovery)
   A preserved draft plus the existing forensic alert is strictly safer once the bump is pushed. Add a criterion that no path deletes a tag `git.push` has published, add a re-drivable finalise entry point (a `workflow_dispatch` taking an explicit version and skipping the bump), and size `timeout-minutes` against a measured double pass.

6. **Make the container lane a first-class Phase 3 deliverable** (addresses: the container-fixture critical, the unbudgeted-runtime and roll-up-membership minors)
   Name the image definitions, the invoke task, the `main.yml` job and the workflow-shape test. Serve test-key-signed miniature trees from a container-reachable HTTP fixture rather than the production release host, so the lane runs pre-merge. Follow the `test:e2e:visualiser:docker` precedent for roll-up exclusion, and state the base image and package set AC6 requires.

7. **Settle every deferred mechanism decision in the plan** (addresses: eight major findings)
   The `Refusal`/`Failed` mapping per new variant with a one-line rationale each; buffer versus `-H` prehash with a stated peak-RSS ceiling; the watchdog (or `reqwest::read_timeout`) as the progress-floor mechanism; the compiled-in artifact set as the export source, with the disk-enumeration wording deleted; a name outside the `ACCELERATOR_<SUB>_BIN` grammar for launcher discovery; `flock` for single-flight too; the `resolve_optional` target crate and error shape; and the symlink-admission question per platform.

8. **Instantiate the thresholds or assign them an owner with a criterion** (addresses: the deferred-threshold major, the warm-path criterion, the Phase 1/2 independence claim)
   Give Phase 1 an interim fetch deadline from its own ~120MB estimate and make "re-derive from Phase 2's measured sizes" an explicit later item with its own criterion, so the handoff is checked rather than assumed. Instantiate the warm-path gate (statistic, n, margin, how the baseline binary is obtained) or move it to Manual Verification with a numeric bound, and say what happens if a regression is measured. Re-derive `sign_file`'s and `download_release_asset`'s timeouts as named numbers.

9. **Extend the platform boundary and harden the cache-root checks per the tradeoff** (addresses: the glibc-floor major, the classifier-input major, the security/portability tradeoff)
   Add a glibc version observation and a shared-library presence check derived from the assembled binary's `DT_NEEDED` list, each with its own remediable downgrade reason. Make the shell-interpreter observation three-valued with an explicit `Unobservable` classification and add distroless to the test shapes. Root the ownership check at the launcher-created `0700` `trees/` directory, and harden it with `symlink_metadata`/`O_NOFOLLOW`, pointer-file ownership and per-component resolution.

10. **Move `design.browser_path` to the personal config level in Phase 3** (addresses: the repo-tracked RCE major)
    Read it from the Personal level only, or refuse a value canonicalising inside the repository being inventoried, with a precedence test over team-set/personal-set. Keep the `visualiser.*` audit as the follow-up.

11. **Give the trust anchors real enforcement and an age bound** (addresses: three security majors, the clock-dependent test minor)
    Specify the enforcing mechanism concretely — a required CI job failing on any `keys/**` or `pins.py` diff without an explicit trust-anchor approval, plus branch protection. Bind the npm registry signature to the tarball's sha512. Add a keyring-age obligation with a scheduled guard rather than a hard unit test, and reword the revoked-key criterion to say what is actually verified. Bring a minimum pin-age tripwire into Phase 2 rather than deferring all vulnerability monitoring.

12. **Name the module decomposition and add the missing seams** (addresses: four code-quality majors, the clock and pup minors)
    Replace the single `tree.rs` header with the concrete module set and each module's responsibility. Introduce `ResolutionError::Tree(TreeError)` with a `const fn class()` the compiler forces exhaustive. Name a clock port for the reaper, the waiter and the marker TTL. Split a pure `classify_status_lines` out of the GPG check. Add `platform` to the `design_adapters_read_in_process` pup rule, and name the domain module owning the Phase 3 failure ordering.

13. **Add one instruction about comments, and prune the redundant ones** (addresses: the doc-comment major)
    State that where a doc comment is genuinely warranted it must be a self-contained statement of the constraint — no ADR, work-item or phase reference, no host-specific measurement — and remove the "record this in a comment" instructions where a named constant already carries the meaning.

14. **Correct the arithmetic and citations** (addresses: the count-drift minor)
    `benchmark.json` carries 21 enumerated references (15 already stale today), not fifteen; `ResolutionError` has fifteen variants; and the `cli/Cargo.toml`, `cli/pup.ron` and `tasks/test/integration.py` line numbers need re-deriving. Phrase the `benchmark.json` criterion as a grep assertion rather than against a count, and make it a standing guard rather than a one-shot state.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: This is an unusually rigorous plan: the port split in `launch::core`, the content-addressed generation layout, the producer/consumer contract pinned in a shared fixture, and the explicit refusal to route trees through `ResolveBinary::resolve` all show a clear grasp of the seams involved. The structural weaknesses are concentrated in three places — a CI job topology that cannot be built as described, several internal contradictions where a mechanism is specified one way in one section and the opposite way in another (the flock lease, the env-var source, the dev-override path), and two missing seams (how tree state reaches the exec'd child, and where the lease acquisition lives) that will be resolved ad hoc at implementation time if the plan does not resolve them. The dependency direction, error-taxonomy mapping and domain vocabulary are otherwise sound.

**Strengths**:
- The three narrow ports instead of one broad `ResolveArtifactTree` is exactly the right call: it keeps the forbidden behaviour out of reach by type rather than by discipline, and puts `repair` in `launch::core` as a use case over ports, mirroring `run_external` over `ResolveBinary` + `ExecBinary`.
- Refusing to route trees through `ResolveBinary::resolve` is correctly reasoned — that port's contract is name → executable path handed straight to `Command::exec`, and its per-exec re-verify is precisely what trees are exempt from.
- Tree-specific `ResolutionError` variants are added rather than folded into `Cache { path, detail }`, with each variant's mapping stated explicitly — and since `swallow_under_fail_safe` only swallows `Failed`, that mapping silently decides degrade-versus-hard-fail.
- The producer/consumer contract across the Rust/Python boundary is anchored in one artefact both sides read plus a drift test, in the same shape as the existing `BUILTIN_SUBCOMMANDS` ↔ clap pin.
- Generations as the mechanism that eliminates the rename-collision branch entirely is a genuinely elegant simplification, and the plan traces its consequences consistently.
- The `cache` verb vocabulary maps cleanly onto how a user would describe the operations, and separating offline-by-construction `verify` from mutating `repair` keeps a diagnostic usable in the degraded conditions a user reaches for it.
- Phase 2 §8 correctly identifies that wiring assembly into every cut creates a large new single point of failure, and closes the accidental-poisoning path rather than just the attack path.

**Findings**:

- **critical** (high confidence) — *Phase 2 §8: Reuse across cuts, and a functional gate (vs. Phase 2 §3: Assembly)*
  The plan places `build.assemble_tree_artifacts` as a workflow step *inside* the `release` job, and then places the functional smoke check in "a separate job with `permissions: {}`", with "the publish step gates on that job". GitHub Actions job dependencies are a DAG: the smoke job must `needs:` the release job to consume its artifacts, and the release job's publish step cannot then wait on the smoke job. **Impact**: the single strongest gate in Phase 2 — the only check distinguishing "signed" from "works" — is unbuildable as specified, and the plan's own fallback silently drops it. Compounding this, §3 rejected job isolation for assembly partly because passing ~1.2GB between jobs is "substantial", while §8 accepts that same transfer. **Suggestion**: move assembly (and verification) into an upstream job with `permissions: {}` that publishes the archives as artifacts, have the release job `needs:` it, download them, and check against `ASSEMBLED_SHA256` before signing.

- **critical** (high confidence) — *Phase 1 Step 1b §2: Tree materialisation — the in-use signal / layout*
  The plan states the lease file is "inside each generation". ADR-0061 decides the opposite: a sidecar *beside* each generation, because the seal would otherwise make it read-only for the dispatches that must take it, and `verify` would report it as an entry absent from the `.files` table. The plan's own layout block lists no lease at all, and its Step 1c criterion asserts `verify` detects "an unexpected extra entry". **Impact**: implemented as written, the lease sits in a `0555` directory of `0444` files, it is not in the `.files` table, so `verify` reports every healthy tree as corrupt and `repair` re-materialises trees that are fine — turning recovery into a loop. **Suggestion**: adopt ADR-0061's sidecar placement, add `.lease` to the layout block, and state that `verify` walks only the generation directory.

- **major** (high confidence) — *Phase 1 Step 1b §2 (`locate`) and §3 (Ports)*
  `locate` is specified as a pure query and `LocateSealedTree` as "pure lookup", but the lease requires the shared `flock` to be taken by the launcher with `FD_CLOEXEC` cleared so it survives the `exec` — and `locate` is the only launcher-side step on that dispatch. The four-step sequence never mentions acquiring it and no other component is assigned the job. **Impact**: the acquisition will be bolted into whichever function the implementer reaches first, breaking the declared contract, and every external dispatch leaks lease FDs into every child. **Suggestion**: name the step explicitly — add it as a fifth `locate` step and redocument the port, or split it into a separate port taken only on the dispatch that will exec a tree consumer.

- **major** (high confidence) — *Phase 3 §3: Tree resolution — "Warm, on every dispatch"*
  No channel exists to export environment to the exec'd child: `ExecBinary::exec(&self, program, args)` takes no environment and `run_external` composes exactly resolve + exec. **Impact**: the default implementation becomes `std::env::set_var` inside an outbound adapter — a hidden global mutation on a path described as a pure query, invisible to `run_external`'s signature, untestable in-process without env-var races. **Suggestion**: widen the seam — give `ExecBinary::exec` an environment parameter (`UnixExec` already builds a `Command`) and thread the locate result through `run_external`, or add a distinct `ExportTreeEnvironment` port.

- **major** (high confidence) — *Phase 3 §3: The `ensure` contract — Discovery (vs. Key Discoveries)*
  Key Discoveries concludes "`ACCELERATOR_LAUNCHER_BIN` is not a free name to export"; Phase 3 §3 exports exactly that name. **Impact**: one namespace carries two incompatible meanings, inherited by every descendant of every dispatch, and a future `launcher` dispatch token would silently turn a discovery hint into a trusted resolution bypass. **Suggestion**: export outside the override namespace and add a guard asserting no exported variable matches the `ACCELERATOR_<SUB>_BIN` shape.

- **major** (high confidence) — *Phase 3 §3: the `ACCELERATOR_DESIGN_BIN` path*
  Two adjacent paragraphs say opposite things: one says the executor reaches `ensure` exactly as on a cold cache, the other says it reports `artifact-unavailable`. **Impact**: under the standard local dev workflow the runtime path either works or is permanently unreachable, and the plan specifies both — the second reading also blocks the configuration the container fixtures are most likely built in. **Suggestion**: give `accelerator-design` a fallback discovery order (exported variable → `${ACCELERATOR_PLUGIN_ROOT}/bin/accelerator` → `PATH`) and reserve `artifact-unavailable` for no-launcher-by-any-route.

- **major** (high confidence) — *Implementation Approach (phase DAG) vs. Step 1a §2 and Step 1b §1*
  The plan asserts Phase 1 and Phase 2 are independent, but Step 1a's deadline "is derived from Phase 2's measured archive sizes", Step 1b §1 requires "decompression throughput over a real archive", and Phase 2's manual verification feeds sizes back into Step 1a. **Impact**: Phase 1 cannot close its own criteria, so either it merges with placeholders nothing forces anyone to revisit — the deadline and bomb ceiling being exactly the values a placeholder disables — or the stated parallelism is illusory. **Suggestion**: make the dependency explicit with a named handoff, or add a Phase 1 step producing a representative archive locally, with a Phase 2 criterion asserting the shipped sizes fall inside Phase 1's bounds.

- **major** (high confidence) — *Phase 1 Step 1b §2: Single-flight (vs. the in-use lease in the same section)*
  Two different liveness mechanisms are adopted for one subsystem within a few paragraphs: PID-owner staleness for single-flight, `flock` for the lease *because* a pid gate "is a repeat of a documented failure". **Impact**: the weaker mechanism sits on the more critical path; a crashed winner blocks every cold materialisation behind a staleness heuristic whose expiry emits `artifact-unavailable` and, via the sticky marker, disables the runtime for the session. **Suggestion**: use an exclusive `flock` for single-flight too — same crash-safety, no sentinel protocol, no waiter budget, and it composes with the lease.

- **major** (medium confidence) — *Phase 3 §3 vs. Performance Considerations*
  The two sections disagree on what drives the export: pointer files on disk versus the compiled-in artifact set. **Impact**: different costs and different coupling; the disk-enumeration reading puts unvalidated on-disk filenames into every dispatched sub-binary's environment, and both readings charge every PreToolUse and SessionStart hook with tree resolution for a subsystem only design consumes. **Suggestion**: settle on the compiled-in set and consider gating the export on the dispatched token.

- **major** (medium confidence) — *Phase 2 §8: A committed expected digest*
  `ASSEMBLED_SHA256` makes every release conditional on byte-identical reproduction, but the digest depends on far more than the pin triple — the gzip implementation and level, tar record choices, the normalisation code, and the Python/`requests` versions. **Impact**: a routine dependency bump or a normalisation refactor makes the project unreleasable, the exact failure mode §8 was written to remove. **Suggestion**: extend the refresh procedure to name assembly-implementation changes as a trigger, and add a fast default-branch CI check that re-assembles and compares.

- **minor** (high confidence) — *Phase 1 Step 1b §3 and §4*
  The plan observes that no pup rule constrains `launch::outbound` and responds by adding three ports to `launch::core` — but the decision logic (entry allowlist, path classification, mode masking, seal policy, pointer validation, layout compatibility) stays in the adapter, with no new pup rule proposed. **Impact**: the trust boundary between an untrusted archive and the filesystem is reachable only through I/O, so its eight-case rejection matrix can only be tested by constructing tarballs. **Suggestion**: lift entry classification into `launch::core` as a pure function returning admit/reject-with-reason, and add a pup rule covering `resolve::tree`.

- **minor** (medium confidence) — *Phase 3 §3: The `ensure` contract*
  A child shelling back out to its own parent binary for a multi-minute ~294MB operation is a new process topology, but the plan does not say where that call sits relative to the `FileLock` on `launcher.lock` (`executor.rs:160`), nor how `ensure`'s single-flight wait composes with the design launcher's timeouts. **Impact**: invoked inside the lock, a first-run download blocks every concurrent design invocation behind a lock reporting `another-launcher-running`. **Suggestion**: state that `ensure` runs before the lock is acquired, alongside the platform probe, and which timeout bounds it.

- **minor** (medium confidence) — *Phase 3 §4*
  The classification is placed correctly as a pure domain function, but the *ordering* state machine has no named owner — the files listed are the new domain module, a new adapter module, and `design-cli/src/executor.rs`, whose own doc comment says "Nothing here decides anything." **Impact**: the sequence accretes as `if` statements in the crate that declares it decides nothing, testable only through process invocation, while the plan's criterion asks for unit level. The same gap applies to the sticky marker. **Suggestion**: name the domain module owning the sequence with the probe, `ensure` and browser-path resolution injected as ports, and give the negative-cache policy a domain home.

### Correctness

**Summary**: The plan is unusually rigorous about state-machine hazards — it correctly identifies the retry-concatenation bug, the pointer-publish crash window, the `VALIDSIG`-accepts-revoked-keys trap, and the `gh release delete` envelope that a large-asset timeout newly reaches — and the great majority of its factual citations verify against the tree. Three logic gaps are nonetheless load-bearing: nothing in the Phase 3 edit set repoints the executor's hardcoded `node` program or preserves ESM resolution of the vendored runtime, so the plan's own Desired End State cannot be reached; and the ten-step `materialise` sequence and the `locate` hit path each omit a step the surrounding prose declares essential. Several arithmetic and cross-section contradictions survive in a revision whose stated purpose was to eliminate them.

**Strengths**:
- The corrected suite-floor arithmetic verifies exactly: `scripts/` discovers 16 `test-*.sh` files minus the excluded `test-helpers.sh` = 15 against a floor of 15, so deleting `test-design.sh` does land at 14.
- The design-automation floors verify: nine `lib/*.test.js` against a floor of 9, and 76 as an at-least floor over the TAP summary.
- The streaming-fetch retry hazard is correctly diagnosed — `get` is safe today only because `try_get` returns a fresh `Vec<u8>`.
- The release-pipeline failure-envelope analysis is accurate and non-obvious: `_release_reverifies` really is built before the `try`, `download_and_verify`'s conversion really is unused, and both re-verify helpers really do call `download_release_asset` bare with its 7.6MB-sized `timeout=120`.
- The `GOODSIG` vs `VALIDSIG` distinction, the exit-0-on-untrusted-key trap, and the primary-key fingerprint requirement are all correct.
- The `persist-credentials` trap is correctly reasoned — adding the flag without a replacement credential would wedge every cut after the version bump has been pushed.
- `Route::Stall` genuinely sends headers and then goes idle, so the stall-not-trickle fixture requirement is correct and the mechanism really is unused.

**Findings**:

- **critical** (high) — *Phase 3 §3* — **The vendored Node binary is never wired in.** `const NODE: &str = "node"` (`executor.rs:28`) is used verbatim as `program: PathBuf::from(NODE)` in `DaemonSpawner` (`:163`) and `ExecClient` (`:174`) — a bare name resolved through `PATH`. Phase 2 assembles a Node binary into the driver tree and even smoke-executes it, but no edit makes the executor use it. **Impact**: the Desired End State and AC6 cannot pass; the spawn fails with `ENOENT` after ~294MB has been fetched, sealed and verified. **Suggestion**: add both `program` fields to the §3 edit set, derive them from the resolved driver tree, state the no-tree fallback, and retire the `NODE` constant.

- **critical** (high) — *Phase 3 §1* — **`NODE_PATH` does not apply to ESM.** `package.json` declares `"type": "module"` and `daemon.js` is ESM. Node's ESM resolver ignores `NODE_PATH` entirely — the loader exists precisely because of this, building an absolute `pathToFileURL(...)` (`playwright-loader.js:63-66`). **Impact**: a bare `import 'playwright-core'` walks `node_modules` upward from the plugin tree, not into the sealed driver tree, so every crawl fails with `ERR_MODULE_NOT_FOUND` on exactly the machines the vendored runtime serves. **Suggestion**: keep an absolute-path import mechanism and retain a test that the resolved module exposes `chromium`; if a bare specifier is wanted, state the mechanism that makes it work.

- **critical** (high) — *Phase 1 Step 1b §2 (`locate`)* — **The enumerated algorithm omits the signature check the same section calls the only cryptographic anchor.** Step 4 is only "Read the attestation; its digest must equal the digest in the directory name", and the summary reads "Two small reads and two stats." No criterion asserts an unsigned or wrongly-signed attestation is refused. **Impact**: the hit path is entirely self-referential on all 100–200 dispatches per crawl. **Suggestion**: add the verification as an explicit numbered step, restate the cost line, and add a criterion (stating whether a tampered signature is a miss or a refusal).

- **major** (high) — *Phase 1 Step 1b §2* — **The layout version is enforced nowhere.** `locate`'s name grammar `<name>-<platform>-<64 lowercase hex>-<gen>` has no layout-version field, contradicting the layout spec, and the reuse scan admits any `trees/<name>-<platform>-D-*` with no version comparison. **Impact**: a launcher shipping a policy fix silently adopts the pre-fix tree from a shared root, and `verify` passes against the old table. **Suggestion**: include the version in the grammar, make unknown-or-higher a miss and a non-candidate, and add both criteria.

- **major** (high) — *Phase 1 Step 1b §2* — **Nothing acquires the lease, and steps 9→10 are explicitly reclaimable.** No step of any algorithm opens it; step 10 says the generation is "reclaimable by the reaper" until the pointer is published, and `reap_orphans` runs from `materialise` and `prune`, which are not stated to hold the single-flight lock. The reuse scan has the same TOCTOU. **Impact**: a concurrent `prune`, or `ensure driver` racing `ensure browser`, can delete a live winner's fully-materialised tree in the 9→10 window, or the tree an adoption is about to point at. **Suggestion**: state where the lease is opened, account for the extra `open`+`flock` in the hit-path cost, and require reclamation to take the same single-flight lock.

- **major** (high) — *Phase 2 §3* — **§3 both requires assembly outside `release:prepare` and wires it into `release_prepare`.** **Impact**: mutually exclusive; the second puts the extracting task inside the step whose `env` carries `GH_TOKEN`, defeating the rationale and failing the plan's own test. **Suggestion**: pick one and delete the other.

- **major** (high) — *Phase 2 §8* — **A separate smoke job cannot gate a publish step inside the monolithic release job.** `needs` is job-level; the publish sequence lives inside the single `release` job, which must not be split for version monotonicity. **Impact**: unimplementable, so the implementer either runs it beside the signing key or drops the gate. **Suggestion**: restructure so assembly + smoke happen in a separate upstream job, or accept an in-job structural-only check and record the reduced assurance.

- **major** (high) — *Phase 1 Step 1a §2* — **`get_to_writer(&mut impl Write)` cannot express the per-attempt truncation and digest reset the same paragraph requires.** Truncation needs `File`/`Seek`; the digest is caller state. **Impact**: an implementer gets exactly the bug the paragraph identifies. **Suggestion**: have the fetcher own per-attempt reset — a closure returning a fresh truncated file plus hasher, or `&mut (impl Write + Seek)` with the fetcher handing back the digest.

- **major** (medium) — *Phase 3 §3* — **The sticky marker conflates a transient single-flight timeout with a persistent failure.** Losing the lock is transient — the winner is actively materialising. **Impact**: on a first run, invocation 2 loses, writes the marker, and 3–200 take the code-only path even though materialisation succeeded; on a slow link the timeout is the likely outcome. **Suggestion**: make lock-wait expiry a distinct non-sticky cause, and add a criterion that a loser timing out while the winner succeeds does not suppress the rest of the crawl.

- **major** (medium) — *Phase 3 §3* — **The "always set or explicitly cleared" invariant is violated on the path the plan discusses.** "Never set" is not "explicitly cleared", and the override is an early `return` at the top of `LazyProductionResolver::resolve`, so no clearing code on the resolve path can run. **Impact**: with `ACCELERATOR_DESIGN_BIN` set, an ambient `ACCELERATOR_TREE_*` passes straight through and is treated as launcher-resolved. **Suggestion**: move the clear-then-set ahead of the override short-circuit and add a criterion.

- **major** (high) — *Key Discoveries vs Phase 3 §3* — **The plan flags `ACCELERATOR_LAUNCHER_BIN` as unsafe and then exports it.** `derive_override_var("launcher")` yields exactly that string, and `launcher` is in `RESERVED_TOKENS`. **Suggestion**: pick a name outside the grammar and add a collision test.

- **major** (medium) — *Phase 3 §4* — **No branch for an absent, unreadable or static `/bin/sh`.** Distroless and scratch containers — which is what the fixtures are — have none; busybox-static on glibc has no `PT_INTERP`. **Impact**: the port has no "observation unavailable" representation, so the adapter invents one, most likely conflating it with "not musl". **Suggestion**: make the observation three-valued, state which way `Unobservable` classifies, and add it to the test shapes.

- **major** (medium) — *Phase 3 §4 / Step 1c* — **Ordering forces a ~177MB browser fetch before the hatch is consulted.** AC12's criterion is ambiguous between "not materialised" and "materialised but broken", and the two readings give opposite behaviour. **Suggestion**: state the predicate precisely and pin it with a zero-browser-fetch criterion when `design.browser_path` is set.

- **minor** (medium) — *Step 1c / ADR-0063* — **`prune` cannot bound growth in the shared-root case it exists for.** A shared root accumulates one pointer per plugin version and a launcher cannot tell a stale one from a live sibling install. **Suggestion**: define reclamation over pointer age or an explicit flag, with a criterion covering two-version roots.

- **minor** (high) — *Phase 3 §6* — **`benchmark.json`: fifteen claimed, twenty-one enumerated and present.** All individual citations are correct; only the totals are wrong (and "eleven already stale" should be fifteen). **Suggestion**: correct the totals and phrase the criterion as a grep assertion.

- **minor** (high) — *Phase 1 Step 1b §3* — **`ResolutionError` has fifteen variants, not sixteen.** The associated claims (five map to `Refusal`; `swallow_under_fail_safe` swallows only `Failed`) are both correct.

- **minor** (high) — *Key Discoveries / Step 1b §1 / Phase 3 §4* — **Systematic line-citation drift.** `cli/Cargo.toml`'s size comment is at `:183-185`; `clap` at `:47`; `reqwest`/`rustls`/`minisign-verify` at `:61-67`; `regex` at `:71-73`; `tempfile`/`rand`/`libc`/`rustix` at `:86-91`; `serde-saphyr` at `:77-79`; the design pup rule at `cli/pup.ron:253`; `_EXPECTED_CONFIG_SUITES` at `:45` and `_REQUIRED_CONFIG_SUITES` at `:67`.

- **minor** (high) — *Step 1b §3 vs Step 1c* — **`repair`'s disposal is specified two ways** (`… → reap` versus "left for `prune`"), and the difference is observable: the live-reader criterion passes trivially under one reading and depends entirely on the lease under the other.

- **minor** (medium) — *Phase 1 Step 1b §2* — **The "fresh generation by construction" invariant has no generator and no collision branch.** A pid- or timestamp-derived generator collides (this repo has been bitten by pid reuse in cache paths), and `rename(2)` onto a non-empty directory is `ENOTEMPTY`. **Suggestion**: specify a CSPRNG-derived suffix or a counter under the lock, and add an explicit branch.

- **minor** (medium) — *Phase 1 Step 1a §1* — **Sentinel reuse and the required-sizes rule conflict, and `bare_sha256` is not reusable as stated.** It is an inherent method on `PlatformEntry`, not a trait, and a sentinel entry has no meaningful sizes while the three fields are required-and-never-zero. **Suggestion**: extract a shared helper, make sizes `Option` gated on the sentinel or omit the platform key, and add a sentinel criterion.

### Security

**Summary**: This is an unusually security-literate plan: it names its trust anchors, separates the credential-bearing steps from the untrusted-input steps, specifies an extraction allowlist rather than a denylist, and correctly identifies the GPG `VALIDSIG` trap and the self-referential-digest trap. The serious problems are concentrated in the one control everything else rests on — the signed attestation that anchors a cache class deliberately exempt from per-exec re-verification. As specified it cannot be verified on the hit path, and even if it could, a signature over a bare archive digest binds neither artifact identity nor release version, leaving cross-artifact substitution and silent rollback open through an unsigned pointer file. Secondary concerns: a repo-tracked config key that executes an attacker-named binary is knowingly shipped with the fix deferred, an offline pinned GPG keyring cannot observe upstream revocation, and the claim that the committed assembled-digest pin closes artifact substitution independently of CI token scope does not hold.

**Strengths**:
- Trust anchors are committed rather than fetched over the channel they validate, with explicit reasoning about why fetching the key from the registry would reproduce the problem one level up.
- The GPG predicate is specified at the right level: not the exit code, not `VALIDSIG` alone, but `GOODSIG` plus explicit rejection of `EXPKEYSIG`/`REVKEYSIG`/`EXPSIG`/`NO_PUBKEY`, compared against the primary-key fingerprint.
- The SLSA predicate is pinned to an expected source repository, workflow identity and subject digest; degraded modes must fail closed.
- Extraction is an allowlist with whole-materialisation rejection, real-root resolution, mode masking that strips setuid/setgid/sticky, and bomb bounds from signed manifest values — with the archive verified before a single entry is extracted.
- Credential scoping is a first-class design constraint, and the residual is stated plainly rather than hand-waved.
- Reuse is authenticated against our own published signature plus a committed digest rather than an evictable CI cache, with the poisoning reasoning spelled out.
- Publish-path completeness is enforced from one registry across four arms, with an explicit criterion that an unassembled artifact fails at signing.

**Findings**:

- **critical** (high) — *Phase 1 Step 1b §2 — "The attestation is signed"* — The manifest's inline signature is over the **archive file's bytes** (`tasks/signing.py:24-43` runs `minisign -S -m <file>`; `tasks/manifest.py:81-108` slurps that `.minisig`), and minisign verification — plain or `-H` — requires the message bytes, which the launcher deletes after extraction. Nothing in Phase 2 §5 produces a small release-key-signed statement verifiable from a 64-hex digest. **Impact**: the one control distinguishing a release-provenanced tree from a locally fabricated one is unimplementable; the likely outcome is a check only against the directory name — the state ADR-0061 exists to eliminate — for a cache class permanently exempt from re-verification, whose `.files` table (and therefore `verify` and `repair`) inherits the same unanchored trust. **Suggestion**: add a producer-side signed statement (JSON: artifact, platform, release version, archive sha256, sizes) signed with `minisign -S`, published/uploaded/re-verified alongside the archive, stored as the `.sealed` file; or persist the already-signed manifest fragment plus `manifest.minisig` beside the generation.

- **critical** (high) — *Phase 1 Step 1b §2 — layout, `locate`, the `.ref` pointer* — Even with a verifiable signature, only the **archive digest** is signed. Identity, platform and version live in unsigned local state; `locate` validates the pointer's *shape* then trusts the generation it names. Any process able to write `trees/` can repoint at a different artifact's, platform's, or **older release version's** generation whose signature is entirely valid. **Impact**: silent rollback of the vendored Chromium/Node to a possibly known-vulnerable version, and cross-artifact substitution, with no network, no manifest and no re-hash on the path that would notice. **Suggestion**: sign the tuple `(name, platform, release version, archive sha256, layout version)` and have `locate` check every field against what it is resolving for.

- **major** (high) — *Phase 2 §3 — "What a step boundary does and does not buy"* — The independence claim does not hold: `tasks/vendor/pins.py` and the enforcement call site live in the **working checkout**, exactly what a path-traversal escape targets (the plan itself names "a `tasks/*.py` module that the later Sign step imports"). An escape defeats the digest gate in the same run, before Sign; `actions/checkout` also persists a `contents: write` App token into that checkout, so an escape can rewrite the anchor for future cuts. **Suggestion**: read and compare `ASSEMBLED_SHA256` from outside the mutable checkout (captured into a step output before extraction, re-asserted inside Sign), or drop the independence claim; also state whether branch protection prevents the persisted token pushing to `main`.

- **major** (high) — *Phase 3 §5 and Removal sweep §5* — `design.browser_path` is readable from the **team** level (`.accelerator/config.md`, repo-tracked; only `config.local.md` is personal per `cli/config/src/level.rs:19`) and passed straight into `chromium.launch({ executablePath })`. **Impact**: arbitrary code execution from repository-supplied content, in a skill designed to be pointed at unfamiliar projects, shipped knowingly with the fix out of scope. **Suggestion**: move the restriction into Phase 3 §5 — Personal level only, or refuse a value canonicalising inside the repository — with a precedence unit test; keep the `visualiser.*` audit as the follow-up.

- **major** (high) — *Phase 1 Step 1b §2 — `locate` steps 1-3* — `stat` follows symlinks, the pointer file's own ownership is never checked, and the ancestors are never checked on the hit path; the cache-root criterion is only reachable via `materialise`, which the warm path never calls. A symlink at `trees/<name>-<platform>-<64hex>-<gen>` pointing at any user-owned non-group-writable directory satisfies every check. **Impact**: on the path that runs on every dispatch including a PreToolUse hook, a permissive relocated cache dir yields an attacker-chosen tree whose `NODE_PATH` and browser executable go to the daemon. **Suggestion**: `symlink_metadata` or `openat` with `O_NOFOLLOW` and reject symlinks outright; check the pointer's own uid and mode; validate the cache root and `trees/` modes on the hit path; state the resulting stat count so the 0189 expectations stay derived.

- **major** (medium) — *Phase 2 §2 — Node GPG verification* — GnuPG emits `REVKEYSIG` only when the **local** keyring carries the revocation, so a key revoked upstream after our snapshot yields `GOODSIG`. The criterion passes in tests and never fires in production. **Suggestion**: add a re-import obligation with a stated maximum age and a test that fails when exceeded; reword the criterion to say what is actually verified.

- **major** (high) — *Phase 2 §2 — the two mechanical guards* — The keyring↔allowlist test detects an inconsistent edit but not a wholesale substitution of both in one PR, and a CODEOWNERS file enforces nothing without branch protection. **Suggestion**: specify a required CI job failing on any `keys/**` or `pins.py` diff without an explicit trust-anchor approval, plus branch protection; make the key test assert out-of-band properties rather than internal consistency.

- **major** (medium) — *Phase 2 §2 — registry signature* — The npm signature covers a packument metadata string, not the tarball. No criterion binds it to the downloaded bytes via the signed `dist.integrity` sha512. **Suggestion**: recompute the sha512, compare against the signed integrity value, add the negative criterion, and match the `SHASUMS256.txt` line by exact filename.

- **major** (medium) — *Removal sweep §5* — All vulnerability monitoring for a shipped browser engine and Node runtime is deferred to an unraised follow-up, with §8's reuse skipping re-verification while pins hold and `cargo-deny` covering Rust only. **Suggestion**: bring a minimum viable stale-pin tripwire into Phase 2; leave advisory-feed integration as the follow-up.

- **major** (medium) — *Phase 2 §8 — the smoke-check job* — A job cannot suspend mid-way to wait on a downstream job that consumes its artifacts, and the plan does not acknowledge this. **Impact**: the control most likely reached in practice is no execution gate, or executing the vendored binaries in the job whose later step holds the signing key. **Suggestion**: run assembly plus smoke in a `permissions: {}` job upstream of `release`, digest-pinned so the transfer is untrusted-but-verified; do not leave "execute beside the signing key" as the path of least resistance.

- **minor** (medium) — *Key Discoveries vs Phase 3 §3* — The `ACCELERATOR_LAUNCHER_BIN` contradiction, with the added observation that outside a launcher-mediated dispatch the variable is inherited from the ambient environment rather than set by the launcher.

- **minor** (medium) — *Phase 2 §8 — `ASSEMBLED_SHA256`* — The strongest gate's value originates from a maintainer's unaudited laptop, and the reviewer cannot reproduce it without repeating the ritual. **Suggestion**: a PR-triggered `permissions: {}` job that reproduces the digests on pin changes.

- **minor** (medium) — *Phase 1 Step 1b §4* — Three things the tar CVE history turns on are unstated: per-component `openat`/`O_NOFOLLOW` (the actual TOCTOU closure), explicit discard of archive uid/gid/mtime/xattrs, and entry-name length/charset policy. Duplicate-path and PAX long-name cases appear only in the Testing Strategy.

- **minor** (medium) — *Phase 3 §3 — the sticky marker* — Placed inside the repository being inventoried, routinely untrusted, with no path validation. **Impact**: a repo can pre-plant a marker to silently suppress design findings for its TTL, or plant a symlink to redirect the write. **Suggestion**: reject a symlink or non-owned path, key validity to the current session, or move it to a per-user state directory.

### Test Coverage

**Summary**: This is an unusually test-literate plan: it names the exact existing harnesses it extends, identifies the `skip_if_no_minisign!` false-green hazard, and corrects two floor arithmetic errors before they redden CI. The weaknesses are at the two ends of the pyramid: Phase 1's hit-path trust anchor has no forgery/tamper criterion at all, so the plan's own cryptographic check could be deleted and every criterion would still pass; and Phase 2/3's most consequential gates are only exercisable inside the release job or in infrastructure the plan never schedules. Several criteria are also stated as thresholds whose values are deferred with no owner, or as "no regression" assertions with no margin.

**Strengths**:
- Identifies the `skip_if_no_minisign!` false-green hazard and forbids the new no-signing tree tests from inheriting it.
- Extends `probes_during` rather than working around it, stating expected counts up front.
- Recognises `Route::Stall` exists and is unused, and requires stop-sending rather than trickle.
- Both unit-lane floors and the config suite floor identified with correct arithmetic, with the case floor read off TAP rather than guessed.
- The extraction rejection set is enumerated concretely and paired with a positive nothing-extracted-before-verification assertion.
- `cache verify`'s criterion explicitly demands a same-size same-mode substitution be detected — the case a stat shortcut would miss.
- Drift tests are two-sided by construction throughout.
- The GPG predicate is at the right paranoia level for a test to pin, with named negatives for revoked and expired.
- Platform classification is a pure function over six injected shapes, with the explicit observation that a container cannot distinguish "detected musl" from "failed otherwise".

**Findings**:

- **critical** (high) — *Phase 1 Step 1b §2 and Success Criteria* — **No criterion tests the signed attestation or the file-table digest.** Apply the mutation lens: delete signature verification from `locate`, or accept any attestation whose embedded digest matches the directory name, and every listed criterion still passes. **Suggestion**: add (a) a non-verifying signature is a miss; (b) an untrusted-keypair attestation is a miss (the harness already generates two keypairs); (c) a `.files` table mutated after sealing is rejected and reported by `verify`.

- **critical** (high) — *Phase 3 Success Criteria and Testing Strategy* — **The three container fixtures carry four acceptance criteria but have no artifact source, no CI job and no file in Changes Required.** The plan concedes the artifact-serving mechanism is unresolved; the repo has one container lane today (`tasks/test/e2e.py`'s pinned Playwright image); and AC6 needs artifacts that exist only after a signed real release, so it cannot run pre-merge. **Suggestion**: make the harness a first-class deliverable with named files, and serve test-key-signed miniature trees from a container-reachable HTTP fixture; add the preflight-fails-rather-than-skips assertion, following the `docker info` precedent at `tasks/test/e2e.py:105`.

- **major** (high) — *Phase 1 Success Criteria — warm path* — Rejects two unsound gate shapes and substitutes nothing: no margin, no sample count, no statistic, no pass rule — and sits under Automated Verification while needing a pre-Phase-1 binary no `mise run test:*` can produce. **Suggestion**: instantiate the gate or move it to Manual with a numeric bound; given 0205's recorded 1.28 ratio, state what happens if a regression is measured.

- **major** (high) — *Phase 2 §8 and Success Criteria* — Most of Phase 2's ~30 criteria concern release-only behaviour, several asserted only as workflow shape, with no negative test anywhere. **Suggestion**: build the assembly path around a miniature fixture triple so determinism, digest matching, reuse fallback, `NOTICES/`, smoke and structural predicates all run in `test:unit:build-system`, each with a paired negative.

- **major** (high) — *Phase 2 §8 — determinism* — The double-assembly test runs in the same process, host and second, so it is invariant to every factor `ASSEMBLED_SHA256` actually depends on. **Suggestion**: assert the mechanisms — sorted order (shuffled input), fixed mtime/uid/gid/uname (assert the tar header values), gzip header bytes 4-7 zero, masked modes — each with a negative, and run the double assembly under a different `TZ`, `LANG` and `umask`.

- **major** (high) — *Phase 1 Success Criteria — crash/concurrency/lease* — Three criteria demand behaviour that needs a deliberate seam the three whole-operation ports cannot express, in a repo with documented flake history here. **Suggestion**: name an injectable `after_step` hook (test-only, not a cargo feature, per the plan's own `--all-features` reasoning) and specify the synchronisation primitive (rendezvous file or pipe, never a sleep). Note `PROBE_ATTEMPTS` is `thread_local!`, so `probes_during` is blind to spawned threads.

- **major** (high) — *Phase 1 Step 1b §2 — versioning* — No criterion exercises the layout/format version, despite an exactly analogous manifest test today (`manifest.rs:223-231` feeding `"future_field": 42`). **Suggestion**: mirror that discipline — higher version refused, lower re-materialised, unknown additive field still parses.

- **major** (high) — *Phase 3 §3 — the sticky marker* — Only the suppression direction is asserted; the clearing paths are not. **Impact**: suppression is the safe direction to get wrong, clearing is not — a marker that never clears silently gives the code-only crawler indefinitely to a user who freed disk or ran the documented remediation. **Suggestion**: add three criteria — successful `ensure` clears it, `cache repair` clears it, an expired marker does not suppress.

- **major** (high) — *Phase 3 §6* — The stale-reference criterion is a one-shot state assertion, in a file the plan itself records as having rotted undetected during the immediately preceding plan, with nothing in CI inspecting content. **Suggestion**: make it a guard — assert every script path and downgrade token named in `evals.json`, `benchmark.json` and `PROTOCOL.md` resolves, in the same shape as the existing "Design script references resolve" guard.

- **major** (high) — *Five threshold sites* — Each is a criterion whose pass condition is determined after the phase it gates, and the download-deadline case is circular across two phases declared independent. **Suggestion**: instantiate provisional values with their derivation, or state which phase owns each, with a Phase 2 criterion asserting the Step 1a update happened.

- **major** (medium) — *Phase 1 Step 1b §1 — size gate* — Cross-compiled artefacts exist only in the release lane, so the ceiling cannot fail on the PR that adds the dependencies. **Suggestion**: add a host-target assertion to the PR lane (the `check-cli` job already builds the launcher).

- **major** (medium) — *Phase 1 — the skip guard* — The signature and end-to-end tests are directed to keep `skip_if_no_minisign!`, so exactly the tests covering the only cryptographic anchor can silently vanish locally. **Suggestion**: fail closed under `CI`, or generate and verify fixtures in-process via `cli/verify`/`minisign-verify`. `minisign` is pinned in `mise.toml:35`, so hard failure costs CI nothing.

- **major** (medium) — *Phase 2 §2 — SLSA* — With an injected runner the criteria assert only that our code reacts to a simulated failure; dropping `--owner`/`--repo` would leave them green. **Suggestion**: add an argv-shape assertion pinning the exact flags, and capture the mock's output from a real invocation.

- **minor** (high) — *Phase 2 — "each unexpired"* — A clock-dependent unit test that reddens CI for every contributor on the day a key expires. **Suggestion**: keep set-equality as a hard test, move expiry to a scheduled guard or a warning-with-grace-period, and use a frozen clock for the revoked/expired negatives.

- **minor** (medium) — *Phase 2 — workflow shape* — `test_workflows.py` already guards this failure mode with `test_invariants_reject_known_bad_shapes` (`:342`) and `test_isolation_rejects_known_bad_shapes` (`:530`). **Suggestion**: require the new criteria to add mutations to both parametrisations.

- **minor** (high) — *All phases — "Failing tests first"* — Unverifiable at review or validation time, and the mismatch matters most in Phase 2 (YAML, constants, orchestration) and the Removal sweep (mostly deletions). **Suggestion**: name the specific first red test per phase.

- **suggestion** (medium) — *Phase 1 — `cache verify`* — The rewritten-`.files`-row case appears in the Testing Strategy but not the criteria, and the table is the oracle. **Suggestion**: promote it — it also pins the attestation↔table binding.

### Safety

**Summary**: This plan is unusually safety-literate for its size: it reasons explicitly about crash-at-every-step recoverability, decompression bombs, retry-append corruption, single-flight duplication, negative caching against a 100–200-invocation runaway, and it correctly identifies that publishing a signed manifest naming absent assets is unrecallable. The weakest area is the release pipeline's failure envelope — the plan narrows the delete arm but does not remove it, does not account for the version bump and marketplace ref already pushed before that arm can fire, and asserts a `--clobber` recovery the workflow's structure makes unreachable. The second weakest is the lifetime contract for materialised trees: the lease is specified for the launcher's exec-inheritance path but not for the `cache ensure` path the cold and dev-override flows actually use.

**Strengths**:
- The crash-at-each-step model (pointer last, everything before reclaimable) is explicit and matched by a criterion per step.
- The retry-into-a-caller-owned-sink trap is identified and closed.
- Bomb bounds are enforced incrementally against manifest-supplied values, with the three sizes deliberately required.
- `repair` builds a new generation beside the in-use tree and swaps a pointer — the correct shape for destructive recovery.
- Sticky negative caching bounds a genuine runaway (tens of gigabytes across a crawl).
- Free space is checked against archive + uncompressed summed across trees before a byte is fetched, with `disk-floor-not-met` retained.
- Phase 2 §5 derives four arms from one registry and states plainly why all are mandatory.

**Findings**:

- **critical** (high) — *Phase 2 §7* — By the time the `except Exception: gh release delete --cleanup-tag` arm runs, `_publish` has already committed, tagged and pushed — and the pushed commit carries `marketplace.update_version`'s edit setting `source.ref` to `vX`. Deleting the tag leaves `main` advertising a ref that no longer exists, breaking installs and `/plugin update` for every user. The stated mitigation fails because the `missing` check sits outside the `try` (`:339-344`), so after the narrowing there are no pre-upload failures inside the envelope at all. **Suggestion**: delete the arm outright — a preserved draft plus the existing forensic alert is strictly safer — add a criterion that no path deletes a pushed tag, and document the manual repair in `RELEASING.md`.

- **major** (high) — *Phase 2 §7* — The `--clobber` recovery for a job timeout is unreachable: `release_prepare` begins with `git.pull` then `version.bump`, so re-running bumps to the *next* version against the already-pushed commit, and `--clobber` only helps if `release:finalise` is re-invoked against the same staged `dist/release/`, which the runner no longer has and no entry point offers. **Impact**: recovery requires a manual local re-sign against the production secret — hours to days, and it puts the secret on a laptop. **Suggestion**: add a re-drivable finalise entry point, or state the manual procedure concretely and walk it once; size `timeout-minutes` against a measured double pass.

- **major** (medium) — *Phase 2 §8* — The smoke job as described is a `needs:` cycle, and the release job cannot be split. **Impact**: the realistic outcome is the weaker fallback, so a structurally-wrong-but-signed tree reaches every user, never self-heals, and is faithfully re-fetched by `cache repair`. **Suggestion**: assemble in a prior job (the artifacts depend only on pins, not the version bump), gate `release` on its smoke check, and pass archives forward — which also removes untrusted extraction from the job holding the secret.

- **major** (medium) — *Phase 1 Step 1b §2 and Phase 3 §3* — The lease's inheritance chain exists only on the warm dispatch path. On the cold path `ensure` is a separate short-lived process whose descriptor closes on exit, and the plan routes the `ACCELERATOR_DESIGN_BIN` case through `ensure` too. **Impact**: on exactly the first-run and dev-override flows, a daemon runs against a tree nothing holds, so a concurrent `prune` or `repair`'s reap would `rm -rf` files a live Chromium is still opening lazily. **Suggestion**: make the lease part of the `ensure` contract — pass the descriptor back, or have the design binary re-open and `LOCK_SH` it — with a criterion covering `prune` sparing an `ensure`-resolved tree.

- **major** (medium) — *Phase 1 Step 1b §2 — steps 9-10 and `reap_orphans`* — Between rename and pointer publication the generation is indistinguishable from crash residue, and the age backstop applies "only for generations carrying no lease file", so a freshly-renamed generation whose lease nobody has opened is eligible for immediate removal by a concurrent `prune` or by a reaper running under a different single-flight key. **Suggestion**: acquire the lease before the step-9 rename and hold it past pointer publication; apply the backstop unconditionally to unreferenced generations; add a prune-races-materialise criterion.

- **major** (medium) — *Phase 3 §3 and Performance Considerations* — `locate` runs on every external dispatch including a PreToolUse hook, against a user-relocatable cache root, with no timeout and no containment. A `stat` against a hard-mounted NFS path blocks uninterruptibly in the kernel. **Impact**: a cache root that becomes unresponsive wedges every Claude Code tool call — a full-session outage from a configuration the documentation encourages. **Suggestion**: confine the export to dispatches that actually consume trees, document the local-filesystem requirement, and treat any `locate` I/O error or slow path as "no variable" behind a hard bound.

- **major** (medium) — *Phase 1 Step 1a §2* — The buffer-versus-prehash decision is left open, and it is the one unresolved decision that changes a runtime resource bound. **Impact**: the buffered default reads ~120MB into a `Vec<u8>` in exactly the memory-limited containers AC6 and AC11 target, where the likely outcome is an OOM kill mid-materialisation. **Suggestion**: settle the signing form before Phase 1 starts; if buffering is accepted, state the peak-RSS ceiling and add a criterion that materialisation succeeds under a container limit set to it.

- **minor** (medium) — *Migration Notes and Step 1c §1* — `prune` emits a ready-to-paste removal command for `${ACCELERATOR_PLAYWRIGHT_CACHE:-...}`, an env var nothing else reads or validates after Phase 3. **Impact**: a user with a broad value gets a tool-authored destructive command for a path outside version control. **Suggestion**: only emit it when the directory matches the legacy `<sha8>` layout, and refuse to name any non-leaf path.

- **minor** (medium) — *Removal sweep §3 and Performance Considerations* — A prerelease-tracking user accumulates ~294MB per platform per upgrade on the default root for up to two weeks, with no signal, no ceiling and no first-party reclamation — and the resulting disk-full manifests as `disk-floor-not-met` in unrelated work. **Suggestion**: let `prune` operate on the default root too (its guards already make that safe) and report total footprint across sibling version roots.

- **suggestion** (medium) — *Phase 2 §6.1* — The new `_assert_staged_manifest_is_current` arm is a key-set comparison, so a manifest with three of four platform entries passes, and the `missing` file check sees only files the registry enumerates. **Suggestion**: assert the full `(artifact, platform)` cross-product against `TREE_ARTIFACTS × TARGETS`, with a criterion covering one missing platform.

### Portability

**Summary**: The plan is unusually strong on the portability surfaces it names explicitly: the pure-Rust `flate2` backend pin protects the fully-static musl cross-build ADR-0046 depends on; tree names carry a platform axis so a shared cache root works across macOS and Linux; and the conjunctive libc+loader boundary with a compile-time gate and a pure injectable classifier is a genuinely portable design that needs no container to test. The weaknesses are on the surfaces it does not name: the glibc *version* floor and Chromium's shared-library dependencies; the `macos-latest` release runner leaving three of four platforms unexecuted and coupling `ASSEMBLED_SHA256` to that image's tar/gzip; and a cache-root ownership refusal that will reject legitimate hosts on precisely the relocation the plan recommends.

**Strengths**:
- The `flate2` pure-Rust pin is correctly identified as load-bearing, with the feature-unification hole closed by extending the `_ABSENT` tuple, and the escalation path constrained to pure-Rust backends.
- Tree names carry the platform alias with a criterion pinning two-platforms-one-root — right for shared home directories and relocated caches.
- The platform probe is a pure function over injected observations across six shapes including macOS, with the Linux gate at compile time.
- The conjunctive boundary separates an irremediable exclusion from a remediable one, both at zero network cost.
- `ACCELERATOR_CACHE_DIR` is escalated to trust-relevant, and `prune` owns the two roots the orphan sweep never reaches.
- The plan removes shell rather than adding it, shrinking the bash 3.2 surface.
- The same extraction rules apply launcher-side and CI-side, with CI extraction staged outside the checkout.

**Findings**:

- **major** (high) — *Phase 3 §4* — Neither the **glibc version floor** (CentOS 7, Debian 10, Ubuntu 18.04 pass both observations then fail with `GLIBC_2.xx not found`) nor **Chromium's shared-library set** (`libnss3`, `libatk`, `libgbm`, `libasound2` — the reason upstream ships `--with-deps`, and a negative consequence ADR-0057 records explicitly) is probed, and the AC6 fixture is described only as "Node absent from `PATH`". **Impact**: both produce the fetch-seal-then-opaque-failure outcome the probe exists to prevent, recurring on every invocation until the sticky marker fires. **Suggestion**: add a glibc version observation and a `DT_NEEDED`-derived shared-library presence check (shipped in the manifest alongside the three sizes), each with its own remediable downgrade reason; state the AC6 base image and package set.

- **major** (high) — *Phase 2 §8* — The release job runs on `macos-latest` (arm64), so only `darwin-arm64` is ever executed before publication; the three others — including the Linux targets the whole exercise exists for — get only a header check. **Suggestion**: the smoke job is already separate and artifact-consuming, so make it a runner matrix (`macos-latest`, `macos-13`/Rosetta, `ubuntu-latest`, `ubuntu-24.04-arm`) and gate publication on all four.

- **major** (medium) — *Phase 2 §8 — determinism* — Byte-identity also depends on the DEFLATE encoder and level, tar PAX/GNU choices, and on macOS APFS filename normalisation when Linux trees are staged; the refresh procedure never requires the regenerating machine to match the release runner, and the manual step invites a local dry run. **Impact**: a digest generated on Linux will not match one assembled on `macos-latest`, and the mismatch is a hard release failure that looks like a supply-chain alarm; every runner image bump is a candidate trigger. **Suggestion**: fix the compression implementation, not only its inputs — pin the encoder and level, or pin only the uncompressed tar digest — and require regeneration on the same runner OS/arch, with a test asserting stability across two host OSes if local dry runs remain supported.

- **major** (medium) — *Phase 2 §3* — Python's `zipfile` does not apply `external_attr` permission bits and materialises symlinks as regular files; `tarfile` preserves modes only with a deliberate filter. **Impact**: a Linux headless shell that lost its executable bit passes the structural check, sha256, minisign and `ASSEMBLED_SHA256`, is sealed at `0444`, and fails at `execve` with `EACCES` — invisible on the one platform smoke-executed. **Suggestion**: state that assembly reconstructs modes from `external_attr` (and symlinks from the `S_IFLNK` marker), and assert every expected binary carries the executable bit for every platform.

- **major** (medium) — *Phase 1 Step 1b §2 — ownership and mode checks* — RHEL/Fedora `umask 002` with user-private groups makes a hand-created cache dir `0775`; Docker Desktop and devcontainer bind mounts present mapped uids; NFS-squashed homes likewise. The default root is created by the installer, not this code, so its mode is umask-dependent. **Impact**: every tree resolution is a permanent miss on those hosts, so the tooling either re-materialises 294MB per attempt or downgrades — on exactly the relocation the plan recommends. **Suggestion**: root the strict check at the launcher-created `0700` `trees/` directory; treat group-writability as refusal only when the group has other members; name the exact `chmod`/`chown` in the refusal.

- **major** (medium) — *Phase 2 §3 / Phase 1 Step 1b §4 — symlinks* — macOS Chromium ships a `.framework` with `Versions/Current` and top-level symlinks; flattening them changes the bundle layout the upstream code signature's `CodeResources` records, which on arm64 macOS is an execution failure, and it duplicates a substantial share of ~177MB. **Suggestion**: settle per platform before Phase 1 fixes the allowlist; expect to keep the branch for darwin. Also revisit the `tar` `default-features = false` justification, which reasons from mode masking and does not address extended attributes.

- **minor** (medium) — *Phase 1 Step 1b §2 — the lease* — `flock(2)` is not uniform: emulated via POSIX locks on NFS (and `ENOLCK` on some configurations), a no-op or failure on SMB and several FUSE backends, with its own history on overlayfs. **Impact**: a spurious probe success reclaims a live daemon's tree; a failed acquisition either blocks materialisation or leaks generations for ever. **Suggestion**: fall back to the age backstop on `ENOLCK`/`EOPNOTSUPP` rather than treating the probe as authoritative, with a criterion.

- **minor** (high) — *Key Discoveries / Step 1a — redirect allowlist* — `ACCELERATOR_RELEASE_BASE_URL` exists but the allowlist is compiled-in `github.com` + `*.githubusercontent.com`, and most enterprise mirrors answer with a 3xx to a different host. **Impact**: an organisation that cannot reach `github.com` has no supported way to serve the vendored runtime, and the coupling is invisible in a compiled-in constant. **Suggestion**: derive the allowlist from the configured base URL's host, document the hatch, and add a criterion for a redirecting override.

- **minor** (medium) — *ADR-0063 / Performance* — Ephemeral environments (CI agents, devcontainers, Codespaces) have no persistent cache and no pre-seed route, and there is no offline provisioning path despite `cache verify` being deliberately offline-capable. **Suggestion**: add `accelerator cache ensure --from <path-or-url>` verified against the same digest and signature, and document the recommended cache placement and mount key.

- **minor** (high) — *Testing Strategy — container fixtures* — The roll-up membership is unstated, against the `test:e2e:visualiser:docker` precedent (own task, excluded from the roll-up, `docker info` preflight). **Impact**: a hard-failing preflight inside the default roll-up makes Docker Desktop mandatory for every contributor. **Suggestion**: state the precedent explicitly, assert the exclusion in `tests/unit/tasks/test_mise.py`, and name per-architecture image tags.

- **minor** (medium) — *Step 1a §2 and Step 1b §3* — Two cross-platform decisions are left as alternatives: `SO_RCVTIMEO` is not reachable through blocking reqwest over rustls without unsafe socket extraction and behaves differently across glibc/musl/Darwin, so only the watchdog is portable; and the `#[cfg(not(unix))]`-versus-Unix-only choice leaves the resolver half-portable while both neighbouring modules keep marker arms. **Suggestion**: name the watchdog outright and keep the marker arms.

- **suggestion** (medium) — *Step 1b §1 — the absence guard* — `_feature_tree()` runs a bare `cargo tree -e features -p accelerator`, host triple only, as its own docstring concedes. A C-backend edge under a target-specific dependency table would not appear. **Suggestion**: parametrise over the four `TARGETS` triples and phrase the criterion as "for every target triple".

### Performance

**Summary**: The plan is unusually rigorous about the warm path: it keeps `locate` to local reads and stats, keeps the manifest, the cache-root probe and the ~490-row file table off it, and anchors the one cryptographic cost to a real measurement rather than an estimate. The cold path is also well-shaped — incremental digesting, per-entry hashing inline with extraction, manifest-supplied bomb bounds, and a single-flight lock avoiding ~588MB of duplicate transfer. The weaknesses are elsewhere: an internal contradiction about what `locate` enumerates (with an unbounded per-dispatch cost in one reading), a pointer scheme that makes `prune` unable to reclaim anything in the mode it exists for, an unresolved buffer-versus-prehash decision, an interaction that converts a slow first fetch into a session-wide downgrade, and a release-job analysis that credits §8's reuse with removing a cost it does not touch.

**Strengths**:
- The hit path is designed against a measured budget: two reads plus two stats, no manifest, no probe, no file table, with one 51.7µs cryptographic term measured in-process against the pinned crate in the shipped release profile.
- Per-entry sha256 is computed inline during extraction, so the table costs no second pass over ~294MB.
- Bomb bounds are enforced against running totals with the three sizes required rather than defaulted.
- The retry/sink analysis is correct and non-obvious.
- Per-request `RequestBuilder::timeout()` over a second `Fetcher` is right for the stated reason — each `Fetcher` installs the rustls provider and a runtime thread, and the resolver is lazy per `resolve()`.
- Single-flight is justified quantitatively and contrasted with `cache::store`'s ~8MB.
- The size budget refuses to back-derive from a composite point — and the code agrees: the warm bootstrap makes exactly one O(size) pass over the launcher (`bin/accelerator:352-354`), while the two `sha256_file` calls at `:291`/`:295` are over the ~475KB shim.
- The `flock` lease replaces a pid protocol with a kernel oracle, and generations make `repair` non-destructive.

**Findings**:

- **major** (high) — *Phase 3 §3 vs Performance Considerations* — Contradictory specification of what `locate` enumerates. **Impact**: under the on-disk reading every dispatch — including a PreToolUse hook on every tool call — does a `readdir` of `trees/` plus per-pointer validation, `stat`, attestation read and Ed25519 verify; since nothing removes stale pointers, a fixed-cost hit path becomes O(releases-ever-installed). **Suggestion**: resolve to the compiled-in set and delete the enumeration wording; consider an artifact→consumer mapping so non-design dispatches pay nothing.

- **major** (medium) — *Step 1b §2 and Step 1c §1* — Version-keyed pointers are never reclaimed, so `prune` reclaims nothing under the escape it exists for; each pin bump adds a permanently-referenced ~294MB generation. **Suggestion**: `prune` should delete pointers for versions other than the running launcher's (or keep N most recent), with reclamation predicated on *live* pointers, and a criterion covering a simulated bump.

- **major** (high) — *Step 1a §2 — signature verification* — The buffer-versus-prehash choice is left open and deferred again by Phase 2's manual step. **Impact**: the buffered branch is a peak RSS one to two orders of magnitude above anything the launcher does today, in the memory-limited containers AC6/AC11 use, doubled if both trees materialise. **Suggestion**: decide it in the plan — prefer `-H` for tree artifacts only, confirm `minisign-verify =0.2.5` and the `cli/verify` shim accept it, and add a peak-RSS criterion.

- **major** (medium) — *Step 1b §2 and Phase 3 §3* — The lock-wait deadline is sized for ~120MB at a low throughput floor, so it will be minutes, against a five-minute crawl bound and a sticky marker. **Impact**: on a cold cache over a slow-but-healthy link the crawl degrades permanently; and nothing resumes a partial download, so a link slow enough to exceed one crawl's budget can never converge. **Suggestion**: separate in-progress from failed, use a short waiter bound plus a resumable download (`Range` against the digest-keyed partial), and state whether `ensure` survives the invoking crawl's termination.

- **major** (medium) — *Phase 2 §7 and §8* — `upload_and_verify_release` uploads and re-verifies in serial per-asset subprocess loops: ~480MB up and ~480MB down per pass, doubled; §8 removes only the re-assembly CPU and may add a download for the second pass whose bytes already sit in `dist/release/`. **Impact**: the claim that §8 "removes the duplication itself" credits it with the smaller term while ~2GB of serial transfer on a 3-vCPU runner is unchanged, and `timeout-minutes` is then sized against an unmeasured baseline with no cleanup arm. **Suggestion**: quantify from a measured dry run; state that the pre.0 pass reuses local copies; consider bounded-parallel upload and re-verify; state free-disk headroom as a number (four platforms' extracted Chromium alone is ~700MB).

- **minor** (medium) — *Step 1c §1 and Phase 3 §3* — Cold first run is fully serialised across two artifacts, each in its own launcher process with its own `Fetcher`, rustls provider, runtime thread and TLS handshake. **Suggestion**: let `ensure` accept multiple names and materialise concurrently, and state the expected first-run wall clock as sum or max so the manual criterion is derived.

- **minor** (medium) — *Step 1b §2 (`locate` step 1) and Phase 1 criteria* — The version-keyed pointer means a plugin upgrade with an unchanged pin misses, so `materialise` runs and its first step is two HTTPS GETs plus a signature verification — only then does the reuse scan find the tree already sealed. **Impact**: on an offline machine with a fully populated cache, the first design run after an upgrade fails outright — weaker than `cache.rs:1-6` documents for single-file binaries. **Suggestion**: let `locate` fall back to another version's pointer (the attestation is signed and the generation content-addressed, so adoption costs the same checks), or state the limitation and narrow the criterion.

- **minor** (medium) — *Step 1b §1 and Manual Verification* — The inflate concern is likely mis-weighted (~294MB through `miniz_oxide` is one to two seconds against a ~120MB download and ~294MB of writes) and its ceiling has no number. **Suggestion**: state a number on end-to-end materialisation excluding download, with inflate reported as a share; if it is under ~20%, close the backend question rather than carrying the escalation.

- **minor** (high) — *Step 1c §1 — `cache verify`* — The ~120ms figure is ADR-0060's *full Chromium* row (297MB, 327 files), not the shipped set (~71ms + ~47ms), and assumes Apple Silicon with hardware sha256 and a warm page cache. **Impact**: the basis is wrong and the budget does not transfer to a software-sha256 host or a cold cache — which matters because `repair` runs `verify` first and the envelopes name `repair` as the remediation. **Suggestion**: restate from the two shipping rows, qualify by host class and cache state, keep the full-hash decision, and let the stat pre-check short-circuit `repair`'s missing-entry case.

- **minor** (medium) — *Testing Strategy and AC6* — The container lane transfers ~120MB, writes ~294MB and hashes ~414MB per run, unbudgeted, potentially under emulation — and 0208 may adopt it as an every-build job. **Suggestion**: exercise the machinery against small synthetic trees and reserve real artifacts for one end-to-end lane with a stated time budget; name the artifact-serving component.

- **suggestion** (medium) — *Step 1a §2 — the stall mechanism* — The workspace pins `reqwest = "=0.12.28"`, which exposes `ClientBuilder::read_timeout`, a genuine idle bound, composing cleanly with the per-request override. **Suggestion**: verify it at the pinned version and set it once on the single production client rather than building a watchdog. Note `TOTAL_TIMEOUT` is per attempt and `MAX_ATTEMPTS` is 3 — bound the total wall clock across attempts.

- **suggestion** (medium) — *Step 1b §1 and Implementation Approach* — Two budget constants derive from inputs the owning phase does not have, and an absolute per-target ceiling drifts until it is bumped reflexively and stops encoding the 1ms budget. **Suggestion**: state an interim deadline in Phase 1 and make re-derivation an explicit later item with a criterion; make the *delta* assertion the enforced one and keep the absolute number as a recorded figure.

### Code Quality

**Summary**: This is an unusually rigorous plan: the port split, the pure-function platform classifier, the generation-based rename discipline and the flock lease are all well-reasoned designs a maintainer would thank the author for. The quality risks are concentrated in three places — decomposition instructions that name a seam without naming the modules, decisions the plan itself labels load-bearing but then defers to the implementer, and a pervasive instruction to record reasoning in doc comments that, given how densely the plan cites ADRs, work items and phase numbers, will very likely produce exactly the stale-prone comments CLAUDE.md forbids. Two internal contradictions would each surface as rework mid-implementation.

**Strengths**:
- The three narrow ports, with the reasoning that find-or-materialise puts the forbidden fetch one argument away and verify-and-repair hides a mutation behind a query.
- `repair` as a use case over the ports, mirroring `run_external` at `core.rs:246-255`.
- The platform classifier as a pure function over injected observations, unit-tested across six shapes — the subtle logic (musl-first beating `gcompat`) in a fast test rather than a container.
- Forbidding `locate` from calling `verify_writable` and stating `materialise`'s probe count up front treats an existing invariant as a design constraint.
- The `TREE_ARTIFACTS` registry with a cross-language drift test removes a five-file-hunt class of maintenance, reusing the established pin shape.
- Requiring the three sizes rather than defaulting them, on the ground that a defaulted 0 fails open.
- Injecting the SLSA runner matches the established default-argument callable convention in `tasks/` (`tasks/manifest.py:85`, `tasks/shared/polling.py:13-14`).

**Findings**:

- **major** (high) — *Phase 1 Step 1b §2/§3/§4* — The plan promises four seams and names one file. **Impact**: followed literally, one module owns eleven responsibilities — the god-module the plan says it wants to avoid — and `repair` has nothing to reuse cleanly. **Suggestion**: replace the header with the concrete module set and each module's responsibility, and state which `repair` consumes.

- **major** (high) — *Phase 1 Step 1b §3* — The `Refusal`/`Failed` mapping is argued to be non-cosmetic and then left to the implementer. **Impact**: whether a path-escape rejection hard-fails or is swallowed under `--fail-safe` becomes an accident. **Suggestion**: name the mapping per variant with a one-line rationale (plausibly extraction/path-escape/seal/attestation as `Refusal`, pointer as `Failed`), and add a `--fail-safe` criterion.

- **major** (high) — *Phase 1 Step 1b §3 — the error enum* — Five tree-only variants take a flat enum spanning two unrelated paths from fifteen to twenty, and the two tests pinning the mapping (`:420-447`, `:450-495`) are hand-maintained `vec![]` literals with no exhaustiveness link. **Impact**: an omitted variant compiles, passes, and ships unclassified. **Suggestion**: `ResolutionError::Tree(TreeError)` plus a `const fn class(&self)` the compiler forces exhaustive, with `From` and both tests derived from it.

- **major** (high) — *Phase 3 §3 — Discovery* — The `ACCELERATOR_LAUNCHER_BIN` self-contradiction; `derive_override_var` (`core.rs:268-293`) produces that identical string. **Impact**: the implementer either exports a variable honoured as an unverified override, or stops mid-phase to invent a replacement the contract, its envelope tests and the dev-override path all depend on. **Suggestion**: pick a name outside the grammar, state why, and add a collision guard test.

- **major** (high) — *Phase 3 §3 vs Performance Considerations* — The export-source contradiction, with the added point that the "always set or explicitly cleared" safety property is **unachievable** under disk-derived enumeration: a tree with no pointer yields no name to clear, so an injected `ACCELERATOR_TREE_BROWSER` survives exactly in the cold-cache case the clearing defends. **Suggestion**: settle on the compiled-in set and add a criterion that an injected variable for an unmaterialised tree is cleared.

- **major** (high) — *Phase 3 §5 — `resolve_optional`* — It returns `Result<_, ComposeError>`, which wraps visualiser-only `PatternError` (`compose.rs:20-25`), and the plan rules out the natural home by quoting `config-adapters`' one-env-var rule. **Impact**: unresolvable as written — the implementer must invent a crate (with its registration checklist), rework the error type and retarget callers, none of it budgeted; the fallback contradicts the reason for extracting. **Suggestion**: name the crate and generic error shape, and rename the function in domain terms — `resolve_optional` says nothing about the precedence it encodes.

- **major** (medium) — *Phase 1 Step 1b §2 and Phase 3 §3* — Three time-dependent behaviours with no injection seam, in a codebase that already has the pattern (`design::executor::ports::Clock`, `corpus::metadata::Clock`). **Impact**: tests must sleep or back-date, and the marker decision cannot be unit-tested in the domain crate under its pup rule (`cli/pup.ron:249-266`) at all — so a suppression bug costing a user tens of gigabytes is only catchable in integration. **Suggestion**: name a clock port for the reaper and waiter, reuse the existing design `Clock`, and add a marker-store port.

- **major** (high) — *Phase 2 §2 — GPG* — All the subtlety in the phase, specified as one operation with no seam between invoking `gpg` and classifying `--status-fd`, then "tested for real" against a host `gpg` the plan concedes it may not pin. **Impact**: the revoked and expired negatives become tests needing crafted keyrings and a particular GnuPG — the same shape as the `skip_if_no_minisign!` trap. **Suggestion**: split out a pure `classify_status_lines(lines) -> Verdict` so every combination is a table-driven unit test over recorded fixtures, and state that an absent `gpg` fails rather than skips.

- **major** (high) — *Throughout — doc-comment instructions* — At least seven sites instruct recording reasoning in comments, and the sections carrying them express rationale almost entirely through ADR/work-item/phase citations and host-specific measurements. **Impact**: CLAUDE.md is explicit that comments are a last resort and that such references must never appear because they go stale; a transcribed 29.92ms figure or per-MB slope is stale the moment the host or toolchain changes. **Suggestion**: add one instruction that a warranted doc comment must be a self-contained statement of the constraint with no citation and no host-specific measurement, and prune the redundant instructions where a named constant already says it.

- **major** (high) — *Phase 3 §3 — the environment vector* — The Current State Analysis claims `executor.rs:139-156` is "the single place a resolved browser path is threaded", but the Node executable is not in that vector: `const NODE: &str = "node"` (`:28`) is set independently as `DaemonSpawner.program` (`:163`) and `ExecClient.program` (`:174`). **Impact**: the single-seam claim is wrong by one, so a Phase 3 implementation changing only the vector still shells out to a system `node`, and AC6 fails at the last step. **Suggestion**: state both program fields in the edit set, and consider collapsing program + environment into one resolved-runtime value so there is genuinely one site.

- **minor** (medium) — *Phase 2 §2/§3/§8/§9* — `verify.py` holds three unrelated trust protocols and `assemble.py` seven concerns; the sibling operations sit under `vendor.*` and `build.*`; and registering a `vendor` collection in `tasks/__init__.py` appears in no Files list. **Suggestion**: split into per-source modules with `pins.py` as the shared anchor, factor the archive/normalisation concerns out of `assemble.py`, unify the namespace, and add `tasks/__init__.py`.

- **minor** (medium) — *Step 1b §4 and Phase 2 §3* — The allowlist is specified twice, in Rust and Python, with nothing binding them. **Suggestion**: commit one adversarial tarball fixture corpus (`../`, escaping symlink, escaping hardlink, absolute path, symlink-then-traverse, FIFO, setuid, duplicate path, PAX long name) that both suites iterate.

- **minor** (high) — *Phase 3 §4 — `design-adapters/src/platform.rs`* — The pup rule enforcing the in-process discipline matches only `^design_adapters::(filesystem|environment)($|::)` (`cli/pup.ron:273-285`), so a new `platform` module falls outside it. **Suggestion**: add `platform` to the match (it performs no spawn) and list `cli/pup.ron` in §4's Files.

- **suggestion** (medium) — *Phase 1 Step 1b §3* — The dispatch path's freedom not to call `MaterialiseTree` is convention only; the probe-count criterion would not catch a materialisation that hits a warm cache in the test. **Suggestion**: have the dispatch composition root accept only `&impl LocateSealedTree`, and consider the `launch::outbound` pup rule the plan's own observation invites.

---

## Re-Review (Pass 2) — 2026-08-17

**Verdict:** REVISE

All eight lenses re-ran against the revised plan (2,261 → 3,518 lines). The pass-1 findings are overwhelmingly closed: of 112 findings, roughly 85 are fully resolved and most of the rest partially, with every pass-1 critical resolved. The revision's own new mechanisms introduced 7 new criticals, concentrated in the two largest changes — the producer-side attestation and the upstream job topology — plus one factual error and one capability introduced without an owning phase.

### Previously Identified Issues

**All 10 pass-1 criticals: Resolved.**

- ✅ **Architecture**: Smoke-check job topology — Resolved (upstream `assemble-runtime` → `smoke-runtime` → `release`)
- ✅ **Architecture**: Lease placement contradicts ADR-0061 — Resolved (sidecar, in the layout block)
- ✅ **Security/Correctness/Test coverage**: Attestation unverifiable, unbound, untested — Resolved in design (producer-side document, `acquire` steps 4-5, mutation-explicit criteria)
- ✅ **Correctness**: Vendored Node never wired in — Resolved (`ResolvedRuntime`, `NODE` retired)
- ✅ **Correctness**: ESM/`NODE_PATH` — Resolved (loader narrowed not deleted; suite floor stays at 9)
- ✅ **Safety**: Tag-delete arm / marketplace ref — Resolved (arm removed outright)
- ✅ **Test coverage**: Container fixtures unscheduled — Partially resolved (harness now named; fixture source conflicts with AC6/AC12 — see below)

**Majors**: 38 of 45 resolved. Notable partials:

- 🟡 **Architecture**: Environment export channel — Partially resolved. The mutation site is named but no port or signature change is given; `ExecBinary::exec` still takes no environment.
- 🟡 **Security**: Trust-anchor review gate — Partially resolved. The mechanism is named but the approval token is unspecified, branch protection is unassertable, and no criterion covers the gate.
- 🟡 **Performance**: Offline cross-version adoption — Still present. `materialise` loads the manifest (two HTTPS GETs) before the reuse scan, so a zero-byte plugin upgrade still fails offline.
- 🟡 **Code quality**: `resolve_optional` target — Partially resolved. `cli/config` is named, but that crate is declared pure domain in `cli/pup.ron:40-44` and the helper reads the environment.
- 🟡 **Portability**: `flock` degradation — Partially resolved. `ENOLCK`/`EOPNOTSUPP` is handled; a silently no-op `flock` (SMB, some FUSE) returns success and is still trusted.

**Minors**: all citation corrections verified against the tree (21 `benchmark.json` references at the exact cited lines, 15 `ResolutionError` variants, `cli/Cargo.toml:183-185`, `cli/pup.ron:253`/`:274`). Two residual drifts found: Removal sweep §1 still cites `integration.py:44`/`:66` while Current State correctly cites `:45`/`:67`; and `verify_writable` is at `cache_root.rs:101-113`, not `:88-102`.

### New Issues Introduced

#### Critical

- 🔴 **Architecture / Security / Performance**: Binding `release_version` in the attestation makes cross-version generation sharing impossible, contradicting the plan's own criterion that two release versions sharing a digest issue zero fetches. The obvious resolution — relaxing the field check — deletes the anti-rollback binding the field exists for.
- 🔴 **Architecture / Security**: The attestation binds a release version that `assemble-runtime` cannot know (it runs before `version.bump`), and one archive set serves two differently-versioned cuts. `table_sha256` can only be computed where extraction happens — the job the plan says must not extract.
- 🔴 **Security**: The attestation document is the one artifact crossing the unsigned inter-job channel with no `ASSEMBLED_SHA256` coverage, and the `upload-artifact` glob in §3's sketch does not even match `.sealed`/`.sealed.sig`.
- 🔴 **Correctness**: Producer-signed `layout_version` — defined as launcher-side policy — makes any future layout bump an unbreakable re-materialisation loop: the launcher cannot rewrite a signed field, so it misses forever.
- 🔴 **Safety**: The new re-drivable finalise `workflow_dispatch` holds the signing key and `--clobber`s assets against an operator-typed version with no stated preconditions (draft-only, tag/commit match, concurrency group).
- 🔴 **Test coverage**: The container lane's newly-specified fixture source (test-key-signed *miniature* trees) cannot execute the real Node and headless shell that AC6, AC12, the `ping` regression and the relocated `lib/*.test.js` suites require.

#### Major (selected — 41 total across lenses)

- 🟡 **Correctness / Performance**: `reqwest 0.12.28`'s blocking `ClientBuilder` does **not** expose `read_timeout` — verified against the vendored source. **Already corrected in the plan** during this pass.
- 🟡 **Security / Safety / Test coverage / Code quality / Portability**: `cache ensure --from <path-or-url>` was introduced in a documentation bullet with no command surface, no phase, no criteria, and a verification contract that requires the network it exists to avoid.
- 🟡 **Architecture / Safety**: The upstream jobs are wired only into `release`; the `prerelease` job — which runs on every push to main and gains the same `ASSEMBLED_SHA256` assertion — is left unwired and uncosted.
- 🟡 **Portability / Test coverage / Architecture**: The `DT_NEEDED` list is routed through a manifest the warm and offline paths never load, into an `ArtifactPlatformEntry` that carries no such field; and `dlopen` is unavailable from a static-musl binary, so presence has no stated resolution mechanism.
- 🟡 **Portability**: `macos-13` is a retired GitHub runner label — the smoke matrix would fail to find a runner. `macos-15-intel` is the surviving Intel image.
- 🟡 **Correctness / Performance / Safety**: The digest-keyed resume contradicts the generation-keyed temp archive name, and the reaper's age backstop reclaims the partial archive resume depends on.
- 🟡 **Correctness**: `acquire` validates before it locks and never re-validates, so a concurrent `prune` can hand back a removed tree; and the reaper cannot derive `(name, platform)` from `.tmp-<gen>` residues to take the lock it is required to hold.
- 🟡 **Correctness**: `cache repair` is a launcher built-in with no knowledge of the inventoried repository, so it cannot clear a sticky marker living in the design binary's repo-scoped state directory — contradicting a criterion.
- 🟡 **Performance**: The "max rather than the sum" claim for concurrent `ensure` is a latency-bound argument applied to a bandwidth-bound transfer; the 600s deadline (120MB ÷ 200 KB/s, zero margin) times out at exactly the floor it was sized for once two streams share the link.
- 🟡 **Performance**: The 0.17% hit-path budget measures one Ed25519 verify while `acquire` now also performs ~15 syscalls, three file reads and an `flock` per tree — against a 1.0ms absolute gate that blocks the phase.
- 🟡 **Performance / Architecture**: Confining `acquire` to "tree-consuming dispatches" is not achievable at the token granularity the launcher decides at, and two arguments rest on it.
- 🟡 **Code quality**: `TreeError`'s classification table covers six variants against roughly ten enumerated `ensure` causes.

### Assessment

The plan is substantially stronger than at pass 1 and no pass-1 critical survives. But the two mechanisms introduced to fix the pass-1 criticals — the attestation tuple and the upstream job split — have not been carried through their own consequences, and four of the new criticals are variations on one root cause: **`release_version` and `layout_version` were added to a signed producer artifact without asking who knows those values, when, and what happens when they change.** That single decision needs revisiting before the rest is worth polishing; the likely resolution (bind artifact identity and content, not plugin release version; let the layout version live only in the directory grammar) collapses several findings at once.

Three further items are cheap and independent: correct the `macos-13` runner label, wire the `prerelease` job, and either promote `cache ensure --from` into Phase 1 with criteria or cut it to a follow-up.

A third pass should re-run **architecture, security, correctness and performance** once the attestation tuple is re-decided; test coverage, safety, portability and code quality can follow, since most of their new findings are downstream of the same decision.

---

## Re-Review (Pass 3) — 2026-08-17

**Verdict:** REVISE

Four lenses re-ran (architecture, correctness, security, performance) with a mandate to verify named mechanisms rather than hunt new design gaps — because three of the last five defects had been mechanisms that do not exist. That mandate was the right call: pass 3's most serious findings are all verification failures, and **three of the five new criticals were introduced by pass 2's own fixes.**

### Verification outcome

Roughly 60 citations and 15 mechanisms were checked against the tree. Most hold — every `core.rs` range, the `pup.ron` rule lines, the suite-floor arithmetic, the `prerelease` job shape, `manifest.rs`, `cache.rs`, `cache_root.rs`, the seven `executor.rs` sites, ADR-0060's ~118ms rows, and work-item 0186's 125.35 → 29.92ms figures. Four failed:

| Claim | Verdict |
|---|---|
| Work-item 0205's method: n=300, Hodges–Lehmann, "recorded 1.28 ratio" | ❌ **Not sourced.** Appears nowhere in `meta/`. 0205's SQ-4 still lists the statistic, interval, resample count and `n` as open questions |
| `exit 127` + `cannot execute: required file not found` | ❌ That is a **bash** message. A missing `PT_INTERP` makes `execve` return `ENOENT`, so `spawn()` fails with no child, no stderr, no exit status |
| `From<async ClientBuilder>` for the blocking client | ✅ Exists (`blocking/client.rs:1190-1197`) — but sets `timeout: Timeout::default()` = **30s**, silently replacing today's 300s |
| Prehashed minisign digest | ⚠️ Uses **BLAKE2b**, not sha256, so `StreamedBody` needs two digests or a second full read |

### Previously identified issues

Pass-2 findings: 26 of 33 resolved. Notable residuals — `cache repair` still cannot clear a marker in the design binary's repo-scoped state directory (unchanged from pass 2); the narrowed loader's ESM entry resolution is still unspecified, and `playwright-core` is CJS so the destructured `chromium` would be `undefined`; and "tree-consuming dispatch" still has no deciding mechanism.

### New Issues Introduced

#### Critical

- 🔴 **Architecture / Security / Performance**: **Deleting `table_sha256` removed the `.files` table's only anchor.** The "archive signature covers it" reasoning holds only while the archive exists — `materialise` discards it after extraction, leaving the table an unsigned `0444` file. `cache verify` becomes a rewritable oracle, and the criterion asserting a rewritten table is still detected is unimplementable. Introduced by pass 2's fix.
- 🔴 **Architecture**: **The compiled-in digest map makes Phase 1 depend on Phase 2 artefacts**, breaking the stated phase independence — `pins`, `ASSEMBLED_SHA256` and `TREE_ARTIFACTS` are all Phase 2 deliverables that Phase 1 now carries criteria over. Introduced by pass 2's fix.
- 🔴 **Architecture / Correctness**: **Spawn-failure classification has no observation point.** The daemon is `setsid`-detached with stderr redirected into a log the domain only *names*; `Spawner::spawn` returns no exit code and no stderr, and the only failure path is a 30s `DaemonStartTimeout`. `ExecClient` uses `exec()`, so there is no Rust process left to classify anything. Introduced by pass 2's fix.
- 🔴 **Security**: **The digest anchor is selectable.** The bootstrap verifies the launcher's *authenticity*, not its version — any past release's signed launcher can occupy the current version's cache slot, and its compiled-in map names the superseded digest. The plan deliberately declines to ownership-check the cache root.
- 🔴 **Safety-adjacent (Security)**: **`prune`'s predicate requires reading another installed launcher's compiled-in constant**, which is private to that binary. Implemented conservatively it reclaims nothing; optimistically it deletes a sibling install's ~294MB.

#### Major (18 across four lenses — selected)

- 🟡 The fabricated 0205 citation (**corrected during this pass**; the criterion is now an explicit dependency and `blocked_by` records it).
- 🟡 `RequestBuilder::timeout` is a **per-read** bound once the body is streamed, so a streamed attempt has no total deadline.
- 🟡 The attestation's `uncompressed_size`/`entry_count` are signed from unpinned inter-job data, making the decompression-bomb ceiling attacker-settable.
- 🟡 `.sealed.sig` has two contradictory producers — §8 has `assemble-runtime` (which holds no key) upload it, §5 has the publish path sign it.
- 🟡 The lease-mtime refresh puts a **write** on the hit path, contradicting the read-only-cache-root guarantee (`bin/accelerator:13-15`) and giving `prune` a forgeable retention signal.
- 🟡 `TreeError`'s `Refusal` class has no consumer that produces the claimed behaviour — `acquire` treats every integrity failure as a *miss*, and Phase 3 lists digest mismatch among the *sticky downgrade* causes.
- 🟡 Step 1b §1 declares only `tar` and `flate2`, but the design needs `flock`, `openat`, `fstatat` and a CSPRNG — none reachable from `std`, and `rustix` is only a transitive dependency.
- 🟡 The tree adapter re-implements `cli/store`'s containment, ownership-check and atomic-publish primitives without acknowledging the divergence.
- 🟡 The ~120MB figure is used at three incompatible granularities, so the 600s deadline is right only by cancelling errors.
- 🟡 The fetch bound is still 6× the crawl bound it runs inside, and nothing bounds `ensure` by the remaining crawl budget.
- 🟡 Spawn-stderr classification lifts unvalidated tokens into an agent-facing envelope, and a spoofed marker silently forces a code-only crawl.
- 🟡 The trust-anchor approval is not bound to a head SHA (approve-then-push bypass), and the workflow implementing the gate is outside the guarded set.
- 🟡 The `prerelease` wiring makes the runtime lane a per-merge cost that no figure in the plan totals.

### Assessment

The plan's design is sound and its factual base is now largely verified — but **three passes have each closed the previous pass's criticals while introducing new ones of the same class**, and this pass found a fabricated citation attributing a specific statistical gate to a document that does not contain it. That pattern, not the remaining findings, is the thing to act on.

The recommendation is to stop revising prose and change the process. Specifically: (1) treat every mechanism as unverified until checked against the tree or a prototype, which is what work-item 0214 did successfully for the four questions it owned; (2) split the plan, since a 3,900-line document revised by whole-file editing is where these regressions come from — Phase 1 alone is now large enough to be its own plan with its own review; and (3) close work-item 0205's SQ-4 before Phase 1 starts, since two gates depend on it.

---

## Acceptance — 2026-08-17

**Verdict changed REVISE → APPROVE.** Accepted by Toby Clemson after the pass-3 blocking
set was addressed. The plan moves to `status: ready`.

### What was fixed to close the review

Seven items from pass 3 that would have made an implementer build the wrong thing or hit a
wall:

1. **`table_sha256` restored** to the signed attestation, with the `.files` table also
   shipping as the archive's first member — the two serve different purposes (single-pass
   extraction verification; a post-discard anchor). Closes the pass-3 critical flagged
   independently by architecture, security and performance.
2. **`BootstrapDiagnostics` port** added, giving spawn-failure classification an
   observation point it did not have, plus a `Spawner` error carrying the raw
   `io::ErrorKind`. `loader-unresolvable`'s post-fetch arm is a spawn errno, not the bash
   message an earlier revision cited. Classification hardened against stderr spoofing.
3. **`prune`'s predicate** replaced with on-disk claim files under `trees/claims/`,
   removing a requirement to read another launcher's compiled-in constant.
4. **`rustix` and a CSPRNG** declared as direct launcher dependencies; the size gate and
   feature-graph assertion re-derived against all four dependencies.
5. **reqwest semantics corrected** — the `From` conversion's 30s default reset, and the
   per-request timeout becoming a per-read bound on a streamed body.
6. **`StreamedBody` carries two digests** (sha256 and BLAKE2b-512), since prehashed
   minisign does not use sha256; resume re-hashes the on-disk prefix and retains the
   longest prefix across attempts.
7. **`.sealed.sig` producer disambiguated** — emitted upstream, signed in the publishing
   job, which also re-derives the three measured fields from the pin-verified archive.

A fabricated citation was also removed: an earlier revision attributed a specific
statistical gate (n = 300, Hodges–Lehmann, a "recorded 1.28 ratio") to work-item 0205.
None of it appears in 0205 or anywhere in `meta/`. The criterion is now an explicit
dependency on 0205 closing SQ-4, and `blocked_by` records it.

### Accepted with residual risk, consciously carried

These pass-3 findings were reviewed and **deliberately not resolved** before
implementation. They are recorded here so the acceptance is auditable and so validation
knows where to look:

- **Phase 1 ↔ Phase 2 pins dependency.** The compiled-in digest map makes Phase 1 depend
  on `pins`/`ASSEMBLED_SHA256`/`TREE_ARTIFACTS`, which Phase 2 creates. Folds into the
  handoff list the plan already carries for the fetch deadline; affects merge order.
- **Launcher-downgrade selectability.** The bootstrap verifies the launcher's authenticity,
  not its version, so a past signed launcher can occupy the current version's cache slot
  and its compiled-in map names a superseded digest. A scope decision: accept, or pin the
  launcher cache entry by digest as the verify shim already is.
- **Trust-anchor approval not bound to a head SHA** (approve-then-push bypass), and the
  workflow implementing the gate sits outside the guarded path set.
- **`ensure` vs the crawl bound.** The fetch bound remains ~6× the five-minute crawl bound
  it runs inside, and nothing bounds `ensure` by the remaining crawl budget.
- **The ~120MB figure** is used at three granularities, so several derived timeouts are
  correct only by cancelling errors.
- **`TreeError`'s `Refusal` class** has no consumer producing the claimed `--fail-safe`
  behaviour: `acquire` treats integrity failures as misses, and Phase 3 lists digest
  mismatch among the sticky downgrade causes.
- **ESM entry resolution** in the narrowed loader is still unspecified, and `playwright-core`
  is CJS — the destructured `chromium` may be `undefined`. Phase 3 §1's first step should
  settle this empirically.
- **`cache repair` clearing the sticky marker** — a launcher built-in has no access to the
  inventoried repository's state directory. Carried from pass 2.

### Standing process note

Three review passes each closed the previous pass's criticals while introducing new ones of
the same class, and every mechanism that failed review was specified on paper rather than
prototyped. Work-item 0214 prototyped its four questions and none of its settled mechanisms
failed a subsequent pass. The recommendation carried into implementation: **validate Phase 1
before Phase 2 or 3 begins**, and treat any newly-named mechanism as unverified until
checked against the tree.
