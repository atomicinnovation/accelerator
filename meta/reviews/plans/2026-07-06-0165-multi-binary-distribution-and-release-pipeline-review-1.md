---
type: plan-review
id: "2026-07-06-0165-multi-binary-distribution-and-release-pipeline-review-1"
title: "Plan Review: Multi-Binary Static Distribution and Release Pipeline with minisign"
date: "2026-07-06T00:46:14+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-07-06-0165-multi-binary-distribution-and-release-pipeline"
target: "plan:2026-07-06-0165-multi-binary-distribution-and-release-pipeline"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, correctness, code-quality, test-coverage, security, safety, compatibility, portability]
review_number: 1
review_pass: 2
tags: [rust, distribution, release, cross-compile, minisign]
last_updated: "2026-07-06T11:01:16+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Multi-Binary Static Distribution and Release Pipeline with minisign

**Verdict:** REVISE

The plan is well-researched and structurally sound: strictly additive so the
visualiser keeps releasing at every commit, faithful to the frozen 0164 consumer
contract (byte-exact signing, default-prehash reconciliation, ET_DYN-aware static
check, transitive version coherence), and it reuses the proven draft-preserve
seam rather than reinventing it. However, two independent lenses each surfaced a
release-breaking defect — a second, parallel upload/publish cycle that defeats
the draft-preserve guarantee, and a static-linking assertion that depends on a
tool absent from the `macos-latest` runner — and a cluster of majors around
secret handling, launcher-binary signing, and a manifest-signature filename that
would 404 every launcher. These are all correctable at the plan level; the
foundations are right.

### Cross-Cutting Themes

- **Two parallel upload/publish cycles defeat the single-transition invariant**
  (flagged by: safety 🔴, architecture 🟡, code-quality 🔵) — `_publish` calls
  both the existing `upload_and_verify` (which flips `--draft=false` in its own
  try-block) and the new `upload_and_verify_launcher` (its own publish + its own
  delete-on-error). Whichever runs first publishes; a second-track failure then
  either "preserves" an already-published release or deletes it. The
  draft-preserve AC is silently unmet.

- **Fail-open vs fail-closed on the signing secret is unresolved**
  (flagged by: correctness 🟡, safety 🟡) — "sign only if
  `ACCELERATOR_RELEASE_SECRET_KEY` is present" (Migration Notes) contradicts a
  `resolve_secret_key` that falls back to a non-existent dev path and raises.
  Left ambiguous, this ships either an unsigned release the launcher rejects, or
  a total release outage (visualiser included) the moment Phase 5 merges without
  the secret.

- **The static-linking guarantee rests on an unprovisioned tool**
  (flagged by: portability 🔴, correctness 🟡) — `readelf`/`llvm-readelf` is not
  on `macos-latest`, not in `mise.toml`, and not pulled by release-prepare. The
  assertion either aborts every release or (if it copies the minisign
  skip-when-absent pattern) silently no-ops and ships unverified binaries.

- **The `.minisig` filename contract is fragile**
  (flagged by: compatibility 🟡, code-quality 🟡) — the generic `sign_file`'s
  `target.with_suffix(suffix + ".minisig")` yields `manifest.json.minisig`, but
  the launcher fetches the hardcoded `manifest.minisig`. Every launcher that
  needs a sub-binary 404s on the manifest signature.

- **Secret-key handling has several sharp edges**
  (flagged by: security, code-quality, correctness, safety) — the `finally`
  cleanup design is self-contradictory, the secret sits in the environment
  during untrusted crate compilation, temp-file mode/residue is unspecified, and
  `keys.generate` prints the base64 secret to stdout.

### Tradeoff Analysis

- **Passwordless `-W` single-key simplicity vs blast radius**: The
  compromise-only, no-overlap, embedded-key model (security 🟡) is operationally
  simple and matches the version-pinned launcher reality, but has no revocation
  path — a compromised key is trusted by the installed base until users upgrade.
  This is an accepted ADR-0046 tradeoff; the ask is not to change the model but
  to have `RELEASING.md` own the detection/response side of it explicitly.

- **Additive-parallel upload track (minimal churn) vs a single unified
  publish gate (integrity)**: Bolting on `upload_and_verify_launcher` next to
  the visualiser flow is the smaller diff, but it is precisely what breaks the
  single draft→published transition. Here integrity should win — stage and
  re-verify all assets across both tracks, then flip `--draft=false` once.

### Findings

#### Critical

- 🔴 **Safety / Architecture / Code-Quality**: Two independent publish/delete
  cycles break the draft-preserve guarantee
  **Location**: Phase 4 (upload_and_verify_launcher) + Phase 5 (_publish wiring)
  The existing `upload_and_verify` publishes inside its own try/except
  (`github.py:164`); the new launcher flow adds a second publish and a second
  delete-release-and-tag path. Whichever runs first flips the release public, so
  a later launcher-track `AssetVerificationError` can no longer preserve a draft
  (it is already public), and a non-verification error can delete an
  already-published release and its pushed tag. The core release-integrity AC is
  silently violated.

- 🔴 **Portability / Correctness**: The static-ELF assertion depends on
  `readelf`/`llvm-readelf`, which is absent on the `macos-latest` release runner
  **Location**: Phase 2, Section 2 (Static-linking assertion)
  macOS ships no GNU binutils; `llvm-readelf` is not on the default PATH; Rust's
  `llvm-tools-preview` provides `llvm-readobj`, not `llvm-readelf`; and
  release-prepare depends only on `rust-targets` + `node`. Nothing in the plan
  provisions the tool. The assertion therefore either fails every release, or —
  if it inherits the codebase's skip-when-absent convention — silently no-ops in
  exactly the environment where staticness must be proven, shipping unverified
  binaries. The plan must both provision the tool (mise/brew) and specify that a
  missing reader fails **closed**.

