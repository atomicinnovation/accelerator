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

Each job follows a **prepare → attest → finalise** structure. The
`actions/attest-build-provenance@v2` step must interleave between the build
and the publish phases, so the CI jobs call `prerelease:prepare` /
`release:prepare`, let the workflow attest, then call `prerelease:finalise` /
`release:finalise`.

The local-dev convenience tasks (`mise run prerelease`, `mise run release`)
skip attestation and **must never run in CI**. A `_refuse_under_ci` guard at
the top of each wrapper in `tasks/release.py` raises `RuntimeError` if
`GITHUB_ACTIONS` or `CI` is set.

### Prerelease job

| Workflow step              | mise task             | Python function                      |
|----------------------------|-----------------------|--------------------------------------|
| `Prepare prerelease`       | `prerelease:prepare`  | `tasks.release.prerelease_prepare`   |
| `Attest binary provenance` | (workflow action)     | `actions/attest-build-provenance@v2` |
| `Finalise prerelease`      | `prerelease:finalise` | `tasks.release.prerelease_finalise`  |

`prerelease_prepare` steps:
1. Configures git identity and pulls `main`
2. Bumps the pre-release counter (`1.2.3-pre.N` → `1.2.3-pre.N+1`)
3. Writes the new version to `Cargo.toml`, `plugin.json`, and `bin/checksums.json`
4. Cross-compiles four-platform binaries via `cargo zigbuild` (`tasks/build.py`)
5. Computes SHA-256 checksums and writes them to `bin/checksums.json`

`prerelease_finalise` steps (shared `_publish` helper in `tasks/release.py`):
6. Commits the version bump
7. Tags `v{version}`
8. Pushes commit and tag
9. Creates a draft GitHub release (`tasks/github.py → create_release`)
10. Uploads all four binaries and their `.debug.tar.gz` archives
11. Downloads and re-verifies each binary against `bin/checksums.json`
12. Publishes the draft

### Release job (stable)

| Workflow step                          | mise task             | Python function                      |
|----------------------------------------|-----------------------|--------------------------------------|
| `Prepare stable release`               | `release:prepare`     | `tasks.release.release_prepare`      |
| `Attest stable binary provenance`      | (workflow action)     | `actions/attest-build-provenance@v2` |
| `Finalise stable release`              | `release:finalise`    | `tasks.release.release_finalise`     |
| `Prepare post-stable prerelease`       | `prerelease:prepare`  | `tasks.release.prerelease_prepare`   |
| `Attest post-stable binary provenance` | (workflow action)     | `actions/attest-build-provenance@v2` |
| `Finalise post-stable prerelease`      | `prerelease:finalise` | `tasks.release.prerelease_finalise`  |

`release_prepare` additionally:
- Promotes the version (`1.2.3-pre.N` → `1.2.3`)
- Updates `.claude-plugin/marketplace.json`
- Marks the `Unreleased` CHANGELOG section with the version (`tasks/changelog.py`)

After the stable release publishes, the job reuses `prerelease_prepare` /
`prerelease_finalise` to cut `1.(Y+1).0-pre.0` immediately.

## Source files

| File                  | Responsibility                                               |
|-----------------------|--------------------------------------------------------------|
| `tasks/release.py`    | Orchestration tasks; `_refuse_under_ci` guard; `_publish`   |
| `tasks/github.py`     | `create_release`, `upload_and_verify`, `download_and_verify` |
| `tasks/build.py`      | `create_checksums`, `validate_version_coherence`             |
| `tasks/version.py`    | Version read / bump / write across all tracked files         |
| `tasks/changelog.py`  | `changelog.release` moves Unreleased to a version heading    |
| `tasks/marketplace.py`| Updates `.claude-plugin/marketplace.json` on stable release  |

## Local diagnostics

To exercise the pipeline locally (e.g. against a fork):

```bash
mise run prerelease    # no attestation — local-dev only
```

Never run these tasks against the `atomic-innovation/accelerator` remote. The
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
gh api repos/atomic-innovation/accelerator/git/refs/tags
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
    --repo atomic-innovation/accelerator
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
