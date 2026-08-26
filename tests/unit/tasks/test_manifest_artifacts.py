"""The manifest carries tree artifacts beside binaries.

collect_artifact_entries reads each staged archive's sha256, its inline
`.minisig`, its byte size, and the extraction bounds from its `.sealed`
attestation — so producer and consumer cannot disagree about the bounds the
launcher enforces. build_manifest keeps `artifacts` additive: a two-argument
call emits no key at all, so an older launcher reading the manifest is
unaffected.
"""

from tasks.manifest import (
    build_manifest,
    collect_artifact_entries,
)
from tasks.shared.hashing import compute_sha256
from tasks.shared.paths import tree_artifact_asset_path
from tasks.shared.vendor.archive import write_deterministic_archive
from tasks.shared.vendor.attestation import build_attestation

_REAL_VERSION = "0.44.0"


def _stage_artifact(staging, name, platform):
    tree = staging / f"{name}-{platform}-tree"
    tree.mkdir(parents=True)
    (tree / "binary").write_bytes(b"#!/bin/sh\n")
    (tree / "binary").chmod(0o755)
    archive = tree_artifact_asset_path(name, platform, staging)
    stats = write_deterministic_archive(tree, archive)
    archive.with_name(archive.name + ".minisig").write_text("untrusted sig\n")
    archive.with_name(archive.name + ".sealed").write_bytes(
        build_attestation(name, platform, stats)
    )
    return stats


def test_collect_artifact_entries_reads_sizes_from_the_attestation(tmp_path):
    stats = _stage_artifact(tmp_path, "driver", "linux-x64")
    entries = collect_artifact_entries(
        ("driver",), staging_dir=tmp_path, platforms=(("_", "linux-x64"),)
    )
    row = entries["driver"].platforms["linux-x64"]
    archive = tree_artifact_asset_path("driver", "linux-x64", tmp_path)
    assert row["sha256"] == compute_sha256(archive)
    assert row["signature"] == "untrusted sig\n"
    assert row["archive_size"] == archive.stat().st_size
    assert row["uncompressed_size"] == stats.uncompressed_size
    assert row["entry_count"] == stats.entry_count


def test_build_manifest_omits_artifacts_when_none_given(tmp_path):
    manifest = build_manifest(_REAL_VERSION, {})
    assert "artifacts" not in manifest


def test_build_manifest_carries_artifacts_when_given(tmp_path):
    _stage_artifact(tmp_path, "driver", "linux-x64")
    entries = collect_artifact_entries(
        ("driver",), staging_dir=tmp_path, platforms=(("_", "linux-x64"),)
    )
    manifest = build_manifest(_REAL_VERSION, {}, artifacts=entries)
    assert set(manifest["artifacts"]) == {"driver"}
    row = manifest["artifacts"]["driver"]["platforms"]["linux-x64"]
    assert set(row) == {
        "sha256",
        "signature",
        "archive_size",
        "uncompressed_size",
        "entry_count",
    }
