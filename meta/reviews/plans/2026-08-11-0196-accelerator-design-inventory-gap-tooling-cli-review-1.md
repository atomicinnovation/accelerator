---
type: plan-review
id: "2026-08-11-0196-accelerator-design-inventory-gap-tooling-cli-review-1"
title: "Plan Review: accelerator-design: Design Inventory and Gap Tooling CLI Implementation Plan"
date: "2026-08-11T16:16:42+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "work-item:0196"
target: "plan:2026-08-11-0196-accelerator-design-inventory-gap-tooling-cli"
reviewer: Toby Clemson
verdict: REVISE
lenses: [architecture, correctness, security, test-coverage, compatibility, performance, safety, code-quality]
review_number: 1
review_pass: 3
tags: [rust, design, cli, playwright, launcher, release-pipeline, tree-artifacts, sub-binary]
last_updated: "2026-08-11T21:49:36+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: accelerator-design: Design Inventory and Gap Tooling CLI

**Verdict:** REVISE

The plan is exceptionally well-researched at the level of *surface*: every by-name pin, floor, registry dict, guard and line-referenced quirk that this change will break is enumerated, the manifest extension's additive claim is verified against the real parser rather than asserted, and the ADR inheritance closes every design question before phase one. What it has not yet closed is the *mechanism* behind three of its load-bearing promises — the repair path that replaces per-exec self-healing cannot detect the corruption it exists for, the read-only seal makes a tree undeletable by the very pruning the caching design depends on, and the launcher→design tree handoff (trigger, failure contract, downgrade ordering) is one sentence where it needs to be a specification. A second cluster sits in the release pipeline: the ~1.2GB artifact set is threaded into the manifest but not into signing, upload or pre-publish re-verification, and the three new upstream verifications name no pinned trust anchors. Finally, the sequencing rests on a factually incorrect premise — `scripts/test-design.sh` *is* discovered and run in CI today, and carries ~200 lines of assertions over surfaces that survive this migration.

### Cross-Cutting Themes

- **The tree integrity story does not close** (flagged by: architecture, correctness, security, performance, safety, code-quality — six of eight lenses) — the sentinel records the *archive* digest, which cannot be recomputed from an extracted tree. `cache verify` therefore compares two strings that agree by construction and learns nothing about the bytes on disk, so `repair` never fires. This is the single most reinforced finding in the review, and it is load-bearing: ADR-0060 gives up automatic self-healing *on the promise* that repair replaces it.

- **Sealing conflicts with removal** (flagged by: architecture, correctness, safety) — `0555` directories mean `remove_dir_all`, `rm -rf` and Claude Code's plugin pruning all fail with `EACCES`. That breaks repair's discard step, breaks the "pruned when Claude Code prunes old plugin versions" rationale that justifies having no eviction logic, and strands ~294MB per plugin version on a plugin that pre-releases often.

- **The warm path as specified requires the network** (flagged by: architecture, correctness, security, performance, safety) — "sentinel digest matches the manifest's" means `load_manifest()`, i.e. two HTTPS GETs plus a signature verification, on every one of the 100–200 executor invocations a crawl makes. It contradicts the plan's own "stat plus a small read", regresses offline operation that works today, and blows the ~30ms budget the plan promises to protect.

