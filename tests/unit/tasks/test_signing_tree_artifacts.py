"""Signing tree artifacts: re-derive the attestation, then sign both files.

The publishing job signs each archive to `.minisig` and its `.sealed`
attestation to `.sealed.sig`, but only after re-deriving the extraction bounds
by walking the pin-verified archive — so a tampered inter-job artifact cannot
obtain a release-key signature over inflated bounds or a forged table anchor. A
partial assembly fails closed, exactly as a partial cross-compile does.
"""

import pytest

from tasks import signing
from tasks.shared.errors import SigningError
from tasks.shared.paths import tree_artifact_asset_path
from tasks.vendor.archive import write_deterministic_archive
from tasks.vendor.attestation import build_attestation

_ONE_PLATFORM = (("_", "linux-x64"),)


def _stage(staging, name="driver", platform="linux-x64", *, tamper=False):
    tree = staging / f"{name}-{platform}-tree"
    tree.mkdir(parents=True, exist_ok=True)
    (tree / "binary").write_bytes(b"#!/bin/sh\n")
    (tree / "binary").chmod(0o755)
    archive = tree_artifact_asset_path(name, platform, staging)
    stats = write_deterministic_archive(tree, archive)
    if tamper:
        stats = type(stats)(
            archive_sha256=stats.archive_sha256,
            archive_size=stats.archive_size,
            uncompressed_size=stats.uncompressed_size + 999,
            entry_count=stats.entry_count,
            table_sha256=stats.table_sha256,
        )
    archive.with_name(archive.name + ".sealed").write_bytes(
        build_attestation(name, platform, stats)
    )
    return archive


def test_targets_are_one_archive_per_artifact_per_platform():
    targets = signing._tree_artifact_signing_targets()
    assert len(targets) == 8  # 2 artifacts x 4 platforms


def test_a_valid_artifact_signs_the_archive_and_the_sealed(tmp_path, mocker):
    archive = _stage(tmp_path)
    signed = mocker.patch.object(signing, "sign_file")
    signing.sign_tree_artifacts(
        tmp_path / "key.sec",
        tokens=("driver",),
        staging_dir=tmp_path,
        platforms=_ONE_PLATFORM,
    )
    targets = {call.args[1].name for call in signed.call_args_list}
    assert archive.name in targets
    assert archive.name + ".sealed" in targets


def test_a_disagreeing_sealed_fails_closed(tmp_path, mocker):
    _stage(tmp_path, tamper=True)
    mocker.patch.object(signing, "sign_file")
    with pytest.raises(SigningError, match="disagrees"):
        signing.sign_tree_artifacts(
            tmp_path / "key.sec",
            tokens=("driver",),
            staging_dir=tmp_path,
            platforms=_ONE_PLATFORM,
        )


def test_a_missing_sealed_fails_closed(tmp_path, mocker):
    archive = _stage(tmp_path)
    archive.with_name(archive.name + ".sealed").unlink()
    mocker.patch.object(signing, "sign_file")
    with pytest.raises(SigningError, match="sealed"):
        signing.sign_tree_artifacts(
            tmp_path / "key.sec",
            tokens=("driver",),
            staging_dir=tmp_path,
            platforms=_ONE_PLATFORM,
        )


def test_a_missing_archive_fails_closed(tmp_path, mocker):
    mocker.patch.object(signing, "sign_file")
    with pytest.raises(SigningError, match="not found"):
        signing.sign_tree_artifacts(
            tmp_path / "key.sec",
            tokens=("driver",),
            staging_dir=tmp_path,
            platforms=_ONE_PLATFORM,
        )
