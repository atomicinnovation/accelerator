# Releasing

This document covers the CI release pipeline for maintainers. For day-to-day
development, see [README](README.md).

## Pipeline shape

Every push to `main` triggers three jobs in `.github/workflows/main.yml`:

```
test  →  prerelease  →  release  (requires `release` Environment approval)
```

- **test** — runs the full test suite (`mise run test`)
- **prerelease** — bumps the pre-release counter, cross-compiles four-platform
  binaries, attests their provenance, and publishes a `*-pre.N` GitHub Release
- **release** — gated by the `release` GitHub Environment (manual approval);
  promotes to a stable `X.Y.Z` release, then immediately cuts the next
  `X.(Y+1).0-pre.0`

## Release flow

Each job follows a **prepare → sign → attest → finalise** structure. The
signing step is scoped so `ACCELERATOR_RELEASE_SECRET_KEY` is in the environment
only there — never during the `cargo zigbuild` compile, which runs untrusted
transitive build scripts. The `actions/attest-build-provenance@v2` step then
interleaves between build/sign and publish, so the CI jobs call
`prerelease:prepare`, `prerelease:sign`, let the workflow attest, then call
`prerelease:finalise` (and the `release:*` equivalents).

The local-dev convenience tasks (`mise run prerelease`, `mise run release`)
skip attestation and **must never run in CI**. A `_refuse_under_ci` guard at
the top of each wrapper in `tasks/release.py` raises `RuntimeError` if
`GITHUB_ACTIONS` or `CI` is set.

### Prerelease job

| Workflow step              | mise task             | Python function                      |
|----------------------------|-----------------------|--------------------------------------|
| `Prepare prerelease`       | `prerelease:prepare`  | `tasks.release.prerelease_prepare`   |
| `Sign prerelease`          | `prerelease:sign`     | `tasks.release.prerelease_sign`      |
| `Attest binary provenance` | (workflow action)     | `actions/attest-build-provenance@v2` |
| `Finalise prerelease`      | `prerelease:finalise` | `tasks.release.prerelease_finalise`  |

`prerelease_prepare` steps:
1. Configures git identity and pulls `main`
2. Bumps the pre-release counter (`1.2.3-pre.N` → `1.2.3-pre.N+1`)
3. Writes the new version to `Cargo.toml`, `plugin.json`, and `bin/checksums.json`
4. Updates `.claude-plugin/marketplace-prerelease.json` to the new version tag
5. Cross-compiles the visualiser + cli launcher via `cargo zigbuild`, asserting
   each staged launcher embeds the release version
6. Computes SHA-256 checksums and writes them to `bin/checksums.json`

`prerelease_sign` (the only step holding the secret):
7. Signs the launcher (and every dispatched sub-binary) into detached `.minisig`
8. Emits and signs `manifest.json` → `manifest.minisig` under one materialised
   key; fails closed if the secret is absent

`prerelease_finalise` steps (shared `_publish` helper in `tasks/release.py`):
9. Asserts no build artifact or secret leaked outside `dist/release/`
10. Commits the version bump, tags `v{version}`, pushes
11. Creates a draft GitHub release (`tasks/github.py → create_release`)
12. Uploads every asset across both tracks, re-verifies each, publishes once
    (`upload_and_verify_release`)

### Release job (stable)

| Workflow step                          | mise task             | Python function                      |
|----------------------------------------|-----------------------|--------------------------------------|
| `Prepare stable release`               | `release:prepare`     | `tasks.release.release_prepare`      |
| `Sign stable release`                  | `release:sign`        | `tasks.release.release_sign`         |
| `Attest stable binary provenance`      | (workflow action)     | `actions/attest-build-provenance@v2` |
| `Finalise stable release`              | `release:finalise`    | `tasks.release.release_finalise`     |
| `Prepare post-stable prerelease`       | `prerelease:prepare`  | `tasks.release.prerelease_prepare`   |
| `Sign post-stable prerelease`          | `prerelease:sign`     | `tasks.release.prerelease_sign`      |
| `Attest post-stable binary provenance` | (workflow action)     | `actions/attest-build-provenance@v2` |
| `Finalise post-stable prerelease`      | `prerelease:finalise` | `tasks.release.prerelease_finalise`  |

`release_prepare` additionally:
- Promotes the version (`1.2.3-pre.N` → `1.2.3`)
- Updates `.claude-plugin/marketplace.json`
- Marks the `Unreleased` CHANGELOG section with the version (`tasks/changelog.py`)

After the stable release publishes, the job reuses `prerelease_prepare` /
`prerelease_finalise` to cut `1.(Y+1).0-pre.0` immediately.