- **The launcher→design handoff is under-specified in a way that blocks three ACs** (flagged by: architecture, correctness, compatibility, performance, code-quality) — no stated trigger (so cheap subcommands may pay a 294MB fetch), no stated failure contract (a launcher-level `ResolutionError` pre-empts ADR-0057's mandated ordering, so AC11/AC12 cannot pass), no variable names, no absent-case behaviour, and the `ACCELERATOR_DESIGN_BIN` override path gets no env at all.

- **Producer and consumer of the new artifact contract are developed in different phases with nothing pinning them together** (flagged by: security, test-coverage, compatibility, safety) — the asset-name/URL convention is never stated, signing/upload/re-verify are never extended, and the only end-to-end check is a one-platform manual dry run.

- **"Measure and record" criteria cannot fail** (flagged by: performance, test-coverage, code-quality) — binary size delta, warm-path timing, artifact sizes and first-run download time all appear in checkable lists with no threshold, baseline or method. This is a regression against work-item:0186's own ratio gate and interleaved-sampling method, which the plan cites as the budget it must not break.

- **Testing accounting is optimistic where the code is riskiest** (flagged by: test-coverage, correctness, safety) — the six named characterization behaviours have no existing oracle in `test-run.sh` (everything past line 65 self-SKIPs without a real Playwright install) and no injection seams are specified; the container fixtures carrying AC6/AC11/AC12 name no images, no task, no CI job and no artifact source.

### Tradeoff Analysis

- **Security hardening vs launcher size/latency**: security asks for more extraction validation (entry-type allowlist, hardlinks, mode masking, size caps) and streaming signature verification; performance notes the launcher is minisign-verified on every warm start so every added byte is a latency term. These are compatible — the extraction rules cost almost nothing in binary size, and the size pressure is really about `flate2`'s backend, not about validation. **Recommendation**: take the security hardening in full; resolve the backend question on *decompression throughput plus musl-static compatibility* (pin `rust_backend` explicitly), not on size alone.

- **Integrity of trees vs the warm-path budget**: recording per-file digests in the sentinel makes `verify` real but adds a walk. **Recommendation**: the walk belongs in `cache verify`/`repair` only, never on the resolution path — the resolution hit stays a stat plus a small read, offline. That satisfies ADR-0060's exemption and AC14 simultaneously.

- **Sealing strength vs removability**: safety and architecture both want trees deletable; ADR-0060 wants accidental modification deterred. **Recommendation**: seal files (`0444`/`0555`) and leave directories `0755`. That achieves the stated deterrence — ADR-0060 already concedes sealing does not stop a same-uid attacker — while keeping pruning, repair and user cleanup working.

- **Deliberate exit-code split vs consumer benefit**: compatibility notes the split is safe (no consumer discriminates) but also unused, while code-quality argues the taxonomy is inverted relative to `kernel::Error::Refusal`'s documented meaning. **Recommendation**: fix the modelling (domain rejection as a *verdict*, not an error) and update `analyse-design-gaps/SKILL.md` to abort on exit 2 rather than entering its three-round revise loop — otherwise the split adds a signal nobody reads.

### Findings

#### Critical

- 🔴 **Architecture / Correctness / Security / Performance / Safety / Code Quality**: `cache verify`/`repair` cannot detect tree corruption, so AC14's recovery path does not exist
  **Location**: Phase 4, Step 4c: `accelerator cache` built-in
  The sentinel records the sha256 of the fetched `.tar.gz`; a `.tar.gz` cannot be reconstructed byte-identically from an extracted tree, so `verify` can only compare the sentinel's stored string against the manifest's stored string — a comparison invariant under *any* mutation of the tree. Deleting, truncating or replacing a file leaves both strings intact, so `repair` ("re-verify, and refetch any tree that fails") never fires. AC14, the Phase 4 criterion ("a truncated tree and a corrupted tree are each returned to a working state") and the Phase 7 manual check ("deleting one file from a sealed tree, then running `accelerator cache repair`, restores a working crawl") are all unachievable as specified. Since ADR-0060 removed per-exec self-healing *on the promise* that repair replaces it, this is the only remaining corruption detector.

- 🔴 **Architecture / Correctness / Safety**: Read-only sealing makes a tree undeletable, breaking pruning, repair and user cleanup
  **Location**: Phase 4, Step 4b: Tree materialisation, order of operations step 6
  On POSIX, unlinking an entry requires write permission on its containing directory, so `0555` directories mean `remove_dir_all`, `rm -rf` and `fs.rm({recursive:true})` all fail with `EACCES` on the first child. Three things break at once: `cache repair` cannot discard the failing tree it replaces; the "artifacts are pruned when Claude Code prunes old plugin versions, so no bespoke eviction logic" rationale — the entire justification for having no eviction — depends on an external recursive delete that will now fail, stranding ~294MB per plugin version; and a user told the cache is safe to delete hits permission denied on a directory they own. Correctness adds that sealing *after* the rename also publishes a writable, mid-chmod tree at its final path, weakening the atomicity the rename exists to provide.

- 🔴 **Performance / Architecture / Correctness / Security / Safety**: The warm tree lookup requires a manifest fetch, putting two network round-trips on every executor invocation
  **Location**: Phase 4, Step 4b: order of operations step 1 (and Performance Considerations)
  Step 1 defines a hit as "sentinel present and parseable, **and its digest matches the manifest's**", while Performance Considerations asserts "a stat plus a small read". These cannot both hold — `load_manifest` (`resolve/mod.rs:116-135`) performs two HTTPS GETs plus a minisign verification and is today called *only* on a cold miss. The single-file path deliberately avoids this: `cache.rs:1-3` records that "the checksum in the name lets a hit resolve offline". At 100–200 invocations per crawl this is 200–400 round-trips on a path budgeted in microseconds, breaks offline operation that works today, and puts every invocation far over work-item:0186's ~30ms target. Security adds the converse horn: if the manifest is *not* consulted, the sentinel becomes entirely self-attesting and forgeable by anything that can write the cache root.

- 🔴 **Architecture / Correctness / Compatibility / Code Quality**: The launcher→design tree handoff special-cases one token in shared dispatch, with no stated trigger, contract, or failure semantics
  **Location**: Phase 7, Section 3: Tree resolution; Section 4: Failure ordering
  "The launcher resolves both trees and hands their paths to `accelerator-design` through the environment" leaves four things unspecified. **When**: the launcher's dispatch is token-agnostic and forwards args verbatim, so either every `accelerator design …` call triggers a ~294MB cold fetch (including `validate-source` and `scrub-secrets`) or the launcher must parse the design binary's subcommand grammar, which it has never done. **What on failure**: a launcher-side `ResolutionError` exits non-zero *before* `accelerator-design` starts, so ADR-0057's mandated ordering and the non-error downgrades AC11 requires are pre-empted — AC11 and AC12 cannot pass. **How**: `ExecBinary::exec` carries no environment, and the variables are unnamed. **The override path**: `ACCELERATOR_DESIGN_BIN` short-circuits resolution, so the documented dev/test route gets no env and cannot exercise the Playwright path at all.

- 🔴 **Safety / Security**: Tree artifacts are threaded into the manifest but not into signing, upload or pre-publish re-verification
  **Location**: Phase 5, Sections 5 and 6
  Phase 5 adds `collect_artifact_entries()` and an artifact arm to `_assert_staged_manifest_is_current`, but touches neither `tasks/signing.py` nor `tasks/github.py`. `sign_staged_binaries` signs an *explicit expected set* derived from `DISPATCHED_SUBBINARIES`; `_release_uploads` and `_release_reverifies` derive their lists the same way, and the `--draft=false` publish flips once those lists re-verify. The `.tar.gz` archives and their `.minisig` sidecars fall outside all three — so the release publishes a signed manifest promising artifacts that were never signed, never uploaded and never re-verified. That is precisely the failure `_assert_staged_manifest_is_current`'s own docstring calls out as one that "cannot be recalled": every user on that version 404s on first design run, and recovery is a whole new release.

- 🔴 **Security**: The three new upstream verifications name no pinned trust anchors
  **Location**: Phase 5, Section 2: Upstream input verification
  AC13's checks are named but not designed. `playwright-core` is verified "against npm's published signing keys" — if those keys are fetched from the registry at build time, signature and key arrive over the same channel from the same host and the check proves nothing. The SLSA step states no expected source repository, workflow identity or subject digest, and `gh attestation verify` without predicates accepts an attestation from any builder. Chromium is "pinned by hash" with no stated location for that hash; if derived from the CDN at assembly time it is trust-on-first-use every release. The Node version has no stated pin either, and GPG verification implemented as "shell out and check the exit code" accepts any key merely present in the keyring — `gpg --verify` exits 0 with only a `WARNING: This key is not certified` on stderr, and `gpg` is not among the tools pinned in `mise.toml`. These checks form the *entire* chain of custody for ~294MB per platform that is then signed with the project's key and executed on every user's machine.

- 🔴 **Compatibility**: AC11's musl downgrade has no detection mechanism, and none exists in the codebase
  **Location**: Phase 7, Section 4: Failure ordering
  `HOST_PLATFORM` (`resolve/mod.rs:21-28`) is a compile-time constant evaluating to `linux-x64` on both Alpine and Debian; `TARGETS` builds Linux against `*-unknown-linux-musl` precisely so one binary runs on every libc; and the manifest's platform axis has no libc dimension. A grep for `musl`/`glibc`/`ldd` finds no runtime probe anywhere in `cli/` or `scripts/`. Without an explicit probe the musl path never reaches `unsupported-platform`: an Alpine user downloads ~294MB of glibc-linked artifacts, seals them, and fails at `execve` with a bare ENOENT from the absent dynamic loader — the hard failure ADR-0057 and AC11 exist to prevent, at maximum cost.

- 🔴 **Test Coverage**: The sequencing premise is factually wrong — `scripts/test-design.sh` runs in CI and carries ~200 lines of surviving assertions
  **Location**: Current State Analysis; Phase 8, Section 1: Floors
  `run_shell_suites(context, "scripts")` glob-discovers `scripts/**/test-*.sh` by exec bit (`tasks/test/helpers.py:96-102`), `test:integration:config` calls it, and `mise run test:integration` runs in CI (`main.yml:91`) — which the plan's own "16 discovered suites to 15" arithmetic implicitly concedes. Beyond being a runner, roughly 200 of the file's 553 lines are inline assertions over surfaces that **survive** this migration: the browser agents' `tools:` frontmatter, the evaluate-payload allowlist, the absence of `evaluate-payload-rejected` and `mcp__playwright__` from executor source, `.mcp.json` non-existence, `PROTOCOL.md`↔`daemon.js` command and env-var sync, `BLOCKING_OPS` containing `links`, and `evals.json`/`benchmark.json` validity. Phase 8 deletes the file wholesale and rescues exactly one assertion.

- 🔴 **Correctness**: The ENOTEMPTY "concurrent winner" branch adopts the very trees step 7 says must be distrusted
  **Location**: Phase 4, Step 4b: order of operations, steps 5 and 7
  Step 7 says a crash between rename and sentinel "leaves an unsealed tree that the next run re-materialises rather than trusts"; step 5 says an already-present rename target "is treated as a concurrent winner — discard the temp tree and take the existing one". After a crash the next run misses at step 1, re-downloads, re-extracts, hits `ENOTEMPTY` at step 5 against the crash leftover, and adopts it. The distrusted tree is handed straight back to the caller, possibly truncated, and no subsequent run can escape — the same sequence repeats forever. The documented crash-recovery path is unreachable and AC7's "never observes a partial one" does not hold.

#### Major

- 🟡 **Architecture / Code Quality / Test Coverage**: Daemon supervision logic lands in the binary crate with no ports, inverting the pattern for the riskiest code in the plan
  **Location**: Phase 6, Section 1: The executor subcommand
  The reuse decision, ±1s tolerance, double-checked short-circuit, state interpretation, lock policy, 30s poll and the exit-code/envelope taxonomy all land in `design-cli` and `design-adapters` with no domain module and no named ports. ADR-0053 makes the CLI "argument parsing and presentation only", ADR-0058 warns these 203 lines "encode hard-won fixes … each regresses silently if the port misses it", and AC2 explicitly requires volatile inputs "supplied through injected ports so the output is deterministic by construction". The plan already applied this lesson in Phase 1 (`derive_at` builds its own `SystemClock`) but not here.

- 🟡 **Correctness / Safety / Code Quality**: The lock-lifetime bullets contradict each other, and Rust's default CLOEXEC silently drops the inherited flock
  **Location**: Phase 6, Sections 2 and 3
  Section 2 preserves "the lock is held for the daemon's lifetime where `run.sh` inherits the flock FD into the child"; section 3 corrects "a Rust `Drop` guard releases it on every path". One guard cannot reproduce both, and the two backends genuinely differ today (`run.sh:126` leaks FD 9; `run.sh:152,202` explicitly `rmdir`s the mkdir lock before `exec`). Rust opens files `O_CLOEXEC` by default, so an unremarked port silently changes contention behaviour — presented as preserved. If either detail is wrong, two daemons start concurrently: two headless browsers, split page state, an orphaned process.

- 🟡 **Correctness**: "Empty expected value means match" preserved contradicts "native JSON parsing removes the branch"
  **Location**: Phase 6, Sections 2 and 3
  Native parsing removes only one *cause* of an empty expected start-time (a missing `jq`), not the branch. A `server-info.json` without `start_time` — from an older daemon, a truncated write, or a future `state.js` change — still yields an empty expected value and still accepts any live PID. The PID-recycle guard remains fully bypassable in exactly the state a partially-migrated or interrupted daemon leaves behind.

- 🟡 **Correctness**: Retrying a streaming fetch into a shared sink corrupts the temp file
  **Location**: Phase 4, Step 4a: Streaming download
  `Fetcher::get` retries three times; today each attempt is safe because `try_get` returns a fresh `Vec<u8>`. `get_to_writer(&self, url, sink: &mut impl Write)` breaks that: an attempt failing partway has already written bytes, and the next appends the full body after them. The sha256 catches it (fail-closed), but the retry loop can never succeed — a transient network blip becomes a permanent unrecoverable failure that presents as a checksum mismatch.

- 🟡 **Correctness / Security / Performance**: Minisign verification has no streaming path, so "no whole-archive buffer" likely means a ~300MB read-back
  **Location**: Phase 4, Step 4a
  sha256 streams trivially, but `TrustedKeys::verifies(&self, data: &[u8], signature: &str)` (`keys.rs:62`) is a contiguous-slice API. Incremental verification in `minisign-verify` requires *prehashed* signatures, and `tasks/signing.py:24-43` signs with a plain `minisign -S` with no `-H`. An implementer following the plan literally reads the 294MB temp file back into a `Vec<u8>`, giving the launcher a peak RSS an order of magnitude above anything it does today — plausibly fatal in the memory-limited containers AC6 and AC11 use.

- 🟡 **Correctness**: Phase 2 must ship the *old* downgrade vocabulary or the intermediate merged state breaks the fallback path
  **Location**: Phase 2, Section 1 (`downgrade.rs`) vs Phase 7, Section 6
  `ensure-playwright.sh` survives until Phase 7 and emits `node-missing`, `node-too-old`, `disk-floor-not-met`, `cache-unwritable` and `bootstrap-failed` (`:131,139,155,280,293,308,339,352`), which `SKILL.md:132` passes verbatim to `notify-downgrade --reason`. Since every phase is "independently mergeable", an intermediate release with the new vocabulary makes every real downgrade exit 2 with "unknown --reason" — the graceful-degradation path fails hard on exactly the machines that need it.

- 🟡 **Compatibility**: The downgrade vocabulary replacement misses three consumers
  **Location**: Phase 7, Section 6
  Beyond `downgrade.rs` and the notify-downgrade fixtures, the retired reasons appear in `evals/evals.json` (eval 20, `executor-bootstrap-failure-fallback`, asserting the literal `bootstrap-failed` message), `evals/benchmark.json` (six occurrences), and `PROTOCOL.md:557-563`, a table mapping every retired reason to an exit code. Phase 6 touches `PROTOCOL.md` for two unrelated stale lines but not this table. Eval 20 should be retargeted onto `artifact-unavailable` rather than deleted.

- 🟡 **Correctness**: "`design executor daemon` is not reachable" conflicts with forwarding args verbatim
  **Location**: Phase 6, Sections 1 and 3
  `run.js:18` dispatches on `args[0] === 'daemon'` and `:20` accepts the state dir from `ACCELERATOR_PLAYWRIGHT_STATE_DIR`, which the executor sets on every path. Verbatim forwarding *is* exposure. A single stray `accelerator design executor daemon` starts a second foreground daemon that binds a fresh port, overwrites `server-info.json`/`server.pid`, orphans the live daemon and never returns.

- 🟡 **Correctness**: Daemon detachment, stdio and exit-status propagation are unspecified where `exec`/`nohup` provided them for free
  **Location**: Phase 6, Section 1
  `nohup … & disown` makes the daemon SIGHUP-immune and reparented; `>>"$BOOTSTRAP_LOG" 2>&1` redirects its stdio; and the final `exec node run.js "$@"` makes the client's exit status, stdout, stderr and signal-death *be* the launcher's. A `Command::spawn` without `setsid` leaves the daemon in the caller's process group, so a Ctrl-C kills it mid-crawl; a spawn-and-wait client silently changes the exit-status semantics `SKILL.md:142-143` discriminates on.

- 🟡 **Correctness**: Making `state.js`'s wall-clock fallback fatal breaks minimal containers
  **Location**: Phase 6, Section 3 ("One more, in the retained JS")
  `state.js:60`'s fallback is reached on any non-Linux/Darwin platform, and on Linux whenever `/proc` is unreadable or `execSync('getconf CLK_TCK')` fails — common in distroless and `hidepid`-hardened environments. It usually lands within the ±1s tolerance because it is captured milliseconds after fork, so today it degrades to working. Making it fatal turns a usually-working path into a daemon that refuses to start, in exactly the container environments AC6 and AC11 exercise.

- 🟡 **Correctness**: No stated mechanism carries the resolved browser path into `chromium.launch()`, and the seal may block playwright-core's registry writes
  **Location**: Phase 7, Sections 1, 2 and 5
  `daemon.js:106` calls `chromium.launch({headless: true})` with no `executablePath`, and Phase 7's only stated `daemon.js` change is the diagnostic string. Without an explicit argument, `playwright-core` resolves from its own browser registry — the mechanism both the bundled tree and the hatch need to override — so AC12 cannot pass. Separately, `playwright-core` validates and in some paths writes registry markers under the browsers root, which a `0444`/`0555` tree makes impossible.

- 🟡 **Security / Correctness**: Extraction validation covers two of the several tar entry types that can escape the root
  **Location**: Phase 4, Step 4b, step 4
  Rejecting `../` paths and escaping symlinks omits hardlink entries (whose link target can equally point outside), absolute-path entries, symlink-then-traverse chains that defeat purely lexical normalisation, device/FIFO entries, and setuid/setgid/sticky mode bits — the last mattering because the plan explicitly preserves archive executable bits. There is also no entry-count or uncompressed-size cap, and no download size bound (the manifest's `PlatformEntry` carries no `size`).

- 🟡 **Security**: Phase 5's CI-side extraction of unverified upstream archives runs with release privileges
  **Location**: Phase 5, Section 3: Assembly
  Assembly extracts an npm tarball and — critically — the Chromium zip whose custody the plan itself records as TLS-only with no signature, inside `Prepare` steps carrying `GH_TOKEN` in a job holding `contents: write` and `attestations: write`. A path-traversal entry could overwrite a `tasks/*.py` module the later Sign step imports, giving code execution in a step that subsequently holds `ACCELERATOR_RELEASE_SECRET_KEY`. This is the one place genuinely untrusted bytes are unpacked with privilege, and the plan specifies hardening only for the launcher.

- 🟡 **Security**: The ported `validate-source` SSRF classification carries its existing gaps forward verbatim
  **Location**: Phase 2, Section 1 (`source_location.rs`)
  A verbatim port preserves real gaps: `classify_internal` matches `::ffff:127.0.0.1` but not `::ffff:169.254.169.254` or `::ffff:10.0.0.1`, misses IPv6 unique-local `fc00::/7`, CGNAT `100.64.0.0/10`, `0.0.0.0/8` beyond the exact string, and `0:0:0:0:0:0:0:1`; the octal rejection `^0[0-9]+\.` inspects only the first octet. Rather than transcribing the regexes, parse with `std::net::IpAddr` and classify via `is_loopback`/`is_private`/`is_link_local`/`is_unspecified` plus explicit `fc00::/7`, CGNAT and IPv4-mapped unwrapping, treating numeric-looking hosts that fail strict parsing as rejections.

- 🟡 **Security / Code Quality**: `resolve-auth` is being ported as a first-class subcommand for a consumer that does not exist
  **Location**: Phase 2, Section 3; Phase 6, Section 5
  `makeAuthHeaderHandler` is imported at `daemon.js:11` and never called, and its second required input `ACCELERATOR_BROWSER_LOCATION_ORIGIN` is set nowhere in the repository — the header-auth path is doubly dead. Meanwhile `SKILL.md:89-95,196` documents the handler's origin allowlist as security-critical. Users are told to put real bearer tokens into the environment of a browser-driving daemon for a feature that never applies them, and a crawl of an authenticated app silently produces an unauthenticated inventory.

- 🟡 **Security / Safety**: `cache repair [<name>]` turns a user-supplied token into a recursive-delete path
  **Location**: Phase 4, Step 4c
  The tree path is built as `trees/<name>-<version>-<sha256>` with no stated validation, and repair's job is to recursively remove whatever that resolves to — outside version control, under the plugin root or wherever `ACCELERATOR_CACHE_DIR` points. Separately, teardown ordering is unspecified: removing the tree before the sentinel leaves a window where a trusted sentinel points at a half-deleted tree.

- 🟡 **Compatibility**: The artifact asset-name/URL convention is never stated, and producer and consumer land in separate phases
  **Location**: Phase 4 Steps 4a/4b; Phase 5 Sections 3 and 5
  For single-file binaries the convention is pinned in one place with a comment (`resolve/mod.rs:144-147`). Here the `.tar.gz` suffix and the driver/browser key naming are new degrees of freedom, Phase 4's consumer is tested only "against a synthetic tarball", and a disagreement surfaces only in Phase 7's container fixture — after both halves are merged.

- 🟡 **Compatibility**: Retiring `browser-executor` removes the only mechanism by which two agents learn an absolute path
  **Location**: Phase 6, Section 7
  No agent in `agents/` references `${CLAUDE_PLUGIN_ROOT}` or invokes `accelerator` today — the established pattern is preload-a-skill-that-injects-resolved-values, and both agents carry a preload guard with a user-facing failure message that goes away with it. If `${CLAUDE_PLUGIN_ROOT}` is not expanded inside a subagent's Bash environment, all ~40 rewritten call sites resolve to `/bin/accelerator` with no diagnostic left. `docs-site/…/releases-and-compatibility.md:41-44` also cites `browser-executor` as one of two mechanisms justifying the documented **minimum Claude Code v2.1.144**, and neither Phase 6 nor Phase 8's doc sweep covers it.

- 🟡 **Compatibility / Correctness**: The Darwin `ps lstart` port introduces a TZ/DST hazard the LANG-only guard does not cover
  **Location**: Phase 6, Sections 1 and 4
  Converting a local wall-clock string to epoch seconds needs the UTC offset *at that instant*. The workspace's `time` crate is featured `["parsing"]` only; `local-offset` is separate and `current_local_offset()` returns `IndeterminateOffset` in a multi-threaded Unix process. The proposed guard covers `LANG`/`LC_ALL` but not `TZ`, and correctness adds that the DST fall-back hour maps one string to two instants — up to 3600s of error against a ±1s tolerance. Prefer a kernel-sourced epoch value (`sysctl KERN_PROC_PID`'s `p_starttime`) on both sides.

- 🟡 **Compatibility**: `flate2`'s backend must be pinned to `rust_backend` or the musl-static build breaks
  **Location**: Phase 4, Step 4b, Section 1
  The plan frames the backend as a size question, but `flate2`'s non-default backends pull `libz-sys`/`zlib-ng-sys`, requiring a C toolchain and breaking the fully-static musl cross-build ADR-0046 depends on. Cargo unifies features across the workspace, so a future crate enabling a C backend pulls it into the launcher silently — `test_launcher_feature_graph.py` pins TLS/DNS crate selection but has no compression entry.

- 🟡 **Performance / Compatibility / Safety**: Version-keyed tree naming forces a full ~294MB refetch on every pre-release
  **Location**: Phase 4, Step 4b layout; Performance Considerations
  `<name>-<version>-<sha256>` embeds `CARGO_PKG_VERSION`; the repo is at `1.24.0-pre.36`, i.e. 36 pre-releases within one minor, each producing a new cache key for byte-identical artifacts. `ACCELERATOR_CACHE_DIR` does not help — the *name* still embeds the version, so a persisted cache accumulates duplicates rather than reusing them, and outside the plugin tree nothing prunes them. ADR-0060's claim that cross-version sharing "would need no redesign" is exactly what this layout prevents.

- 🟡 **Performance / Safety**: The large-payload fetch has no chosen deadline, no throughput floor, no stall detection and no resume
  **Location**: Phase 4, Step 4a
  "Give tree fetches a larger one" names no value. Three properties compound: retries re-download from byte zero with no `Range` resume; blocking reqwest has no idle/read-stall timeout (stated in the constant's own comment), so the deadline is the only bound and a connection stalled at byte one is indistinguishable from a slow one; and the 294MB figure is the *uncompressed tree*, while the deadline governs the *compressed archive* — a number Phase 5 only "records". A user on a 2Mbps link waits three full attempts, transfers ~750MB, and still fails with no diagnostic.

- 🟡 **Performance / Architecture**: Concurrent cold materialisation duplicates the entire download and extraction
  **Location**: Phase 4, Step 4b step 5; Phase 4 Success Criteria
  The design is "both extract, first rename wins, loser discards", and the criterion only asserts both succeed. Each racer independently streams ~294MB, hashes, verifies, extracts and recursively chmods — and one deletes all of it. Two concurrent first runs cost ~588MB of transfer and ~1.2GB transient disk. `bin/accelerator:317-345`'s `acquire_lock` guards only the launcher binary fetch, not tree resolution.

- 🟡 **Performance / Test Coverage / Code Quality**: The performance criteria record observations rather than gate against thresholds
  **Location**: Phase 4/5/7 Manual Verification; Performance Considerations
  "In the same order as today's"; "measured and recorded"; "acceptable, and is recorded"; "recorded against the ~117MB / 177MB estimates" — none names a pass/fail condition, baseline or method. This is a regression against work-item:0186, which used a host-relative ratio gate (`after ≤ 0.5 × before`), 50 interleaved samples in one process with alternating order, and recorded host, OS build and inode. A 2× warm-path regression, a doubled binary size or a 20-minute first run all pass as written.

- 🟡 **Performance**: The "reconsider if it exceeds a few hundred KB" gate is unactionable and targets the wrong axis
  **Location**: Phase 4, Step 4b Section 1
  0186 measured shim exec plus minisign verify of a 7.6MB launcher at ~6.8ms, with minisign alone ~2.3ms for 8MB — roughly 0.3ms/MB, so a few hundred KB is ~0.1ms, likely below the measurement noise floor. Meanwhile the axis the plan does not mention is the one that actually matters for the backend choice: `miniz_oxide` is materially slower at inflating than a zlib-ng build, and the cold path inflates ~294MB.

- 🟡 **Safety**: A persistent materialisation failure becomes a full-size refetch on every executor invocation
  **Location**: Phase 7, Sections 3 and 4
  With 100–200 invocations per crawl and no negative caching, backoff or per-session memory, a persistent failure (disk full, read-only plugin root, flapping link, a 404 for one platform) produces a fresh full-size attempt — times three fetch retries — on each one. A single crawl on a failing machine can attempt tens of gigabytes and repeatedly fill the user's disk with partial archives.

- 🟡 **Safety / Correctness / Code Quality**: The one-hour orphan reaper races a slow legitimate extraction and can publish an incomplete tree
  **Location**: Phase 4, Step 4b: Orphan reaping
  Age alone is the liveness signal, and the temp directory's mtime is set at creation, not while it is filled — while the same phase deliberately raises the fetch deadline past 300s. The benign outcome is a failed extraction; the dangerous one is deletion in the window between extraction completing and the rename, publishing an incomplete tree that is then sealed and sentinelled as verified, which per ADR-0060 nothing will ever re-check. The sweep also runs only on a miss, so once a tree materialises orphans persist forever, and it does not cover orphaned temp *archives*.

- 🟡 **Safety**: The mid-use repair safety argument does not transfer from single-file exec to a lazily-loaded tree
  **Location**: Phase 4, Step 4c
  The inode argument holds for an executable already mapped by an exec'd process. It does not hold for a directory Chromium and Node open lazily over the process lifetime — locale packs, `.pak` resources, `icudtl.dat`, later-`require`d modules. And since `rename` onto a non-empty directory is `ENOTEMPTY`, repair must delete the old tree first, unlinking exactly the paths a live daemon has not yet opened. The likely moment to run repair is *during* a misbehaving crawl.

- 🟡 **Safety**: Every release now depends on three third-party hosts and a GPG key set, with no reuse of pinned artifacts
  **Location**: Phase 5, Sections 2 and 3
  Assembly is wired into both `prerelease_prepare` and `release_prepare` and fetches from `registry.npmjs.org`, `nodejs.org/dist` and `cdn.playwright.dev` on every cut — yet all three inputs are pinned by exact version and hash, so the produced bytes are identical release after release. An npm outage, a key rotation or a yanked version makes the pipeline unreleasable, including for an urgent fix to something unrelated.

- 🟡 **Safety**: No functional validation of the assembled artifact before it reaches every user
  **Location**: Phase 5, Section 3 and Success Criteria
  Every Phase 5 gate is about provenance and shape; nothing ever executes what was built. A brand-new step composing four platforms from three upstreams can produce a correctly-signed, correctly-hashed, structurally-wrong tree — wrong architecture, missing `NOTICES/`, a layout `playwright-core` cannot resolve — that passes every gate, reaches every user, self-heals never, and is faithfully re-fetched by `cache repair`.

- 🟡 **Safety**: A 1.2GB payload makes the destructive `gh release delete --cleanup-tag` fallback far more reachable
  **Location**: Phase 5, Section 7
  `upload_and_verify_release` treats any non-`AssetVerificationError` exception as a reason to delete the release *and its tag*, after `_publish` has already committed, tagged and pushed the version bump. `download_release_asset` hard-codes `timeout=120`, which a 177MB artifact will blow through on the re-verify path — raising `TimeoutExpired`, not `AssetVerificationError`, landing squarely in the delete branch. A transient hiccup burns a version number and leaves repo and release host inconsistent.

- 🟡 **Safety**: Dropping `disk-floor-not-met` and `cache-unwritable` removes the only pre-flight disk guard, as the footprint grows tenfold
  **Location**: Phase 7, Section 6
  Both conditions still arise and are now more likely: first run needs headroom for a ~294MB archive *plus* its extracted copy (~600MB peak, more with both trees or a concurrent resolution), and the cache root's unwritability is already modelled as `CacheRootUnavailable`. Today `ensure-playwright.sh` refuses up front with a named reason; afterwards a disk-full condition surfaces mid-extraction as a generic `artifact-unavailable`, having already consumed the remaining space.

- 🟡 **Test Coverage**: Phase 2's and Phase 6's "remove their invocations" understates the edit, and leaves CI red on merge
  **Location**: Phase 2, Section 6; Phase 6, Section 8
  The deleted scripts are exercised by *inline* assertions, not just invocations: `test-design.sh:169-274` runs ~60 `validate-source.sh` assertions directly, `:282-336` covers `resolve-auth`/`scrub-secrets`, `:350-425` covers `audit-cue-phrases`, and `:442-482`/`:518-528` assert the `browser-executor` contract Phase 6 retires. Since that file runs in CI, both phases as written leave `test:integration:config` red — so neither is independently mergeable as described.

- 🟡 **Test Coverage**: "One success path and one failure path per subcommand" discards an order of magnitude of boundary coverage
  **Location**: Phase 2 Success Criteria
  For `validate-source` alone, the deleted suites pin the RFC1918 boundary at both edges *and* the two just-outside cases (differentiating the reject path by stderr content), IPv6 zone-id/mapped/wildcard/bracketed forms, decimal/hex/octal encodings, the `user:pass@127.0.0.1@evil.com` confusion class, and unknown-flag exit 2. Widen the RFC1918 bound or drop the userinfo rejection and a two-tests-per-subcommand suite still passes. AC1 states "at least" one of each as a floor; adopting the floor as the plan silently drops SSRF-shaped coverage where it matters most.

- 🟡 **Test Coverage**: The container fixtures carrying AC6, AC11 and AC12 are asserted but nowhere specified
  **Location**: Phase 7 Success Criteria; Testing Strategy
  No base images, no mise/invoke task, no workflow job (contrast Phase 6, which does wire `test:unit:design-automation`), no statement of where the artifacts under test come from — the real release host will not carry `artifacts` entries until a release built *after* this work merges — and no treatment of building the launcher and design binary for the container's platform. The repo's only container precedent (`tasks/test/e2e.py:34-125`) specifies image, platform, mounts, networking and a docker preflight.

- 🟡 **Test Coverage**: "Characterization tests derived from `test-run.sh`" — that oracle does not exist
  **Location**: Phase 6 Success Criteria
  `test-run.sh` covers none of the six named behaviours: it contains structural/shellcheck checks, the `start_time_of` locale comparison, a ping/daemon-stop/links block and a survives-shell-exit smoke test — and everything from line 65 self-SKIPs without a real Playwright install, which CI does not have. The only oracle is `run.sh`'s source, and no seams are specified, so "PID-recycle rejection" has no deterministic construction and "lock contention" invites sleep-based synchronisation.

- 🟡 **Test Coverage**: Three of Step 4b's stated properties have no automated test
  **Location**: Phase 4 Success Criteria
  Sealing appears only under Manual Verification and nothing asserts the *executable* bit survives — a `0444` Node binary breaks every downstream path with an errno rather than a red test. The crash-between-rename-and-sentinel window is claimed but never constructed. Orphan reaping is time-dependent with no clock seam, so it is either untested or tested by sleeping. All three are AC7's own properties.

- 🟡 **Test Coverage**: Phase 1's `FakeClock` test cannot verify what AC15 claims
  **Location**: Phase 1, Section 4
  `Clock::filename_timestamp` *is* the seam a `FakeClock` replaces, so a fake returns a canned string and the renderer never runs — the test asserts plumbing, not format. The property AC15 wants is already covered: `format_filename_timestamp` is pure and has a unit test pinning a fixed instant to `"2026-07-13-090507"`, digit-for-digit `inventory-metadata.sh:11`'s format. The genuinely new risk — the arg→variant mapping — is only covered by a shape-level golden.

- 🟡 **Test Coverage**: `design notices` has no tests, and AC16 is manual-only in both phases that touch it
  **Location**: Phase 7, Section 7; Phase 5, Section 8
  `notices` is one of the seven recorded subcommands, so AC1 requires a success and a failure path; it has neither. The redistribution-notice obligation — the plan's stated substitute for a legal review gate — has no automated guard, so an assembly refactor that drops a `NOTICES/` component ships silently.

- 🟡 **Test Coverage**: AC8's producer/consumer round trip is verified only by hand, on one platform
  **Location**: Phase 5 Success Criteria
  Nothing closes the loop between `tasks/manifest.py`'s new `collect_artifact_entries()` and the launcher's new `ArtifactEntry` parser. The two halves of the tree-artifact contract are developed in different languages in different phases; a field-name or digest-scope mismatch surfaces at release time.

- 🟡 **Test Coverage**: Two of the three upstream verifications cannot be tested against recorded fixtures as written
  **Location**: Phase 5 Success Criteria
  `gh attestation verify` contacts a transparency log and GPG needs a keyring; if both are mocked the tests assert that a subprocess was invoked, not that its verdict is honoured — reversing the exit-code check would still pass. Node/GPG *is* fully offline-verifiable against a committed key plus a recorded signed `SHASUMS256.txt`; the SLSA branch should inject the runner and assert both outcomes, with the plan stating plainly that the attestation's content is not verified in tests.

- 🟡 **Architecture**: Tree resolution has no port — it is an adapter-only module reached from the composition root
  **Location**: Phase 4, Steps 4b and 4c
  `resolve/tree.rs` is a new module in the outbound adapter subtree, called directly from `main.rs` and from the new `cache` built-in. `launch/core.rs` holds both existing driven ports and the error taxonomy, and `cli/pup.ron` pins it to std/kernel/self — so the second artifact class arrives with no vocabulary in the core at all. The `cache verify|repair` use case then has no fakeable seam.

- 🟡 **Architecture**: "Each independently mergeable" is overstated, and Phase 5 ships ~1.2GB per release with no consumer
  **Location**: Implementation Approach; Phase 5
  The plan itself records Phase 3→1, Phase 7→4+5+6, Phase 8→everything. More consequentially, Phase 4 adds `tar`+`flate2` to a launcher whose size is a per-invocation cost for *every* sub-binary, and Phase 5 begins assembling and publishing ~1.2GB per release (twice per stable cut) before any user-visible consumer exists in Phase 7. If Phase 7 slips, that cost accrues indefinitely.

- 🟡 **Code Quality**: `include_str!` as the production data source splits two vocabularies across crates
  **Location**: Phase 2, Section 3
  The downgrade enum lives in the domain crate while its message table is `include_str!`'d into the binary, coupled only by runtime string equality — a new variant compiles cleanly and fails at runtime. The cue-phrase file's stated role as canonical-shared-with-`extract-work-items` becomes unenforced. The workspace already has the right pattern (`cli/corpus/src/frontmatter_validation/schema.rs:277`): canonical data as a `const` in the domain crate, `include_str!` only inside a `#[cfg(test)]` drift test.

- 🟡 **Code Quality**: Three outcome classes mapped onto a two-variant error taxonomy, inverting `Refusal`'s documented meaning
  **Location**: Phase 2, Section 3
  Every sub-binary maps `Refusal → 2` and everything else → 1, with `Refusal` documented as "a subcommand-scoped, caller-actionable refusal". Under the plan's mapping a *usage* error becomes the `Refusal` while a *domain rejection* — the most caller-actionable outcome in the binary — becomes `Failed`, sharing exit 1 with genuine internal failures. Model the rejection as a domain verdict rendered by the command layer, not as an error.

- 🟡 **Code Quality / Architecture**: The domain modules are a one-per-deleted-script decomposition carrying shell vocabulary
  **Location**: Phase 2, Section 1
  The five modules map one-to-one onto the five deleted scripts and several are named for activities (`secret_scan`, `cue_phrases`) rather than domain concepts; `source_location.rs` bundles four separable concerns. Compare the corpus crate this plan says it copies, whose modules are domain nouns. Architecture adds that the crate has no stated organising concept, so "does this have one reason to change?" has no answer at crate level.

- 🟡 **Code Quality**: Two phases delete scripts whose SKILL.md call sites are not in the phase's change list
  **Location**: Phase 6, Section 8; Phase 7, Section 8
  Migration Notes claims call sites are rewired in the phase that deletes them, but Phase 6 deletes `run.sh` without listing `inventory-design/SKILL.md`, which invokes `scripts/playwright/run.sh ping` at `:139`; and Phase 7 deletes `ensure-playwright.sh` while listing no SKILL.md at all, though Steps 4–6 (`:117-133`) invoke it and parse its `ACCELERATOR_DOWNGRADE_REASON=` stderr protocol. No phase removes the residual `Bash(...scripts/*)` rules.

- 🟡 **Code Quality**: The downgrade reason is decided in one process from evidence that only exists in another
  **Location**: Phase 7, Sections 3–4
  The executor can only observe presence or absence of an environment variable, so network failure, `SignatureMismatch`, an all-zeros sentinel and a corrupt tree all collapse into the same `artifact-unavailable`. The plan also does not say what replaces the `ACCELERATOR_DOWNGRADE_REASON=` stderr protocol `SKILL.md:127` greps for, nor whether tree failures are `Refusal` or `Failed` — which silently decides `--fail-safe` behaviour via `swallow_under_fail_safe`.

#### Minor

- 🔵 **Correctness**: The unlocked stale-state deletion in the pre-lock check is a TOCTOU neither list mentions
  **Location**: Phase 6, Section 1
  `run.sh:106-121`'s first reuse check runs before the lock and ends with an unconditional `rm -f "$INFO" "$PID_FILE"`. `state.js:63-66` writes those two files as separate atomic renames, so a launcher reading between them judges the state stale and deletes a live daemon's files outside any lock. Two concurrent launchers can orphan a healthy daemon.

- 🔵 **Correctness**: The Linux start-time computation must use truncating integer division
  **Location**: Phase 6, Section 1
  `run.sh:25` and `state.js:49` both truncate. Floating-point division, or `(btime * hz + ticks) / hz`, differs by up to a second — absorbed by the ±1s tolerance in isolation, but consuming the entire budget the tolerance exists to provide for whole-second-boundary drift.

- 🔵 **Correctness**: `about:blank` is an accept path a "whole decision tree" reimplementation would plausibly drop
  **Location**: Phase 2, Section 1
  `validate-source.sh:198` classifies `about:blank` as its own scheme which falls through every rejection to `exit 0`. Relatedly, Phase 6 lists "exit codes 0–3 keep their current meanings" as preserved while Phase 7 redefines exit 3, leaving `PROTOCOL.md:555-566`'s exit-code table describing a contract nothing implements.

- 🔵 **Security**: The Playwright daemon has no request authentication
  **Location**: Phase 6
  The daemon serves JSON commands on `127.0.0.1` with an OS-assigned port and no auth (`daemon.js:286-338`); loopback binding is not a uid boundary. Any local process can drive a browser that may hold the user's authenticated session. While the launcher is being rewritten, a random per-daemon token in the already-`0700` `server-info.json` closes this for a few lines.

- 🔵 **Security**: `scrub-secrets`' literal-substring scan misses the bare auth token
  **Location**: Phase 2, Section 1
  `ACCELERATOR_BROWSER_AUTH_HEADER` holds a full `Name: value` pair (`auth-header.js:14-17` splits on the first colon), so the scan only fires if the artifact contains name *and* credential together. An artifact rendering just the bearer token — the likely leakage shape — matches nothing.

- 🔵 **Compatibility**: An absent `artifacts` key, an absent platform entry and the all-zeros sentinel have no stated common outcome
  **Location**: Phase 4, Step 4a
  The reused `bare_sha256` path returns `AssetNotFound` with the literal detail "manifest sentinel (no binary for this version)" — misleading for a tree, and a hard error rather than the `artifact-unavailable` downgrade Phase 7 requires. With `ACCELERATOR_RELEASE_BASE_URL` pointed at an older release, or during a partial publish, the executor fails hard.

- 🔵 **Compatibility**: The exit-code split adds a signal the only consumer ignores
  **Location**: Phase 2, Section 3
  `analyse-design-gaps/SKILL.md:125-132` treats any non-zero `audit-cue-phrases` exit as a content failure and retries up to three times — so a usage error (missing file) now sends the model into a three-round revise loop against a file it cannot read. A two-line SKILL.md edit is what makes the split worth having.

- 🔵 **Test Coverage**: The new node-suite task gains no discovery floor while the suite set churns
  **Location**: Phase 6, Section 6; Phase 8, Section 1
  Phase 6 deletes two suites and Phase 7 a third. The repo's established answer is a count floor (`_EXPECTED_CONFIG_SUITES` and friends), documented as guarding against a regression net silently vanishing while the task still exits 0 — which is exactly what this task is being added to prevent.

- 🔵 **Test Coverage**: Decrementing `_EXPECTED_CONFIG_SUITES` to 14 against 15 actual suites bakes in a blind spot
  **Location**: Phase 8, Section 1
  `_require_suite_floor` is an at-least floor whose documented job is to fail when an exec bit is dropped. A floor equal to the actual count is the guard at full strength; each unit of headroom is one suite that can silently leave CI.

- 🔵 **Test Coverage**: The deleted `test-notify-downgrade.sh` enforced exhaustiveness the plan does not carry forward
  **Location**: Phase 2, Section 6; Phase 7, Section 6
  It loops every key in the message map for golden equality and asserts set-equality between map keys and fixture directory. Phase 7 rewrites the whole vocabulary; without an exhaustiveness guard a reason can gain a message with no golden, or keep a golden after being dropped, and nothing fails. Iterating the enum in the test makes this a compile-or-test failure.

- 🔵 **Safety**: Nothing surfaces the repair path at the moment a tree is discovered broken
  **Location**: Phase 4, Step 4c; Phase 7, Section 4
  ADR-0060 accepts that a truncated tree "surfaces as a confusing runtime failure until the repair path is run", and the plan's only mitigation is documentation. Self-healing needed no discovery; this needs the user to already know a command exists that is not mentioned in the failure they are looking at.

- 🔵 **Safety**: Hundreds of megabytes are stranded on both sides of the migration with no reclaim verb
  **Location**: Migration Notes; Performance Considerations
  The legacy `${ACCELERATOR_PLAYWRIGHT_CACHE}/<sha8>` namespaces — one per historical lockfile hash, each holding a full Chromium — are abandoned wholesale and the sweep dies with `ensure-playwright.sh`. Meanwhile `ACCELERATOR_CACHE_DIR`, the documented escape, sits outside the plugin tree where the pruning argument does not apply. The plan already creates a namespace "with room for later verbs"; spend one on `prune`.

- 🔵 **Code Quality**: Porting runtime sanitisation of data the binary itself compiles in
  **Location**: Phase 2, Section 1 (`downgrade.rs`)
  The bidi-override and printable-ASCII filters are meaningful in the shell because messages are read at runtime from a JSON file. Once the reason is a clap enum and messages are compiled in, the only input those filters see is the binary's own constants — dead defensive logic testable only by mutating the shipped table. Keep the invariant as a test over the constant table; drop the per-invocation filter.

- 🔵 **Code Quality**: A shell-availability workaround becomes two lock protocols with different lifetimes
  **Location**: Phase 6, Section 1
  The flock-or-mkdir dichotomy exists because the `flock(1)` *binary* is absent on macOS — a constraint that vanishes in Rust, where `flock(2)`/`fcntl` is available on every supported target. ADR-0058 already records that nothing external depends on the mkdir form. Collapse to one implementation unless there is an NFS reason, and say so if there is.

- 🔵 **Code Quality**: Dead-code observations in the retained JavaScript sit under "Changes Required" with no disposition
  **Location**: Phase 6, Section 5
  Three findings (the uncalled `makeAuthHeaderHandler` import, the orphaned `/dev/null` fd, two stale `PROTOCOL.md` lines) are listed with no stated change, so they will either be silently skipped or fixed ad hoc mid-phase.

- 🔵 **Code Quality**: Orphan reaping is bolted onto the lookup function with a hardcoded age and no clock seam
  **Location**: Phase 4, Step 4b, Section 2
  `find`'s single-file counterpart is `#[must_use]` and read-only; giving the tree version a destructive side effect means a maintainer asking "is this cached?" gets a garbage collector. The same module also accumulates layout, download, verification, extraction, rename, sealing, sentinel and sweep — seven responsibilities before it is written, several of which `cache repair` needs independently.

- 🔵 **Code Quality**: A retained message is pinned byte-for-byte while naming a script deleted an earlier phase
  **Location**: Phase 2 Manual Verification; Phase 7, Section 6
  `executor-ping-failed`'s message tells the user to "run `run.sh ping` manually to diagnose". Phase 6 deletes `run.sh`; Phase 7 rewrites the messages. Between them the plugin ships a diagnostic whose remediation cannot be followed — and Phase 2's byte-for-byte pin actively prevents fixing it early.

- 🔵 **Architecture**: No single-flight or resumable transfer; the reaper is age-based rather than ownership-aware
  **Location**: Phase 4, Steps 4a and 4b
  A failure at 90% of a 294MB transfer discards everything. The repo's other reclaim protocols gate on the owner's liveness; this one does not.

- 🔵 **Architecture**: The work item's cross-item coordination requirement is not carried into the plan
  **Location**: Implementation Approach; Phases 4 and 5
  The work item requires owners to sync before merging any change to `resolve/` or `tasks/release.py`, naming 0195 and 0197 as concurrent siblings. The plan touches both surfaces heavily and never mentions it; an unsequenced merge produces semantic conflicts a textual merge will not catch.

- 🔵 **Compatibility**: The new `tree.rs` breaks `cache.rs`'s `#[cfg(unix)]` convention
  **Location**: Phase 4, Step 4b, Section 2
  Windows is correctly out of scope, but `cache.rs` deliberately keeps `#[cfg(unix)]`/`#[cfg(not(unix))]` pairs so the launcher still type-checks off Unix. Either follow it or state in the module docs that tree materialisation is Unix-only by design.

- 🔵 **Performance**: A second `Fetcher` on the warm path, and the release assembly runs twice per stable cut
  **Location**: Phase 4, Step 4a; Phase 5, Section 7
  Each `Fetcher` builds a `reqwest::blocking::Client`, installing the rustls provider and a background runtime thread — and `FetchVerifyCacheResolver::new` already builds one on *every* invocation including warm hits. Use `RequestBuilder::timeout()` on the single client instead, constructed lazily. Separately, the pre.0 pass assembles byte-identical artifacts from the same pinned inputs; skipping any archive already present with a matching digest halves the most expensive runner's work.

- 🔵 **Performance**: Tree resolution is not scoped to the executor subcommand
  **Location**: Phase 7, Section 3
  Five of seven subcommands are pure local computation. On a warm cache this is two wasted sentinel reads per invocation; on a cold cache, `accelerator design validate-source https://example.com` triggers a ~294MB download to validate a URL.

#### Suggestions

- 🔵 **Architecture**: Name tree entries by `<name>-<sha256>` and record the release version in the sentinel
  **Location**: Phase 4, Step 4b layout
  Lookup then keys on the digest the manifest names, an unchanged artifact across two plugin versions is a hit for anyone with a shared cache root, and the plugin-root default keeps today's per-version pruning behaviour unchanged. This makes ADR-0060's "no redesign needed" claim true by construction.

- 🔵 **Architecture**: The artifact set has no registry, unlike `DISPATCHED_SUBBINARIES`
  **Location**: Phase 5, Section 6
  `driver` and `browser` would be spelled independently in `assemble.py`, `manifest.py`, the release guard, the contract test and the launcher's consumer, with no guard tying them together — precisely the drift the registration checklist prevents for sub-binaries.

- 🔵 **Test Coverage**: `resolve_optional` is copied verbatim with no mention of copying or sharing its tests
  **Location**: Phase 7, Section 5
  The env-beats-config precedence and whitespace-collapse edges are the mechanism AC12 rests on, and would otherwise be verified only through an under-specified container fixture.

- 🔵 **Test Coverage**: Phases 3, 7 and 8 lack the "failing test written first" opening criterion the other five carry
  **Location**: Phase 3/7/8 Success Criteria
  Phase 7 is the largest behavioural change in the plan and its criteria are dominated by container fixtures — making it the likeliest place for tests to be written after the fact around whatever the code does.

### Strengths

- ✅ The manifest extension's additive claim is *verified* against the real parser rather than asserted — no `deny_unknown_fields`, `binaries` already `#[serde(default)]`, a gate that rejects only strictly-greater versions, and an existing `"future_field": 42` test. Not bumping `SCHEMA_VERSION` is correct and avoids a flag day for five other tokens.
- ✅ The genuine mixed-version window is smaller than it looks and survives scrutiny: `release_base_url()` pins the base to the `v{version}` tag and `parse_and_validate` requires exact version equality, so a deployed launcher only ever reads the manifest cut for its own release.
- ✅ Verify-before-extract is stated as an invariant with the right test shape — the trees directory asserted *empty* after a rejected archive, so a failed check leaves nothing to clean up.
- ✅ Tree layout is designed against the real quirks of `cache::find` rather than an idealised model: a dedicated `trees/` subdirectory, ASCII-only names because `cache.rs:56` aborts the whole scan on one non-UTF-8 entry, and a sentinel that is never `*.minisig`.
- ✅ Trees are deliberately kept out of `ResolveBinary::resolve` — that port's contract is name → executable path with per-exec re-verify, which is precisely what trees are exempt from. Widening it would have polluted an interface every sub-binary depends on.
- ✅ Phase 6's explicit "behaviours preserved deliberately" / "behaviours corrected deliberately" split is exactly the discipline a behaviour-preserving port needs, naming each asymmetry alongside the consumer that depends on it.
- ✅ The locale regression guard from `test-run.sh:44-63` is not merely preserved but strengthened — three locale settings plus agreement with `lib/state.js`, plus a fixture-pinned oracle (`proc-stat-linux.txt` → `1700145620`).
- ✅ Phase 1 spots that `derive_at` builds its own `SystemClock` and pushes the byte-for-byte assertion down to the adapter level rather than faking determinism at the binary boundary.
- ✅ Phase 3's "Divergences to accept explicitly" argues five real behavioural deltas between the bash scripts and corpus rather than absorbing them into a parity claim.
- ✅ The plan identifies and removes vacuously-passing tests: `identity.test.js:70-95` cross-validates against a `launcher-helpers.sh` that no longer exists and swallows the failure via `catch { return; }`.
- ✅ Artifact assembly is placed in `prepare`, never `sign`, preserving the deliberate scoping of `ACCELERATOR_RELEASE_SECRET_KEY` to the Sign steps.
- ✅ Pipeline verification tests are pinned to recorded upstream fixtures rather than live network calls, so checks cannot silently pass because a host was unreachable.
- ✅ `playwright-core` is pinned to an exact version with a release-failing guard, collapsing three drifting version choices into one reviewed edit.
- ✅ The per-exec re-verification exemption is driven by real measurement (hash throughput, artifact sizes, file counts, invocations per crawl) expressed both absolutely and as a fraction of the crawl budget.
- ✅ `chromium-headless-shell` over full Chromium is well-reasoned (177MB/14 files vs 297MB/327) with a stated fallback if fidelity proves inadequate.
- ✅ The registration surface is enumerated at the level of individual mutable constants — `_SUBBINARY_DESCRIPTIONS` KeyErroring, `_DUAL_USE_SCRIPTS` pinning a literal path, `EXPECTED_INJECTION_SKILLS` being an equality not a floor — which is exactly the tacit knowledge that makes this change expensive when missing.
- ✅ `design.browser_path` via `EXTRA_KEYS` is correctly costed against the catalogue count assertion and the Rust↔bash drift test's actual extraction behaviour.
- ✅ The flat `dist/release/` naming is reasoned from a concrete upstream behaviour (`@actions/glob`'s `*` not crossing `/`) and tied to the specific test that would otherwise silently pass.
- ✅ ADR-0060's integrity model states its limits plainly rather than overclaiming, and the plan requires that difference to actually be documented in `resolve/`.
- ✅ Phase 5 adds a `timeout-minutes` and a pre-assembly disk assertion to a release job that today has neither.

### Recommended Changes

1. **Give the sentinel a tree-content integrity value** (addresses: the six-lens `cache verify`/`repair` finding, plus the `verify` naming and `repair` cost findings)
   Record a digest over the sorted `(relative path, mode, size, file sha256)` list at seal time. `verify` recomputes it by walking the sealed tree; `repair` discards and re-materialises only trees that fail. Keep that walk exclusively in `cache verify`/`repair` — never on the resolution path. If a per-file list is rejected, instead define `repair <name>` as an unconditional re-materialisation and scope `verify`'s help text honestly to "the sentinel and manifest agree".

2. **Seal files, not directories; unseal before every removal** (addresses: the sealing/removability critical; the repair-teardown and stranded-disk findings)
   Set `0444`/`0555` on files and leave directories `0755`. Seal the temp tree *before* the rename so the published tree is sealed from its first instant. Add an explicit unseal step in front of any `remove_dir_all` in repair and the reaper, and add a test asserting a sealed tree is removable by the mechanism expected to prune it — plus a test asserting the executable bit survives sealing.

3. **Make the tree hit purely local and offline** (addresses: the warm-path critical; the offline-regression and self-attesting-sentinel horns)
   Prefix-scan `trees/` for `{name}-{version}-`, take the expected digest from the directory name, and require only that the sentinel exists, parses and records that same digest. Consult the manifest solely on a miss and in `cache verify`. Add an automated criterion asserting a warm tree resolution issues **zero** HTTP requests against the `MockServer`. Consider dropping the version from the directory name (record it in the sentinel instead) so an unchanged artifact is a hit across plugin versions.

4. **Specify the launcher→design contract as an explicit section** (addresses: the handoff critical; the failure-ordering, AC11/AC12, dev-override and cheap-subcommand findings)
   Name the variables; state that resolution is **best-effort and advisory** — on failure the variables are simply absent and dispatch proceeds, so the whole downgrade ordering stays inside `accelerator-design`; state the trigger (a per-token artifact requirement declared in the manifest, or a launcher `cache ensure` the design binary invokes — not argument sniffing); state the behaviour when the design binary runs with no variables set (override path, direct invocation, tests); and carry a structured unavailability cause across the boundary so `artifact-unavailable` can distinguish a CDN failure from a signature mismatch.

5. **Add a runtime libc probe before any artifact resolution** (addresses: the musl-detection critical)
   Probe for `/lib/ld-musl-*.so.1` versus `/lib64/ld-linux-*.so.2` (or read the resolved driver Node binary's interpreter) and emit `unsupported-platform` with no network cost. Unit-test it over an injected filesystem port for both shapes — the Alpine container alone cannot distinguish "detected musl" from "failed for another reason".

6. **Extend signing, upload and re-verification to the tree artifacts, driven by one registry** (addresses: the publish-gap critical; the artifact-registry and asset-naming findings)
   Add explicit artifact arms to `sign_staged_binaries`, `_release_uploads` and `_release_reverifies`, all derived from a `TREE_ARTIFACTS` constant beside `DISPATCHED_SUBBINARIES`, with a unit test pinning the assembled/signed/manifested/uploaded/re-verified sets against each other. State the asset convention once (`accelerator-{key}-{platform}.tar.gz`) and pin it in `manifest.example.json` so both `manifest.rs`'s golden test and `test_manifest_contract.py` hold producer and consumer to one fixture. Scale `download_release_asset`'s 120s timeout with asset size and narrow the destructive `except Exception` so a transport failure preserves the draft.

7. **Pin every upstream trust anchor, and verify GPG properly** (addresses: the upstream-verification critical)
   Commit the npm registry signing keys alongside the Node GPG key set under one refresh procedure; state the SLSA predicate explicitly (expected source repo, workflow, subject digest bound to the fetched tarball) and fail without a match; commit the Chromium archive sha256 and the Node version as reviewed constants changing only alongside the `playwright-core` pin. Verify GPG against a dedicated keyring with `--status-fd` parsed for `VALIDSIG` and the fingerprint checked against a committed allowlist, and pin `gpg` in `mise.toml`. Apply the launcher's extraction rules to the CI-side extraction too, into a staging directory outside the checkout, with `GH_TOKEN` dropped from that step.

8. **Correct the CI premise and enumerate `test-design.sh`'s assertions before deleting it** (addresses: the sequencing critical; the Phase 2/6 mergeability and boundary-coverage findings)
   Fix Current State Analysis, then classify every assertion keep/retire: the PROTOCOL↔daemon sync, `BLOCKING_OPS`, the evaluate-payload allowlist and the `mcp__playwright__` guards move into the new node suites; the skill/agent structural and evals-JSON assertions move into `test-skill-frontmatter-conformance.sh` or a pytest. Replace "one success and one failure path per subcommand" with an enumerated migration checklist mapping each deleted assertion to a named Rust test, recording deliberate drops with a reason. Decrement `_EXPECTED_CONFIG_SUITES` to the exact post-deletion count, and give the new node-suite task its own discovery floor.

9. **Resolve the three Phase 6 self-contradictions and specify the executor's seams** (addresses: the lock-lifetime, empty-start-time and daemon-reachability findings, plus the ports finding)
   Decide one lock lifetime for both backends (releasing at launcher exit is the more defensible choice) and move the flock-inheritance bullet accordingly, noting the explicit `FD_CLOEXEC` handling if inheritance is kept. Treat an absent or unparseable `start_time` as a mismatch, in the corrected list. Add an explicit reserved-token rejection for `daemon`. Add a `cli/design/src/executor/` domain module holding the reuse verdict, tolerance comparison, state interpretation and envelope/exit-code mapping as pure functions over `Clock`, `ProcessProbe`, `StateStore`, `Lock` and `Spawner` ports, with `design-adapters` implementing them — which is also what makes the six named characterization tests deterministic and AC2's injected-port clause satisfiable. Specify daemon detachment (`setsid`, stdio to the bootstrap log) and exit-status/signal propagation. Prefer a kernel-sourced epoch start time on Darwin, and specify truncating integer division on Linux.

10. **Fix the streaming fetch's retry, verification and bounds** (addresses: the retry-corruption, minisign-buffer, deadline and thundering-herd findings)
    Create/truncate the temp file inside the retry loop and reset the digest state with it. Name the streaming signature mechanism explicitly and confirm `minisign -S` emits prehashed signatures — if not, add `-H` for tree artifacts or state the buffered cost and use mmap. Express the deadline as a throughput floor over the *measured compressed* size, add a progress floor so stalls fail fast while slow-but-progressing transfers get their full budget, and either add `Range` resume or reduce `MAX_ATTEMPTS` for trees. Serialise materialisation per tree key with a PID-liveness-gated lock, and change the concurrency criterion from "both succeed" to "exactly one archive fetch occurs".

11. **Make the tar extraction allowlist-based** (addresses: the entry-type findings)
    Accept only regular files, directories and inside-root symlinks; reject hardlinks, absolute paths, device/FIFO entries and every other type; mask mode bits to `0755`/`0644` before sealing. Add an additive `size` field to the artifact platform entries enforced as a download cap and as an uncompressed-bytes/entry-count ceiling. Extend the rejection tests to a hardlink escape and a setuid entry.

12. **Fix the extraction/adoption state machine** (addresses: the ENOTEMPTY critical and the reaper race)
    Make the ENOTEMPTY branch discriminate: adopt an existing target only once a valid sentinel for it appears (bounded poll); otherwise treat it as a crash leftover, unseal, remove and retry the rename once. Add a test that crashes between rename and sentinel and asserts replacement rather than adoption. Make the reaper PID-liveness-gated rather than age-based, run it on every resolution rather than only on a miss, extend it to orphaned temp *archives*, and extract it as `reap_orphans(root, cutoff)` so `find` stays a query.

13. **Close the Phase 2 → Phase 7 vocabulary and call-site gaps** (addresses: the vocabulary-ordering, missed-consumer, SKILL.md and stale-message findings)
    State that Phase 2 ports the vocabulary as-is and Phase 7 performs the replacement, and add `evals.json`, `benchmark.json` and `PROTOCOL.md:557-563` to Phase 7's file list with eval 20 retargeted onto `artifact-unavailable`. Add `inventory-design/SKILL.md` to Phases 6 and 7 with the new Step 4/5 shape stated, name the phase that drops the residual `Bash(...scripts/*)` rules, and rewrite `executor-ping-failed`'s remediation text in Phase 6 rather than Phase 7 (relaxing Phase 2's byte-for-byte pin to the messages that genuinely survive).

14. **Retain a pre-flight disk guard and add negative caching** (addresses: the dropped-reason and refetch-storm findings)
    Keep (or re-derive) `disk-floor-not-met` and `cache-unwritable`, checking free space against the manifest's known archive size *before* starting a fetch, and remove a partial temp tree eagerly on failure rather than leaving it to the sweep. Make the first `artifact-unavailable` sticky for the session so the remaining 100+ invocations take the code-only path immediately. Have tree-failure envelopes name `accelerator cache repair <name>` so recovery is discoverable from the failure. Add a `cache prune` verb covering both stale versioned trees and the abandoned legacy playwright namespace.

15. **Add a functional smoke gate to the assembly, and reuse pinned artifacts across cuts** (addresses: the no-functional-validation and third-party-dependency findings)
    Unpack the just-built driver and browser on the host platform, execute each with `--version`, and assert `NOTICES/` is populated — the only gate distinguishing "signed" from "works". Cache assembled trees keyed on (`playwright-core` version, Node version, Chromium revision) so an unchanged pin re-signs known-good bytes and the pre.0 pass reuses the stable pass's output.

16. **Convert the measurement criteria into gates** (addresses: the unfalsifiable-criteria finding across three lenses)
    Warm executor invocation `after ≤ 1.1 × before`, measured with work-item:0186's interleaved-sampling method against a pre-Phase-4 launcher on the same host. A numeric KB ceiling for the launcher size delta, derived from 0186's ~0.3ms/MB verify rate. A stated minimum throughput and wall-clock ceiling for first-run download, with host and connection recorded. Pin `flate2 = { default-features = false, features = ["rust_backend"] }` in `[workspace.dependencies]` with the musl rationale, add `libz-sys`/`zlib-ng-sys`/`zlib-sys` to `_ABSENT` in `test_launcher_feature_graph.py`, and evaluate the backend on decompression throughput as well as size. Move the genuinely informational figures out of the criteria lists.

17. **Fix the domain modelling** (addresses: the exit-code taxonomy, `include_str!`, module-decomposition, SSRF-classification and dead-sanitisation findings)
    Model domain rejection as a verdict rendered by the command layer, reserving `kernel::Error::Refusal` for its documented caller-actionable meaning, and state the three-class mapping in one table. Make the canonical vocabularies `const`s in the domain crate with `include_str!` confined to `#[cfg(test)]` drift tests. Decompose by domain concept (`Host` owning canonicalisation, `HostReach` classification, `AccessPolicy` producing a verdict) rather than one module per deleted script, and replace the transcribed regexes with `std::net::IpAddr` classification covering IPv4-mapped, `fc00::/7`, CGNAT and `0.0.0.0/8`. Recording explicitly that the check is pre-resolution only makes the DNS-rebinding gap a taken position. Drop the per-invocation bidi/ASCII filter in favour of a test over the constant table.

18. **Decide the auth surface rather than porting it unresolved** (addresses: the dead-auth-path and secret-scan findings)
    Either wire `makeAuthHeaderHandler` into the page lifecycle and set `ACCELERATOR_BROWSER_LOCATION_ORIGIN` from the validated location, with a test asserting the header attaches only for the matching origin — or retire `resolve-auth`, the `ACCELERATOR_BROWSER_*` variables and their scrub rules together. If it stays, split the header on the first colon in the scrubber so a bare token is detected, and add a per-daemon auth token to close local cross-process access while the launcher is being rewritten anyway.

19. **Declare a tree-resolution port and validate `cache repair`'s argument** (addresses: the no-port and path-construction findings)
    Declare `ResolveArtifactTree` / `VerifyArtifactTree` in `launch::core` with tree-specific `ResolutionError` variants (extraction, path-escape, seal, sentinel), implement in `resolve/tree.rs`, and put the `cache` built-in's decision logic behind the port. Validate `<name>` against the manifest's `artifacts` keys (default-deny, no path construction from raw input) and assert the canonicalised target is a direct child of `trees/` before any removal. Split `tree.rs` along its natural seams — `cache repair` needs several independently.

20. **Correct the sequencing claims** (addresses: the phase-independence, unconsumed-artifact and coordination findings)
    Replace "each independently mergeable" with the actual dependency graph, state the intended release-window relationship between Phases 5 and 7 (or make assembly conditional so no unconsumed 1.2GB is published), and add the 0195/0197 sync obligation to Phases 4 and 5. Add the "failing test written first" opening criterion to Phases 3, 7 and 8, and for Phase 7 name the unit tests pinning the failure-ordering state machine so the ordering is verified in a fast test rather than only in a container.

---

## Per-Lens Results

### Architecture

**Summary**: The plan is unusually well-evidenced: it names its seams by file:line, it inherits four accepted ADRs that close the hard decisions, and its three-crate split follows the corpus/vcs/work precedent faithfully. The structural risk is concentrated in the two places where it extends shared infrastructure — the launcher's resolver and the launcher→sub-binary handoff — where the tree-artifact abstraction is introduced as a free-standing adapter module with no port, the trigger and failure contract is left unstated, and the sealing/eviction/repair lifecycle has a self-contradiction. A second cluster: the `run.sh` port — the most regression-prone code in the plan, and the one AC explicitly demanding injected ports — is placed in the binary crate rather than the domain crate, inverting the pattern ADR-0053 makes load-bearing.

**Strengths**: trees correctly kept out of `ResolveBinary::resolve`; the manifest extension is genuinely additive and evidenced as such; the order-of-operations puts the crash-consistency boundary in the right place and treats an already-present rename target as a concurrent winner; the three-crate split matches precedent exactly including the pup rule and single-item-`use` workaround; cross-language contracts are named rather than implicit; every open question is resolved before phase one.

**Findings**:

- 🔴 **critical** (high) — *Read-only sealing makes a tree undeletable, breaking both the delegated pruning strategy and `cache repair`* — Phase 4, Step 4b sealing and Step 4c. On POSIX, unlinking requires the write bit on the containing directory, so a `0555` tree cannot be emptied even by the owning user. Claude Code's pruning, a user's `rm -rf`, and repair's discard step all hit `EACCES`. The one mechanism bounding ~294MB per plugin version cannot reclaim it, and AC14's repair cannot remove the tree it replaces. *Suggestion*: seal files only, or add an explicit chmod-then-remove to repair, with a test asserting a sealed tree is removable by the mechanism expected to prune it.
- 🔴 **critical** (high) — *The launcher→design tree handoff special-cases one token in shared dispatch, with no stated trigger or failure contract* — Phase 7 §3–4. Today's external-dispatch path is entirely token-agnostic (`LazyProductionResolver::resolve` then `UnixExec::exec`), and `exec` replaces the process so env must be set before dispatch. Unanswered: **when** (eager means `scrub-secrets` and `notices` trigger a 294MB fetch; conditional means parsing the design binary's grammar), **what on failure** (a launcher `ResolutionError` maps to `Failed` and exits before `accelerator-design` starts, pre-empting ADR-0057's ordering and AC11's non-error exit), and **the override path** (`override_path` short-circuits resolution, so the dev/test route gets no env). *Suggestion*: declare the artifact requirement per token in the manifest or via a `cache ensure` built-in; make tree-resolution failure advisory so the downgrade ordering stays wholly inside `accelerator-design`.
- 🟡 **major** (high) — *The warm sentinel hit is defined against the manifest, requiring a network round trip per invocation* — Phase 4, Step 4b step 1. `load_manifest()` is two HTTPS GETs plus verification (`resolve/mod.rs:116-135`); the single-file path deliberately avoids it (`cache.rs:1-6`: "the checksum in the name lets a hit resolve offline"). 100–200 invocations per crawl = 200–400 round trips, breaking offline operation the `design.browser_path` air-gap case depends on. *Suggestion*: prefix-scan `trees/`, accept the entry whose adjacent sentinel records the digest in its own directory name, consult the manifest only on a miss, and assert zero network I/O on a hit.
- 🟡 **major** (high) — *No tree-content integrity data exists, so `cache verify` cannot detect the corruption `repair` must fix* — Phase 4, Step 4c. A `.tar.gz` cannot be reconstructed byte-identically from an extracted tree, so what remains is a string comparison detecting a stale release and nothing about contents — exactly the failure mode ADR-0060 accepts as the cost of the exemption. *Suggestion*: record a per-file `path → sha256` manifest in the sentinel, or make `repair` unconditionally re-materialise while `verify` reports only what it can check.
- 🟡 **major** (high) — *Daemon supervision logic lands in the binary crate with no ports* — Phase 6 §1. The reuse decision, tolerance, double-checked short-circuit, state interpretation, lock policy, 30s poll and exit-code taxonomy are all real domain logic placed outside `cli/design/`, with no ports named. ADR-0053 says no business logic in the command layer; ADR-0058 warns these lines encode hard-won fixes that regress silently; AC2 explicitly requires injected ports. *Suggestion*: add a `cli/design/src/executor/` module over `Clock`, `ProcessProbe`, `StateStore` and `Lock` ports, mirroring `work`/`work-adapters`/`work-cli`.
- 🟡 **major** (medium) — *Tree resolution has no port — an adapter-only module reached from the composition root* — Phase 4 §4b–4c. `launch::core` holds both existing driven ports and the error taxonomy, and pup pins it to std/kernel/self, so the second artifact class arrives with no core vocabulary; `cache verify|repair` gets no fakeable seam. *Suggestion*: declare `ResolveArtifactTree`/`VerifyArtifactTree` in `launch::core` and put the built-in's decision logic behind them.
- 🟡 **major** (medium) — *"Each independently mergeable" is overstated, and Phase 5 ships ~1.2GB per release with no consumer* — Implementation Approach; Phase 5. The plan itself records Phase 3→1, 7→4+5+6, 8→everything. Phase 4 adds `tar`+`flate2` to a launcher verified on every warm start for every sub-binary, and Phase 5 publishes ~1.2GB twice per stable release before Phase 7 gives it a consumer. *Suggestion*: state the real dependency graph and the intended release-window relationship, or make assembly conditional.
- 🔵 **minor** (medium) — *The work item's cross-item coordination requirement is not carried into the plan* — Phases 4 and 5. 0195 and 0197 register sub-binaries and touch `resolve/` and `tasks/release.py` in the same window; unsequenced merges produce semantic conflicts a textual merge will not catch.
- 🔵 **minor** (medium) — *No single-flight or resumable transfer for a 294MB fetch* — Phase 4 §4a–4b. A failure at 90% discards everything; two concurrent cold invocations each fetch and extract the full set; the age-based reaper can race a slow in-flight download. *Suggestion*: record the resilience posture explicitly, or add a materialisation lock and a PID-liveness-gated reaper consistent with the repo's other reclaim protocols.
- 🔵 **minor** (medium) — *The `design` domain crate is scoped by directory rather than by domain* — Phase 2 §1. Five modules with no shared concept, serving two different skills, later joined by daemon supervision and browser resolution. "Does this have one reason to change?" has no answer at crate level. *Suggestion*: state what `design` means as a bounded context and organise into named sub-domains.
- 🔵 **suggestion** (medium) — *Putting the release version in the tree directory name forecloses the cross-version reuse ADR-0060 says needs no redesign* — Phase 4 §4b. A tree's identity is fully determined by its digest; the version component is what prevents a stable `ACCELERATOR_CACHE_DIR` from avoiding the refetch. *Suggestion*: name entries `<name>-<sha256>` and record the version in the sentinel.
- 🔵 **suggestion** (medium) — *The artifact set has no registry, unlike `DISPATCHED_SUBBINARIES`* — Phase 5 §6. `driver`/`browser` spelled independently across five files with no guard tying them together.

### Correctness

**Summary**: The plan is unusually well-grounded — it traces existing shell and Rust code line-by-line and names most of the hazards it inherits — but several load-bearing state transitions do not close. The tree-materialisation state machine has a genuine logic error, the sentinel cannot detect the corruption the repair path is required to fix, and the seal-after-rename ordering both publishes a mutable tree and makes later removal impossible. In the `run.sh` port, the "preserved" and "corrected" lists contradict each other in two places, and the launcher-resolves-trees design conflicts with ADR-0057's mandatory failure ordering.

**Strengths**: the behavioural contract is derived from actual source rather than prose (field indices, tolerance rationale, envelope shape, exit-0 asymmetry, the non-UTF-8 scan abort all correctly characterised); verify-before-extract is stated as an invariant with the right test; the preserved/corrected split is exactly the right discipline; Phase 3 enumerates five real deltas rather than assuming equivalence; the locale guard is carried forward and strengthened.

**Findings**:

- 🔴 **critical** (high) — *The ENOTEMPTY "concurrent winner" branch adopts the very trees step 7 says must be distrusted* — Phase 4 §4b steps 5 and 7. After a crash the next run misses, re-downloads, re-extracts, hits `ENOTEMPTY` against the leftover and adopts it; the sequence repeats forever. The documented crash-recovery path is unreachable and AC7's partial-tree guarantee does not hold. *Suggestion*: adopt only after a valid sentinel appears (bounded poll); otherwise unseal, remove and retry the rename once, with a test that crashes in the window.
- 🔴 **critical** (high) — *An archive-digest sentinel cannot detect tree corruption, so `verify`/`repair` cannot satisfy AC14* — Phase 4 §4c. gzip output is not reproducible from tree contents, so the only comparison possible is invariant under any mutation. Since ADR-0060 removed self-healing on the promise repair replaces it, this is the only remaining detector. *Suggestion*: record a digest over the sorted `(path, mode, size, sha256)` list at seal time, or make repair unconditional and drop the "refetch any tree that fails" wording.
- 🔴 **critical** (high) — *Sealing after the rename publishes a writable tree, and 0555 directories make later removal impossible* — Phase 4 §4b steps 5 and 6. Between rename and seal the tree is published at its final path while still writable and mid-chmod, so an adopting or concurrent process can execute from a partially-sealed tree — the window the atomic rename exists to eliminate. And `remove_dir_all` over a sealed tree fails with `EACCES`. *Suggestion*: seal the temp tree before the rename (bottom-up, root last), and add an explicit unseal step before every `remove_dir_all`.
- 🟡 **major** (high) — *The lock-lifetime bullets contradict each other, and Rust's default CLOEXEC silently drops the inherited flock* — Phase 6 §2–3. The two backends genuinely differ today (`run.sh:126` leaks FD 9, confirmed by `test-run.sh:160-163`; `run.sh:152,202` explicitly `rmdir`s the mkdir lock before `exec`). One guard cannot reproduce both, and Rust's `O_CLOEXEC` default silently drops inheritance. *Suggestion*: decide one lifetime and state it; if inheritance is kept, specify the explicit `FD_CLOEXEC` clearing.
- 🟡 **major** (high) — *"Empty expected value means match" preserved contradicts "native JSON parsing removes the branch"* — Phase 6 §2–3. Native parsing removes one *cause*, not the branch; a `server-info.json` without `start_time` still accepts any live PID — the state a partially-migrated or interrupted daemon leaves. *Suggestion*: treat absent/unparseable `start_time` as a mismatch, recorded as a deliberate change.
- 🟡 **major** (high) — *Launcher-side eager tree resolution runs before, and can pre-empt, the mandated downgrade ordering* — Phase 7 §3–4. Dispatch is name → path with no subcommand knowledge, so either every design call triggers a 294MB fetch or the launcher must inspect `args[0]`, which the plan neither states nor designs; and a launcher-side failure kills the process before the ordering logic runs, so AC11 and AC12 cannot pass. *Suggestion*: make resolution best-effort and subcommand-scoped, with absence owned entirely by the design binary.
- 🟡 **major** (high) — *The tree cache-hit test requires a manifest fetch, contradicting the microsecond warm path* — Phase 4 §4b step 1. Directly contradicts the "stat plus a small read" claim, the manual timing criterion and offline operation. *Suggestion*: make the hit test purely local; the digest is already in the directory name.
- 🟡 **major** (high) — *Retrying a streaming fetch into a shared sink corrupts the temp file* — Phase 4 §4a. Today each attempt is safe because `try_get` returns a fresh `Vec<u8>`; `get_to_writer` breaks that invariant and the plan does not mention truncating between attempts. A transient blip becomes a permanent failure presenting as a checksum mismatch. *Suggestion*: create/truncate inside the retry loop and reset digest state, with a mock-server test failing the first attempt after N bytes.
- 🟡 **major** (medium) — *Minisign verification has no streaming path through the existing key API* — Phase 4 §4a. `TrustedKeys::verifies` takes a contiguous slice; streaming Ed25519 verification requires prehashed mode via an API the plan never names. A literal implementation reads 294MB back into a `Vec<u8>` — plausibly fatal in the containers AC6 and AC11 use. *Suggestion*: extend `TrustedKeys` with an incremental verifier and test that the pipeline's signatures are prehashed, so a signing-flag change fails loudly.
- 🟡 **major** (high) — *Phase 2 must ship the old downgrade vocabulary or the intermediate merged state breaks the fallback path* — Phase 2 §1 vs Phase 7 §6. `ensure-playwright.sh` survives until Phase 7 and emits five reasons `SKILL.md:132` passes verbatim; the new vocabulary makes every real downgrade exit 2 with "unknown --reason", on exactly the machines that need the fallback.
- 🟡 **major** (high) — *"`design executor daemon` is not reachable" conflicts with forwarding args verbatim* — Phase 6 §1 and §3. `run.js:18` dispatches on `args[0] === 'daemon'` and `:20` takes the state dir from the env the executor always sets. A stray invocation orphans the running daemon, never returns, and leaves state files pointing at the wrong process. *Suggestion*: an explicit reserved-token rejection with a test.
- 🟡 **major** (medium) — *Daemon detachment, stdio and exit-status propagation are unspecified where `exec`/`nohup` provided them free* — Phase 6 §1. Without `setsid`, a Ctrl-C kills the daemon mid-crawl; a spawn-and-wait client changes exit-status and signal semantics `SKILL.md:142-143` depends on. Both regress silently — the failure mode ADR-0058 names as the port's main risk.
- 🟡 **major** (medium) — *Making `processStartSeconds` fail loudly converts a benign degradation into a hard daemon-start failure* — Phase 6 §3. The fallback is reached on non-Linux/Darwin, and on Linux whenever `/proc` is unreadable or `getconf CLK_TCK` fails — common in distroless and `hidepid` environments — yet usually lands within tolerance. *Suggestion*: record its provenance in `server-info.json` so the reader decides, or remove the `getconf` dependency before making it fatal.
- 🟡 **major** (medium) — *No stated mechanism carries the resolved browser path into `chromium.launch()`, and the seal blocks registry writes* — Phase 7 §1,2,5. `daemon.js:106` passes no `executablePath`, so `playwright-core` resolves from its own registry — the mechanism both the tree and the hatch must override. AC12 cannot pass, and a sealed browsers root may fail with a permission error on first run.
- 🔵 **minor** (medium) — *Orphan reaping runs only on a cache miss* — Phase 4 §4b. Once a tree materialises the sweep never runs again, so a partial archive or half-extracted tree is never reclaimed; and the enlarged fetch deadline means a slow download can outlive the one-hour threshold and be reaped mid-write.
- 🔵 **minor** (medium) — *Extraction validation covers only two of the tar entry types that can escape the root* — Phase 4 §4b step 4. Hardlinks, character/block/fifo entries and setuid/setgid bits are neither rejected nor masked, in the code that enforces the trust boundary after signature verification.
- 🔵 **minor** (medium) — *The unlocked stale-state deletion in the pre-lock check is a TOCTOU neither list mentions* — Phase 6 §1. `state.js:63-66` writes the two files as separate atomic renames, so a launcher reading between them deletes a live daemon's state outside any lock. *Suggestion*: make the pre-lock check purely read-only; delete only under the lock, and add it to the corrected list.
- 🔵 **minor** (medium) — *`ps lstart` local-time parsing is ambiguous across the DST fall-back hour* — Phase 6 §1. The repeated hour maps one string to two instants, and the Rust port would have to resolve it identically to V8's parser. A daemon started in that window is judged stale by up to 3600s. *Suggestion*: prefer `sysctl`/`libproc`'s epoch-based `p_starttime` on both sides.
- 🔵 **minor** (high) — *The Linux start-time computation must use truncating integer division* — Phase 6 §1. `run.sh:25` and `state.js:49` both truncate; rounding consumes the whole ±1s budget the tolerance exists to provide. *Suggestion*: state it explicitly and add a fixture whose tick count does not divide evenly.
- 🔵 **minor** (high) — *Two omitted details: `about:blank` acceptance and exit 3's redefinition* — Phase 2 §1; Phase 6 §2. `validate-source.sh:198` accepts `about:blank` through a fall-through a "whole decision tree" reimplementation would plausibly turn into a rejection; and Phase 7 redefines exit 3 while Phase 6 lists it as preserved, leaving `PROTOCOL.md:555-566` describing a contract nothing implements.

### Security

**Summary**: The plan inherits a genuinely strong distribution trust model and is unusually honest about where the boundary sits — ADR-0060's admission that sealing deters accident rather than an attacker is the right posture. The weaknesses concentrate in two places: the new upstream verification (AC13) is named but not designed, so each of its three checks has a well-known bypass the plan does not close; and the repair path — the sole replacement for the self-healing being given up — cannot detect the corruption or tampering it exists to remediate. Secondarily, the release pipeline's signing/upload/re-verify surfaces are not extended to the tree archives, the extraction hardening omits several standard tar vectors, and the ported design guards carry their existing gaps intact.

**Strengths**: verify strictly before extract, with a test asserting the trees directory is empty after a failure; temp-then-single-syscall-rename with the sentinel written last; path-escape and symlink-escape rejection as explicit criteria rather than library defaults; exactly one holder of the embedded key and no new CLI verification primitive; assembly in `prepare`, never `sign`; recorded fixtures rather than live network; the secret scanner's report-the-name-never-the-value contract carried into Rust; `playwright-core` pinned exactly with a release-failing guard; ADR-0060 stating the model's limits plainly and the plan requiring that to be documented in `resolve/`.

**Findings**:

- 🔴 **critical** (high) — *`cache verify` reads none of the tree's bytes* — Phase 4 §4c. A metadata comparison invariant under file deletion, truncation or executable replacement. AC14 and the corrupted/truncated criteria cannot be satisfied, and the Phase 7 manual check would report a tree with a deleted file as healthy. *Suggestion*: write a per-file `path → sha256` list (or Merkle root) into the sentinel at materialisation and walk the sealed tree against it; or define repair as an unconditional discard-and-refetch and say so, so nobody mistakes `verify` for an integrity check.
- 🔴 **critical** (high) — *AC13's three verifications name no trust anchors* — Phase 5 §2. npm keys fetched from the registry make signature and key arrive over the same channel from the same host. `gh attestation verify` without `--owner`/`--repo` predicates accepts an attestation from any builder; no expected source repo, workflow identity or subject digest is stated. Chromium's "pinned by hash" never says where the hash lives — if derived from the CDN at assembly time it is trust-on-first-use every release. The Node version has no stated pin. *Suggestion*: commit the npm keys alongside the Node key set under one refresh procedure; state the attestation predicate and fail without a match; commit the Chromium sha256 and Node version as reviewed constants changing only alongside the `playwright-core` pin.
- 🟡 **major** (high) — *GPG verification has a well-known no-op failure mode* — Phase 5 §2. `gpg --verify` exits 0 for a signature from a key merely present in the keyring, printing only `WARNING: This key is not certified` to stderr; and `gpg` is not pinned in `mise.toml`, so its presence on the release runner is incidental. *Suggestion*: verify against a dedicated keyring with `--status-fd` parsed for `VALIDSIG` and the fingerprint checked against a committed allowlist; pin the tool and fail loudly when absent.
- 🟡 **major** (high) — *Phase 5's assembly extracts unverified upstream archives with release privileges* — Phase 5 §3. The Chromium zip's custody is TLS-only, and extraction happens inside `Prepare` steps carrying `GH_TOKEN` in a job with `contents: write` and `attestations: write`. A traversal entry could overwrite a `tasks/*.py` module the later Sign step imports, giving code execution in a step that then holds the release secret key. *Suggestion*: apply the launcher's rules CI-side, extract outside the checkout, and drop `GH_TOKEN` from that step.
- 🟡 **major** (high) — *Extraction hardening omits several standard tar vectors* — Phase 4 §4b. Hardlinks, absolute paths, symlink-then-traverse chains, device/FIFO entries, setuid/setgid/sticky bits (relevant because archive executable bits are deliberately preserved), plus no entry-count, uncompressed-size or download-size bound. *Suggestion*: enumerate accepted entry types, mask permission bits at extraction, and add an additive `size` field enforced as a download cap and an uncompressed ceiling.
- 🟡 **major** (high) — *Tree archives are never signed, uploaded or re-verified* — Phase 5 §5–6. `sign_staged_binaries` signs an explicit expected list built from `DISPATCHED_SUBBINARIES` and never scans a directory; `_release_uploads`/`_release_reverifies` iterate `manifest["binaries"]` only. This is exactly the failure `_assert_staged_manifest_is_current`'s docstring warns cannot be recalled. *Suggestion*: extend all three in the same fail-closed style and add the artifact arm to the upload-count pin.
- 🟡 **major** (high) — *The warm-hit fork has no good branch as written* — Phase 4 §4b step 1. Manifest comparison costs a network round trip and loses offline resolution; no comparison makes the sentinel self-attesting and forgeable by anything that can write the cache root. Nothing checks ownership or mode of the tree root, and `ACCELERATOR_CACHE_DIR` lets the cache point at a shared location where ADR-0060's same-uid assumption fails. *Suggestion*: resolve the expected digest offline from the directory name, and add a `stat` gate refusing entries not owned by the current uid or that are group/world-writable.
- 🟡 **major** (medium) — *`cache repair [<name>]` is a deletion primitive over an unvalidated token* — Phase 4 §4c. `repair ../../something` reaches outside `trees/` on a recursive-delete path; and since rename over a non-empty directory is `ENOTEMPTY`, repair must chmod a sealed tree writable while its sentinel still marks it trusted. *Suggestion*: validate against the manifest's `artifacts` keys (default-deny) and remove the sentinel *first*, then rename aside, then delete — revoking trust before the tree becomes writable.
- 🟡 **major** (high) — *The ported `validate-source` SSRF guards carry their gaps forward* — Phase 2 §1. `::ffff:169.254.169.254`, `::ffff:10.0.0.1`, `fc00::/7`, `100.64.0.0/10`, `0.0.0.0/8` beyond the exact string, and `0:0:0:0:0:0:0:1` are all unmatched; the octal rejection inspects only the first octet. *Suggestion*: classify with `std::net::IpAddr` plus explicit `fc00::/7`, CGNAT and IPv4-mapped unwrapping, rejecting numeric-looking hosts that fail strict parsing; record the pre-resolution-only scope as a taken position.
- 🟡 **major** (high) — *`resolve-auth` is ported for a consumer that does not exist* — Phase 2 §3; Phase 6 §5. `makeAuthHeaderHandler` is imported and never called, and `ACCELERATOR_BROWSER_LOCATION_ORIGIN` is set nowhere. Users place real bearer tokens in a browser-driving daemon's environment for a feature that never applies them, and an authenticated crawl silently produces an unauthenticated inventory. *Suggestion*: wire it up with an origin-allowlist test, or retire the command and its variables together.
- 🔵 **minor** (medium) — *The daemon has no request authentication* — Phase 6. JSON commands on `127.0.0.1` with no auth (`daemon.js:286-338`); loopback is not a uid boundary. *Suggestion*: a random per-daemon token in the already-`0700` `server-info.json`, required as a header.
- 🔵 **minor** (medium) — *"No whole-archive buffer" is not achievable through the current key API* — Phase 4 §4a. A ~300MB allocation on first run is a memory-pressure failure in the containers AC6/AC11 target. *Suggestion*: name the streaming path and confirm `minisign -S` produces prehashed signatures under the existing `allow_legacy: false` setting; if not, say so and bound the buffered read.
- 🔵 **minor** (high) — *`scrub-secrets`' literal scan misses the bare auth token* — Phase 2 §1. The stored value is a full `Name: value` pair, so an artifact rendering just the token matches nothing. *Suggestion*: split on the first colon and scan the value component too, still reporting only the name.

### Test Coverage

**Summary**: The plan is unusually strong on test *architecture* — I/O-free domain crates, the MockServer + real-minisign precedent, recorded-fixture pipeline tests, explicit negative tests for archive escape, and the honesty to delete two vacuously-passing suites. Its weakness is coverage *accounting*: it rests on an incorrect premise about CI, several ACs have no concretely specified automated verification, and the highest-risk paths are covered by prose rather than by a named test with a named seam.

**Strengths**: most phases carry an explicit failing-test-first criterion; the domain split makes exhaustive boundary testing cheap; vacuously-passing tests are identified and removed rather than ported; the locale guard is strengthened with a fixture-pinned oracle; Phase 4's negative tests are concrete and mutation-sensitive, and `MockServer::hits()` already supports the AC9 no-refetch assertion; pipeline tests are hermetic; AC15's assertion is correctly located where a clock seam actually exists.

**Findings**:

- 🔴 **critical** (high) — *`scripts/test-design.sh` is in CI, and carries ~200 lines of surviving assertions* — Current State Analysis; Phase 8 §1. Discovered by `run_shell_suites(context, "scripts")` via exec bit and run by `mise run test:integration` at `main.yml:91`. Beyond being a runner it asserts the browser agents' `tools:` frontmatter, the evaluate-payload allowlist, the absence of `evaluate-payload-rejected`/`mcp__playwright__` from executor source, `.mcp.json` non-existence, `PROTOCOL.md`↔`daemon.js` sync, `BLOCKING_OPS`, and evals/benchmark JSON validity. Phase 8 deletes it and rescues one assertion. *Suggestion*: correct the premise, classify every assertion keep/retire, and re-home the keepers before deleting.
- 🟡 **major** (high) — *"Remove their invocations" understates the edit and leaves CI red* — Phase 2 §6; Phase 6 §8. The deleted scripts are exercised by inline assertions at `:169-274`, `:282-336`, `:350-425`, `:442-482` and `:518-528`. *Suggestion*: name the exact line ranges per phase and state, per block, whether the assertion is superseded or deliberately retired.
- 🟡 **major** (high) — *"One success and one failure path per subcommand" discards boundary coverage* — Phase 2 criteria. The deleted suites pin RFC1918 at both edges plus both just-outside cases (differentiated by stderr content), IPv6 zone-id/mapped/wildcard/bracketed forms, decimal/hex/octal encodings, the userinfo confusion class, and unknown-flag exit 2. Widen the RFC1918 bound and a two-tests suite still passes. *Suggestion*: an enumerated migration checklist mapping each assertion to a named Rust test.
- 🟡 **major** (high) — *The container fixtures carrying AC6/AC11/AC12 are nowhere specified* — Phase 7 criteria; Testing Strategy. No images, no task, no CI job, no artifact source (the release host will not carry `artifacts` until a release built after this merges), no treatment of building for the container's platform. `tasks/test/e2e.py:34-125` is the precedent. *Suggestion*: specify a `test:integration:design-containers` task modelled on it, or state they are manual-only and downgrade the criteria.
- 🟡 **major** (high) — *"Characterization tests derived from `test-run.sh`" — that oracle does not exist* — Phase 6 criteria. `test-run.sh` covers none of the six named behaviours and self-SKIPs from line 65 without a real Playwright install. No seams are specified, so "PID-recycle rejection" has no deterministic construction and "lock contention" invites sleeps. *Suggestion*: drop the framing, name the ports that make each deterministic, and state the induced scenario per test.
- 🟡 **major** (medium) — *AC2's determinism clause has no named ports or envelopes* — Phase 6 §2. No port is designed for the ephemeral port, the state-directory path or the clock, and no golden is specified for the daemon-side envelopes that carry them. *Suggestion*: name the ports and the envelopes, or narrow AC2 in the plan to the launcher-level envelopes and say so.
- 🟡 **major** (high) — *Three Step 4b properties have no automated test* — Phase 4 criteria. Sealing is manual-only and nothing asserts the executable bit survives (a `0444` Node binary breaks downstream with an errno, not a red test); the crash window is claimed but never constructed; the reaper is time-dependent with no clock seam. All three are AC7's own properties.
- 🟡 **major** (high) — *Phase 1's `FakeClock` test cannot verify what AC15 claims* — Phase 1 §4. `Clock::filename_timestamp` *is* the seam a fake replaces, so the renderer never runs. The property is already covered by an existing pure-function test pinning `"2026-07-13-090507"`. *Suggestion*: point AC15 at that test and spend the new one on the arg→variant mapping.
- 🟡 **major** (high) — *`design notices` has no tests and AC16 is manual-only* — Phase 7 §7; Phase 5 §8. The notice obligation is the stated substitute for a legal gate; an assembly refactor dropping a component ships silently. *Suggestion*: an assembly-side `NOTICES/` assertion plus success and failure paths for `notices`, including `--artifact`.
- 🟡 **major** (medium) — *AC8's producer/consumer round trip is manual and single-platform* — Phase 5 criteria. Nothing connects `collect_artifact_entries()` to the launcher's `ArtifactEntry` parser. *Suggestion*: assemble a tiny synthetic tree through the real code path, emit through the real `build_manifest`, sign with a test key, and drive the launcher's resolver over it.
- 🟡 **major** (medium) — *Two of the three upstream verifications cannot be fixture-tested as written* — Phase 5 criteria. Mocking `gh attestation verify` and GPG asserts that a subprocess ran, not that its verdict is honoured. *Suggestion*: test Node/GPG for real offline against a committed key and a recorded signed `SHASUMS256.txt` including the tampered-signature negative; inject the runner for SLSA and assert both branches, stating that the attestation's content is not verified.
- 🔵 **minor** (high) — *The new node-suite task gains no discovery floor while the suite set churns* — Phase 6 §6; Phase 8 §1. Two suites deleted in Phase 6 and one in Phase 7; the repo's established answer is a count floor. Without one, a renamed suite removes CI meaning without failing anything.
- 🔵 **minor** (high) — *Decrementing to 14 against 15 actual suites bakes in a blind spot* — Phase 8 §1. `_require_suite_floor` is an at-least floor whose documented job is to fail when an exec bit is dropped; headroom is the blind spot it exists to close.
- 🔵 **minor** (high) — *"Measure and record" entries cannot fail* — Phase 4/5/7 criteria. The plan itself supplies a prose threshold ("a few hundred KB") and treats the ~30ms bootstrap as load-bearing, yet neither gets a regression guard. *Suggestion*: convert the two with numbers into assertions and move the informational ones out of the criteria lists.
- 🔵 **minor** (medium) — *The deleted `test-notify-downgrade.sh` enforced exhaustiveness the plan drops* — Phase 2 §6; Phase 7 §6. It loops every message key for golden equality and asserts set-equality with the fixture directory. *Suggestion*: iterate the reason enum in the test so a new variant without a golden is a failure.
- 🔵 **suggestion** (medium) — *`resolve_optional` is copied verbatim with no mention of its tests* — Phase 7 §5. The precedence and whitespace-collapse edges are AC12's mechanism. *Suggestion*: extract with its tests, or add explicit precedence tests at the new site.
- 🔵 **suggestion** (medium) — *Phases 3, 7 and 8 lack the failing-test-first criterion* — Phase 3/7/8 criteria. Phase 7 is the largest behavioural change and its criteria are dominated by container fixtures. *Suggestion*: add the criterion and name the unit tests pinning the failure-ordering state machine.

### Compatibility

**Summary**: The plan is unusually strong on the contract that carries the most risk — the `manifest.json` extension — and its additive reasoning checks out against the actual parser, including a version-pinned base URL plus exact version equality that closes the mixed-version window entirely. The weaker seams are the *new* contracts it introduces without specifying them: the artifact asset-name convention agreed across separate PRs, the launcher→sub-binary environment protocol, and the runtime musl/glibc discrimination AC11 depends on, which cannot be derived from `HOST_PLATFORM` or the manifest's platform axis.

**Strengths**: the additive claim is verified against the real parser; the mixed-version window is genuinely small and the plan's reasoning survives it; the tree layout is designed against `cache::find`'s real quirks; the exit-code split is safe for the shipped consumers; `design.browser_path` via `EXTRA_KEYS` is correctly costed against the count assertion and the drift test's actual extraction; verify-before-extract keeps the trust boundary where the single-file path has it; the flat naming is reasoned from concrete upstream behaviour.

**Findings**:

- 🔴 **critical** (high) — *AC11's musl downgrade has no detection mechanism* — Phase 7 §4. `HOST_PLATFORM` is a compile-time constant evaluating to `linux-x64` on Alpine and Debian alike, `TARGETS` builds musl precisely so one binary runs everywhere, the manifest has no libc dimension, and no runtime probe exists anywhere in `cli/` or `scripts/`. An Alpine user downloads ~294MB of glibc artifacts and fails at `execve` with a bare ENOENT. *Suggestion*: probe the dynamic loader (or the driver Node binary's interpreter) before any resolution, unit-tested over an injected filesystem port for both shapes.
- 🟡 **major** (high) — *The artifact asset-name/URL convention is never stated* — Phase 4 §4a–4b; Phase 5 §3,§5. The single-file equivalent is pinned in one commented place; here the `.tar.gz` suffix and key naming are new degrees of freedom, and Phase 4's consumer is tested only against a synthetic tarball. *Suggestion*: state it once and pin it in `manifest.example.json`, asserted from both `manifest.rs`'s golden test and `test_manifest_contract.py`.
- 🟡 **major** (medium) — *The launcher↔sub-binary environment contract has no name, shape or absent-case behaviour* — Phase 7 §3. `ExecBinary::exec` carries no environment; dispatch is token-only, so every design call would resolve trees unless the launcher learns the subcommand grammar; and `ACCELERATOR_DESIGN_BIN` bypasses resolution so the dev override cannot exercise the Playwright path. *Suggestion*: name the variables, state that absence maps to `artifact-unavailable`, and gate resolution on `executor`.
- 🟡 **major** (high) — *The vocabulary replacement misses three consumers* — Phase 7 §6. `evals/evals.json` (eval 20 asserts the literal `bootstrap-failed` message), `evals/benchmark.json` (six occurrences), and `PROTOCOL.md:557-563`'s exit-code table. *Suggestion*: add them to the file list and retarget eval 20 onto `artifact-unavailable`.
- 🟡 **major** (medium) — *Retiring `browser-executor` removes the only path-resolution mechanism the two agents have* — Phase 6 §7. No agent references `${CLAUDE_PLUGIN_ROOT}` today; the pattern is preload-a-skill-that-injects, and the preload guard with its user-facing message goes away. `releases-and-compatibility.md:41-44` also cites the skill as one of two mechanisms justifying the v2.1.144 minimum, uncovered by Phase 8's sweep. *Suggestion*: confirm `${CLAUDE_PLUGIN_ROOT}` expansion inside a subagent Bash call first, or keep a minimal preload skill; add the two docs pages to the file list.
- 🟡 **major** (medium) — *The Darwin start-time port introduces a TZ/DST axis the guard does not cover* — Phase 6 §1,§4. Local-wall-clock→epoch needs the offset at that instant; `time` is featured `["parsing"]` only and `current_local_offset()` returns `IndeterminateOffset` in a multi-threaded Unix process. A wrong offset shifts by whole hours against a ±1s tolerance. *Suggestion*: prefer `sysctl KERN_PROC_PID`'s `p_starttime`; extend the guard across `TZ` ∈ {unset, UTC, half-hour offset, DST-observing} and pin the `CLK_TCK` source in a static musl binary.
- 🟡 **major** (medium) — *`flate2`'s backend must be pinned to `rust_backend`* — Phase 4 §4b §1. Non-default backends pull `libz-sys`/`zlib-ng-sys` requiring a C toolchain, breaking the musl-static build; Cargo feature unification means a future crate could pull one in silently, and `test_launcher_feature_graph.py` has no compression entry. *Suggestion*: pin explicitly with the musl rationale and add the sys crates to `_ABSENT`.
- 🟡 **major** (medium) — *The version-in-path layout is what prevents the cross-version sharing the plan claims needs no redesign* — Performance Considerations; What We're NOT Doing. And `ACCELERATOR_CACHE_DIR` moves the cache outside the only thing that prunes it, while the new namespace ships no `prune`. *Suggestion*: key on the digest alone and record the version in the sentinel, or add `cache prune` while the namespace is being designed.
- 🔵 **minor** (medium) — *An absent `artifacts` key, an absent platform entry and the all-zeros sentinel have no stated common outcome* — Phase 4 §4a. The reused `bare_sha256` path returns `AssetNotFound` with a detail string saying "no binary for this version" — misleading, and a hard error rather than a downgrade. *Suggestion*: map all three to one artifact-unavailable outcome, add the negative test, and fix the message.
- 🔵 **minor** (medium) — *The exit-code split adds a signal the only consumer ignores* — Phase 2 §3. `analyse-design-gaps/SKILL.md:125-132` sends a usage error into a three-round revise loop against a file it cannot read. *Suggestion*: abort on 2, retry only on 1 — a two-line edit that makes the split worth having.
- 🔵 **suggestion** (medium) — *`tree.rs` breaks `cache.rs`'s `#[cfg(unix)]` convention* — Phase 4 §4b §2. Windows is correctly out of scope, but the neighbouring module keeps non-Unix arms so the launcher still type-checks. *Suggestion*: follow it, or state Unix-only by design in the module docs.

### Performance

**Summary**: The plan inherits good measurement work from ADR-0060 and makes the right structural calls — stream instead of buffer, verify before extract, ship the 177MB shell rather than 297MB Chromium. But the warm path it claims to protect is specified two contradictory ways, and the latter puts two network round-trips on every one of the 100–200 invocations a crawl makes. The cold path is under-specified in the other direction: a ~294MB payload with no throughput floor, no resume, no materialisation lock, an unnamed timeout, and a version-keyed cache name forcing a full refetch on every pre-release. Most performance criteria record observations rather than gate, which is a regression against work-item:0186's own ratio gate and method.

**Strengths**: the exemption is driven by real measurement expressed both absolutely and as a fraction of budget; replacing the double-buffering `bytes().to_vec()` with a streaming sink is the correct I/O fix and costs small-asset callers nothing; verify-before-extract makes a failed check the cheapest possible failure; the headless-shell choice cuts cost by more than an order of magnitude on the file-count axis with a stated fallback; trees are deliberately not routed through `ResolveBinary::resolve`; the plan has a Performance Considerations section naming both load-bearing budgets and connecting them to prior work.

**Findings**:

- 🔴 **critical** (high) — *The warm tree lookup as specified requires a manifest fetch* — Phase 4 §4b step 1 (and Performance Considerations). `load_manifest` is two HTTPS GETs plus verification and is today called only on a cold miss — precisely why the single-file warm path resolves offline. 100–200 invocations per crawl means 200–400 requests, tens of seconds, failure when offline, and every invocation far over the ~30ms target. *Suggestion*: resolve offline as `cache::find` does, restate step 1 to say no manifest is loaded on a hit, and assert **zero** HTTP requests on a warm resolution against the `MockServer`.
- 🟡 **major** (high) — *Version-keyed naming forces a full ~294MB refetch on every pre-release* — Phase 4 §4b; Performance Considerations. `<version>` is the launcher's `CARGO_PKG_VERSION`, currently at `1.24.0-pre.36` — 36 keys within one minor for byte-identical artifacts. `ACCELERATOR_CACHE_DIR` does not help because the *name* embeds the version, so a persisted directory accumulates duplicates. On the order of 10GB of redundant transfer and disk per release cycle. *Suggestion*: match on digest with version as a secondary attribute, or state the deferral in "What We're NOT Doing" with the volume named.
- 🟡 **major** (high) — *No chosen deadline, throughput floor or resume for the large-payload fetch* — Phase 4 §4a. Retries re-download from byte zero; blocking reqwest has no idle timeout so the deadline is the only bound; the 294MB figure is the uncompressed tree while the deadline governs the compressed archive, a number Phase 5 only records. The one-hour reap TTL is not derived from worst-case materialisation time. A 2Mbps user waits three attempts, transfers ~750MB and fails. *Suggestion*: state the deadline as a throughput floor over the *measured compressed* size in the constant's doc comment, make Phase 5's measurement a named input, and either add `Range` resume or reduce `MAX_ATTEMPTS` for trees.
- 🟡 **major** (medium) — *Concurrent cold materialisation is a 294MB thundering herd* — Phase 4 §4b step 5; criteria. Each racer independently streams, hashes, verifies, extracts and chmods ~294MB, and one deletes it all. `bin/accelerator:317-345`'s lock guards only the launcher binary fetch. *Suggestion*: serialise per tree key with a PID-liveness-gated lock directory, and change the criterion to "exactly one archive fetch occurs".
- 🟡 **major** (medium) — *`cache verify` has no cheap on-disk signal, so `repair` degenerates to an unconditional full refetch* — Phase 4 §4c. Re-taring ~294MB would not reproduce byte-identically anyway, so `verify` compares strings; yet AC14 needs real detection. Either `verify` is vacuous or `repair` always costs ~294MB — more expensive than the self-healing it replaces. *Suggestion*: write per-file sizes and digests into the sentinel (14 entries for the shell, ~490 for the driver — tens of KB), making `verify` a stat-and-compare pass that escalates to hashing only on a size mismatch.
- 🟡 **major** (high) — *The performance criteria record rather than gate* — Phase 4/5/7 Manual Verification. "In the same order as"; "measured and recorded"; "acceptable, and is recorded". 0186 used a host-relative ratio gate, 50 interleaved samples in one process with alternating order, two instrument floors, and recorded host/OS/inode — and explicitly noted that a naive bash-loop method is not comparable. A 2× regression passes as written. *Suggestion*: `after ≤ 1.1 × before` with 0186's method; a numeric KB ceiling; a stated minimum throughput and wall-clock ceiling.
- 🟡 **major** (medium) — *The "few hundred KB" gate is unactionable and targets the wrong axis* — Phase 4 §4b §1. At 0186's measured ~0.3ms/MB, a few hundred KB is ~0.1ms — likely below the noise floor, so the plan over-weights this term; meanwhile the backend's dominant consequence is that `miniz_oxide` inflates materially slower than a zlib-ng build, and the cold path inflates ~294MB. *Suggestion*: express the budget in milliseconds derived from the measured rate, convert to a numeric KB ceiling, and evaluate the backend on decompression throughput too.
- 🔵 **minor** (medium) — *The "no whole-archive buffer" claim may not survive minisign verification* — Phase 4 §4a; §4b step 3. `verifies` needs the whole message; incremental verification requires prehashed signatures and `tasks/signing.py:24-43` signs with plain `minisign -S`. *Suggestion*: confirm the form before Phase 4 starts; if not prehashed, add `-H` for tree artifacts (checking the vendored shim and `minisign-verify = 0.2.5` accept it) or drop the claim and state the peak memory, with mmap as the cheaper option.
- 🔵 **minor** (medium) — *Tree resolution is not scoped to the executor subcommand* — Phase 7 §3. Five of seven subcommands are pure local computation, and `validate-source` is called per-location from SKILL.md. On a cold cache, validating a URL triggers a ~294MB download. *Suggestion*: resolve only for `executor` (and `notices`), with a criterion asserting no fetch for the others on an empty cache.
- 🔵 **minor** (medium) — *A second `Fetcher` on the warm path, and an unaddressed doubling of the release assembly* — Phase 4 §4a; Phase 5 §7. Each `Fetcher` builds a `reqwest::blocking::Client` (rustls provider install plus a background runtime thread), and `FetchVerifyCacheResolver::new` already builds one on every invocation including warm hits. The pre.0 pass assembles byte-identical artifacts from the same pinned inputs. *Suggestion*: `RequestBuilder::timeout()` on one lazily-constructed client; skip any archive already present with a matching digest.

### Safety

**Summary**: The plan is unusually careful about the *materialisation* half of tree artifacts and inherits a release pipeline that already fails closed in several places. The safety weight, however, has moved to the halves the plan treats briefly: the repair path cannot detect the corruption it is meant to fix, and sealing directories `0555` makes a tree undeletable — which breaks repair, breaks the pruning story the whole no-eviction design rests on, and strands hundreds of megabytes per plugin version. On the release side, the new ~1.2GB artifact set is threaded through the manifest but not through the upload/re-verify/sign paths that make publication recoverable, and the existing destructive `--cleanup-tag` fallback becomes much more reachable at that payload size.

**Strengths**: verification before extraction with an emptiness assertion after a rejected archive; sentinel written last so a crash leaves an untrusted tree; the concurrent-winner branch and the two-resolution criterion address the partial-tree exposure that matters most given the exemption; path-escape and symlink rejection as explicit criteria; a `timeout-minutes` and pre-assembly disk assertion added to a job with neither; every upstream verification framed as failing the release rather than the user's run; skills rewired inside the phase that deletes what they call; ADR-0060's documented difference required to actually be documented.

**Findings**:

- 🔴 **critical** (high) — *`verify`/`repair` cannot detect tree corruption, so AC14's recovery path does not exist* — Phase 4 §4c. The archive is gone after extraction, a `.tar.gz` digest is not recomputable, and no per-file digests are recorded — the check reduces to comparing two strings that agree by construction. ADR-0060 removes self-healing on the promise repair restores it; as specified the replacement is a no-op against exactly the failure modes it exists for. *Suggestion*: record a content digest so `verify` detects drift, or make `repair <name>` unconditional and state the limitation in help text.
- 🔴 **critical** (high) — *Sealing directories `0555` makes the tree undeletable* — Phase 4 §4b step 6. `rm -rf`, `remove_dir_all` and `fs.rm({recursive:true})` all fail with `EACCES`. Repair cannot discard; the "pruned when Claude Code prunes" rationale depends on an external recursive delete that now fails, stranding ~294MB per plugin version and potentially failing the plugin update; and a user told the cache is safe to delete hits permission denied on a directory they own. *Suggestion*: seal files, leave directories `0755` — that achieves the stated deterrence — or chmod before removal and verify by experiment that plugin pruning can remove a sealed tree.
- 🔴 **critical** (high) — *Tree artifacts are in the manifest but not in signing, upload or re-verification* — Phase 5 §5–6. `sign_staged_binaries`, `_release_uploads` and `_release_reverifies` all derive from `DISPATCHED_SUBBINARIES`; the archives and sidecars fall outside all three, while `--draft=false` flips once those lists re-verify. Precisely the failure `_assert_staged_manifest_is_current`'s docstring says cannot be recalled. *Suggestion*: explicit artifact arms driven by one shared registry, with a unit test pinning the five sets against each other.
- 🟡 **major** (high) — *A persistent materialisation failure becomes a full refetch on every invocation* — Phase 7 §3–4. No negative caching, backoff or per-session memory across 100–200 invocations; a single crawl on a failing machine can attempt tens of gigabytes and repeatedly fill the disk with partial archives. This did not exist for megabyte-scale single files. *Suggestion*: a short-lived failure marker suppressing re-attempts for the session, and a sticky first downgrade so the rest of the crawl takes the code-only path immediately.
- 🟡 **major** (high) — *The one-hour reaper races a slow extraction and can publish an incomplete tree* — Phase 4 §4b. Age alone is the liveness signal and mtime is set at creation, while the same phase raises the deadline past 300s. Deletion in the window between extraction completing and the rename publishes an incomplete tree that is then sealed and sentinelled as verified — and nothing will ever re-check it. The sweep also misses partial temp *archives*, the largest residue. *Suggestion*: reap on PID liveness (the temp name already carries the pid), mirroring the repo's existing reclaim discipline, and extend it to archives.
- 🟡 **major** (medium) — *The mid-use repair inode argument does not transfer to a lazily-loaded tree* — Phase 4 §4c. Chromium and Node open locale packs, `.pak` resources, `icudtl.dat` and later modules over the process lifetime; and `ENOTEMPTY` means repair must delete before renaming, unlinking exactly what a live daemon has not yet opened. Repair's likely moment is *during* a misbehaving crawl, and the sentinel/tree removal order is unspecified. *Suggestion*: remove the sentinel first, materialise into a new digest-named path, remove the old tree last — and state the true guarantee.
- 🟡 **major** (medium) — *Warm tree resolution appears to require a manifest fetch, breaking offline operation* — Phase 4 §4b step 1. Each invocation is a fresh launcher process, so the hit condition means two HTTPS round trips 100–200 times per crawl, and a fully-populated cache stops working offline — directly contrary to ADR-0060's "microseconds" basis. *Suggestion*: prefix-scan and trust the digest in the directory name, deferring manifest comparison to `verify`/`repair` and the miss path.
- 🟡 **major** (high) — *Every release now depends on three third-party hosts and a GPG key set* — Phase 5 §2–3. Wired into both prepare paths, fetching on every cut, against a key set the plan itself notes "a stale set fails releases" — yet all inputs are pinned so the bytes are identical release after release. An outage, rotation or yank makes the pipeline unreleasable, including for an urgent unrelated fix. *Suggestion*: cache assembled trees keyed on the three pins so an unchanged pin re-signs known-good bytes.
- 🟡 **major** (high) — *No functional validation of the assembled artifact before publication* — Phase 5 §3, criteria. Every gate is provenance and shape; nothing executes what was built. A structurally-wrong tree passes every gate, reaches every user, self-heals never, and is faithfully re-fetched by `cache repair`. *Suggestion*: a host-platform smoke step unpacking and running each with `--version` plus a `NOTICES/` assertion — cheap, and the only gate distinguishing "signed" from "works".
- 🟡 **major** (medium) — *A 1.2GB upload makes the `--cleanup-tag` fallback far more reachable, and the 120s timeout is undersized* — Phase 5 §7. Any non-`AssetVerificationError` exception deletes the release and its tag, after `_publish` has already committed, tagged and pushed the bump; `download_release_asset`'s hard-coded `timeout=120` will be blown by a 177MB artifact, raising `TimeoutExpired` and landing in the delete branch. Burns a version number and leaves repo and host inconsistent under the release concurrency lock. *Suggestion*: scale the timeout with asset size, add bounded retry around `_upload_clobber`, and narrow the destructive `except`.
- 🟡 **major** (medium) — *Enlarging the deadline without a stall timeout turns a hung transfer into a long silent hang* — Phase 4 §4a. `fetcher.rs`'s own comment records that the total deadline is the only bound; a connection stalled at byte one is indistinguishable from a slow one until it expires, three times over — inside a tool call with no progress output and no cancel. *Suggestion*: the switch to `get_to_writer` makes a progress floor cheap; record both numbers in the doc comment.
- 🟡 **major** (medium) — *Dropping `disk-floor-not-met` and `cache-unwritable` removes the only pre-flight disk guard* — Phase 7 §6. Both still arise and are now likelier: ~600MB peak for archive plus extracted copy, and the cache root's unwritability is already modelled as `CacheRootUnavailable`. Today the tooling refuses up front with a named reason; afterwards it fails mid-extraction having already consumed the remaining space. *Suggestion*: retain both, check free space against the manifest's known size before fetching, and remove partial trees eagerly.
- 🔵 **minor** (medium) — *`repair [<name>]` feeds path construction for a recursive delete* — Phase 4 §4c. A name containing separators or `..`, or an empty name with an empty version, turns a typo into a recursive delete outside version control. *Suggestion*: accept only manifest `artifacts` keys, reject non-plain-ASCII tokens, and assert the canonicalised target is a direct child of `trees/`.
- 🔵 **minor** (high) — *Nothing surfaces the repair path when a tree is discovered broken* — Phase 4 §4c; Phase 7 §4. Self-healing needed no discovery; this needs the user to already know a command exists that the failure never mentions. *Suggestion*: carry a remediation string naming `accelerator cache repair <name>` in the relevant envelopes.
- 🔵 **minor** (high) — *Hundreds of megabytes are stranded on both sides of the migration* — Migration Notes; Performance Considerations. The legacy per-lockhash namespaces (each holding a full Chromium) are abandoned and their sweep dies with `ensure-playwright.sh`; and `ACCELERATOR_CACHE_DIR`, the documented escape, sits where the pruning argument does not apply. *Suggestion*: spend one of the namespace's "later verbs" on `prune`, covering both.
- 🔵 **minor** (medium) — *The lock-lifetime guarantee and the Drop-guard correction are in tension for the mkdir path* — Phase 6 §2–3. Releasing on every exit path means the lock is gone the moment the launcher returns after spawning; and Rust's `FD_CLOEXEC` default means flock inheritance must be arranged explicitly. If either is wrong, two daemons start — two browsers, split page state, an orphaned process. *Suggestion*: state the lifetime per mode and add a concurrent-start test under both `ACCELERATOR_LOCK_FORCE_MKDIR` and flock.

### Code Quality

**Summary**: The plan is unusually well-researched about the *surface* it touches, and the domain/adapters/CLI split is correctly drawn for the five simple ported scripts. Where it weakens is modelling: the proposed domain modules are a one-per-deleted-script decomposition carrying shell vocabulary, two data files are baked in with `include_str!` against the codebase's own const-plus-drift-test precedent, the three outcome classes are squeezed onto a two-variant error taxonomy in a way that inverts `Refusal`'s documented meaning, and the single subcommand with real logic is placed entirely in the binary crate with no domain module and no injection seams.

**Strengths**: the preserved/corrected split names each asymmetry alongside its dependent consumer; Phase 1 correctly pushes the byte-for-byte assertion to the adapter level; keeping trees out of `ResolveBinary::resolve` avoids polluting an interface every sub-binary depends on; Phase 3 argues five behavioural differences rather than absorbing them; the registration surface is enumerated at the level of individual mutable constants; the domain crate is specified as genuinely I/O-free with a pup rule copied from precedent including the grouped-import caveat.

**Findings**:

- 🟡 **major** (high) — *The executor gets no domain module and no injection seams* — Phase 6. Every other subcommand gets one; the executor's logic — the most complex in the plan — goes to `design-cli` plus `design-adapters` with no port trait named for process, clock, sleep or filesystem. The phase's own criteria demand characterization tests for six behaviours none of which can be written deterministically without seams; the timeout test either takes 30 real seconds or does not exist. The plan already demonstrated awareness of this failure mode in Phase 1. *Suggestion*: put the pure decisions in `cli/design/` behind `ProcessProbe`/`Clock`/`Spawner` ports, mirroring `corpus`/`corpus-adapters`.
- 🟡 **major** (high) — *`include_str!` as the production data source splits two vocabularies across crates* — Phase 2 §3. A Rust enum and a JSON key set coupled only by runtime string equality (a new variant compiles and fails at runtime, and exhaustiveness is unprovable); and the cue-phrase file's own header calls it "canonical … for extract-work-items and audit-cue-phrases.sh", a claim that becomes unenforced after Phase 8. *Suggestion*: follow `cli/corpus/src/frontmatter_validation/schema.rs:277` — canonical data as a `const` in the domain crate, `include_str!` only inside a `#[cfg(test)]` drift test.
- 🟡 **major** (high) — *Three outcome classes on a two-variant taxonomy, inverting `Refusal`* — Phase 2 §3. Every sub-binary maps `Refusal → 2` and everything else → 1, with `Refusal` documented as caller-actionable. The plan makes a *usage* error the `Refusal` and a *domain rejection* — the most caller-actionable outcome — a `Failed` sharing exit 1 with internal failures, so an operator cannot distinguish "refused your input" from "the tool broke". The stated benefit is thin: `SKILL.md:63,86` only checks non-zero. *Suggestion*: model rejection as a domain verdict rendered by the command layer, reserve `Refusal` for its documented meaning, and state the three-class mapping in one table.
- 🟡 **major** (high) — *The downgrade reason is decided in one process from evidence that exists only in another* — Phase 7 §3–4. The executor can only observe an env variable's presence, so network failure, `SignatureMismatch`, an all-zeros sentinel and a corrupt tree collapse into one reason — the 3am diagnosis question has no answer. The plan also does not say what replaces the `ACCELERATOR_DOWNGRADE_REASON=` stderr protocol `SKILL.md:127` greps for, nor whether tree failures are `Refusal` or `Failed`, which silently decides `--fail-safe` behaviour via `swallow_under_fail_safe` (`launch/core.rs:218-224`). *Suggestion*: define the contract, add tree-specific `ResolutionError` variants, and state their `Refusal`/`Failed` mapping.
- 🟡 **major** (medium) — *`cache verify`'s contract cannot detect what it exists to detect* — Phase 4 §4c. An extracted tree cannot be rehashed into a byte-identical `.tar.gz`, so `verify` compares two strings. A verb named `verify` that reports "sealed and matching" for a mutated tree is worse than no verb — an abstraction whose name promises more than it can deliver, and AC14's whole recovery story rests on it.
- 🟡 **major** (medium) — *The domain modules mirror the deleted scripts rather than the domain* — Phase 2 §1. Five modules map one-to-one onto five scripts; several are named for activities; `source_location.rs` bundles scheme classification, host canonicalisation, reachability classification and the flag matrix. Compare the corpus crate's domain nouns (`doc_type`, `record`, `slug`, `typed_ref`, `linkage`, `cluster`). The reusable pieces stay welded into a script-shaped decision tree. *Suggestion*: decompose into `SourceLocation`, a `Host` value type owning canonicalisation, a `HostReach` classification and an `AccessPolicy` producing a verdict.
- 🟡 **major** (high) — *Two phases delete scripts whose SKILL.md call sites are not in the phase's change list* — Phase 6 §8; Phase 7 §8. Phase 6 deletes `run.sh` without listing `inventory-design/SKILL.md`, which invokes it at `:139`; Phase 7 deletes `ensure-playwright.sh` listing no SKILL.md at all, though Steps 4–6 (`:117-133`) invoke it and parse its stderr protocol. No phase drops the residual `Bash(...scripts/*)` rules. *Suggestion*: add both files with the new Step 4/5 shape stated, and name the phase that removes the residual rules.
- 🔵 **minor** (high) — *Porting runtime sanitisation of data the binary compiles in* — Phase 2 §1. Once the reason is a clap enum and the messages are compiled in, the bidi and printable-ASCII filters can only ever see the binary's own constants — dead defensive logic whose threat model the next maintainer must reconstruct. *Suggestion*: keep the invariant as a test over the constant table; drop the per-invocation filter.
- 🔵 **minor** (medium) — *A shell-availability workaround becomes two lock protocols with different lifetimes* — Phase 6 §1. The dichotomy exists because `flock(1)` is absent on macOS, a constraint that vanishes in Rust; ADR-0058 already records that nothing external depends on the mkdir form. And the FD-inheritance invariant is stated only as prose — exactly what a later refactor silently breaks. *Suggestion*: collapse to one implementation unless there is an NFS reason, and express the invariant in a named guard type.
- 🔵 **minor** (medium) — *Dead-code observations sit under "Changes Required" with no change stated* — Phase 6 §5. Three findings with no disposition, under a heading meant to specify changes. The first is not cosmetic: `SKILL.md:196` documents the uncalled handler's origin allowlist as security-critical, and `auth_mode.rs` is being created for a consumer that may be dead. *Suggestion*: state a disposition for each, and confirm `header` mode has a live consumer before porting it.
- 🔵 **minor** (high) — *"Measure and record" criteria with no threshold, decision rule or home for the record* — Phase 4/5/7. Each is a decision deferred to whoever runs the step, made once and unrecoverable — six months later there is no way to tell whether a 400KB launcher growth was accepted deliberately or never looked at, which is the failure mode the plan avoids elsewhere with its "corrected deliberately" sections. *Suggestion*: set a numeric budget for the launcher size delta and make it an assertion; give the rest a threshold or demote them to notes, naming the file the figures live in.
- 🔵 **minor** (medium) — *Orphan reaping bolted onto the lookup function, with a hardcoded age and no clock seam* — Phase 4 §4b §2. `find`'s single-file counterpart is `#[must_use]` and read-only; and `tree.rs` accumulates seven responsibilities before it is written. *Suggestion*: extract `reap_orphans(root, cutoff)` taking the instant as a parameter, and split `tree.rs` along its natural seams since `cache repair` needs several independently.
- 🔵 **minor** (high) — *A retained message is pinned byte-for-byte while naming a script deleted an earlier phase* — Phase 2 manual verification; Phase 7 §6. `executor-ping-failed` tells the user to run `run.sh ping`; Phase 6 deletes it; Phase 7 rewrites the messages. Between them the plugin ships a diagnostic whose remediation cannot be followed, and Phase 2's pin prevents fixing it early. *Suggestion*: rewrite the remediation text in Phase 6 and relax the pin to messages that genuinely survive.

---

## Re-Review (Pass 2) — 2026-08-11

**Verdict:** REVISE

All eight lenses re-ran fresh against the revised plan. Seven of pass 1's nine
criticals are cleanly resolved and confirmed as strengths by multiple lenses; two
are only partially resolved. But the revision introduced **seven new criticals**,
six of them in mechanisms added or rewritten during the fix pass. The plan is
materially better — the trust model, the publish path, the sealing model and the
CI premise are all now sound — but the tree recovery story regressed while being
repaired, and two claims about external APIs turn out to be false.

### Previously Identified Issues

- 🔴 **Architecture/Correctness/Security/Performance/Safety/Code Quality**: tree
  integrity story does not close — **Partially resolved.** The per-file
  `(path, mode, size, sha256)` table is the right instrument, but the stated
  algorithm and the repair path both defeat it (see N1, N2).
- 🔴 **Architecture/Correctness/Safety**: `0555` sealing makes trees undeletable —
  **Resolved.** Files read-only, directories owner-writable, with an explicit
  removability criterion. Named as a strength by three lenses.
- 🔴 **Performance/Architecture/Correctness/Security/Safety**: warm path requires a
  manifest fetch — **Resolved.** Pointer plus sentinel, zero-HTTP asserted against
  the MockServer. Two follow-on issues remain (N7, N8).
- 🔴 **Architecture/Correctness/Compatibility/Code Quality**: launcher→design
  handoff unspecified — **Partially resolved.** The contract, failure semantics and
  trigger are now stated, but the export is not actually token-agnostic and the
  reverse `cache ensure` call has no discovery mechanism (N9).
- 🔴 **Safety/Security**: tree artifacts never signed, uploaded or re-verified —
  **Resolved.** Four mandatory arms driven from `TREE_ARTIFACTS`. The privilege
  split it introduced does not exist as wired (N10).
- 🔴 **Security**: upstream trust anchors unpinned — **Resolved.** Committed npm and
  Node keys, pinned SLSA predicate, committed Chromium hashes, `--status-fd`
  parsing. One gap remains in the GPG status handling (N11).
- 🔴 **Compatibility**: AC11's musl downgrade has no detection mechanism —
  **Resolved.** Loader-path probe, classified in the domain, unit-tested over an
  injected listing, running before any resolution.
- 🔴 **Test Coverage**: `test-design.sh` CI premise factually wrong — **Resolved.**
  Premise corrected with the mechanism recorded; the block table was verified
  line-complete against the real 553-line file. Three rows are bucketed wrongly
  (N12).
- 🔴 **Correctness**: ENOTEMPTY branch adopts crash leftovers — **Resolved for the
  crash case**, but the same predicate now mis-handles the repair case (N2).

Pass 1's majors are broadly resolved: the lock-lifetime contradiction, the
empty-`start_time` hole, the Darwin TZ/DST hazard, `flate2`'s backend, the
`include_str!` split, the domain modelling, the exit-code taxonomy, the phase
dependency graph, and the unfalsifiable performance criteria are all confirmed
addressed. The `state.js` fail-loudly reversal and the Phase 1 `FakeClock` removal
were both endorsed on the reasoning given.

### New Issues Introduced

- 🔴 **N1 — Correctness/Safety/Performance**: `cache verify`'s "stat per entry,
  escalating to a hash only where size or mode disagree" cannot detect the same-size
  substitution its own success criterion asserts. The escalation predicate never
  fires for that case, so the per-file digest table is never consulted on the only
  path that reads it. ADR-0060 measures a full hash of the whole set at ~120ms on a
  user-invoked command, so the optimisation buys almost nothing.
- 🔴 **N2 — Correctness/Safety**: `cache repair` cannot make progress. Content
  addressing means a refetch produces the *same* digest and therefore the *same*
  path, so "extracts a fresh directory alongside the old one" is impossible; and
  Step 4b step 6 adopts an already-present target whenever it carries a valid
  sentinel — which a post-materialisation corruption still does. Repair downloads
  ~294MB, hits ENOTEMPTY, adopts the corrupt tree and reports success. AC14 is
  unsatisfiable.
- 🔴 **N3 — Correctness**: `chromium.executablePath()` does not report a
  `launch({executablePath})` argument — it is computed from the browser registry and
  takes no argument. Phase 7 §2's claim that the ping diagnostic is "truthful under
  the hatch" is false, and `daemon.js:123`'s `access(execPath)` would throw on every
  invocation, returning `chromium-not-found` and degrading every crawl to code-only.
  AC6 and AC12 both fail.
- 🔴 **N4 — Test Coverage**: the container fixtures cannot verify self-built
  artifacts. `cli/launcher/build.rs:32` copies `keys/accelerator-release.pub` into
  `OUT_DIR` unconditionally and `keys.rs:12` `include_str!`s it — no env override, no
  feature gate — so a compiled launcher only accepts artifacts signed with the real
  release secret. `ACCELERATOR_RELEASE_BASE_URL` solves *where*, not *who signed it*.
  Confirmed directly against the source.
- 🔴 **N5 — Test Coverage**: three retained Node suites self-skip permanently after
  Phase 7. `test-run.js:94` gates all sixteen tests on `existsSync(cacheRoot)` and
  `daemon.test.js:18-22,72` derives its namespace from `package-lock.json` — both
  deleted in Phase 7 §8. These are the suites asserting the envelope shapes AC2
  pins. A file-count floor counts files, not executed tests, so the task still exits
  0. Confirmed directly against the source.
- 🔴 **N6 — Security/Safety/Architecture/Code Quality**: the pin-triple assembly
  cache bypasses the whole verification chain with no specified store, key or
  post-restore integrity check. On an unchanged pin triple — by design, almost every
  release — cached bytes are signed with the release key and published without npm,
  SLSA, GPG or Chromium-hash re-running. If the store is the Actions cache it is
  writable from other workflows and evictable.
- 🟡 **N7 — Performance**: the sentinel's ~490-row digest table is parsed on every
  dispatch (~0.3–0.5ms per tree), and Phase 7 §3 exports trees token-agnostically —
  so `accelerator vcs guard`, a PreToolUse hook, pays it too. The plan polices
  launcher size to a 1ms budget while adding a comparable file-count-proportional
  cost to the same path.
- 🟡 **N8 — Correctness/Safety**: the warm hit never stats the tree directory. A
  removed tree with an intact pointer and sentinel resolves to a non-existent path,
  which is exported and fails deep inside Node rather than taking the miss path.
- 🟡 **N9 — Architecture/Compatibility**: the tree export is `DESIGN`-prefixed and
  injected into every sub-binary's environment, so the token-agnostic claim is not
  delivered; and the reverse `accelerator cache ensure` call has no stated discovery
  mechanism, envelope, or behaviour against a pre-Phase-4 launcher.
- 🟡 **N10 — Security**: the verify/assemble privilege split does not exist as
  wired — both invoke tasks run inside one `mise run release:prepare` step with
  `GH_TOKEN` in its environment, so the criterion asserting the assemble step has no
  token has no step to assert against.
- 🟡 **N11 — Security**: `VALIDSIG` is emitted for expired and revoked keys;
  distinguishing them needs `GOODSIG` plus explicit `EXPKEYSIG`/`REVKEYSIG`
  rejection. A revoked Node release key would still produce a green release.
- 🟡 **N12 — Test Coverage**: three `test-design.sh` blocks are bucketed wrongly —
  `:426-427` (a skill-structure assertion inside the audit block) would be silently
  deleted, `:359-364` asserts the `audit-cue-phrases.sh` call site Phase 2 rewrites,
  and `:154-155` asserts the `scripts/*` glob Phase 7 drops. Phases 2 and 7 each
  leave CI red on merge.
- 🟡 **N13 — Correctness/Compatibility/Code Quality**: the downgrade vocabulary is
  stated two incompatible ways — the decisions list drops `disk-floor-not-met` and
  `cache-unwritable` while Phase 7 §6 retains them and builds a pre-flight check on
  them. Phase 6 §3 also still carries the `design executor daemon` bullet that §1
  explicitly refutes, and miscounts its own list.
- 🟡 **N14 — Correctness/Security/Compatibility/Performance/Safety**: the single
  manifest `size` field is given three incompatible meanings — download cap
  (compressed), decompression ceiling (uncompressed), and free-space precheck — and
  `PlatformEntry` is declared both "reused unchanged" and extended. No producer arm
  emits it, and `manifest.schema.json` is never updated for `artifacts` at all.
- 🟡 **N15 — Security**: no ownership or mode check on the cache root, while the plan
  actively steers users to relocate it via `ACCELERATOR_CACHE_DIR`. On a shared root
  another local user can plant a tree, sentinel and pointer that the launcher exports
  into `chromium.launch({executablePath})`. Tree names also omit the platform axis, so
  a shared root mixes platforms.

### Assessment

The plan needs another pass, but a narrower one than pass 1. The architecture is
now sound in the places that matter — the trust model, the publish path, the phase
sequencing, the domain modelling and the CI premise all survived adversarial
re-reading, and several lenses independently called out the same design decisions
as strengths.

What the pass exposed is that fixing nine criticals at speed introduced seven more,
and the pattern is instructive: five of the seven are in mechanisms *added* during
the fix pass (the verify algorithm, the repair path, the assembly cache, the tree
export, the `size` field), and two are false claims about external APIs
(`executablePath()`, and the compile-time trust root) that would have been caught by
reading the source rather than reasoning about it. The recovery story in particular
is now worse than before it was "fixed": pass 1 said `verify` could not detect
corruption, and pass 2 says it still cannot *and* `repair` no longer works either.

The tree lifecycle — `find`, `ensure`, `verify`, `repair`, and the sentinel that
serves all four — should be redesigned as one coherent unit rather than patched
again, since every one of N1, N2, N7, N8 and N15 lives in it. The remaining items
are localised corrections.

---

## Post-Pass-2 Remediation — 2026-08-11

Every pass-2 finding above — 7 critical, ~50 major and minor across the eight lenses —
has been addressed in the plan. **The plan has not been re-reviewed since**, so this
section records what changed rather than asserting a new verdict.

The seven criticals were closed in four groups. **N1, N2, N7, N8 and N15** were all in
the tree lifecycle, so it was redesigned as one unit rather than patched: a
**generation** suffix on the tree directory makes every rename target fresh, which
deletes the collision branch entirely, lets `repair` build a complete replacement
beside a tree a live daemon is reading, and leaves the working copy intact when a
refetch fails. The sentinel split into a small hit-path attestation and a
`verify`-only file table. `verify` now hashes every entry, since the stat-and-escalate
predicate could never fire for the same-size substitution the table exists to catch.
`locate` stats the tree and validates the pointer; the cache root and every sidecar are
ownership- and mode-checked; names carry the platform; `prune` and `repair --force`
were added.

**N3** was a false claim about `chromium.executablePath()`, which is registry-derived
and does not reflect a `launch({executablePath})` argument — so `ping` would have
returned `chromium-not-found` on every invocation and degraded every crawl to
code-only. **N4** needed a feature-gated second embedded key, since `build.rs` embeds
the production key unconditionally and no test can hold the release secret; three
mechanical guards keep it out of a release. **N5** needed the node-suite runner to
assert *executed* tests rather than discovered files, because `test-run.js:94` and
`daemon.test.js:72` gate on exactly what Phase 7 deletes. **N6** was closed by making
assembly deterministic, committing `ASSEMBLED_SHA256`, and sourcing reuse from the
previous release's own minisign-verified asset rather than a mutable CI cache.

Beyond those: the manifest gained a distinct `ArtifactPlatformEntry` with three named
sizes (one field could not serve download cap, extraction ceiling and free-space
precheck); `manifest.schema.json` gained `artifacts`; the verify/assemble privilege
split became a real workflow step, with an honest accounting of what a step boundary
does *not* buy; the GPG check now requires `GOODSIG` and rejects `EXPKEYSIG`/
`REVKEYSIG`; the launcher took over start-time ownership entirely, since Node has no
`sysctl` binding and the JS half of the previous design had no mechanism; the reuse
verdict became a total function over a stated table; the tree ports split three ways;
and the `design` crate gained its third sub-domain, a `CuePhraseMatcher` port (its
regexes cannot compile under the domain's pup rule), and a stated rule separating usage
errors from domain rejections.

Two findings were **accepted rather than fixed**, with reasons recorded in the plan:
the dead header-auth path stays in place (wiring it is new feature work, deleting it
removes a documented capability) with the false security claim corrected in the
documentation this plan already rewrites; and `navigate` URL classification is a
follow-up, since `validate-source` guards only the initial location while
`daemon.js:165-167` is the real navigation surface. Full CI-job isolation for assembly
was also declined, on the grounds that the committed digest closes the
artifact-substitution path and the release job must stay single-job for version
monotonicity.

A third review pass is warranted before implementation: the tree lifecycle, the
start-time inversion, the port split and the assembly-reuse chain are all new since
pass 2 and none has been adversarially read.

---

## Re-Review (Pass 3) — 2026-08-11

**Verdict:** REVISE

All eight lenses re-ran fresh against the plan as revised after pass 2. Every pass-2
finding is confirmed addressed, and the lenses independently named the new mechanisms
as strengths — the generation-suffixed layout, the attestation/table split, the
three-way port split, the verdict table, the committed trust anchors, the GPG
predicate, the registry-driven publish path. But the pass found **eight new
criticals**, and *all eight are in material added during the pass-1 and pass-2
remediation*. That is the same pattern as pass 2, at the same rate.

### New Issues Introduced

- 🔴 **P1 — Correctness**: the libc probe classifies **macOS as unsupported**. "Neither
  loader present is itself an answer — `unsupported-platform`", with no `target_os`
  gate and only three unit-test shapes (musl/glibc/neither). macOS has neither loader,
  is in ADR-0057's matrix, and is the primary development platform — so the runtime
  crawler would be unavailable on every Mac, and AC12 could not pass on the reference
  host. NixOS and both-loaders-present hosts are also misclassified.
- 🔴 **P2 — Correctness/Architecture/Compatibility/Safety**: the start-time inversion
  opens a window where a **live daemon has no identity record**. The launcher writes it
  "once the daemon reports ready", but readiness *is* `server-info.json` appearing; a
  launcher killed in between leaves a live daemon whose record lacks a start time,
  which pass 2's own new rule (`AbsentOrUnparseable → recover`) turns into deleting the
  state and spawning a second daemon. Today `state.js` publishes the start time in the
  same atomic write as the pid, so the window does not exist. Compounded by
  `server-info.json` now having two whole-file-rename writers with no stated ownership,
  and by the token needing to be enforceable before the launcher writes it.
- 🔴 **P3 — Correctness/Safety**: the reaper and `prune` gate on "the owning pid and its
  start time" plus "a skip for any generation a live process holds" — and **neither has
  a data source**. Temp names carry only `<gen>`, no pid is recorded anywhere, and after
  step 9's rename nothing records who created a generation. So for the two residues
  that matter (a crash between rename and pointer, and a generation superseded by
  `repair`) only the age backstop applies — and a superseded generation is old by
  construction, so `repair`'s carefully-designed safety against a live reader is undone
  by the next `ensure`. `prune` also holds no single-flight lock.
- 🔴 **P4 — Security**: the tree hit path performs **no cryptographic check**. Four
  local checks accept a ~294MB executable tree, the attestation is self-referential, and
  the trusted location is `ACCELERATOR_CACHE_DIR` — which this plan actively recommends
  and which is settable from per-project config. Ownership plus mode cannot distinguish
  a materialised tree from attacker-supplied content the user happens to own.
- 🔴 **P5 — Security**: the host-platform smoke check **executes unverified Chromium
  inside the job that holds the signing secret**. Chromium is the one input ADR-0059
  records as TLS-trust-only, and the plan's own rule ("the step that handles untrusted
  input holds no credential") was applied to extraction but not to *execution*, which is
  strictly stronger handling.
- 🔴 **P6 — Test Coverage**: the `zero skipped tests` gate **cannot pass in any CI lane**.
  All 14 `test-run.js` tests are `skip:`-gated on a Playwright install no workflow
  creates, and `test:unit` runs on both ubuntu and macos — so Phase 6 is not mergeable
  green.
- 🔴 **P7 — Test Coverage**: the skip-count detector is **blind to the vacuous-pass
  pattern the plan itself diagnosed**. `daemon.test.js:72` is `if (!nsRoot) return;`
  inside the test body, which `node --test` reports as *passed*, not skipped — the same
  shape as the `catch { return; }` the plan correctly condemns in `identity.test.js`.
- 🔴 **P8 — Compatibility**: `test-trust-root` is **unconditionally enabled by the
  repo's own lint and test lanes**. `tasks/lint/cli.py:7` and `tasks/test/cli.py:13`
  both pass `--all-features` (deliberately, per the latter's docstring, to enable
  `bash-parity`), so `mise run cli:check` and `mise run test:unit:cli` build the
  launcher with the widened trust root and `ACCELERATOR_TEST_PUBLIC_KEY_FILE` unset —
  making Phase 4's own "`mise run` exits 0" criterion unsatisfiable, or silently
  trusting the test key in every `cli/target/` launcher the dev override can exec.
  Verified directly against both files.

### Also Substantial (major)

Recurring across lenses: the `ACCELERATOR_TREE_<NAME>` export is self-contradictory
(disk-enumerated names cannot be "always cleared") and derives variable names from
untrusted filenames on every dispatch, including the PreToolUse hook;
`ACCELERATOR_LAUNCHER_BIN` is an **existing** dev-override input with incompatible
semantics, so exporting it breaks the override; the tree layout version's stated
"higher refused" gate cannot refuse the *older* layout it exists to refuse, and is
absent from both the name grammar and the reuse-scan glob; `prune`'s "no pointer
references it" can never fire because per-version pointers are never retired;
`ensure` loads the manifest *before* its reuse scan, so the dev-override path pays two
HTTPS GETs per invocation; `persist-credentials: false` would break `git push` and
therefore every release; the token-less assembly step cannot run `gh release download`
for the reuse path; the all-zeros sentinel is unrealisable with three required
assembly-measured sizes; `.gitignore` is never extended for `bin/trees/`; and
`Verdict<Reason>`, `HostReach`'s variants, the usage-vs-rejection rule and the tree
directory name are each specified two different ways in two different places.

Three of the plan's own counts are wrong: `test-run.js` has 14 skip-gated tests not 16,
there are 10 `lib/*.js` modules not 11, and `test-design.sh` is 553 lines not 552.

### Assessment

The plan's *design* is now strong, and pass 3 confirms that: no lens challenged the
tree lifecycle's shape, the port boundaries, the trust-anchor model or the publish
path. What pass 3 establishes is a process problem rather than a design problem.

Three consecutive passes have each closed the previous pass's findings and introduced
new criticals **in the fix material itself** — 7 after pass 1, 8 after pass 2. The
defects are not random: they cluster in mechanisms invented to close a finding
(`prune`'s liveness gate, the smoke check, the zero-skip assertion, the test trust
root, the start-time inversion), and several are contradictions between a new paragraph
and one written minutes earlier in a different section. At ~3,200 lines with some
rationales in four or five homes, the document has outgrown the ability of a
single-pass edit to keep it coherent — the code-quality lens traces four of this pass's
findings directly to divergent duplicate copies.

Recommendation: stop remediating in place. Two changes to the approach are more
valuable than a fourth fix pass. First, **split the plan** — Phases 1–3 and 6 (the
CLI migration and the `run.sh` port) are separable from Phases 4, 5 and 7 (tree
artifacts, the release pipeline, the runtime swap), and the second group is where every
critical in all three passes has landed. Second, for the tree-artifact group, prefer a
**spike over further planning**: P1, P3, P4 and the layout-version gate are all
questions about what the filesystem, the loader and the launcher actually do, and each
has been answered wrongly twice on paper.

---
*Review generated by /accelerator:review-plan*
