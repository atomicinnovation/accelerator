"""The release-lane invoke tasks for the vendored runtime.

Three CI entry points sharing the job's filesystem: ``verify_upstream_inputs``
runs in the step that holds ``GH_TOKEN`` (for the SLSA check) and stages the
verified inputs under ``dist/vendor-inputs/<platform>/``;
``assemble_tree_artifacts`` runs in a step with no token, reading those inputs
and producing the archives; and ``smoke_runtime`` runs per platform on a native
host, executing the downloaded binaries.
"""

import os
from pathlib import Path

from invoke import Context, task

from tasks.shared.paths import KEYS_DIR, RELEASE_STAGING, REPO_ROOT
from tasks.shared.targets import TARGETS
from tasks.shared.vendor import (
    archive,
    assemble,
    attestation,
    trust_anchors,
    upstream,
)

VENDOR_INPUTS = REPO_ROOT / "dist" / "vendor-inputs"
PLAYWRIGHT_PACKAGE_JSON = (
    REPO_ROOT / "skills/design/inventory-design/scripts/playwright/package.json"
)


@task(name="check-trust-anchors")
def check_trust_anchors(context: Context) -> None:
    """Fail if the vendored-runtime trust anchors are still placeholders.

    Runs first in the assembly job so a release cut before the refresh procedure
    stops with a named remediation rather than a missing-key traceback.
    """
    trust_anchors.assert_ready()


@task(name="verify-upstream-inputs")
def verify_upstream_inputs(context: Context) -> None:
    """Fetch and verify playwright-core, Node and Chromium for every target."""
    for _triple, platform in TARGETS:
        upstream.verify_upstream_inputs(
            platform=platform,
            staging_dir=VENDOR_INPUTS / platform,
            package_json=PLAYWRIGHT_PACKAGE_JSON,
            keys_dir=KEYS_DIR,
        )


@task(name="assemble-tree-artifacts")
def assemble_tree_artifacts(context: Context) -> None:
    """Assemble every target's tree archives from the verified inputs.

    Smoke is left to the per-platform matrix, since this one host cannot execute
    the other targets' binaries.
    """
    for _triple, platform in TARGETS:
        staging = VENDOR_INPUTS / platform
        assemble.assemble_tree_artifacts(
            playwright_tarball=next(staging.glob("playwright-core-*.tgz")),
            node_tarball=next(staging.glob("node-v*.tar.gz")),
            chromium_archive=next(
                staging.glob("chromium-headless-shell-*.zip")
            ),
            platform=platform,
            staging_dir=RELEASE_STAGING / f".assemble-{platform}",
            dist_dir=RELEASE_STAGING,
            spec_builder=assemble.default_spec_builder,
            run_smoke=False,
        )


@task(name="smoke-runtime")
def smoke_runtime(context: Context, platform: str = "") -> None:
    """Execute the downloaded tree binaries for one platform, natively.

    The platform comes from ``--platform`` or, for the CI matrix, the
    ``SMOKE_PLATFORM`` environment variable.
    """
    resolved = platform or os.environ.get("SMOKE_PLATFORM", "")
    if not resolved:
        raise ValueError(
            "no platform given (pass --platform or set SMOKE_PLATFORM)"
        )
    assemble.smoke_downloaded_archives(RELEASE_STAGING, resolved)


@task(name="build-archive")
def build_archive(
    context: Context, tree: str, dest: str, platform: str
) -> None:
    """Build one deterministic tree archive and its detached attestation.

    Writes the archive to ``dest`` and its attestation to ``<dest>.sealed``.
    The cross-language contract test drives this to prove the Python producer
    and the Rust consumer agree on the archive and attestation formats.
    """
    stats = archive.write_deterministic_archive(Path(tree), Path(dest))
    Path(f"{dest}.sealed").write_bytes(
        attestation.build_attestation("driver", platform, stats)
    )