#### Major

- 🟡 **Compatibility / Code-Quality**: Manifest signature asset name resolves to
  `manifest.json.minisig`, but the launcher fetches `manifest.minisig`
  **Location**: Phase 1 (sign_file) → Phase 3 (emit_manifest) → Phase 4 (upload)
  `resolve/mod.rs:123` hardcodes `GET .../manifest.minisig`. The generic
  `sign_file` naming (and minisign's own default) yields `manifest.json.minisig`.
  The prose says "produces `manifest.minisig`" but the shown mechanics
  contradict it. Total launcher-track breakage for all users; near-critical
  impact. Have `sign_file` take an explicit signature output path and add a
  producer test asserting the uploaded set contains literally `manifest.minisig`.

- 🟡 **Correctness**: Signing of the launcher binaries themselves is
  under-specified
  **Location**: Phase 4 & Phase 5
  The bootstrap fetches `accelerator-{platform}` + a **detached**
  `accelerator-{platform}.minisig` (not a manifest entry). No phase's Changes
  Required crisply produces those four `.minisig` files; Phase 3's `emit_manifest`
  signs only the manifest. If the launcher binaries aren't signed, Phase 4
  uploads a `.minisig` with no source and the bootstrap fails closed on first
  use. Add an explicit "sign every staged binary" step distinct from manifest
  emission.

- 🟡 **Correctness / Safety**: Fail-open vs fail-closed on a missing signing
  secret is ambiguous
  **Location**: Phase 1 (resolve_secret_key) + Phase 5 / Migration Notes
  "Sign only if the secret is present" reads as a conditional skip, but
  `resolve_secret_key` falls back to a non-existent dev path and raises. Pick one
  contract: either genuinely skip the launcher-signing steps when absent (so the
  visualiser still releases), or state Phase 5 hard-requires the secret and must
  merge only after provisioning. Signing must never half-run.

- 🟡 **Safety**: Merging Phase 5 before the secret is provisioned fails ALL
  releases, not just the launcher track
  **Location**: Phase 5 / Migration Notes
  `cli_cross_compile` + `emit_manifest` wire into the same prepare step that
  builds the visualiser. A fail-closed missing secret aborts the whole prepare,
  breaking the plan's "visualiser keeps releasing at every commit" invariant —
  and the unblock is held by a repo admin, not the merging author. Make Phase 5
  the explicit last merge, gated on a checklist item, with a preflight that fails
  with a clear "secret not provisioned — do not merge Phase 5 yet" message.

- 🟡 **Security**: The signing secret is in the environment during untrusted
  crate compilation
  **Location**: Phase 5, Section 2 (workflow signing step)
  Phase 5 injects `ACCELERATOR_RELEASE_SECRET_KEY` into the `Prepare*` steps,
  which run four `cargo zigbuild` invocations (executing every transitive crate's
  build scripts and proc-macros) **before** signing. A single malicious
  dependency can read the secret from `/proc/self/environ`. Scope the secret to
  the signing operation only — separate cross-compile and sign, and expose the
  secret to the signing step alone.

- 🟡 **Security**: The current committed "placeholder" key must be *mandated*
  untrusted, not optionally replaced
  **Location**: What We're NOT Doing; Phase 1, Section 3
  Nothing forces the admin to regenerate rather than provision a secret for the
  already-committed structurally-valid key (secret half of unknown provenance).
  Make the runbook mandate a freshly generated `-W` key whose secret has never
  left the admin's control before any real sub-binary ships, and add an
  AC/check that the release-signing HEAD's `.pub` differs from the placeholder.

- 🟡 **Security**: Passwordless single-key model has large blast radius and no
  revocation path
  **Location**: Phase 1, Section 3; Migration Notes
  Compromise (env exposure, leaked GHA secret, admin takeover) yields
  undetectable long-lived forgery against the installed base. Have `RELEASING.md`
  own compromise detection/response, restrict the `release` environment so the
  secret is reachable only from the approved job, and preserve the launcher's
  latent verify-any-of headroom for a future overlap window.

- 🟡 **Code-Quality / Correctness**: `resolve_secret_key` finally-cleanup design
  is internally contradictory
  **Location**: Phase 1, Section 1
  A function that *returns* the temp key `Path` cannot also unlink it in its own
  `finally` (the key is deleted before signing). As described it either deletes
  too early or leaks the key on the exception path; the batch lifetime across
  many signs is also unspecified. Specify a `@contextmanager` yielding the key
  path, bracketing the whole sign loop, with a test asserting the temp file is
  gone after the block including on exception.

- 🟡 **Code-Quality**: `sign_file` swallows minisign stderr and raises a generic
  `CalledProcessError`
  **Location**: Phase 1, Section 1
  `check=True, capture_output=True` hides minisign's real diagnostic (bad key,
  TTY prompt) behind "returned non-zero exit status 1", and the untyped error
  muddies the `AssetVerificationError`-vs-other distinction the draft-preserve
  seam branches on. Mirror the module convention: `check=False`, and on non-zero
  raise a new `SigningError` including `result.stderr.strip()`.

- 🟡 **Architecture / Correctness**: Presence-guarded `manifest.version`
  coherence weakens the anti-rollback twin
  **Location**: Phase 3, Section 2
  Reading `manifest.version` only "when the staged manifest exists" means the
  gitignored staging artifact drops out of most coherence runs, and in the emit
  path the check is tautological (version is set from the same argument). The
  real anti-rollback rests on the workspace-Cargo.toml entry plus cross-compile
  ordering. Enforce coherence unconditionally within the emit flow (fail hard if
  the staged manifest is missing at that point) and document where the real
  enforcement lives.

