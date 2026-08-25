"""Fetch and verify the three upstream inputs, returning verified local paths.

Orchestrates the per-source checks (npm registry signature + integrity + SLSA,
Node GPG-signed checksums, Chromium pinned hash) over the fetch layer, whose
functions are injected so the wiring is exercised against recorded fixtures
rather than the live registry, nodejs.org and CDN.

Three constants are validated in the release lane rather than here — the SLSA
signer workflow, the Chromium CDN base, and the CDN's per-platform names — since
they follow upstream publishing choices this repository does not control. A
wrong value fails the release loudly (a failed SLSA check or a hash mismatch),
never silently accepts an unverified input.
"""

import json
from dataclasses import dataclass
from pathlib import Path

from tasks.shared.paths import PINS_TOML
from tasks.shared.vendor import assemble, chromium, nodejs, npm, pins
from tasks.shared.vendor.fetch import Fetcher, JsonFetcher, download, get_json

NPM_REGISTRY = "https://registry.npmjs.org"
NODE_DIST = "https://nodejs.org/dist"
PLAYWRIGHT_PACKAGE = "playwright-core"

# The signing-certificate identity the SLSA provenance must carry: the GitHub
# Actions OIDC issuer, and a SAN anchored to microsoft/playwright's npm publish
# publish workflow (the tag ref varies per release, so it is matched by prefix).
GITHUB_ACTIONS_OIDC_ISSUER = "https://token.actions.githubusercontent.com"
PLAYWRIGHT_PROVENANCE_IDENTITY = (
    r"^https://github\.com/microsoft/playwright/\.github/workflows/"
    r"publish_release_npm\.yml@"
)

CHROMIUM_CDN = (
    "https://cdn.playwright.dev/dbazure/download/playwright/builds/chromium"
)
_CHROMIUM_PLATFORM = {
    "linux-x64": "linux",
    "linux-arm64": "linux-arm64",
    "darwin-x64": "mac",
    "darwin-arm64": "mac-arm64",
}


@dataclass(frozen=True)
class VerifiedInputs:
    """Verified local paths and the versions they were fetched for."""

    playwright_tarball: Path
    node_tarball: Path
    node_tarball_name: str
    chromium_archive: Path
    playwright_version: str
    chromium_revision: str
    node_version: str


def npm_packument_url(package: str) -> str:
    return f"{NPM_REGISTRY}/{package}"


def node_shasums_url(version: str) -> str:
    return f"{NODE_DIST}/v{version}/SHASUMS256.txt"


def node_tarball_name(version: str, platform: str) -> str:
    return f"node-v{version}-{platform}.tar.gz"


def node_tarball_url(version: str, platform: str) -> str:
    return f"{NODE_DIST}/v{version}/{node_tarball_name(version, platform)}"


def chromium_url(revision: str, platform: str) -> str:
    build = _CHROMIUM_PLATFORM[platform]
    return f"{CHROMIUM_CDN}/{revision}/chromium-headless-shell-{build}.zip"


def verify_upstream_inputs(
    *,
    platform: str,
    staging_dir: Path,
    package_json: Path,
    keys_dir: Path,
    pins_path: Path = PINS_TOML,
    fetch: Fetcher = download,
    fetch_json: JsonFetcher = get_json,
    slsa_runner: npm.SlsaRunner | None = None,
    gpg_runner: nodejs.gpg.Runner | None = None,
) -> VerifiedInputs:
    """Fetch and verify playwright-core, Node and Chromium for one platform."""
    staging_dir.mkdir(parents=True, exist_ok=True)
    playwright_version = assemble.pinned_playwright_version(package_json)

    packument = fetch_json(npm_packument_url(PLAYWRIGHT_PACKAGE))
    dist = npm.packument_dist(packument, playwright_version)
    playwright_tarball = (
        staging_dir / f"{PLAYWRIGHT_PACKAGE}-{playwright_version}.tgz"
    )
    fetch(dist.tarball, playwright_tarball)
    npm.verify_registry_signature(
        message=npm.signed_message(
            PLAYWRIGHT_PACKAGE, playwright_version, dist.integrity
        ),
        signature_b64=dist.signature_b64,
        public_key_pem=(keys_dir / "npm-registry.pem").read_bytes(),
    )
    npm.assert_integrity_binds_tarball(
        integrity=dist.integrity, tarball=playwright_tarball
    )
    bundle = npm.provenance_bundle(fetch_json(dist.attestations_url))
    bundle_path = staging_dir / f"{PLAYWRIGHT_PACKAGE}.slsa-bundle.json"
    bundle_path.write_text(json.dumps(bundle))
    npm.verify_slsa(
        tarball=playwright_tarball,
        bundle=bundle_path,
        identity_regexp=PLAYWRIGHT_PROVENANCE_IDENTITY,
        oidc_issuer=GITHUB_ACTIONS_OIDC_ISSUER,
        runner=slsa_runner,
    )

    node_version = pins.node_version(pins_path)
    shasums = staging_dir / "SHASUMS256.txt"
    fetch(node_shasums_url(node_version), shasums)
    # The detached `.sig`, not the clearsigned `.asc`: the verifier checks a
    # detached signature over SHASUMS256.txt, and Node's `.asc` embeds the
    # checksums inline (a clearsigned document), which is not a detached sig.
    signature = staging_dir / "SHASUMS256.txt.sig"
    fetch(node_shasums_url(node_version) + ".sig", signature)
    node_name = node_tarball_name(node_version, platform)
    node_tarball = staging_dir / node_name
    fetch(node_tarball_url(node_version, platform), node_tarball)
    nodejs.verify_node_runtime(
        tarball=node_tarball,
        filename=node_name,
        shasums=shasums,
        signature=signature,
        keyring=keys_dir / "nodejs-release.asc",
        fingerprints=nodejs.NODE_RELEASE_FINGERPRINTS,
        runner=gpg_runner,
    )

    revision = pins.chromium_revision(pins_path)
    chromium_archive = staging_dir / f"chromium-headless-shell-{platform}.zip"
    fetch(chromium_url(revision, platform), chromium_archive)
    chromium.assert_chromium_bytes(
        chromium_archive, platform=platform, pins_path=pins_path
    )

    return VerifiedInputs(
        playwright_tarball=playwright_tarball,
        node_tarball=node_tarball,
        node_tarball_name=node_name,
        chromium_archive=chromium_archive,
        playwright_version=playwright_version,
        chromium_revision=revision,
        node_version=node_version,
    )
