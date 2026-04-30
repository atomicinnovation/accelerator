---
date: "2026-05-01T01:30:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-30-meta-visualiser-phase-12-packaging-docs-and-release.md"
review_number: 1
verdict: REVISE
lenses: [architecture, code-quality, test-coverage, correctness, security, compatibility, safety, documentation]
review_pass: 4
status: complete
---

## Plan Review: Meta Visualiser — Phase 12: Packaging, docs, and release

**Verdict:** REVISE

The plan is fundamentally sound: it extends the existing invoke-task layer rather than introducing a parallel release mechanism, separates pure helpers from orchestration cleanly, and applies disciplined TDD throughout. The atomic-flow ordering, custom typed exceptions, and SHA-256 manifest verification are real strengths. However, two critical issues block implementation as-written: (1) the `dry_run=True` path in `release_binaries.build()` will silently overwrite the real `bin/checksums.json` with sentinel zeros — directly contradicting both the test and the documented manual-verification flow; and (2) `gh release create` is called with an unprefixed version string while `tasks/git.py:tag_version` and `launch-server.sh` both assume `v`-prefixed tags — the launcher will be unable to fetch any binary the first CI release publishes. Beyond these, twenty-seven major findings cluster around dry-run/test-mode coupling, Cargo.toml regex fragility, draft-release abandonment leaving published bad assets, prerelease cuts gaining write-token authority without an environment gate, and disproportionate README scope.

### Cross-Cutting Themes

- **`dry_run` semantics conflate testing with operator preview** (flagged by: architecture, code-quality, test-coverage, correctness, safety) — the parameter mutates real `bin/checksums.json`, the orchestration test passes via `fake_repo_tree` while the documented manual `--dry-run` invocation corrupts the working tree, and the pattern conflicts with the project's stated convention against dry-run UX on destructive ops.
- **Cargo.toml regex fragility** (flagged by: code-quality, test-coverage, correctness, security, architecture) — `^(version\s*=\s*)"[^"]*"` with `count=1` matches the first occurrence anywhere in the file, not anchored to the `[package]` section. Reader and writer share the same flaw, so coherence validation cannot detect the failure mode. `tomllib` is in stdlib for the pinned Python 3.14.4.
- **Verify-after-upload publishes bad releases** (flagged by: architecture, safety) — the plan removed the draft-release flow on the assumption that "next push supersedes," but a verify failure leaves a published tag + Release with bad assets indefinitely. Users on the broken pre-release version have no recovery path.
- **Pre-release auto-publishes with `contents: write` and no environment gate** (flagged by: architecture, security, safety) — every push to `main` gets repo-write authority and worldwide-distribution capability with no human gate, no kill switch, and no branch protection requirement documented.
- **Cleanup-on-failure missing from code samples** (flagged by: code-quality, correctness, test-coverage) — `update_checksums_json` test asserts try/finally cleanup but the canonical code block doesn't include it; implementers will copy the buggy shape and have to retrofit.

### Tradeoff Analysis

- **Safety/security vs. operational cadence** — A draft-release flow, environment gate on prerelease, kill-switch commit token, or build provenance attestation each add ceremony to a per-merge prerelease cadence the plan deliberately keeps frictionless. The plan's current position favours velocity; the security/safety lenses argue that the marginal cost is small relative to the trust this pipeline establishes for binary distribution. Recommendation: at minimum scope `permissions: contents: write` per-job (cheap), add a commit-message kill switch (cheap), and require branch protection on `main` (operational, not code).
- **dry-run convenience vs. project convention** — The user's documented preference is "no dry-run/preview/confirm UX on destructive ops; VCS revert is the recovery path." The current `dry_run` design adds operator-facing preview semantics that contradict this. Recommendation: drop `dry_run` from the public task signature and rely on `mocker.patch` for tests; if a real preview is needed it should be a separate task.

### Findings

#### Critical

- 🔴 **Correctness**: dry_run=True clobbers the real checksums.json
  **Location**: Phase 12.3, Section: The Python invoke task (lines 905-938, 977-978)
  `_CHECKSUMS` is bound to the real repo path, and `update_checksums_json(_CHECKSUMS, version, hashes)` is called unconditionally regardless of `dry_run`. The plan's manual-verification step explicitly invites running `invoke release-binaries.build --version 1.20.0-pre.5 --dry-run` against the real repo, which will silently overwrite `bin/checksums.json` with all-zero sentinel hashes. The pytest test passes only because it patches `_CHECKSUMS` via `fake_repo_tree`.

- 🔴 **Compatibility**: Tag-prefix mismatch between git tag (v<ver>) and gh release create (<ver>) breaks the launcher's asset URL
  **Location**: Phase 12.4 §2 and the existing `tasks/github.py:create_release`
  `tasks/git.py:tag_version` creates `v<version>` tags; `tasks/github.py:create_release` calls `gh release create "<version>"` without the `v` prefix; `launch-server.sh:145` constructs download URLs as `${RELEASES_URL_BASE}/v${PLUGIN_VERSION}/...`. With current call shape, the first CI release publishes at `/download/<ver>/...` while the launcher fetches from `/download/v<ver>/...` — every fresh `/accelerator:visualise` will fail. This is caught only by the irreversible Phase 12.7 smoke matrix.

#### Major

- 🟡 **Architecture**: No idempotency / concurrency guard for prerelease-per-push cadence
  **Location**: Phase 12.3 atomic flow + Phase 12.4 wiring
  Two pushes landing in close succession will race on `version.bump()`, tag-push, and asset upload, producing a Release whose assets don't match any tagged commit. No `concurrency:` group is specified in the workflow.

- 🟡 **Architecture**: Verify-after-upload places verification gate after irreversible side effects
  **Location**: Phase 12.3 steps 13-15
  By the time `verify_uploaded_asset` fires, the Release is public and users with the URL can already download bad binaries. The plan removed the draft flow on the assumption "next push supersedes," but the bad release stays on GitHub indefinitely.

- 🟡 **Architecture**: Prerelease job gains write permissions without an environment gate
  **Location**: Phase 12.4 — workflow permissions
  The trust-boundary shift from "release only" to "every push to main" deserves at least an audit-loggable Environment, even one without approvers, so the pipeline is disable-able via the GitHub UI.

- 🟡 **Architecture**: Conflating dry-run and test-mocking in the same parameter mixes architectural concerns
  **Location**: Phase 12.3 — `dry_run` parameter design
  Production code branches on test-only conditions (`hashes = ... if not dry_run else {p: '0'*64 for p in binaries}`); the tests verify the dry-run path, not the real path.

- 🟡 **Code Quality**: dry_run flag argument creates inconsistent semantics across helpers
  **Location**: Phase 12.3 — `build()` body
  Some helpers honour `dry_run` (skipping `cargo zigbuild`), others run unconditionally (`update_checksums_json`, `validate_version_coherence`). A maintainer running the documented `--dry-run` invocation corrupts the manifest.

- 🟡 **Code Quality**: Canonical `update_checksums_json` signature contradicts the atomic-cleanup claim
  **Location**: Phase 12.2 §2
  Code block shown lacks try/finally; test description requires it. Implementers reading the canonical sample will copy the buggy shape and only discover the gap when the test fails.

- 🟡 **Test Coverage**: Smoke test marked `#[ignore]` and only runs via opt-in mise task
  **Location**: Phase 12.5 — Mise integration
  `cargo test` alone silently skips the most valuable end-to-end test; only `mise run test:integration:binary-acquisition` invokes it via `-- --ignored`.

