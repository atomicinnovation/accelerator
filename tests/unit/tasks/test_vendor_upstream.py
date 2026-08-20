"""Fetch-and-verify orchestration, exercised with injected fetchers.

No live network: the JSON and file fetchers are injected and return recorded
fixtures, so the wiring of npm/nodejs/chromium verification is tested without
the registry, nodejs.org or the CDN. The URL builders are pinned against their
documented formats.
"""

import base64
import hashlib

import pytest

from tasks.vendor import upstream


def test_the_npm_packument_url_names_the_package():
    assert upstream.npm_packument_url("playwright-core").endswith(
        "/playwright-core"
    )


def test_the_node_urls_carry_the_version():
    assert upstream.node_shasums_url("20.11.1").endswith(
        "/v20.11.1/SHASUMS256.txt"
    )
    assert upstream.node_shasums_url("20.11.1").startswith("https://")
    assert (
        upstream.node_tarball_name("20.11.1", "linux-x64")
        == "node-v20.11.1-linux-x64.tar.gz"
    )


def test_verify_upstream_inputs_wires_the_three_checks(tmp_path, mocker):
    version = "1.55.1"
    revision = "1181"
    node_version = "20.11.1"

    package_json = tmp_path / "package.json"
    package_json.write_text('{"dependencies": {"playwright": "1.55.1"}}\n')

    keys = tmp_path / "keys"
    keys.mkdir()
    (keys / "npm-registry.pem").write_bytes(b"pem")

    pins = tmp_path / "pins.toml"
    node_bytes = b"node tarball"
    chromium_bytes = b"chromium zip"
    pins.write_text(
        f'[chromium]\nrevision = "{revision}"\n\n'
        f"[chromium.sha256]\n"
        f'linux-x64 = "{hashlib.sha256(chromium_bytes).hexdigest()}"\n\n'
        f'[node]\nversion = "{node_version}"\n'
    )

    tarball_bytes = b"playwright-core tarball"
    integrity = (
        "sha512-"
        + base64.b64encode(hashlib.sha512(tarball_bytes).digest()).decode()
    )
    packument = {
        "name": "playwright-core",
        "versions": {
            version: {
                "dist": {
                    "tarball": "https://registry/pw.tgz",
                    "integrity": integrity,
                    "signatures": [{"keyid": "k", "sig": "sig"}],
                    "attestations": {"url": "https://registry/attestations"},
                }
            }
        },
    }
    attestations = {
        "attestations": [
            {
                "predicateType": "https://slsa.dev/provenance/v1",
                "bundle": {"dsseEnvelope": {}},
            }
        ]
    }

    def fetch_json(url):
        return attestations if "attestations" in url else packument

    def fetch(url, dest):
        if "pw.tgz" in url:
            dest.write_bytes(tarball_bytes)
        elif url.endswith((".sig", ".asc")):
            dest.write_text("signature")
        elif "SHASUMS256.txt" in url:
            digest = hashlib.sha256(node_bytes).hexdigest()
            dest.write_text(
                f"{digest}  node-v{node_version}-linux-x64.tar.gz\n"
            )
        elif "node-v" in url:
            dest.write_bytes(node_bytes)
        else:
            dest.write_bytes(chromium_bytes)

    # Stub the verifiers that need real crypto/gpg/browsers.json; the point of
    # this test is the orchestration order and the returned paths.
    mocker.patch.object(upstream.npm, "verify_registry_signature")
    mocker.patch.object(upstream.npm, "assert_integrity_binds_tarball")
    mocker.patch.object(upstream.npm, "verify_slsa")
    mocker.patch.object(upstream.nodejs, "verify_node_runtime")
    mocker.patch.object(upstream.chromium, "assert_chromium_bytes")

    result = upstream.verify_upstream_inputs(
        platform="linux-x64",
        staging_dir=tmp_path / "in",
        package_json=package_json,
        keys_dir=keys,
        pins_path=pins,
        fetch=fetch,
        fetch_json=fetch_json,
        slsa_runner=lambda _argv: 0,
        gpg_runner=lambda _s, _t, _k: ["[GNUPG:] GOODSIG"],
    )

    assert result.playwright_version == version
    assert result.playwright_tarball.read_bytes() == tarball_bytes
    assert result.node_tarball.read_bytes() == node_bytes
    assert result.chromium_archive.read_bytes() == chromium_bytes
    upstream.npm.verify_slsa.assert_called_once()
    upstream.nodejs.verify_node_runtime.assert_called_once()
    upstream.chromium.assert_chromium_bytes.assert_called_once()


def test_a_pin_mismatched_playwright_version_fails_before_fetching(tmp_path):
    package_json = tmp_path / "package.json"
    package_json.write_text('{"dependencies": {"playwright": "^1.55.1"}}\n')
    with pytest.raises(ValueError, match="exact"):
        upstream.verify_upstream_inputs(
            platform="linux-x64",
            staging_dir=tmp_path / "in",
            package_json=package_json,
            keys_dir=tmp_path / "keys",
            pins_path=tmp_path / "pins.toml",
            fetch=lambda _u, _d: None,
            fetch_json=lambda _u: {},
        )