- 🟡 **Safety**: `git.commit_version`'s `git add .` runs after build/sign and can
  sweep release artifacts (or secret material) into the commit
  **Location**: Phase 5 (release.py _publish) + Phase 2 (dist/ gitignore)
  `_publish` → `git.commit_version` → `git add .` (`git.py:73`) after the prepare
  step wrote binaries, `manifest.json`, and every `.minisig`. The plan gitignores
  `/dist/` but never states that `manifest.json`, `manifest.minisig`, and the
  per-binary `.minisig` all land there. Require every staged artifact under
  `dist/release/` and add a guard that `git status --porcelain` is free of build
  artifacts before `commit_version`.

- 🟡 **Test-Coverage**: No automated test feeds a producer-emitted *non-empty*
  manifest through the real launcher verifier
  **Location**: Phase 3 & 4 Success Criteria; Testing Strategy
  With `binaries: {}` at HEAD, the per-binary `{sha256, signature}` assembly and
  the launcher's `verify_binary` are never exercised together automatically — the
  core deliverable is validated only by jsonschema shape and a manual dry-run.
  Realise the fixture-crate approach concretely: build a tiny fixture sub-binary,
  sign it, emit a non-empty manifest, and assert the launcher's own verifier (or
  the shim per platform entry) accepts it at HEAD.

- 🟡 **Test-Coverage**: The crown-jewel sign→verify round-trip can silently
  no-op under the minisign-absent skip guard
  **Location**: Phase 1 & 3 Success Criteria
  The tests that prove pipeline signatures satisfy `allow_legacy=false` return
  green when `minisign` is off PATH. In CI (detectable via an env marker), fail
  or `xfail(strict=True)` rather than skip, so a signing-invocation regression
  can't pass without running.

- 🟡 **Test-Coverage**: The static-ELF assertion is tested only against
  hand-captured `readelf` output
  **Location**: Phase 2 Success Criteria
  The mock duplicates the parser's assumptions rather than the tool's real
  contract, and the `readelf` vs `llvm-readelf` variance the plan allows for goes
  unexercised. Capture fixtures from both tools on genuinely-static and
  genuinely-dynamic binaries, and add a native-host smoke test that runs the real
  assertion against a freshly-built host binary.

- 🟡 **Portability**: The re-verify shim must resolve to the runner's host arch
  **Location**: Phase 4, Section 1 (_reverify_via_shim, VENDORED_SHIM)
  The shims are per-platform; `macos-latest` is now arm64. A `VENDORED_SHIM`
  defaulted to `darwin-x64` fails to exec, crashing the very gate that keeps a
  bad release in draft. Resolve via the host `uname` mapping in
  `tasks/shared/targets.py`, or re-verify with the freshly-built host
  `accelerator-verify` instead of a committed cross-compiled shim.

#### Minor

- 🔵 **Correctness**: `cli_cross_compile` must run *after* `version.bump`, else
  the launcher embeds the old `CARGO_PKG_VERSION` and rejects its own manifest
  (`ManifestVersionMismatch`). Sequence explicitly in `*_prepare` and assert the
  cross-compiled launcher's embedded version equals the release version.
  **Location**: Phase 5, Section 1

- 🔵 **Architecture / Safety / Portability**: The committed vendored shims have
  no drift guard against `cli/verify`, are non-reproducible, and need the full
  cross-compile toolchain to regenerate. Add a lightweight CI check that fails
  when `cli/verify/**` changed since the shims were last vendored (recorded
  source hash), and document the regeneration environment.
  **Location**: Phase 2, Section 4