- 🟡 **Test Coverage**: Cross-compile correctness only verified by a one-shot manual `file` command
  **Location**: Phase 12.3 steps 3-4
  Three of four target architectures (everything except the maintainer's host) have zero automated coverage that the right binary was produced — only call-count assertions.

- 🟡 **Test Coverage**: No test exercises post-failure state (release left in place, partial uploads)
  **Location**: Phase 12.3 steps 13-15
  Partial-upload semantics (binary 3 of 4 fails — does upload continue? abort? leave half-uploaded assets?) are undocumented and untested. Idempotency of `upload_and_verify` against an existing Release is unverified.

- 🟡 **Test Coverage**: `update_checksums_json` runs for real even in dry-run, contradicting test invariant
  **Location**: Phase 12.3 §3 — `test_dry_run_creates_no_real_artefacts`
  Test passes via `fake_repo_tree` swap; a dry-run from the real CLI corrupts the real manifest. The test as designed cannot catch this bug.

- 🟡 **Test Coverage**: Cargo.toml regex writer has no malformed-input or workspace tests
  **Location**: Phase 12.4 §1
  Sterile fixture `'[package]\nname = "x"\nversion = "1.20.0"\n'` cannot expose ordering/section-anchor failure modes. No test for `[workspace.package]`, `[dependencies] foo = { version = "..." }`, missing `[package]`, etc.

- 🟡 **Correctness**: Tag created at version-bump commit with stale checksums.json violates the coherence invariant
  **Location**: Phase 12.4 §3 — `release()` post-release pre.0 bump
  After stable, the next-minor `*-pre.0` is bumped, committed, and tagged — but `release_binaries.build()` is not called for it. The tag points at a commit where `plugin.json`/`Cargo.toml` are at `1.21.0-pre.0` but `bin/checksums.json` still says `1.20.0`. The plan's claim that "validate_version_coherence passes at any tagged commit" is false here.

- 🟡 **Correctness**: Idempotency claim for re-running a crashed CI job is incorrect
  **Location**: Phase 12.2 §3 — `validate_version_coherence` docstring/description
  The plan claims the helper makes the release task idempotent, but `prerelease()` always calls `version.bump(PRE)` which always increments. "What We're NOT Doing" section confirms no retry. The two statements contradict each other.

- 🟡 **Correctness**: Cargo.toml version regex matches the first `version =` line, not necessarily [package].version
  **Location**: Phase 12.4 §1
  Cargo doesn't require `[package]` to come first; dependency tables and `[workspace.package]` contain `version = "..."` keys. The regex with `count=1, MULTILINE` is unanchored to the package section.

- 🟡 **Correctness**: Tmp file cleanup on failure documented in tests but missing from the helper code
  **Location**: Phase 12.2 §2
  Code block lacks try/finally; test description and parenthetical aside add it. Plan inconsistency between code shown and test contract.

- 🟡 **Security**: Workflow-wide `contents: write` over-privileges the `test` job
  **Location**: Phase 12.4 §5
  Workflow-scoped permissions apply to every job. A vulnerability in any test-time step (npm/cargo lockfile poisoning, malicious test asset) escalates to repo write access on push events.

- 🟡 **Security**: Pre-release publish is automatic, ungated, and self-pushing on every merge
  **Location**: Phase 12.1 Decision; Phase 12.4 §4
  No environment gate, no branch protection requirement, no required reviewers, no kill switch — a single compromised commit publishes a malicious binary that every visualiser user fetches and executes locally on first run.

- 🟡 **Security**: `verify_uploaded_asset` provides no defence against a compromised CI runner
  **Location**: Phase 12.3 step 14; Phase 12.2 §5
  The expected hash and the asset both originate from the same untrusted runner. The verify step catches GitHub-side corruption (rare) but not toolchain compromise or runner compromise (much higher likelihood). Build provenance attestation (SLSA L2 / `actions/attest-build-provenance`) would close this gap.

- 🟡 **Compatibility**: `mise.toml` postinstall hook does not accept a list value
  **Location**: Phase 12.1 §4
  The plan converts `postinstall` from a string to a list and notes a fallback verbally. If mise rejects the list syntax on the pinned version, `mise install` fails on every fresh checkout, blocking CI.

- 🟡 **Compatibility**: Existing `bin/checksums.json` version (1.19.0-pre.2) lags `plugin.json` (1.19.0-pre.4)
  **Location**: Phase 12.2 §3 + existing repo state
  `validate_version_coherence` raises until the first prerelease bump completes. Any debug/validation tooling run between Phase 12.2 merging and Phase 12.4 activating will fail.

- 🟡 **Compatibility**: `gh release download --output` semantics with `--pattern` may not behave as expected on gh 2.89.0
  **Location**: Phase 12.2 §5
  The combination `--pattern <exact-name> --output <single-path>` works only when the pattern matches exactly one asset. Future siblings (`<name>.sig`, etc.) could break verification silently.

- 🟡 **Safety**: checksums.json mutation occurs before commit with no rollback on subsequent failure
  **Location**: Phase 12.3 step 6
  Manifest mutation at step 6 with commit at step 9 means failures at any intervening step leave a real-hash manifest with no matching commit — not auto-recoverable on next CI run.

- 🟡 **Safety**: dry_run path mutates the real bin/checksums.json contrary to its stated guarantee
  **Location**: Phase 12.3 — `build()` body (duplicates the Critical Correctness finding from a safety perspective)

- 🟡 **Safety**: Verify-failure path leaves a published Release with bad assets and a tag pointing at it
  **Location**: Phase 12.3 step 15 + "Why no draft-release flow"
  Users pinned to or evaluating the broken prerelease have no recovery path. Either auto-delete on verify failure, or restore the draft flow (one extra `gh` call for real safety gain).

- 🟡 **Documentation**: 300-line README section is disproportionate to existing README style
  **Location**: Phase 12.6 §1
  Current README is ~393 lines total, terse and table-driven. A 300-line section dwarfs comparable skill sections (~30 lines each) and pushes the README past 700 lines.

- 🟡 **Documentation**: `dry_run` parameter has no documented home
  **Location**: Phase 12.6 + Phase 12.3
  No CONTRIBUTING content, no docstring coverage of dry-run semantics, no `--help` excerpt — maintainers debugging a failed CI release have to read source.

- 🟡 **Documentation**: No documented home for the new release pipeline (maintainer audience)
  **Location**: Plan as a whole
  Phase 12.6 is end-user only. The 16-step atomic flow, recovery procedures, and four-platform smoke matrix are all archived inside the plan; new maintainers will reverse-engineer from source.

#### Minor

- 🔵 **Architecture**: `verify_uploaded_asset` breaks the pure-helper boundary (Phase 12.2 §5) — module docstring promises "no I/O outside the strict scope"; the helper does subprocess + network + tempfile. Move to `tasks/release_binaries.py`.
- 🔵 **Architecture**: Cargo.toml version is parsed and written via regex in two separate places (Phase 12.2 + Phase 12.4) — centralise in `tasks/version.py` or use `tomllib`.
- 🔵 **Architecture**: Post-release `next-minor pre.0` bump shares a single CI job with the release itself (Phase 12.4 §3) — couples publishing the release with advancing the development line; decouple into separate steps.
- 🔵 **Architecture**: Smoke test boundaries depend on `ACCELERATOR_VISUALISER_SKILL_ROOT` (Phase 12.5) — once the test exists, that env var becomes part of the architectural surface; document the test-seam contract.
- 🔵 **Code Quality**: Regex-based Cargo.toml writer is fragile and section-blind — use `tomllib` (stdlib in 3.11+, already pinned at 3.14.4).
- 🔵 **Code Quality**: Unused `prerelease` parameter in `upload_and_verify` is a flag-argument code smell (Phase 12.3) — remove it.
- 🔵 **Code Quality**: Tuple-of-dicts return type from `build()` invites primitive obsession (Phase 12.3) — introduce a small `BuildArtefacts` dataclass.
- 🔵 **Code Quality**: Undefined `_DEFAULT_REPO_ROOT` referenced (Phase 12.2 §3) — add module-level constant or make `repo_root` required.
- 🔵 **Code Quality**: Pre-flight checks `jq` and `sha256sum/shasum` despite no-bash discipline (Phase 12.3 step 1.1) — drop dead validation; helpers use Python `hashlib` and `json`.
- 🔵 **Code Quality**: Test uses `git status` in a `jj`-based repo (Phase 12.3 §3) — implementation note and success criterion disagree; project uses jj.
- 🔵 **Code Quality**: File handles opened without context managers leak on error (Phase 12.4 §1) — use `Path.read_text()` / `Path.write_text()`.
- 🔵 **Test Coverage**: Tests assert on command strings, not behaviour — fragile to flag refactors.
- 🔵 **Test Coverage**: Smoke test does not exercise the failure paths it claims (sentinel rejection, mismatched SHA, 404).
- 🔵 **Test Coverage**: Atomic-write test description leaves the helper's actual try/finally implicit.
- 🔵 **Test Coverage**: Manual platform matrix accepts partial coverage as the final acceptance gate (3 of 4 platforms could ship untested).
- 🔵 **Test Coverage**: End-to-end release flow tests claim to verify ordering but mock all dependencies — couples to call sequence rather than the manifest-staged-before-commit invariant.
- 🔵 **Test Coverage**: Asset verification mocks `subprocess.run` but does not test the `gh` argument shape.
- 🔵 **Test Coverage**: Coherence helper not tested against real plugin file edge cases (BOM, whitespace, non-string version values).
- 🔵 **Correctness**: No early validation that `--version` matches plugin.json before disk mutations.
- 🔵 **Correctness**: Mid-flight failures between push and `create_release` leave a published tag with no Release.
- 🔵 **Correctness**: `git push origin HEAD --tags` pushes ALL local tags, not just the new one.
- 🔵 **Correctness**: Strip step is hand-waved — host `strip` on Linux runner cannot strip Mach-O.
- 🔵 **Correctness**: Same regex risk on read side; reader and writer must agree.
- 🔵 **Security**: Unanchored Cargo.toml regex can be steered by `[dependencies]`-section content (latent supply-chain).
- 🔵 **Security**: `ACCELERATOR_VISUALISER_INSECURE_DOWNLOAD` bypass remains for plaintext mirrors — README must require HTTPS for mirrors.
- 🔵 **Security**: `gh release create` invocation uses an f-string with quoted version — switch to argv form for consistency with `verify_uploaded_asset`.
- 🔵 **Security**: No revoke/recall plan for a malicious prerelease that reaches users — document an incident-response playbook.
- 🔵 **Compatibility**: `semver.Version.parse` strict-semver semantics — pin `semver>=3.0.4,<4` in pyproject.toml; or use truthy check for prerelease.
- 🔵 **Compatibility**: `cargo zigbuild --strip` support varies by target on cargo-zigbuild 0.19.5 — pin to a single strategy and validate via `file`.
- 🔵 **Compatibility**: Docker `arm64v8/ubuntu:22.04` smoke test on darwin-arm64 host requires QEMU emulation — document host requirements.
- 🔵 **Compatibility**: First version-bump commit jumps Cargo.toml from 0.1.0 to plugin.json's version — align in a separate prep commit, or document.
- 🔵 **Safety**: Plan does not specify whether a partially-completed CI job leaves the remote tag/commit pushed — add a "Failure modes by step" table.
- 🔵 **Safety**: Pre-release cuts have no kill switch (e.g., `[skip prerelease]` commit token).
- 🔵 **Safety**: Reverting Phase 12.4 does not undo the published GitHub Release or pushed tag — expand recovery instructions with explicit cleanup sequence.
- 🔵 **Safety**: Stripping symbols removes diagnostic information without preserving debug artefacts.
- 🔵 **Safety**: Heavy reliance on `dry_run` for safety contradicts the project's stated convention against dry-run UX on destructive ops.
- 🔵 **Safety**: External `gh` subprocess calls have no timeout — flaky GitHub API can hang CI for 6 hours.
- 🔵 **Documentation**: CHANGELOG entry omits binary-distribution mechanics that affect upgraders (network requirement, mirror env var).
- 🔵 **Documentation**: Migration Notes omits CLI wrapper installation and config-precedence interaction.
- 🔵 **Documentation**: `ACCELERATOR_VISUALISER_SKILL_ROOT` introduced silently — decide public vs. test-only and name accordingly.
- 🔵 **Documentation**: CHANGELOG version header committed in plan as `1.20.0` — repo is already at `1.21.0-pre.1`; use a placeholder.
- 🔵 **Documentation**: `grep -c '/accelerator:visualise' README.md` is a weak completeness check.
- 🔵 **Documentation**: Cargo.toml regex contract is undocumented across two modules — add reader/writer cross-link comment.

### Strengths

- ✅ Extends the existing invoke-task layer rather than introducing a parallel release mechanism; helpers, orchestration, and CI wiring are well-separated and each phase produces a testable deliverable.
- ✅ Custom typed exceptions (`ReleaseHelperError`, `VersionCoherenceError`, `AssetVerificationError`, `InvalidVersionError`) give precise failure shapes for `pytest.raises` and orchestrator handling.
- ✅ Pre-release detection is centralised in `is_prerelease_version` — single point of truth that `tasks/github.py` and `tasks/release.py` both consume.
- ✅ Atomic-write semantics for `checksums.json` (`.tmp` + `os.replace`) and version-coherence validation give a recoverable invariant on tagged commits.
- ✅ The decision to use Python invoke tasks throughout instead of bash scripts is consistent with the existing `tasks/` layer and improves testability.
- ✅ The plan explicitly declines a draft-release flow on the basis that CI re-cuts supersede failures — an acknowledged tradeoff rather than implicit (though see findings on whether the tradeoff actually holds for users).
- ✅ The smoke test (Phase 12.5) targets the full download/verify/launch boundary with an in-process HTTP mirror — many projects skip this.
- ✅ Pull requests do not trigger the prerelease job (`if: github.event_name == 'push'`), so untrusted PR contributors cannot reach the privileged token path.
- ✅ macOS notarisation skip is correctly reasoned: binaries are spawned via `nohup` after SHA-256 verification, never opened by Finder/LaunchServices, so Gatekeeper would not have run anyway.
- ✅ Asset naming convention (`accelerator-visualiser-darwin-arm64`, etc.) matches exactly what `launch-server.sh` constructs from `uname` mapping.
- ✅ musl static linking for both Linux targets is the right choice for distro portability.
- ✅ Pinning `zig` and `cargo-zigbuild` in `mise.toml` provides reproducibility across CI runners and dev hosts.
- ✅ Helper docstrings explain *why* rather than *what* (e.g., 64-KiB chunking "to keep memory bounded for the ~8 MB release binaries"), matching project documentation norms.
- ✅ The CHANGELOG entry deliberately consolidates Phases 5-11 deliverables into a single user-facing summary rather than dumping all sub-phase changes.
- ✅ Pytest layout (`tests/tasks/`) explicitly avoids colliding with the existing `tasks/test/` invoke sub-collection.

### Recommended Changes

Ordered roughly by impact. Each addresses one or more findings.

1. **Fix the dry-run side-effect bug** (addresses: Critical Correctness, Major Test Coverage, Major Safety, Major Code Quality, Major Architecture)
   In `release_binaries.build()`, gate the `update_checksums_json` and `validate_version_coherence` calls behind `if not dry_run:`, OR accept a `manifest_path: Path` parameter so tests pass a tempdir explicitly. Add a regression test that runs `dry_run=True` against the real `_REPO_ROOT` (not `fake_repo_tree`) and asserts `bin/checksums.json` is byte-identical before and after. Consider dropping `dry_run` from the public task signature entirely and rely on `mocker.patch` for tests — this aligns with the project's documented convention against dry-run UX.

2. **Fix the tag-prefix mismatch** (addresses: Critical Compatibility)
   Update `tasks/github.py:create_release` to pass `f"v{resolved_version}"` as both tag and title to `gh release create`. Update `verify_uploaded_asset` callers and Phase 12.7 success criteria's `gh release view` invocations to use `v`-prefixed tags. Add a Phase 12.2 test that asserts the call shape includes the `v` prefix. Verify the existing `tasks/git.py:tag_version` actually produces `v`-prefixed tags before depending on it.

3. **Replace the Cargo.toml regex with `tomllib`** (addresses: Major Code Quality, Major Test Coverage, Major Correctness, Minor Security, Minor Architecture)
   Use `tomllib` (stdlib in Python 3.11+, repo pins 3.14.4) for reads; use a TOML-aware writer such as `tomlkit`/`tomli-w` if comment preservation matters, or a regex anchored to the `[package]` section header otherwise. Have `release_helpers._read_cargo_toml_version` import the reader from `tasks/version.py` rather than reimplementing the regex. Add tests for `[workspace.package]` precedence, dependency-table `version = "..."` keys, missing `[package]` section, and BOM/whitespace edge cases.

4. **Fix `update_checksums_json` canonical sample** (addresses: Major Code Quality, Major Correctness, Minor Test Coverage)
   Update the code block in Phase 12.2 §2 to include the try/finally cleanup inline, matching the test description. The same applies to verifying the test asserts both `(a)` original manifest content unchanged and `(b)` `.tmp` sibling does not remain on disk.

5. **Restore a draft-release flow OR add automatic verify-failure cleanup** (addresses: Major Architecture, Major Safety)
   Either: (a) on verify failure, automatically `gh release delete` the bad release (the tag remains for forensics, the next CI run cuts a fresh `*-pre.N+1`); or (b) reinstate the draft flow — one extra `gh release create --draft` call followed by `gh release edit --draft=false` after verify passes. Pick a strategy and document the recovery posture explicitly.

6. **Tighten workflow permissions and add a kill switch** (addresses: Major Security, Major Architecture, Minor Safety)
   Set `permissions: {}` (or `contents: read`) at workflow level. Add `permissions: contents: write` only inside the `prerelease` and `release` jobs. Add a `prerelease` GitHub Environment (no approvers needed) for audit/disable capability. Honour a `[skip prerelease]` commit-message token in the workflow (analogous to `[skip ci]`) for fast incident response. Document a required-branch-protection-on-main expectation in the plan.

7. **Add concurrency group to prerelease job** (addresses: Major Architecture)
   Add `concurrency: { group: prerelease, cancel-in-progress: false }` to the prerelease job in `.github/workflows/main.yml` so subsequent pushes queue rather than race.

8. **Fix the post-release coherence-invariant violation** (addresses: Major Correctness)
   The `release()` flow's final `next-minor pre.0` bump → commit → tag should either also call `release_binaries.build()` (with sentinel hashes is acceptable) so the tag is coherent, OR drop the tag for that intermediate bump (it serves no release purpose), OR document explicitly that `validate_version_coherence` does not hold at all tagged commits.

9. **Strengthen automated cross-arch coverage** (addresses: Major Test Coverage, Minor Correctness)
   After `release_binaries.build` runs in non-dry mode (CI), assert each staged binary's filename matches its declared platform tuple AND its file magic bytes match the expected ELF/Mach-O signature (pure Python: `open(path, 'rb').read(4)` against known signatures). Add a partial-upload-failure pytest case and a verify-short-circuit-on-first-mismatch case.

10. **Address the prerelease/test job permission split** (addresses: Major Security)
    Same change as item 6 — calling out separately because per-job permission scoping is the single most defensive change in the plan.

11. **Reconsider the `prerelease` parameter and tuple return on `build()`/`upload_and_verify`** (addresses: Minor Code Quality x2)
    Drop the unused `prerelease` parameter from `upload_and_verify`. Replace the `(binaries, hashes)` tuple return with a `@dataclass(frozen=True) class BuildArtefacts: version, binaries, hashes`.

12. **Move `verify_uploaded_asset` out of the helpers module** (addresses: Minor Architecture)
    The helper performs subprocess + network + tempfile I/O, which violates the module's stated "no I/O outside the strict scope" docstring. Relocate to `tasks/release_binaries.py` as a private helper.

13. **Pin `semver` major version in pyproject.toml** (addresses: Minor Compatibility)
    Pin `semver>=3.0.4,<4` and replace `parsed.prerelease is not None` with `if parsed.prerelease:` for cross-version safety.

14. **Add subprocess timeouts** (addresses: Minor Safety)
    Add `timeout=120` (or similar) to `subprocess.run` in `verify_uploaded_asset`; pass equivalent timeouts to `context.run` for upload/create commands. On timeout, raise `AssetVerificationError`.

15. **Right-size and re-target Phase 12.6 documentation** (addresses: Major Documentation x3, Minor Documentation x6)
    Cap the README section at ~50-80 lines (matching adjacent skill sections); push detailed customisation/mirror docs to a `docs/visualiser.md` or the `/accelerator:visualise` SKILL.md. Add a `RELEASING.md` covering the prerelease/release flow, recovery procedures, and four-platform smoke matrix. Document `dry_run` semantics in the task docstring (or remove `dry_run` per item 1). Use a `<next-version>` placeholder in the CHANGELOG entry header. Decide whether `ACCELERATOR_VISUALISER_SKILL_ROOT` is public or test-only and name accordingly. Strengthen the Phase 12.6 success-criteria check to grep for each customisation env var. Add reader/writer cross-link comments for the Cargo.toml regex helpers (or moot via item 3).

16. **Document failure modes by step** (addresses: Minor Safety x2, Minor Correctness)
    Add a "Failure modes by step" subsection to Phase 12.3 listing each step boundary, the resulting remote state, and whether it's auto-recoverable on next push. Cover: build fails before commit (working-tree dirty on dev host); commit fails between bump and push (tag missing); push fails between commit and create-release (tag pushed, no Release); create-release fails (tag pushed, no Release); upload fails partially (Release with some assets); verify fails (Release with mismatched assets). Expand Phase 12.7's revert instructions with explicit `gh release delete` + `git push --delete origin <tag>` cleanup sequence.

17. **Pin or document edge-cases in cross-compile/strip and mise postinstall list syntax** (addresses: Major Compatibility, Minor Compatibility x2)
    Validate the mise `postinstall` list syntax against the pinned mise version before declaring 12.1 success; prefer a chained shell form if list syntax is unsupported. Pick a single strip strategy (cargo-zigbuild's `--strip` flag or `llvm-strip`) and validate output via `file` in CI. Document QEMU requirement for linux-arm64 smoke testing on non-arm64 hosts.

18. **Optionally add build provenance for real supply-chain defence** (addresses: Major Security)
    Add `actions/attest-build-provenance` (or sigstore keyless signing) to the workflow with `id-token: write` + `attestations: write`. Update the launcher (or document a verify-provenance escape hatch) so users can independently verify a binary was produced from a specific commit on a trusted workflow. Marked optional because it's a meaningful scope extension; flagged because the current `verify_uploaded_asset` is misleading defence.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan extends an established invoke-task layer rather than introducing a parallel release mechanism, which is architecturally sound: helpers, orchestration, and CI wiring are well-separated and each phase produces a testable deliverable. However, several non-trivial coupling and resilience concerns appear in the atomic flow ordering, the prerelease-on-every-merge cadence, and the workflow's permission/concurrency model. The plan also concentrates an unusual number of responsibilities on the existing prerelease job (commit, tag, push, build, release-create, upload, verify) without addressing the failure-mode interactions between those stages.

**Strengths**: Clear separation between pure helpers, orchestration, and existing flow integration; functional core / imperative shell boundary respected; pre-release detection centralised; atomic-write semantics; explicit decline of draft-release flow as acknowledged tradeoff; smoke test targets the full download/verify/launch boundary.

**Findings**: 4 major (concurrency, verify-after-upload, no environment gate, dry_run conflation), 4 minor (verify_uploaded_asset boundary, Cargo.toml regex duplication, post-release single-job coupling, smoke-test env var contract).

### Code Quality

**Summary**: Strong testability orientation with TDD via pytest, custom typed exceptions, pure helpers; cleanly extends the existing invoke-task pattern. Several issues: the dry_run flag-argument creates inconsistent semantics across helpers, the canonical helper signatures shown contradict the prose around atomicity and cleanup, primitive-obsession in inter-task data passing, and a fragile regex-based Cargo.toml writer.

**Strengths**: Custom exception hierarchy gives precise failure shapes; helpers are pure with explicit dependency injection; clear separation of concerns; atomic-write idiom; module signature laid out up-front; pytest layout avoids collision with existing test directory.

**Findings**: 2 major (dry_run inconsistent semantics, canonical signature contradicts atomic-cleanup claim), 7 minor (regex fragility, unused parameter, primitive obsession, undefined constant, dead pre-flight checks, jj/git inconsistency, file handle leaks).

### Test Coverage

**Summary**: Strong commitment to TDD with a clean three-tier strategy (pytest unit, pytest dry-run integration, cargo ignored smoke) and explicit RED-first ordering. Coverage of helpers is generally thorough, but several risk areas are under-tested: cross-compile/strip step has no real-binary verification beyond a one-shot manual check, the smoke test is opt-in (`#[ignore]`), and important edge cases in orchestration (partial-failure recovery, retry idempotency, atomicity properties) lack tests.

**Strengths**: Clear RED-first discipline; typed exception design enables precise `pytest.raises`; dry_run parameter pragmatic for fast deterministic tests; atomic-write pytest case explicitly tests failure recovery; good edge-case spread for `is_prerelease_version`; smoke test stands up real axum mirror serving real binary; shared `fake_repo_tree` fixture.

**Findings**: 5 major (smoke test ignore, manual `file` check insufficient, no post-failure tests, dry-run real-mutation contradiction, Cargo.toml regex untested edges), 7 minor (string-based assertions, smoke failure-path gaps, atomicity test implicitness, partial platform matrix, mocked-ordering coupling, gh argument shape, plugin-file edge cases).

### Correctness

**Summary**: The plan's logic is broadly sound for the happy CI path, but several state-coherence and ordering invariants don't hold in the failure / intermediate-commit paths it claims to support. The most serious concrete bug is that the binary-build task writes to the real `bin/checksums.json` even with `dry_run=True`, contradicting the plan's own claim that dry-run is side-effect-free. Several invariants (validate_version_coherence holds at every tagged commit; idempotent re-runs of failed CI jobs) are stated but provably false given the wiring proposed in Phase 12.4.

**Strengths**: Atomic-write of checksums.json correct on POSIX; 16-step flow correctly stages the manifest into the version-bump commit; pre-release detection delegated to semver parser; verification re-downloads and re-hashes; typed exceptions.

**Findings**: 1 critical (dry_run=True clobbers real checksums.json), 4 major (post-release tag coherence violation, false idempotency claim, regex section-blindness, missing try/finally), 5 minor (no early --version validation, mid-flight push/create-release gap, --tags pushes all tags, hand-waved strip step, reader/writer regex risk).

### Security

**Summary**: The plan inherits a sound trust model from existing infrastructure: PRs do not trigger prerelease/release jobs, the macOS no-notarisation decision is justified, and the SHA-256 manifest provides genuine tamper-evidence. However, defence-in-depth is thin: `permissions: contents: write` is workflow-wide rather than job-scoped; the prerelease publish path has no environment gate or branch protection; Cargo.toml mutation via unanchored regex; asset re-verification only catches GitHub-side corruption, not a compromised CI runner that produced a malicious binary in the same job that wrote the manifest.

**Strengths**: PRs cannot reach the privileged path; macOS skip correctly reasoned; manifest verification real (committed SHA-256, launcher rejects on mismatch and on sentinel); curl flags enforce HTTPS/TLS/redirects/size-limit; subprocess argv form avoids shell injection; release retains environment approval gate.

**Findings**: 3 major (workflow-wide write permission, ungated auto-publish, runner-compromise blind spot), 4 minor (regex-steerable supply-chain, plaintext mirror bypass, f-string in create_release, no revoke/recall plan).

### Compatibility

**Summary**: The plan's compatibility story is mostly coherent — four target triples align with the launcher's `${OS}-${ARCH}` asset-name convention, musl static linking is appropriately distro-portable, and the no-code-signing decision is defensible. However, there is a critical incompatibility between the git tag prefix (`v<version>`) and the unprefixed argument passed to `gh release create "<version>"`, which the plan inherits unchanged. The asset URL constructed by `launch-server.sh` requires `v`-prefixed tags, so the existing call shape will produce releases that the launcher cannot fetch from.

**Strengths**: Asset-naming matches launcher convention exactly; musl static linking right for distro portability; macOS code-signing skip correctly reasoned (curl-downloaded files don't acquire quarantine xattr); is_prerelease_version reuses same semver lib as version.bump; mise pinning provides reproducibility.

**Findings**: 1 critical (tag-prefix mismatch breaks launcher URL), 3 major (mise list syntax, existing checksums.json version drift, gh release download --pattern/--output ambiguity), 4 minor (semver pin laxity, strip target variance, QEMU host requirement, Cargo.toml major-version leap).

### Safety

**Summary**: The plan acknowledges the irreversibility of CI-cut releases but leaves several safety gaps around partial-failure recovery, working-tree consistency, and dry-run isolation. The blast radius of a single CI release failure is bounded (next prerelease supersedes) but the intermediate states the repo can be left in are under-specified, and the dry_run code path as written touches real repo files. Given the project is a developer tool with low-stakes blast radius (no user data, no production traffic), most issues are operational rather than catastrophic.

**Strengths**: Atomic flow ordering explicit; tmp-file + os.replace + try/finally; verify_uploaded_asset re-downloads; validate_version_coherence enables idempotent re-runs after crash; release environment provides manual approval gate; pre-release recovery proportional to project criticality; workflow permissions scoped to contents: write rather than broader default.

**Findings**: 3 major (pre-commit manifest mutation no rollback, dry_run real-file mutation, verify-failure leaves bad release published), 6 minor (failure-mode states unspecified, no kill switch, revert doesn't undo published release, strip removes diagnostics, dry_run conflicts with project convention, no subprocess timeouts).

### Documentation

**Summary**: The documentation phase covers the right essential topics — what the visualiser is, launch commands, first-run download, customisation hooks, privacy/security — and the CHANGELOG entry consolidates Phases 5-11 cleanly. However, the planned ~300-line README section is disproportionate to the ~393-line existing README (terse and table-driven), the migration notes section omits CLI-wrapper PATH symlinking and `visualiser.binary` precedence semantics, and the plan provides no guidance on where dev maintainers learn about the new release pipeline. Several pieces of cross-referenced documentation (the `dry_run` switch, the Cargo.toml/version.py contract, `ACCELERATOR_VISUALISER_SKILL_ROOT`) have no documented home.

**Strengths**: Phase 12.6 enumerates concrete audience-appropriate content; CHANGELOG consolidation appropriate for feature launch; pre-release policy cross-referenced in two places; helper docstrings explain why; custom exception types self-documenting; Migration Notes addresses upgrade behaviour.

**Findings**: 3 major (300-line README disproportionate, dry_run undocumented, no maintainer release-pipeline doc), 6 minor (CHANGELOG omits binary-distribution mechanics, Migration Notes omits CLI wrapper, SKILL_ROOT introduced silently, hardcoded version in CHANGELOG header, weak grep success-check, undocumented Cargo.toml regex contract).

---

## Re-Review (Pass 2) — 2026-04-30

**Verdict:** REVISE

The iteration successfully resolved both Critical findings from the initial review (dry_run real-file mutation; tag-prefix mismatch) and the great majority of Major findings. The plan is structurally much stronger: draft-release atomicity, per-job permissions, SLSA build provenance, kill-switch Environment, tomllib-based Cargo.toml writer, dataclass return, failure-modes table, and explicit recovery procedures all land cleanly. However, the iteration introduced one new **Critical** correctness bug and several new Major architectural and definitional gaps that block implementation as currently written. A second revision pass is required before the plan can be shipped.

### Previously Identified Issues

#### Critical (initial review)
- 🔴 **Correctness**: dry_run=True clobbers the real checksums.json — **Resolved** (dry_run removed entirely; tests use mocker.patch).
- 🔴 **Compatibility**: Tag-prefix mismatch breaks launcher URL — **Resolved** (`f"v{resolved_version}"` flows through every gh call; new test locks the contract).

#### Major (initial review) — 27 total
- 🟡 Architecture: No concurrency guard for prerelease — **Resolved** (`concurrency: { group: prerelease-${{ github.ref }} }` added).
- 🟡 Architecture: Verify-after-upload places gate after irreversible side effects — **Resolved** (draft flow restored; auto-cleanup on failure; nothing public until verify passes).
- 🟡 Architecture: Prerelease ungated — **Partially resolved** (Environment kill switch added; new re-review finding flags it doesn't halt in-flight runs).
- 🟡 Architecture: dry_run conflation — **Resolved**.
- 🟡 Code Quality: dry_run inconsistent semantics — **Resolved**.
- 🟡 Code Quality: update_checksums_json signature contradicted atomicity claim — **Resolved** (try/except inline).
- 🟡 Test Coverage: Smoke test #[ignore] silent-skip — **Partially resolved** (documented; still flagged in re-review as needing structural enforcement).
- 🟡 Test Coverage: Cross-compile correctness only manual — **Resolved** (magic-byte assertions added).
- 🟡 Test Coverage: No post-failure-state tests — **Mostly resolved** (draft-flow failure tests added; new gaps around cleanup-failure cases).
- 🟡 Test Coverage: dry_run real-file mutation contradicts test invariant — **Resolved**.
- 🟡 Test Coverage: Cargo.toml regex untested edges — **Resolved** (BOM, workspace, dependency table tests).
- 🟡 Correctness: Tag at version-bump commit with stale checksums.json — **Resolved** for the post-release pre.0 (full rebuild now), **but a new Critical surfaces** at the pre-flight check timing.
- 🟡 Correctness: Idempotency claim incorrect — **Resolved** (claim removed).
- 🟡 Correctness: Cargo.toml regex matches first version line — **Resolved** (tomllib).
- 🟡 Correctness: Tmp file cleanup missing in code sample — **Resolved**.
- 🟡 Security: Workflow-wide contents:write — **Resolved** (per-job permissions).
- 🟡 Security: Pre-release ungated — **Partially resolved** (Environment + SLSA attestation; in-flight halt gap remains).
- 🟡 Security: verify_uploaded_asset no defence vs runner compromise — **Resolved** (SLSA attestation + opt-in launcher verification).
- 🟡 Compatibility: mise postinstall list-vs-string — **Resolved** (validation guidance + chained-shell fallback).
- 🟡 Compatibility: Existing checksums.json version drift — **Resolved** (Phase 12.1 §4a alignment commit).
- 🟡 Compatibility: gh release download --pattern + --output — **Still present** (re-review flags `--clobber` doesn't address pattern-matches-multiple-files semantics).
- 🟡 Safety: Pre-commit manifest mutation no rollback — **Resolved** (auto-cleanup wrapper).
- 🟡 Safety: dry_run mutates real file — **Resolved**.
- 🟡 Safety: Verify-failure leaves bad release published — **Resolved** (draft flow).
- 🟡 Documentation: 300-line README disproportionate — **Resolved** (50–80 line target).
- 🟡 Documentation: dry_run undocumented — **Resolved** (eliminated).
- 🟡 Documentation: No maintainer release-pipeline doc — **Resolved** (RELEASING.md added).

#### Minor (initial review) — 43 total
The vast majority of Minor findings are resolved or partially resolved. Notable still-open items:
- 🔵 Correctness: `git push origin HEAD --tags` pushes ALL local tags — **Still present** (not addressed in iteration).
- 🔵 Safety: Stripping symbols removes diagnostic info, no debug archive — **Still present** (re-review repeats this finding).
- 🔵 Test Coverage: Manual platform matrix accepts partial coverage — **Still present** (re-review escalates this to Major and proposes QEMU + macos-13/14 runner coverage).
- 🔵 Test Coverage: Tests assert command strings vs. behaviour — **Still present** (re-review escalates).
- 🔵 Test Coverage: End-to-end mocked sequences couple to call ordering — **Still present** (re-review proposes index-based ordering assertions).

### New Issues Introduced

#### Critical
- 🔴 **Correctness**: Pre-flight `validate_version_coherence` will fail every CI run because `version.bump` updates plugin.json and Cargo.toml but **does not** update `bin/checksums.json` — checksums.json carries the old version while the other two carry the new version, and the pre-flight (step 1.4) sees the inconsistency before `release_binaries.build` step 6 has a chance to update it. This affects every prerelease and stable cut. **Fix**: either extend `version.write` to also rewrite checksums.json's `version` field (preserving hashes or resetting to sentinels), or split `validate_version_coherence` into a pre-flight `(plugin, cargo)`-only check and a post-update `(plugin, cargo, checksums)` check.

#### Major
- 🟡 **Architecture / Code Quality / Correctness** (3 lenses): Workflow YAML references four release-task halves (`stable-prepare`, `stable-publish`, `post-stable-prepare`, `post-stable-publish`) but Phase 12.4 §3a only sketches `release_prepare` / `release_finalize`. The four-task split is undefined — implementers will have a workflow YAML calling mise tasks that don't exist.
- 🟡 **Architecture / Code Quality** (2 lenses): `BuildArtefacts.from_disk` underspecified — does it strip the `sha256:` prefix? How does it derive paths from `_TARGETS`? An implementer following the plan literally will hit a verify failure on the first finalize call. Plus: the cross-task contract (prepare writes, finalize reads) is not exercised by any listed test.
- 🟡 **Architecture / Code Quality** (2 lenses): Two parallel orchestration paths (single-call `prerelease()/release()` for local-dev + split `*_prepare/*_finalize` for CI) create silent-drift risk. Plan acknowledges with documentation, not code structure.
- 🟡 **Code Quality**: `@task` decorator misapplied to `build()` (returns `BuildArtefacts`) and `upload_and_verify()` (takes `BuildArtefacts` — invoke can't construct one from CLI argv). The mise wrappers `release:binaries:upload` are effectively dead surface.
- 🟡 **Architecture**: Auto-cut `*-pre.0` releases per stable cut have no defined lifecycle/retention policy, may mask dogfoodable changes if downstream tools pick "latest prerelease" naively.
- 🟡 **Security**: Hardcoded `--owner atomic-innovation` in launcher's `gh attestation verify` call makes provenance verification fork-hostile and potentially defeatable if a malicious fork ships a binary byte-equal to a legitimate upstream binary.
- 🟡 **Security / Safety** (2 lenses): Prerelease "kill switch" Environment doesn't halt in-flight runs — branch-policy gates evaluate at job-start, not mid-job. Operators relying on the kill switch for incident response will see in-flight publishes complete despite the toggle.
- 🟡 **Compatibility**: `actions/attest-build-provenance@v2` is a floating major tag; mise.toml pins exact versions for everything else. Inconsistent reproducibility posture.
- 🟡 **Compatibility**: `gh attestation verify` minimum version not stated — was added in gh 2.49.0, has had behaviour changes since. Both CI (`gh = "2.89.0"` is fine) and end-user systems (whatever gh is on PATH) need this assertion.
- 🟡 **Compatibility**: `gh release download --pattern + --output + --clobber` semantics still risky if pattern matches more than one asset (e.g., future `.intoto.jsonl` sidecar files).
- 🟡 **Test Coverage**: No tests for the prepare/finalize split, `BuildArtefacts.from_disk` reconstruction, cleanup wrapper failure cases (`KeyboardInterrupt`, cleanup-itself-failing), or post-release pre.0 path.
- 🟡 **Test Coverage**: Three-of-four-platform partial coverage retained as final acceptance gate — re-review escalates to Major and proposes CI matrix coverage via QEMU + macOS-13/14 hosted runners.
- 🟡 **Correctness / Safety / Security** (3 lenses): `try / except BaseException` cleanup runs `gh release delete` during `KeyboardInterrupt` / `SystemExit` / `GeneratorExit` — interferes with graceful shutdown and may produce inconsistent cleanup states on Ctrl-C.
- 🟡 **Documentation**: New mise tasks (six of them) referenced everywhere but defined and documented nowhere — implementers cannot wire CI without reverse-engineering the convention.
- 🟡 **Documentation**: Provenance opt-in lacks operator-decision guidance — README documents the *what* but not the *should-I-enable-it* threat-model context.
- 🟡 **Safety**: Prepare → attest → finalize state handoff between mise invocations (separate Python processes) relies on uncommitted working-tree state. Runner preemption between halves leaves an orphan attestation in the transparency log without a corresponding public asset.
- 🟡 **Safety**: Stable + post-release pre.0 run within a single CI workflow without cross-step rollback. If the post-stable pre.0 publish fails after the stable publish succeeded, an operator is left with a published stable plus a pushed `*-pre.0` commit that has no matching Release.

#### Minor (selected new findings)
- 🔵 **Code Quality**: Duplicate `_DEFAULT_REPO_ROOT` (release_helpers.py) vs `_REPO_ROOT` (release_binaries.py) — should consolidate to one shared constant.
- 🔵 **Code Quality**: Cleanup logic inline in `upload_and_verify` rather than extracted; will be copy-pasted across the four release-task halves.
- 🔵 **Code Quality**: `release_helpers._read_cargo_toml_version` imports from `tasks.version`, pulling invoke into the helper layer. Should extract to a `tasks/cargo_toml.py` module that depends on neither.
- 🔵 **Architecture**: Concurrency group `prerelease-${{ github.ref }}` shared between prerelease and release jobs — name masks intent; queueing behaviour during stable approval may surprise.
- 🔵 **Security**: Auto-cleanup destroys forensic evidence on `AssetVerificationError` — a SHA mismatch is exactly the supply-chain tampering signal that should be preserved for triage.
- 🔵 **Security**: `/tmp/visualiser-attest.err` in launcher provenance hook is symlink-attack surface on multi-user hosts; should use `mktemp`.
- 🔵 **Safety**: Pre-flight does not check working-tree cleanliness (only relevant for local-dev runs, but the local task is documented as supported).
- 🔵 **Safety**: Cleanup `context.run` lacks its own timeout; under network partition the cleanup can hang indefinitely, defeating the auto-cleanup wrapper's purpose.
- 🔵 **Documentation**: Out-of-band `gh attestation verify` invocation not documented for users without the launcher flag.
- 🔵 **Documentation**: Provenance-failure error embeds raw stderr inside a single sentence — hard to read and fragile if `die_json` doesn't escape embedded newlines/quotes.
- 🔵 **Compatibility**: `tomlkit` dependency added without a version constraint (whereas `semver` is correctly pinned) — version drift could break round-trip-preservation tests.
- 🔵 **Test Coverage**: Provenance opt-in path has tests for skip / fail / missing-gh but no positive-path test (flag set + valid attestation → launcher proceeds normally).

### Assessment

The iteration moved the plan a long way forward. Both Criticals from the initial review are gone; about 24 of 27 Majors are fully resolved; the remaining structural concerns from the initial review (kill switch, post-release coherence, provenance) all have answers — though some answers introduce new concerns of their own.

The plan **cannot ship in its current form**, because:
1. The new Critical (pre-flight will fail every run) is a definitive correctness bug.
2. Four CI mise tasks are referenced by the workflow YAML but never defined.
3. `BuildArtefacts.from_disk` is the load-bearing bridge between prepare and finalize, and is underspecified.

A second revision pass should focus on those three gaps plus the Major findings around `@task` misapplication, the kill-switch in-flight gap, and the launcher's hardcoded owner. The Minor findings can be triaged.

Total finding count for this re-review: 1 Critical, 17 Major, ~25 Minor (approximate; some overlap across lenses). Compared to the initial review's 2 Critical, 27 Major, 43 Minor, the trend is strongly positive — but a third pass is required to get to APPROVE.

---

## Re-Review (Pass 3) — 2026-05-01

**Verdict:** REVISE

The pass-2 critical (pre-flight `validate_version_coherence` would fail every CI run) is fully resolved by `version.write`'s lockstep advance across all three coherence files. The pass-2 definitional gaps (four release-task halves undefined; `BuildArtefacts.from_disk` underspecified; six mise wrappers missing) are all closed with explicit code and Python signatures. The trend remains strongly positive — no Critical findings this pass — but enough Major findings remain that another revision is warranted.

### Trend
- Pass 1: 2 Critical, 27 Major, 43 Minor
- Pass 2: 1 Critical, 17 Major, ~25 Minor
- Pass 3: 0 Critical, 15 Major, ~28 Minor (some new Majors counterbalance the resolutions)

### Previously Identified Issues

#### Critical (pass 2)
- 🔴 **Correctness**: Pre-flight `validate_version_coherence` will fail every CI run — **Resolved** (lockstep `version.write` extends to checksums.json's `version` field; new tests assert the post-write coherence invariant).

#### Major (pass 2) — resolved
- 🟡 Workflow YAML referenced four release-task halves but only two sketched — **Resolved** (all four halves now defined in §3a with full Python bodies; six mise wrappers in §3b).
- 🟡 `BuildArtefacts.from_disk` underspecified — **Resolved** (full code with `sha256:` prefix stripping, version-mismatch guard, missing-binary check).
- 🟡 Two parallel orchestration paths silent-drift risk — **Resolved** (single-call versions are now ten-line wrappers with `_refuse_under_ci` guard).
- 🟡 `@task` decorator misapplied to `build()`/`upload_and_verify()` — **Resolved** (decorators removed; dead mise wrappers replaced with documentation).
- 🟡 Hardcoded `--owner atomic-innovation` (byte-equal-fork attack) — **Resolved** (`--repo` form + `ACCELERATOR_VISUALISER_PROVENANCE_REPO` override).
- 🟡 `actions/attest-build-provenance@v2` floating — **Resolved** (pinned `@v2.4.0`).
- 🟡 `gh attestation verify` minimum version — **Resolved** (`_gh_version_at_least 2.49.0` runtime check + RELEASING.md doc).
- 🟡 `BaseException` cleanup runs gh during interrupts — **Resolved** (`Exception` only; `BaseException` retained for local unlink with explanatory comment).
- 🟡 New mise tasks defined nowhere — **Resolved** (Phase 12.4 §3b enumerates all six).
- 🟡 Provenance opt-in lacks operator guidance — **Resolved** (README expanded with threat model, network requirement, fork override).

#### Major (pass 2) — partially / not addressed
- 🟡 Auto-cut `*-pre.0` lifecycle/retention policy — **Partially resolved** (auto-generated notes claimed to mark scaffolding cuts; not all lenses agree this is sufficient).
- 🟡 Kill-switch in-flight gap — **Resolved at the documentation level** (RELEASING.md now describes `gh run cancel <id>` for in-flight halt) but no code-level halt mechanism exists.
- 🟡 Concurrency group shared between prerelease and release jobs — **Still present** (re-flagged in this pass by architecture, correctness, compatibility lenses).
- 🟡 Three-of-four-platform partial coverage retained — **Not addressed**.
- 🟡 Smoke test #[ignore] silent skip risk — **Not addressed**.
- 🟡 `gh release download --pattern --output --clobber` ambiguity — **Not addressed** (re-flagged this pass).
- 🟡 Tests still couple to call ordering / command-string equality — **Not addressed**.

### New Issues Introduced in Pass 3

#### Critical
*(none)*

#### Major

- 🟡 **Compatibility / Correctness / Safety** (3 lenses, high confidence): The launcher provenance snippet at Phase 12.4 §6 uses `local gh_version`, `local repo`, `local err_file`, and `trap '...' RETURN` — **all of which are only valid inside a shell function**. The snippet is documented as inserted "after the existing SHA-256 check passes, before the binary is exec'd" — i.e., at script top level. Bash will hard-fail with `local: can only be used in a function` on every invocation when `ACCELERATOR_VISUALISER_VERIFY_PROVENANCE=1` is set. Fix: wrap the block in a `_verify_provenance()` helper function in `launcher-helpers.sh`, or replace `local` with bare assignments and `trap ... RETURN` with explicit `rm -f` after the `if`/`fi`.

- 🟡 **Compatibility** (medium confidence): `_gh_version_at_least` "uses `sort -V`" but `sort -V` is not in BSD `sort` on macOS 11 and earlier. Users opting in to provenance verification on older macOS get a confusing `gh >= 2.49.0` error citing the version they actually have installed. Fix: replace `sort -V` with a portable awk-based numeric comparator.

- 🟡 **Correctness / Safety** (2 lenses, high confidence): `write_checksums_version` (Phase 12.4 §1) drops the atomic-write discipline that `update_checksums_json` carefully maintains. It writes via `_CHECKSUMS_JSON.write_text(...)` directly with no `.tmp`-then-replace pattern. A failure mid-write (SIGTERM, disk-full, OOM) leaves checksums.json truncated. Fix: factor a private `_atomic_write_json(path, data)` helper that both functions call.

- 🟡 **Correctness / Safety** (2 lenses, medium confidence): `version.write` is not transactionally atomic across the three files (plugin.json + Cargo.toml + checksums.json). A partial advance leaves the working tree with three files at three different versions; `validate_version_coherence` then fails every subsequent run with no automatic recovery. Fix: stage all three writes in tempdir then `os.replace` atomically, or document the partial-write recovery procedure explicitly.

- 🟡 **Architecture** (high confidence): `tasks/version.py` is now coupled to a single skill's distribution layout — it hard-codes `Path("skills/visualisation/visualise/bin/checksums.json")`. Adding a second skill that ships binaries would force re-architecting `version.write` rather than allowing a sibling extension. Fix: factor coherence-tracked-files registration out of `version.py` into a list of "coherence advancers", or document the technical-debt acceptance.

- 🟡 **Architecture** (high confidence): Orphan-attestation cleanup gap on any `*_publish` failure post-attestation. The failure-modes table only documents this for runner-preemption between halves, but every CI failure that the orchestration auto-cleans actually leaves a permanent orphan attestation in the public transparency log. Fix: update the failure-modes table to note this is a known operational signal; document in RELEASING.md.

- 🟡 **Code Quality** (high confidence): The four `*_publish` halves contain near-identical commit/tag/push/create-release/upload sequences (~5 lines × 3-4 sites = 15-20 duplicated lines). Fix: extract `_finalize_and_publish(context, version)` private helper called from all halves.

- 🟡 **Code Quality** (high confidence): `tasks/release_helpers.py` (the pure-helper layer) is documented to import `read_cargo_toml_version` from `tasks/version.py` (the invoke-task layer) — a layering inversion. Helpers should not depend on tasks. Fix: move pure Cargo.toml read/parse logic into `release_helpers.py` (or a new `tasks/_toml.py`); have `version.write_cargo_toml` be the orchestration-side wrapper.

- 🟡 **Test Coverage** (high confidence): `BuildArtefacts.from_disk` has a hardened contract (version-mismatch ValueError, missing binary FileNotFoundError, malformed `sha256:` prefix ValueError) but Phase 12.3 §3 / Phase 12.4 §3a do not commit specific tests against any of these failure modes. Without tests, a mutation that flipped `!=` to `==` in the version-mismatch guard would silently publish wrong-version releases. Fix: enumerate at minimum five test cases for `from_disk` (happy path; version mismatch; missing binary; missing prefix; empty hex).

- 🟡 **Test Coverage** (high confidence): The cleanup wrapper's KeyboardInterrupt and cleanup-failure paths are documented in prose but untested. The Exception-vs-BaseException design choice (deliberately added in pass-3) is not exercised by any listed test. Fix: add `test_keyboard_interrupt_skips_cleanup`, `test_cleanup_failure_does_not_mask_original_error`, and `test_cleanup_invocation_uses_warn_and_timeout`.

- 🟡 **Test Coverage** (high confidence): The four new release-task halves and the `_refuse_under_ci` guard have no test bullets. Pass-3 introduced six new `@task`-decorated functions plus the guard; the test list is unchanged from the single-call era. A regression that swapped halve order or broke the guard would only be caught manually by Phase 12.7. Fix: add per-halve sequence tests plus a `_refuse_under_ci` assertion test.

- 🟡 **Test Coverage** (medium confidence — re-flagged): Smoke test still `#[ignore]`'d with no structural enforcement that the mise task's `--ignored` flag actually runs the test. Fix: replace `#[ignore]` with a runtime guard, OR add a Success Criterion that asserts the test name appears in the cargo-test output.

#### Minor (highlights)

- 🔵 **Architecture** (high): `_refuse_under_ci` only checks `GITHUB_ACTIONS=true` but its docstring says "under CI" generally. Either rename to `_refuse_under_github_actions` or expand to `CI=true` as a fallback.
- 🔵 **Architecture** (high): Concurrency group `prerelease-${{ github.ref }}` shared between prerelease and release jobs — still misnamed for the release job.
- 🔵 **Code Quality** (high): Duplicate `_DEFAULT_REPO_ROOT` (release_helpers.py) vs `_REPO_ROOT` (release_binaries.py) — consolidate to one shared constant in `tasks/__init__.py` or `tasks/_paths.py`.
- 🔵 **Code Quality** (medium): `NamedTemporaryFile(delete=False)` + manual `try/finally unlink` in `verify_uploaded_asset` — replace with `tempfile.TemporaryDirectory()` scope.
- 🔵 **Correctness** (high): `from_disk` raises bare `KeyError` on missing platform entry rather than a typed `ValueError` — tighten the error contract.
- 🔵 **Correctness** (high): `is_prerelease_version` return value discarded; refactor to a named `validate_version_string()` helper with clearer intent.
- 🔵 **Security** (medium): RETURN trap scope and `_gh_version_at_least` definition under-specified — see Compatibility Major above for the related hard-fail.
- 🔵 **Security** (high — re-flagged): Auto-cleanup still destroys forensic evidence on `AssetVerificationError` (a SHA mismatch is exactly the supply-chain tampering signal that should be preserved). Differentiate cleanup paths by exception type.
- 🔵 **Security** (medium — re-flagged): `ACCELERATOR_VISUALISER_INSECURE_DOWNLOAD` bypass still has no localhost enforcement code-side, only documentation.
- 🔵 **Compatibility** (high): tomlkit 0.13 formatting differs from 0.12 — pre-emptively normalise Cargo.toml in a separate Phase 12.1 §4a sub-step so the version-bump diffs are clean.
- 🔵 **Safety** (high): No working-tree cleanliness check in `_preflight` for the local-dev wrapper path.
- 🔵 **Safety** (medium): Stable-published-then-post-stable-failed leaves no automatic recovery; document the manual command sequence in RELEASING.md.
- 🔵 **Documentation** (high): Workflow-step → mise-task → Python-function map in RELEASING.md only enumerates prerelease step labels; stable halves abbreviated.
- 🔵 **Documentation** (medium): `_refuse_under_ci` guard invisible to operators reading user-facing docs.

#### Suggestions

- 🔵 **Architecture**: Six sequential mise invocations duplicate Python startup overhead (~5s/invocation × 6 = ~30s on stable cuts). Document as accepted cost.
- 🔵 **Code Quality**: `from_disk` validation logic mixes data-shape and parsing concerns — extract `_load_build_artefacts(version, repo_root)` free function.
- 🔵 **Code Quality**: `os.environ.get("GITHUB_ACTIONS") == "true"` is fragile; use truthy check or membership test.
- 🔵 **Security**: Provenance fails-closed on `api.github.com` outages — consider degradation env var.
- 🔵 **Security**: Document the duplicate-ish attestation entries (stable + post-stable pre.0) as intentional in RELEASING.md.
- 🔵 **Safety**: No deadman cleanup for orphan drafts left by Ctrl-C — add a quarterly sweep workflow.
- 🔵 **Documentation**: CHANGELOG `Notes` section restates `Added` bullet content.

### Assessment

The plan continues to converge cleanly. Pass-3 fixes resolved the load-bearing pass-2 Critical and definitional gaps; the remaining concerns are mostly **tactical refinements**:

- Three new Major findings concentrate on a single 18-line shell snippet (Phase 12.4 §6 launcher) that has hard-fail bugs (`local`/`trap RETURN` outside a function, `sort -V` on older macOS, `_gh_version_at_least` body unspecified). Wrap the snippet in a function and replace `sort -V` with portable awk → all three resolved at once.
- The atomicity gap in `write_checksums_version` and the broader `version.write` non-atomicity are closely related — one shared `_atomic_write_json` helper plus a brief documentation note covers both.
- The test-coverage Majors are mostly enumeration: the new functions exist but the test list wasn't extended. Adding 8-12 specific test bullets across `from_disk`, the cleanup wrapper, the four release halves, and `_refuse_under_ci` closes the gap.
- The architecture/code-quality concerns (DRY in publish halves, helper-layer-imports-task-layer, version.py coupled to one skill) are refactors that improve maintainability without changing behaviour.

The plan is **two passes away from APPROVE** — one focused pass on the launcher shell snippet, atomicity, and tests; one polish pass for the remaining Minors. Total finding count for this re-review: **0 Critical, 15 Major, ~28 Minor, ~7 Suggestion** — strong downward trend from 2/27/43 in pass 1.

---

## Re-Review (Pass 4) — 2026-05-01

**Verdict:** REVISE (very close to APPROVE — 7 Major findings remain, all tactical)

The pass-3 critical and definitional gaps were resolved in pass-4. Pass-5 polish closed many of the pass-3 Major findings (concurrency split, DRY extraction, forensic preservation, smoke-feature gating, `_REPO_ROOT` consolidation, RELEASING.md completeness). The remaining Major findings are concentrated around: (1) tests that need updating to match new pass-4/5 code semantics, (2) two compatibility issues in newly-introduced launcher helpers, and (3) a few documentation gaps for new behaviour. None are blocking — the plan is implementable as-written and the trend is strongly converging.

### Trend
- Pass 1: 2 Critical, 27 Major, 43 Minor
- Pass 2: 1 Critical, 17 Major, ~25 Minor
- Pass 3: 0 Critical, 15 Major, ~28 Minor
- Pass 4: 0 Critical, 7 Major, ~28 Minor, ~10 Suggestion

### Previously Identified Issues

#### Critical
- All Criticals from passes 1-3 remain resolved. None re-emerge.

#### Pass-3 Major findings — resolution status

- 🟡 Launcher hard-fail bug (`local`/`trap RETURN`) — **Resolved** (wrapped in `_verify_provenance()` function).
- 🟡 `sort -V` not portable — **Resolved** (replaced with awk).
- 🟡 `write_checksums_version` non-atomic — **Resolved** (factored `_atomic_write_text` in helper layer).
- 🟡 `version.write` not transactionally atomic — **Resolved** ("render first, write second" pattern; partial-advance is recoverable via re-run).
- 🟡 BuildArtefacts.from_disk untested — **Resolved** (5 explicit pytest cases).
- 🟡 Cleanup wrapper untested — **Resolved** (3 cases for KeyboardInterrupt / cleanup-failure / warn+timeout invocation).
- 🟡 Four release halves and `_refuse_under_ci` untested — **Resolved** (6 cases).
- 🟡 Smoke test silent-skip risk — **Resolved** (Cargo `feature = "smoke"` gating).
- 🟡 DRY violation across four publish halves — **Resolved** (`_finalize_and_publish` helper extracted).
- 🟡 Concurrency group sharing — **Resolved** (split into `prerelease-` vs `stable-release-` groups).
- 🟡 Helper layer imported invoke-task layer — **Resolved** (`_atomic_write_text` lives in `release_helpers.py`).
- 🟡 Three-of-four-platform partial coverage — **Carryover** (still flagged as accepted gap).
- 🟡 `tasks/version.py` coupled to single skill — **Carryover** (still flagged; deferred).
- 🟡 Tests still couple to call ordering / command-string equality — **Carryover** (still flagged).
- 🟡 `gh release download` flag combo — **Resolved** (positional asset-name form).

### New / Outstanding Issues After Pass 4

#### Critical
*(none)*

#### Major (7)

- 🟡 **Test Coverage** (high confidence): `test_verify_failure_deletes_draft` now contradicts the pass-4/5 forensic-preservation semantic. The test description still asserts `gh release delete` is called on `AssetVerificationError`, but the new code preserves the draft + tag. Fix: split into `test_asset_verification_error_preserves_draft_and_emits_alert` (negative cleanup, positive alert call) and `test_generic_exception_deletes_draft` (transient cleanup path).

- 🟡 **Test Coverage** (high confidence): Debug-symbol archive uploads contradict `test_stable_full_flow`'s "exactly four `gh release upload` calls" assertion. With `*.debug.tar.gz` archives uploaded alongside binaries, the count should be 8. Fix: update the upload-count assertion and add `test_debug_archives_not_in_checksums_manifest` + `test_verify_skips_debug_archives`.

- 🟡 **Test Coverage** (high confidence): Loopback-only enforcement (`_assert_insecure_download_loopback_only`) has only one negative test. The four positive branches (flag unset, 127.0.0.1, ::1, localhost) and edge-case URL shapes (IPv6 brackets, userinfo, schemeless) are untested. Fix: add four sibling smoke tests covering the positive branches plus URL-shape edge cases.

- 🟡 **Compatibility** (high confidence): URL host extraction via `awk -F[/:] '{print $4}'` in `_assert_insecure_download_loopback_only` cannot recognise the bracketed-IPv6 form (`http://[::1]:port/`). The case statement allow-lists `::1` but the parser produces `[` for that input, so an IPv6 loopback URL is rejected. Fix: replace the awk one-liner with explicit bash parameter expansion that handles `[ipv6]` brackets, or drop `::1` from the allow-list and document IPv4 loopback only.

- 🟡 **Compatibility** (high confidence): Cargo `feature = "smoke"` is referenced (`cargo test --features smoke` in mise task) but the `[features] smoke = []` declaration is never shown being added to `server/Cargo.toml`. First execution of `mise run test:integration:binary-acquisition` will fail with `error: Package does not have feature 'smoke'`. Fix: add an explicit Cargo.toml diff to Phase 12.5 §2 showing the `[features]` table addition.

- 🟡 **Safety** (high confidence): The AssetVerificationError forensic-preservation path leaves a non-trivial half-state (draft + tag retained, blocking the tag namespace) with no entry in the failure-modes-by-step table — the table still says verify failure auto-cleans. Operators reading the table will follow the wrong recovery procedure. Fix: update the table's "Verify (step 14)" row to distinguish AssetVerificationError (preserve, triage) from transient failures (auto-clean), and document the triage procedure in RELEASING.md.

- 🟡 **Architecture** (high confidence — carryover): `tasks/version.py` is now hard-coupled to the visualisation skill's filesystem layout (`_CARGO_TOML`, `_CHECKSUMS_JSON`). Adding a second binary-bearing skill would require modifying `version.py` rather than registering. Fix: introduce a `VersionedFile` registry pattern, or document explicitly as accepted technical debt with a marker for the future-skill case.

#### Minor (highlights)

- 🔵 **Test Coverage** (high): `test_cleanup_invocation_uses_warn_and_timeout` asserts `timeout=60` but the actual code uses `timeout=120`. One of these is wrong; reconcile both sites.
- 🔵 **Test Coverage** (high): CI guard test only covers `GITHUB_ACTIONS=true`, not `CI=1`/`CI=yes`. Parametrise the test over the four shapes the helper accepts.
- 🔵 **Code Quality** (medium): `_emit_forensic_alert` is borderline over-extracted (single-line print) and takes an unused `context` parameter. Either inline at call site or drop the unused parameter.
- 🔵 **Code Quality** (medium): NamedTemporaryFile lifecycle still awkward (carryover from pass-3). `tempfile.TemporaryDirectory()` would be cleaner.
- 🔵 **Code Quality** (medium): `BuildArtefacts.from_disk` classmethod still mixes parsing + validation + path derivation in one 25-line body. Extract `_parse_manifest_entry` and `_resolve_staged_path` if the method grows.
- 🔵 **Code Quality** (low): Mixed import styles (`from . import release_helpers` AND `from .release_helpers import compute_sha256`) in `release_binaries.py`.
- 🔵 **Correctness** (medium): `_gh_version_at_least` silently treats `2.49.0-rc.1` as `2.49.0`. Document the simplification or strip prerelease suffixes explicitly.
- 🔵 **Compatibility** (medium): `actions/attest-build-provenance@v2.4.0`'s `subject-path: 'accelerator-visualiser-*'` glob will match the `.debug.tar.gz` archives too, doubling the attestation subjects. Either tighten the glob or document the broader attestation surface.
- 🔵 **Compatibility** (medium): `gh --version` parse via `awk '$3'` fragile if upstream changes the version line format. Use a more defensive regex match.
- 🔵 **Security** (medium): `gh release download` raw stderr interpolated into AssetVerificationError message (and thus the workflow log traceback) could leak token-bearing error context on private forks. Sanitise stderr at the boundary.
- 🔵 **Safety** (medium): jj-status-based dirty-file check in `_preflight` needs a concrete predicate (substring vs full-path match) to avoid false negatives.
- 🔵 **Safety** (medium): `::error` annotation is a weak alert for a supply-chain-tampering signal. Couple with `gh issue create --label security` or document a CODEOWNERS notification rule.
- 🔵 **Documentation** (high): INSECURE_DOWNLOAD loopback enforcement not in README customisation-hooks bullet.
- 🔵 **Documentation** (high): Forensic-preservation behaviour for AssetVerificationError not in RELEASING.md content list.
- 🔵 **Documentation** (medium): Working-tree cleanliness check error message not specified.
- 🔵 **Documentation** (medium): Debug-symbol archives have no maintainer-facing usage docs (how to symbolicate a customer crash).
- 🔵 **Documentation** (medium): `_finalize_and_publish` not in RELEASING.md source-of-truth pointer list.

#### Suggestions (~10)

Notable: forensic-preservation creates an undocumented operational state (architecture); smoke feature mixes test concerns into Cargo features (architecture); render-first-write-second has unsnapshotted reads (correctness); `::error` annotation lacks asset/SHA detail (correctness); concurrency-group split allows stale-checkout race window (security); debug-archive name prefix could be confused for release binaries (safety); cross-references between README/RELEASING.md/Migration Notes for provenance env vars (documentation).

### Assessment

The plan is **at or near APPROVE**. The 7 Major findings cluster into three tractable categories:

1. **Test/code drift introduced by pass-4/5 changes** (3 majors). The most egregious is `test_verify_failure_deletes_draft` now contradicting the new forensic-preservation semantic. These are tactical edits — update the test descriptions to match the new contracts; no design rework needed.
2. **Two compatibility issues in newly-introduced launcher helpers** (2 majors). The IPv6 URL parsing is a real but localised bug; the missing Cargo `[features]` declaration is a 3-line Cargo.toml diff. Both fix in <30 minutes.
3. **Documentation/architecture carryovers** (2 majors). The failure-modes table needs a row update for AssetVerificationError; the `version.py` coupling can stay if explicitly documented as deferred.

A focused pass-5b applying these seven fixes would produce a clean APPROVE. Alternatively, given the strongly-converging trend (no Criticals, decreasing Majors, no new structural issues in pass-4), the project owner could reasonably choose to **accept this iteration as APPROVE-with-conditions** — track the seven Majors as implementation TODOs that surface during the actual merge of Phase 12.4, rather than blocking on another revision pass.

This is the closest the plan has been to APPROVE in any iteration. The structural soundness, defence-in-depth, and operational safety are all in place; what remains is paperwork.