## Release signing key lifecycle

Every distributed binary — the `accelerator` launcher (fetched by the bootstrap)
and every manifest-listed sub-binary (fetched by the launcher) — is signed with
minisign. The launcher and bootstrap embed the committed public key
(`keys/accelerator-release.pub`) at build time and refuse anything it does not
verify. The matching secret must never be committed (`/keys/*.sec` is
gitignored).

**The currently-committed public key is of unknown secret provenance** and must
be treated as untrusted. Before any real (non-empty) sub-binary ships, an admin
must generate a fresh `-W` keypair whose secret has never left their control and
replace the committed public half.

### Generating and provisioning the keypair

```bash
mise run keys:generate            # writes keys/accelerator-release.{pub,sec}
gh secret set ACCELERATOR_RELEASE_SECRET_KEY < keys/accelerator-release.sec
```

`keys:generate` runs `minisign -G -W -f` non-interactively and never prints the
secret; provision it straight from the written `.sec` (piped, never echoed) so
it never lands in scrollback or shell history. Delete the local `.sec` once the
GitHub secret is set.

The `ACCELERATOR_RELEASE_SECRET_KEY` secret is materialised to a mode-`0600`
temp file only inside the dedicated signing step (`*:sign`), never during
compilation.

### Strict rollout sequence

The order is load-bearing — a launcher only trusts the key it was built with:

1. Commit the new `keys/accelerator-release.pub`. `cli/launcher/build.rs`
   re-embeds it, so a launcher built from that HEAD trusts the new key.
2. Cut and distribute the launcher built from that HEAD.
3. Only then sign any release with the matching secret.

Signing a release before a launcher that embeds the new public key exists in the
field would produce assets no deployed launcher can verify.

### Signing authority is push-to-`main`

The `prerelease` job runs unapproved on every push and cannot carry the
approval-gated `release` environment (that would hold the release concurrency
lock through the whole approval wait and deadlock later prereleases). So
`ACCELERATOR_RELEASE_SECRET_KEY` is a **repository/org secret** readable by the
`prerelease` job, and **any merge to `main` is signed by the production key and
published as a launcher-trusted prerelease with no release-time human gate**.

This is bounded by version-pinning — a launcher trusts only its own release's
manifest — but it means **required PR review + branch protection on `main` is
the control equivalent to signing authority**. `main` must enforce required
review before merge; that gate, not a release-time approval, is what stands
between an untrusted change and a signed prerelease. The stable-release path
keeps its separate `approve-release` human gate.

### Compromise detection and response

- **Access** — only the release admin and the `ACCELERATOR_RELEASE_SECRET_KEY`
  GitHub secret (readable by the release jobs) hold the secret. It is never
  written to disk outside the ephemeral signing step.
- **Detection** — an unexpected release, a signature that verifies under a key
  that was never provisioned, or a maintainer report of an unrecognised
  published asset.