- 🔵 **Architecture / Code-Quality**: The draft-preserve/delete policy is
  duplicated across `upload_and_verify` and `upload_and_verify_launcher`. Extract
  one upload→reverify→publish envelope parameterised by asset set + per-asset
  verify strategy. (Same root cause as critical #1.)
  **Location**: Phase 4, Section 1

- 🔵 **Code-Quality**: The `BinaryEntry`/`PlatformAsset` types and the map
  assembly (read Cargo description, compute sha256, slurp `.minisig`) are
  unlocated — risk of a fat inline blob in `release.py`. Locate a
  `collect_entries(...)` in `tasks/manifest.py`; keep `release.py` a thin caller.
  **Location**: Phase 3, Section 1 + Phase 5, Section 1

- 🔵 **Security**: The materialised secret-key temp file has unspecified
  permissions and crash residue. Create it mode `0600` under a
  `TemporaryDirectory`; note local signing must be on a non-shared machine.
  **Location**: Phase 1, Section 1

- 🔵 **Security**: `keys.generate` prints the base64 secret to stdout (unmasked
  on an admin laptop → scrollback/screen-share/history). Direct the admin to read
  from the written `.sec` or pipe straight into `gh secret set`; mask if ever
  emitted in CI.
  **Location**: Phase 1, Section 2

- 🔵 **Test-Coverage**: `keys.generate` writes the tracked
  `keys/accelerator-release.pub`; a naive test would clobber the committed key.
  Parameterise output paths (default to repo locations) so tests target
  `tmp_path`, mirroring `test_build.py`'s path-patching.
  **Location**: Phase 1 Success Criteria

- 🔵 **Test-Coverage**: `jsonschema` is not in `pyproject.toml` (the existing
  contract test hand-rolls regex), so the Phase 3 criterion is unrunnable as
  written; and schema validation is a weaker proxy than a serde round-trip.
  Add + pin `jsonschema` and treat it as a complement, not a replacement.
  **Location**: Phase 3 Success Criteria

- 🔵 **Test-Coverage**: Phases 4 & 5 cite `tests/unit/tasks/test_github.py` /
  `test_release.py`, but those suites live under `tests/integration/tasks/`.
  Correct the paths and extend the existing draft-preserve fixtures there.
  **Location**: Phase 4 & 5 Success Criteria

- 🔵 **Security**: `_reverify_via_shim`'s `pub` must be the committed
  `keys/accelerator-release.pub` (the embedded key), not one derived from the
  signing secret — otherwise the "signed by a key launchers embed" guard passes
  tautologically. Assert a non-committed-key signature fails re-verify.
  **Location**: Phase 4, Section 1

- 🔵 **Compatibility**: The producer/consumer key handshake must be sequenced so
  no release is signed by a secret whose public half isn't embedded in the
  concurrently-shipped launcher. Make this an explicit checklisted gate in
  `RELEASING.md`. (Relates to Phase 5 gating.)
  **Location**: Phase 5 / Migration Notes

- 🔵 **Compatibility**: The per-sub-binary detached `.minisig` assets Phase 4
  uploads are never fetched by the launcher (it uses the inline manifest
  `signature`). Harmless, but tests must assert the **inline** signature path,
  not the detached asset's presence.
  **Location**: Phase 4, Section 1

- 🔵 **Safety**: The forensic alert hardcodes `title=Visualiser release` and will
  mislabel launcher/manifest failures during triage. Parameterise the track label.
  **Location**: Phase 4 (reuse of `_emit_forensic_alert`)

- 🔵 **Safety**: `gh release upload` has no `--clobber`, so a re-run after a
  preserved draft or mid-upload failure collides on existing assets. Make the
  launcher/manifest uploads idempotent and document the retry procedure.
  **Location**: Phase 4, Section 1

- 🔵 **Compatibility**: The whole signature-acceptance contract depends on the
  pinned minisign defaulting to prehash. Keep the exact `0.12` pin and make the
  round-trip guard a required gate on any minisign version change.
  **Location**: Current State Analysis, Key Discoveries / `mise.toml:32`

#### Suggestions

- 🔵 **Architecture**: Ensure the Phase 3 fixture-crate round-trip exercises the
  *full* resolver path (fetch → parse → platform_entry → sha256 → minisign), so
  0168 inherits a proven producer→consumer contract rather than discovering gaps.

- 🔵 **Code-Quality**: Express the manifest as a `TypedDict` (or build from the
  typed entries) so load-bearing field names are checked by pyrefly rather than
  failing only at the launcher.

- 🔵 **Security**: Use argv lists (or `shlex.quote`) for the shim/`gh`
  invocations, matching the existing `download_release_asset` convention.

- 🔵 **Architecture**: Consider deriving `DISPATCHED_SUBBINARIES` from a single
  declaration (workspace members carrying a description, or a `package.metadata`
  flag) so onboarding a sub-binary touches one source, not three.

### Strengths

- ✅ Strictly additive phasing: the visualiser release path is untouched, so
  every intermediate commit still ships — a clean open/closed extension.
- ✅ Correctly preserves the sign-exact-bytes invariant (serialise once, sign
  that file, upload it) matching the launcher's verify-raw-bytes-before-parse.
- ✅ Correctly reconciles "no `-H`" with `allow_legacy=false` (minisign 0.12
  prehashes by default), grounded in the existing `cli/verify` round-trip.
- ✅ Static assertion correctly checks `PT_INTERP`/`DT_NEEDED` absence and does
  **not** assert `EXEC` (musl static-PIE is `ET_DYN`).
- ✅ Feeds both consumer paths correctly (bootstrap detached whole-file minisig;
  launcher manifest with inline per-binary `{sha256, signature}`).
- ✅ Version coherence extension transitively guarantees `manifest.version ==
  launcher CARGO_PKG_VERSION`, satisfying the exact-equality anti-rollback.
- ✅ Reuses the proven draft-preserve-on-`AssetVerificationError` seam; deliberately
  avoids the third-party `minisign-action` supply-chain dependency; keeps shim
  vendoring out of the release hot path; never commits the secret.
- ✅ Good functional-core/imperative-shell split (`build_manifest` pure,
  `emit_manifest` owns I/O + signing) in cohesive new modules.

### Recommended Changes

1. **Unify the publish gate** (addresses: two-publish-cycle critical, duplicated
   draft-preserve, retry-collision) — Refactor `_publish` so all assets across
   both the visualiser and launcher tracks are uploaded and re-verified first,
   then `--draft=false` fires **exactly once**; the delete path must never run
   after publish. Extract a single shared upload→reverify→publish envelope
   parameterised by asset set + verify strategy, and make uploads idempotent.

2. **Provision and harden the static-ELF check** (addresses: readelf-availability
   critical, fail-closed, tool-divergence tests) — Add a pinned `readelf`/
   `llvm-readelf` provider to `mise.toml` (or a `brew`/workflow step), make
   release-prepare depend on it, specify the assertion fails **closed** on a
   missing reader, and anchor the parser test to real output from both tools.

3. **Make signing explicit and correct** (addresses: launcher-binary signing,
   manifest `.minisig` name, `sign_file` errors, secret lifetime) — Add a
   dedicated "sign every staged binary" step producing the four
   `accelerator-{platform}.minisig`; have `sign_file` take an explicit signature
   output path so the manifest signs to `manifest.minisig`; run with
   `check=False` raising a typed `SigningError` with stderr; specify
   `resolve_secret_key` as a `@contextmanager` bracketing the whole sign loop.

4. **Resolve the fail-open/fail-closed contract and Phase 5 gating** (addresses:
   secret ambiguity, all-releases-fail, key handshake) — State plainly that
   signing is unconditional once wired (fail closed, with a clear preflight
   message), make Phase 5 the explicit last merge gated on a provisioning
   checklist, and add the commit-key → ship-launcher → sign-release sequence to
   `RELEASING.md`.

5. **Tighten secret-handling and key policy** (addresses: env exposure,
   placeholder key, blast radius, temp-file perms, stdout print, re-verify key) —
   Scope the secret to the signing step only (out of the cargo compile
   environment); mandate a freshly generated `-W` key replacing the placeholder,
   with an AC that HEAD's `.pub` differs; give `RELEASING.md` compromise
   detection/response; create the temp key `0600`; stop printing the secret;
   re-verify against the committed `.pub`.

6. **Sequence cross-compile after the version bump** (addresses: embedded-version
   mismatch) — Explicitly order `cli_cross_compile` after `version.bump` in
   `*_prepare` and assert the cross-compiled launcher's embedded version equals
   the release version.

7. **Close the test gaps** (addresses: non-empty manifest, skip-guard no-op,
   jsonschema dep, keys.generate clobber, test paths) — Add a fixture-crate
   end-to-end test that accepts a producer-emitted non-empty manifest through the
   launcher verifier; fail/xfail-strict the round-trips in CI when minisign is
   absent; add+pin `jsonschema`; parameterise `keys.generate` output paths; and
   correct the `github`/`release` test paths to `tests/integration/tasks/`.

8. **Add drift/label guards** (addresses: shim drift, forensic mislabel) — A CI
   check that flags un-revendored shims when `cli/verify/**` changes, and
   parameterise the forensic-alert track label.

## Per-Lens Results

### Architecture

**Summary**: Structurally sound and strictly additive, with a clean
functional-core/imperative-shell split and correct treatment of the frozen 0164
contract. The most significant gap is two independent upload/publish flows racing
on a single shared draft-state transition, which defeats draft-preserve for the
launcher track. Secondary: presence-guarded `manifest.version` coherence and no
enforced sync between the committed vendored shims and their source crate.

**Strengths**:
- Additive phasing is a clean open/closed extension, not a rewrite.
- `build_manifest` (pure) / `emit_manifest` (I/O + signing) split; cohesive
  `signing.py` / `manifest.py`.
- Respects the byte-exact trust boundary (serialise once, sign that file, upload
  the same bytes).
- Reuses the draft-preserve seam; keeps shim vendoring out of the release hot
  path to avoid committing non-reproducible binaries via `git add .`.

**Findings**:
- 🟡 (high) Two independent upload flows race on a single shared draft-publish
  transition (Phase 4 / Phase 5). Whichever flow publishes first defeats
  draft-preserve for the other; a sibling `except Exception` could delete an
  already-published release. Make the draft→published transition a single gate
  after all assets across both tracks re-verify.
- 🟡 (medium) Presence-guarded `manifest.version` check weakens the anti-rollback
  producer twin (Phase 3). The gitignored staging manifest drops out of most
  coherence runs. Enforce unconditionally within the emit flow.
- 🔵 (medium) Committed vendored shims have no enforced sync to `cli/verify`
  (Phase 2). Add a staleness guard (rebuild-and-diff or source-revision marker).
- 🔵 (medium) Draft-preserve/delete policy duplicated across two upload functions
  (Phase 4). Extract one shared guard envelope.
- 🔵 (low) Adding a sub-binary requires synchronised edits across workspace
  members, `DISPATCHED_SUBBINARIES`, and Cargo `description` (Phase 2/3). Derive
  from a single declaration.
- 🔵 (medium, suggestion) End-to-end trust path only exercisable against a
  fixture until 0168; ensure the fixture round-trip exercises the full resolver.

### Correctness

**Summary**: Unusually rigorous about the load-bearing invariants — preserves
sign-exact-bytes, reconciles "no `-H`" against `allow_legacy=false`, and avoids
requiring `ET_EXEC` for musl static-PIE. Material gaps: launcher-binary signing
is under-specified, the static-ELF tool availability + fail-open/closed on the
macOS runner is undefined, and "sign only if secret present" contradicts a
`resolve_secret_key` that fails hard when the secret is absent.

**Strengths**:
- Preserves the sign-exact-bytes invariant (serialise once via
  `atomic_write_text`, sign that file, upload it) matching `mod.rs:129-130`.
- Reconciles "sign whole-file, not `-H`" with `allow_legacy=false` via the
  existing `cli/verify/tests/verify.rs` round-trip.
- Static-ELF check correctly targets `PT_INTERP`/`DT_NEEDED`, not `EXEC`.
- Recognises `binaries:{}` as schema-legal and plans a fixture-crate for the
  verify path.
- Reuses the draft-preserve seam, distinguishing `AssetVerificationError` from
  other exceptions.
- Bare lowercase-hex sha256 matches the launcher's prefix-tolerant compare.

**Findings**:
- 🟡 (medium) Launcher-binary signing under-specified — no crisp step produces
  the four `accelerator-{platform}.minisig` the bootstrap fetches (Phase 4/5).
- 🟡 (medium) Static-ELF assertion tool availability + fail-open/closed
  unspecified on `macos-latest` (Phase 2).
- 🟡 (medium) "Sign only if secret present" contradicts a `resolve_secret_key`
  that raises when the env var is unset (Phase 1 + Phase 5 / Migration Notes).
- 🔵 (medium) `cli_cross_compile` must run after `version.bump`, else the
  launcher embeds the old version and rejects its own manifest (Phase 5).
- 🔵 (medium) `manifest.version` coherence is tautological in the emit path
  (Phase 3).
- 🔵 (low) Materialised secret-key lifetime across the multi-binary signing batch
  is unspecified (Phase 1).

### Code Quality

**Summary**: Well-structured and idiomatic reuse of the producer helpers, TDD per
module. Three concerns stand out: `sign_file` swallows minisign's stderr and
raises a generic `CalledProcessError` (breaking the error taxonomy the
draft-preserve seam relies on); the `resolve_secret_key` `finally`-cleanup design
is internally contradictory; and `sign_file` bakes in a signature-naming
convention that mis-names the manifest signature. Secondary: duplicated
orchestration and unlocated cross-module entry assembly.

**Strengths**:
- Idiomatic reuse of `atomic_write_text`, `compute_sha256`, `TARGETS`, the
  magic-byte check; mirrors `create_checksums`' coherence-before-and-after.
- TDD per new module with isolatable tests (subprocess mocking, real-shim
  round-trips, skip-when-absent guards).
- Six additive, independently-mergeable phases; respects the low-comment
  convention.
- The serialise-once/sign/upload rule is called out explicitly.

**Findings**:
- 🔴 (high) `sign_file` swallows minisign stderr, raises a generic
  `CalledProcessError`; no `SigningError` sibling (Phase 1). Use `check=False` +
  typed error with stderr.
- 🟡 (high) `resolve_secret_key` finally-cleanup is internally contradictory —
  can't return the path and unlink it in its own `finally` (Phase 1). Use a
  `@contextmanager`.
- 🟡 (high) `sign_file` hardcodes `<name>.<ext>.minisig`, yielding
  `manifest.json.minisig` where the contract needs `manifest.minisig` (Phase 1/3).
- 🔵 (medium) Parallel `upload_and_verify_launcher` duplicates the draft-preserve
  orchestration (Phase 4).
- 🔵 (medium) `BinaryEntry`/`PlatformAsset` assembly unlocated across modules
  (Phase 3/5); risk of a god-function in `release.py`.
- 🔵 (low, suggestion) Manifest built as an untyped nested dict; use a
  `TypedDict` (Phase 3).

### Test Coverage

**Summary**: Solid unit foundation mirroring repo patterns and correct edge-case
enumeration. But the single most valuable behaviour — a producer-emitted
non-empty manifest accepted by the real launcher verifier — has no automated test
and is deferred to manual/0168, and the security-critical sign→verify round-trip
is skip-guarded so it can silently no-op. Several infra details (jsonschema not
declared, test-file location mismatches, `keys.generate` mutating tracked files)
would break or weaken the suite as written.

**Strengths**:
- Accept/reject pairs for the highest-risk logic (`_assert_static_elf`,
  coherence, missing description).
- Edge cases enumerated: empty `binaries:{}`, missing description, swapped
  `.minisig`, corrupted re-download, unrelated error deletes.
- minisign skip guard reuses the proven `verify.rs:60-70` pattern; round-trips
  assert through the real shim.
- Additive strategy keeps existing tests green; `test:unit:cli` pins the frozen
  contract.

**Findings**:
- 🔴 (high) No automated test feeds a producer-emitted non-empty manifest through
  the real launcher verifier (Phase 3/4).
- 🔴 (high) The sign→verify round-trip can silently no-op under the
  minisign-absent skip guard (Phase 1/3).
- 🟡 (medium) `_assert_static_elf` tested only against hand-captured `readelf`
  output; parser anchored to itself (Phase 2).
- 🔵 (high) `jsonschema` is an undeclared dependency and a weaker proxy than the
  serde parser (Phase 3).
- 🔵 (medium) `keys.generate` test risks clobbering the tracked
  `keys/accelerator-release.pub` (Phase 1).
- 🔵 (medium) Plan cites `tests/unit/tasks/` for github/release suites that live
  under `tests/integration/tasks/` (Phase 4/5).

### Security

**Summary**: Gets the core cryptographic hygiene right (sign exact bytes,
re-verify before publish, sha256-as-corruption/minisign-as-trust, no committed
secret, no third-party action). Material gaps are operational secret handling:
the passwordless secret is exposed to untrusted crate build scripts, temp-file
perms/residue are unspecified, and the placeholder-key provenance question is
deferred without a forcing function. The `-W` single-key model has a large blast
radius and no revocation path the runbook should own.

**Strengths**:
- Re-download-and-re-verify through the shim before publishing; fail closed,
  preserve draft (Phase 4).
- Signs the exact serialised bytes; forbids re-serialisation (Phase 3).
- Avoids the third-party `minisign-action` (What We're NOT Doing).
- Keeps sha256 (corruption) + minisign (trust); debug archives unsigned.
- Secret never committed; materialised to a temp file unlinked in `finally`;
  `capture_output` keeps minisign output out of logs (Phase 1).

**Findings**:
- 🔴 (high) Signing secret in the environment during untrusted crate compilation
  (Phase 5). Scope it to the signing step only.
- 🟡 (medium) The committed "placeholder" key must be mandated untrusted and
  replaced, not optionally provisioned (What We're NOT Doing / Phase 1).
- 🟡 (medium) Passwordless single-key model — large blast radius, no revocation
  path; runbook must own detection/response (Phase 1).
- 🔵 (medium) Materialised temp key perms (`0600`) and crash residue unspecified
  (Phase 1).
- 🔵 (medium) `keys.generate` prints the base64 secret to stdout (Phase 1).
- 🔵 (low) `_reverify_via_shim` must pin the committed `.pub`, not the
  just-generated key (Phase 4).
- 🔵 (low, suggestion) Prefer argv lists over f-string shell commands (Phase 4).

### Safety

**Summary**: Commendably additive and reuses the proven draft-preserve seam, but
the release-integrity guarantee is at risk: Phase 4 bolts a second, independent
upload→verify→publish cycle alongside the existing one, creating two
`--draft=false` points and a second destructive delete path that can fire against
an already-published release. Secondary: fail-open/closed ambiguity on the
secret, `git add .` sweeping artifacts into the release commit, and the blast
radius of merging Phase 5 before provisioning.

**Strengths**:
- Strictly additive (Phases 1–4 wired into nothing live).
- Reuses the battle-tested draft-preserve-vs-delete seam.
- Signs exact bytes and re-verifies every re-downloaded asset before publish.
- `resolve_secret_key` materialises to a temp file unlinked in `finally`;
  `/keys/*.sec` gitignored.
- Static + magic-byte assertions run before any binary is staged.

**Findings**:
- 🔴 (high) Two independent publish/delete cycles break the draft-preserve
  guarantee (Phase 4 / Phase 5). Single final publish gate after all assets
  re-verify.
- 🟡 (medium) Fail-open vs fail-closed on a missing signing secret is ambiguous
  (Phase 5 / Migration Notes).
- 🟡 (medium) `git.commit_version`'s `git add .` runs after build/sign and can
  sweep artifacts (or secret material) into the commit (Phase 5 / Phase 2).
- 🟡 (medium) Merging Phase 5 before the secret is provisioned fails ALL releases
  (Phase 5 / Migration Notes).
- 🔵 (medium) Retry after a partial multi-asset upload collides on existing
  assets; needs `--clobber` (Phase 4).
- 🔵 (high) Forensic alert hardcodes "Visualiser release" and mislabels launcher
  failures (Phase 4).
- 🔵 (low, suggestion) No drift guard between the committed trust-root shims and
  `cli/verify` (Phase 2).

### Compatibility

**Summary**: A producer-side implementation explicitly engineered to conform to
the frozen 0164 contract, and on the verifiable points it conforms well (asset
naming, four-line inline signature, bare-hex sha256, integer `schema_version`,
sign-once, default-prehash reconciliation, transitive version coherence). The one
material gap: the manifest signature asset name resolves to `manifest.json.minisig`
where the launcher fetches `manifest.minisig`.

**Strengths**:
- Feeds both consumer paths correctly (bootstrap detached minisig; launcher
  inline per-binary `{sha256, signature}`).
- Sign-the-exact-bytes preserves pre-parse verification.
- Default-prehash / `allow_legacy=false` reconciliation verified against the real
  contract and the existing round-trip.
- Version coherence transitively guarantees the exact-equality anti-rollback.
- Emits only the frozen additive shape with `binaries:{}` at HEAD.
- Re-verification uses the same shim + committed key consumers use.

**Findings**:
- 🟡 (high) Manifest signature asset name resolves to `manifest.json.minisig`,
  but the launcher fetches `manifest.minisig` (Phase 1/3/4).
- 🔵 (medium) Producer/consumer key handshake must be sequenced so no release is
  signed by a secret whose public half isn't embedded in the shipped launcher
  (Phase 5 / Migration Notes).
- 🔵 (high) Per-sub-binary detached `.minisig` assets are never fetched by the
  launcher (it uses the inline signature); test the inline path (Phase 4).
- 🔵 (low) The whole signature-acceptance contract depends on the pinned minisign
  defaulting to prehash; keep the pin exact and the round-trip guard mandatory.

### Portability

**Summary**: The core distribution model is portability-strong (four targets from
one macOS runner via `cargo zigbuild`, fully-static musl, ET_DYN-aware static
check). But the plan hinges on `readelf`/`llvm-readelf` to prove staticness, and
that tool is not on `macos-latest` nor provisioned anywhere — the single largest
risk. Secondary: host-arch selection of the vendored re-verify shim and the
toolchain needed to regenerate the committed shims.

**Strengths**:
- Cross-compiling four targets from one macOS runner via bundled zig avoids
  native Linux/ARM runners.
- Fully-static musl binaries run on any Linux regardless of libc version.
- Static assertion correctly avoids a false `EXEC` check.
- Verify shim is key-agnostic (public key as argument), refreshable off the hot
  path.
- rustls-only enforced out-of-band by cargo-deny.

**Findings**:
- 🔴 (high) `readelf`/`llvm-readelf` is not available on the `macos-latest`
  release runner and is unprovisioned (Phase 2).
- 🟡 (medium) Tool-absence behaviour of the static assertion must fail closed
  (Phase 2).
- 🟡 (medium) The re-verify shim must resolve to the runner's host arch
  (`macos-latest` is arm64) (Phase 4).
- 🔵 (medium) Regenerating the committed per-platform shims requires the full
  cross-compile toolchain; non-reproducible (Phase 2).
- 🔵 (low) `readelf` vs `llvm-readelf` (not `llvm-readobj`) output divergence, and
  the darwin-dev manual step assumes a tool a stock macOS box lacks (Phase 2).

## Re-Review (Pass 2) — 2026-07-06

**Verdict:** APPROVE

All 8 lenses re-ran fresh against the revised plan. Both criticals and ~11 of the
prior majors are resolved. The re-review surfaced a small set of *introduced
regressions* (from the pass-1 edits) plus deeper pre-existing gaps; the
regressions and clear tightening items were corrected in-place during this pass.

### Previously Identified Issues

- 🔴 **Safety/Architecture**: Two independent publish/delete cycles — **Resolved.**
  Single-gate publish confirmed; delete path confined to the pre-publish envelope.
- 🔴 **Portability/Correctness**: `readelf` absent on `macos-latest` — **Resolved.**
  Fail-closed + explicit provisioning + host-arch shim resolution confirmed.
- 🟡 **Compatibility/Code-Quality**: `manifest.json.minisig` naming — **Resolved.**
  Verified correct against `resolve/mod.rs:123`.
- 🟡 **Correctness**: Launcher-binary signing — **Resolved** (`sign_staged_binaries`).
- 🟡 **Correctness/Safety**: Fail-open/closed secret — **Resolved** (fail-closed explicit).
- 🟡 **Security**: Secret in compile environment — **Resolved** (scoped to `Sign*` step).
- 🟡 **Security**: Placeholder key untrusted — **Resolved** (regenerate mandate).
- 🟡 **Security**: Passwordless blast radius — **Resolved** (runbook owns response).
- 🟡 **Code-Quality/Correctness**: `resolve_secret_key` `finally` — **Resolved** (contextmanager).
- 🟡 **Code-Quality**: `sign_file` stderr/`SigningError` — **Resolved.**
- 🟡 **Safety**: `git add .` sweeps artifacts — **Resolved** (`dist/release/` + guard).
- 🟡 **Test-Coverage**: crown-jewel round-trip skip — **Resolved** (fail-closed in CI).
- 🟡 **Test-Coverage**: static-ELF hand-captured — **Resolved** (real fixtures + smoke).
- 🟡 **Portability**: re-verify host arch — **Resolved.**
- 🟡 **Architecture/Correctness**: Presence-guarded coherence — ⚠️ **Regressed then fixed.**
  The pass-1 "unconditional / fail-hard on missing manifest" wording contradicted
  `create_checksums` (runs in prepare before the manifest exists). Corrected to a
  `require_manifest` parameter: `create_checksums` opts out, `emit_manifest`
  validates once *after* writing.
- 🟡 **Test-Coverage**: Non-empty manifest test — **Strengthened.** Now routes
  producer-emitted bytes through the real Rust `parse_and_validate` /
  `FetchVerifyCacheResolver`, not just the minisign shim.

### New Issues Introduced (and resolution this pass)

- 🟡 **Correctness/Code-Quality**: `sign_staged_binaries` over-globs "every staged
  binary" (would sign the verify shims staged in `dist/release/`) — **Fixed:** now
  an explicit `TARGETS × (accelerator + DISPATCHED_SUBBINARIES)` set with a
  presence guard, shims excluded.
- 🟡 **Security**: `environment: release` scoping contradiction — the runbook
  claimed "reachable only from the approved release environment," but `prerelease`
  runs on every push and cannot carry `environment: release` (concurrency
  deadlock). **Fixed + decided:** the false claim is corrected; the author chose to
  accept push-to-`main` as prerelease-signing authority, with **required review +
  branch protection on `main`** documented in `RELEASING.md` as the control
  equivalent to signing authority (secret is a repo/org secret; stable keeps its
  `approve-release` gate).
- 🟡 **Architecture/Code-Quality**: Transient Phase-4/5 window + old-function fate —
  **Fixed:** Phase 4 now *adds* `upload_and_verify_release` alongside the intact
  `upload_and_verify`; the old function is removed when Phase 5 repoints `_publish`.
- 🟡 **Architecture/Safety/Security**: Drift guard tracked `cli/verify/**` only —
  **Fixed:** widened to the `minisign-verify` pin + `cli/Cargo.lock` closure.
- 🟡 **Portability**: `llvm` tool named abstractly — **Fixed:** pin a concrete
  package + `llvm-readelf --version` preflight.
- 🔵 **Correctness**: Embedded-version assertion unrunnable cross-arch — **Fixed:**
  grep the version string from each binary rather than exec.
- 🔵 **Correctness**: `emit_manifest` omitted the secret-key param — **Fixed.**
- 🔵 **Code-Quality**: `SigningError` module placement — **Fixed:** `tasks/shared/errors.py`.

### Conscious Deferrals (documented, not fixed)

- 🟡 **Test-Coverage/Portability**: The three foreign-arch shims/launchers are never
  *executed* on their target OS/arch (root of trust for the bootstrap). Now called
  out in "What We're NOT Doing" as a deliberate deferral to consumer-side testing
  (future qemu/Rosetta smoke) — the highest-consequence residual coverage gap.
- 🟡 **Safety**: The single gate covers the GitHub publish but not the `git push`;
  the version-bump commit + tag advance before re-verify. Recovery entrypoint and
  orphaned-draft/tag cleanup are now documented in Migration Notes; a `push`
  re-order is future tightening.
- 🔵 **Security**: Prereleases auto-signed with the production key on every push —
  folded into the (a)/(b) author decision above.
- 🔵 **Code-Quality/Compatibility**: structured `llvm-readobj --elf-output-style=JSON`
  over text-scraping; bare-hex sha256 fixture assertion; prehash-algorithm-byte
  assertion — minor robustness suggestions left for implementation.

### Assessment

The plan is in materially good shape and implementation-ready: the
release-integrity and secret-handling foundations that drove the pass-1 REVISE are
now sound, the regressions the revised draft introduced have been corrected, and
the one open author decision (prerelease signing-secret scope) is resolved — accept
push-to-`main` as prerelease-signing authority, gated by required review + branch
protection on `main` (documented in `RELEASING.md`). The deferred
per-platform-execution gap is the item most worth revisiting before the first real
sub-binary (0168) ships.

---
*Review generated by /accelerator:review-plan*
