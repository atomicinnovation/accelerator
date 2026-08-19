"""The attestation document's field set matches the launcher's reader."""

import json
import re
from pathlib import Path

from tasks.vendor.archive import ArchiveStats
from tasks.vendor.attestation import build_attestation

_REPO = Path(__file__).resolve().parents[3]
_ATTESTATION_RS = (
    _REPO / "cli/launcher/src/launch/outbound/resolve/tree/attestation.rs"
)


def _stats() -> ArchiveStats:
    return ArchiveStats(
        archive_sha256="a" * 64,
        archive_size=42,
        uncompressed_size=185790464,
        entry_count=14,
        table_sha256="b" * 64,
    )


def test_the_document_carries_exactly_the_reader_s_fields() -> None:
    document = json.loads(build_attestation("browser", "linux-x64", _stats()))
    assert document == {
        "attestation_format_version": 1,
        "artifact": "browser",
        "platform": "linux-x64",
        "archive_sha256": "a" * 64,
        "uncompressed_size": 185790464,
        "entry_count": 14,
        "table_sha256": "b" * 64,
    }


def test_no_release_or_layout_version_leaks_into_the_body() -> None:
    document = json.loads(build_attestation("browser", "linux-x64", _stats()))
    assert "release_version" not in document
    assert "layout_version" not in document


def test_the_field_names_appear_in_the_rust_reader_struct() -> None:
    # A cheap drift guard: every emitted key names a field of the Rust
    # Attestation struct, so a rename on one side fails here.
    document = json.loads(build_attestation("browser", "linux-x64", _stats()))
    source = _ATTESTATION_RS.read_text()
    fields = set(re.findall(r"pub (\w+):", source))
    assert set(document) <= fields, set(document) - fields


def test_the_bytes_are_stable_and_newline_terminated() -> None:
    first = build_attestation("driver", "darwin-arm64", _stats())
    second = build_attestation("driver", "darwin-arm64", _stats())
    assert first == second
    assert first.endswith(b"\n")
