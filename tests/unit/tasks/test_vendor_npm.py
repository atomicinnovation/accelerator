"""playwright-core provenance: npm registry signature, sha512 binding, SLSA.

The registry signature proves only that the registry asserted an integrity
value, so the binding — the tarball's own sha512 equals the integrity inside the
signed message — is what completes it; without it the npm guarantee collapses
onto SLSA alone. The SLSA runner is injected and its argv pinned, so dropping a
predicate flag fails here rather than leaving every SLSA test green.
"""

import base64
import hashlib

import pytest
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec

from tasks.vendor.npm import (
    assert_integrity_binds_tarball,
    signed_message,
    verify_registry_signature,
    verify_slsa,
)


def _keypair():
    private = ec.generate_private_key(ec.SECP256R1())
    pem = private.public_key().public_bytes(
        serialization.Encoding.PEM,
        serialization.PublicFormat.SubjectPublicKeyInfo,
    )
    return private, pem


def _integrity(data):
    return "sha512-" + base64.b64encode(hashlib.sha512(data).digest()).decode()


def test_a_valid_registry_signature_verifies(tmp_path):
    private, pem = _keypair()
    message = signed_message("playwright-core", "1.55.1", _integrity(b"x"))
    signature = private.sign(message.encode(), ec.ECDSA(hashes.SHA256()))
    verify_registry_signature(
        message=message,
        signature_b64=base64.b64encode(signature).decode(),
        public_key_pem=pem,
    )


def test_a_tampered_message_fails_verification(tmp_path):
    private, pem = _keypair()
    message = signed_message("playwright-core", "1.55.1", _integrity(b"x"))
    signature = private.sign(message.encode(), ec.ECDSA(hashes.SHA256()))
    with pytest.raises(ValueError, match="does not verify"):
        verify_registry_signature(
            message=signed_message(
                "playwright-core", "9.9.9", _integrity(b"x")
            ),
            signature_b64=base64.b64encode(signature).decode(),
            public_key_pem=pem,
        )


def test_a_signature_from_another_key_fails(tmp_path):
    private, _pem = _keypair()
    _other, other_pem = _keypair()
    message = signed_message("playwright-core", "1.55.1", _integrity(b"x"))
    signature = private.sign(message.encode(), ec.ECDSA(hashes.SHA256()))
    with pytest.raises(ValueError, match="does not verify"):
        verify_registry_signature(
            message=message,
            signature_b64=base64.b64encode(signature).decode(),
            public_key_pem=other_pem,
        )


def test_matching_integrity_binds_the_tarball(tmp_path):
    data = b"playwright-core tarball"
    tarball = tmp_path / "pw.tgz"
    tarball.write_bytes(data)
    assert_integrity_binds_tarball(integrity=_integrity(data), tarball=tarball)


def test_a_tarball_not_matching_the_signed_integrity_fails(tmp_path):
    tarball = tmp_path / "pw.tgz"
    tarball.write_bytes(b"tampered")
    with pytest.raises(ValueError, match="sha512"):
        assert_integrity_binds_tarball(
            integrity=_integrity(b"original"), tarball=tarball
        )


def test_a_non_sha512_integrity_is_refused(tmp_path):
    tarball = tmp_path / "pw.tgz"
    tarball.write_bytes(b"x")
    with pytest.raises(ValueError, match="sha512"):
        assert_integrity_binds_tarball(integrity="sha1-abc", tarball=tarball)


def test_the_slsa_argv_is_pinned(tmp_path):
    tarball = tmp_path / "pw.tgz"
    tarball.write_bytes(b"x")
    bundle = tmp_path / "b.json"
    bundle.write_text("{}")
    captured = {}

    def runner(argv):
        captured["argv"] = argv
        return 0

    verify_slsa(
        tarball=tarball,
        bundle=bundle,
        identity_regexp="^https://github.com/microsoft/playwright/",
        oidc_issuer="https://token.actions.githubusercontent.com",
        runner=runner,
    )
    argv = captured["argv"]
    assert argv[:2] == ["cosign", "verify-blob-attestation"]
    assert "--bundle" in argv and str(bundle) in argv
    assert "--new-bundle-format" in argv
    assert "--certificate-identity-regexp" in argv
    assert "^https://github.com/microsoft/playwright/" in argv
    assert "--certificate-oidc-issuer" in argv
    assert "https://token.actions.githubusercontent.com" in argv
    assert argv[argv.index("--type") + 1] == "https://slsa.dev/provenance/v1"
    assert str(tarball) in argv


def test_a_failing_slsa_check_fails_the_release(tmp_path):
    tarball = tmp_path / "pw.tgz"
    tarball.write_bytes(b"x")
    bundle = tmp_path / "b.json"
    bundle.write_text("{}")
    with pytest.raises(ValueError, match="SLSA"):
        verify_slsa(
            tarball=tarball,
            bundle=bundle,
            identity_regexp="^https://github.com/microsoft/playwright/",
            oidc_issuer="https://token.actions.githubusercontent.com",
            runner=lambda _argv: 1,
        )


def _packument(version="1.55.1", *, signatures=True, attestations=True):
    dist = {
        "tarball": (
            "https://registry.npmjs.org/playwright-core/-/"
            f"playwright-core-{version}.tgz"
        ),
        "integrity": "sha512-abc",
    }
    if signatures:
        dist["signatures"] = [{"keyid": "SHA256:key", "sig": "MEUCIQ=="}]
    if attestations:
        dist["attestations"] = {
            "url": f"https://registry.npmjs.org/-/npm/v1/attestations/"
            f"playwright-core@{version}"
        }
    return {"name": "playwright-core", "versions": {version: {"dist": dist}}}


def test_packument_dist_extracts_the_signed_fields():
    from tasks.vendor.npm import packument_dist

    dist = packument_dist(_packument(), "1.55.1")
    assert dist.tarball.endswith("playwright-core-1.55.1.tgz")
    assert dist.integrity == "sha512-abc"
    assert dist.signature_b64 == "MEUCIQ=="
    assert dist.attestations_url.endswith("playwright-core@1.55.1")


def test_a_version_absent_from_the_packument_raises():
    from tasks.vendor.npm import packument_dist

    with pytest.raises(ValueError, match="absent"):
        packument_dist(_packument(), "1.99.0")


def test_an_unsigned_version_is_refused():
    from tasks.vendor.npm import packument_dist

    with pytest.raises(ValueError, match="signature"):
        packument_dist(_packument(signatures=False), "1.55.1")


def test_a_version_without_attestations_is_refused():
    from tasks.vendor.npm import packument_dist

    with pytest.raises(ValueError, match="attestations"):
        packument_dist(_packument(attestations=False), "1.55.1")


def test_provenance_bundle_selects_the_slsa_attestation():
    from tasks.vendor.npm import provenance_bundle

    doc = {
        "attestations": [
            {"predicateType": "https://github.com/npm/...", "bundle": {"n": 1}},
            {
                "predicateType": "https://slsa.dev/provenance/v1",
                "bundle": {"slsa": True},
            },
        ]
    }
    assert provenance_bundle(doc) == {"slsa": True}


def test_a_document_without_slsa_provenance_is_refused():
    from tasks.vendor.npm import provenance_bundle

    doc = {"attestations": [{"predicateType": "https://github.com/npm/..."}]}
    with pytest.raises(ValueError, match="SLSA"):
        provenance_bundle(doc)