- **Response** — rotate on compromise only, by embedding a new public key in the
  next launcher release (steps 1–3 above). Because launchers are version-pinned
  (each trusts only its own release's manifest), the blast radius is bounded to
  versions cut with the compromised secret. The launcher's verify-any-of keyring
  (`keys.rs`) leaves headroom to embed both the old and new keys for an overlap
  window if a staged rotation is ever needed.

## Vendored verify shims

`bin/accelerator-verify-{platform}` are committed, cross-compiled copies of the
`accelerator-verify` root-of-trust binary — the bootstrap runs the per-platform
shim to verify the launcher it fetches. They ship inside the plugin package
(unlike the uploaded launcher/manifest assets) and are key-agnostic (the public
key is passed as an argument), so they are refreshed **on demand**, never in the
release hot path.

Regenerate them when `lint:vendor-shims:check` fails (a `cli/verify` source
change, a `minisign-verify` bump, or a lockfile change):

```bash
mise run build:vendor-verify-shims   # cross-compile + copy + refresh the marker
```

This needs the cross-compile toolchain: the four `rustup` targets
(`deps:install:rust-targets`), `cargo-zigbuild`, and `ziglang` (both
uv-provisioned). Commit the refreshed `bin/accelerator-verify-*` binaries and
`bin/accelerator-verify.vendored.sha256` together. Cross-compiled binaries are
not byte-reproducible, so the marker — a hash over `cli/verify`'s build inputs,
not the shim bytes — is what the drift guard compares.

## Source files

| File                   | Responsibility                                                                                  |
|------------------------|-------------------------------------------------------------------------------------------------|
| `tasks/release.py`     | Orchestration tasks; `_refuse_under_ci` guard; `_sign`; `_publish`                              |
| `tasks/github.py`      | `create_release`, `upload_and_verify_release`, `download_and_verify`                            |
| `tasks/build.py`       | `create_checksums`, `cli_cross_compile`, `validate_version_coherence`                           |
| `tasks/signing.py`     | `sign_file`, `resolve_secret_key`, `sign_staged_binaries`, `keys.generate`                      |
| `tasks/manifest.py`    | `collect_entries`, `build_manifest`, `emit_manifest`                                            |
| `tasks/version.py`     | Version read / bump / write across all tracked files                                            |
| `tasks/changelog.py`   | `changelog.release` moves Unreleased to a version heading                                       |
| `tasks/marketplace.py` | Updates `marketplace.json` on stable release; `marketplace-prerelease.json` on every prerelease |

## Local diagnostics

To exercise the pipeline locally (e.g. against a fork):

```bash
mise run prerelease    # no attestation — local-dev only
```

Never run these tasks against the `atomicinnovation/accelerator` remote. The
`_refuse_under_ci` guard only blocks CI; it does not prevent running against
the wrong remote.

## Recovery procedures

### Cleaning up a bad publish

```bash
gh release delete v<ver> --cleanup-tag --yes
```

Then revert the version-bump commit or push a fresh `pre.N+1` through CI for
forward recovery.

### Partial failure triage

A failed mid-step run may leave an orphan draft release and tag. Check with:

```bash
gh release list
gh api repos/atomicinnovation/accelerator/git/refs/tags
```

Delete orphans before re-running.

### AssetVerificationError

When a run shows:

```
::error title=Visualiser release v<ver>::AssetVerificationError — draft + tag PRESERVED for triage
```

The draft and tag are preserved deliberately. Download the suspect asset
out-of-band, compare its SHA-256 against `bin/checksums.json` on the tagged
commit, and escalate as a security incident if there is a mismatch. Only run
`gh release delete v<ver> --cleanup-tag --yes` after triage closes. **Do not**
treat a preserved draft as a routine orphan.

**Residual git state.** `_publish` commits, tags, and **pushes** before the
upload + re-verify runs, so a re-verify failure leaves the version-bump commit
and its pushed tag advanced while the release stays draft. Two recovery paths:

- *Forward-fix the preserved draft* — after fixing the cause, re-run only the
  upload/verify against the **same preserved tag** (uploads are `--clobber`
  idempotent). Re-running the whole workflow instead re-bumps `pre.N` and cuts a
  new release, orphaning the draft + tag.
- *Abandon it* — `gh release delete v<ver> --cleanup-tag --yes`, then reconcile
  the already-pushed version-bump commit (it persists on `main` and must be
  rolled forward or reverted; it does not disappear with the draft).

## Incident response — halting prereleases

To stop all future prereleases without a code change:

1. Go to **Settings → Environments → prerelease** in the GitHub repository.
2. Either set a deployment branch policy that excludes `main`, or add a
   required reviewer who will not approve.

Re-enabling is the reverse. This does not stop an already-running prerelease
job. For in-flight jobs, also run:

```bash
gh run list --workflow=main.yml --status=in_progress
gh run cancel <run-id>
```

## Out-of-band provenance verification

Any user can verify a binary's SLSA provenance independently of the launcher:

```bash
gh attestation verify accelerator-visualiser-<os>-<arch> \
    --repo atomicinnovation/accelerator
```

Requires `gh >= 2.49.0`. The same command runs inside `launch-server.sh` when
`ACCELERATOR_VISUALISER_VERIFY_PROVENANCE=1` is set.

## Debug archives and crash symbolication

Every release uploads `accelerator-visualiser-<os>-<arch>.debug.tar.gz`
alongside each binary, containing the unstripped binary (and `.dSYM` on
macOS). To symbolicate a crash:

```bash
gh release download v<ver> --pattern '*.debug.tar.gz'
# ELF:    addr2line -e accelerator-visualiser-linux-<arch> <address>
# Mach-O: atos -o accelerator-visualiser-darwin-<arch> -l <load-addr> <address>
```

Debug archives are not in `bin/checksums.json` and are never fetched by
`launch-server.sh`.

## GitHub Environment configuration

The `release` Environment should be configured with:
- Required reviewers: release-owner team
- Deployment branch policy: `main` only
- Wait timer: 0 minutes

Forks setting up their own pipeline should mirror this. The `prerelease`
Environment requires no approvers but can be locked down via deployment branch
policy to halt automatic prereleases (see incident response above).
