"""Deterministic archives and the ``.files`` table's wire format.

Each normalisation is asserted with a negative case, since "assemble twice and
compare" alone is invariant to the very factors that threaten reproducibility.
"""

import hashlib
import tarfile

from tasks.vendor.archive import (
    TABLE_NAME,
    write_deterministic_archive,
)


def _tree(root):
    root.mkdir(parents=True, exist_ok=True)
    (root / "lib").mkdir()
    (root / "lib" / "data.pak").write_bytes(b"resource bytes")
    shell = root / "node"
    shell.write_bytes(b"#!/bin/sh\n")
    shell.chmod(0o755)
    (root / "lib" / "current").symlink_to("data.pak")
    return root


def test_the_table_is_the_first_member_and_names_no_row_for_itself(tmp_path):
    dest = tmp_path / "out.tar.gz"
    write_deterministic_archive(_tree(tmp_path / "tree"), dest)
    with tarfile.open(dest, "r:gz") as archive:
        names = archive.getnames()
    assert names[0] == TABLE_NAME
    assert names.count(TABLE_NAME) == 1


def test_the_table_rows_match_the_launcher_wire_format(tmp_path):
    write_deterministic_archive(
        _tree(tmp_path / "tree"), tmp_path / "out.tar.gz"
    )
    with tarfile.open(tmp_path / "out.tar.gz", "r:gz") as archive:
        table = archive.extractfile(TABLE_NAME).read().decode()
    lines = table.splitlines()
    assert lines[0] == "version 1"
    rows = {line.split("\t")[4]: line.split("\t") for line in lines[1:]}
    # A directory: mode 755, dash digest, no link field.
    assert rows["lib"][0:4] == ["d", "755", "0", "-"]
    # An executable file: mode 755, 64-hex digest.
    node = rows["node"]
    assert node[0] == "f"
    assert node[1] == "755"
    assert len(node[3]) == 64
    # A symlink: kind l, dash digest, a link-target field.
    current = rows["lib/current"]
    assert current[0] == "l"
    assert current[3] == "-"
    assert current[5] == "data.pak"


def test_a_non_executable_file_is_recorded_0644(tmp_path):
    write_deterministic_archive(
        _tree(tmp_path / "tree"), tmp_path / "out.tar.gz"
    )
    with tarfile.open(tmp_path / "out.tar.gz", "r:gz") as archive:
        table = archive.extractfile(TABLE_NAME).read().decode()
    row = next(
        line for line in table.splitlines() if line.endswith("lib/data.pak")
    )
    assert row.split("\t")[1] == "644"


def test_the_digest_matches_the_file_bytes(tmp_path):
    write_deterministic_archive(
        _tree(tmp_path / "tree"), tmp_path / "out.tar.gz"
    )
    with tarfile.open(tmp_path / "out.tar.gz", "r:gz") as archive:
        table = archive.extractfile(TABLE_NAME).read().decode()
    row = next(
        line for line in table.splitlines() if line.endswith("lib/data.pak")
    )
    assert row.split("\t")[3] == hashlib.sha256(b"resource bytes").hexdigest()


def test_two_assemblies_of_one_tree_are_byte_identical(tmp_path):
    first = tmp_path / "a.tar.gz"
    second = tmp_path / "b.tar.gz"
    write_deterministic_archive(_tree(tmp_path / "tree"), first)
    write_deterministic_archive(_tree(tmp_path / "tree2"), second)
    # tree2 has different inode/timestamps; the archives must still match.
    assert first.read_bytes() == second.read_bytes()


def test_a_shuffled_directory_still_produces_the_same_bytes(tmp_path):
    # Sorted emission makes the archive invariant to readdir order.
    dest_a = tmp_path / "a.tar.gz"
    stats_a = write_deterministic_archive(_tree(tmp_path / "t1"), dest_a)

    root = tmp_path / "t2"
    # Create the same tree with children written in a different order.
    root.mkdir()
    (root / "node").write_bytes(b"#!/bin/sh\n")
    (root / "node").chmod(0o755)
    (root / "lib").mkdir()
    (root / "lib" / "current").symlink_to("data.pak")
    (root / "lib" / "data.pak").write_bytes(b"resource bytes")
    dest_b = tmp_path / "b.tar.gz"
    stats_b = write_deterministic_archive(root, dest_b)

    assert dest_a.read_bytes() == dest_b.read_bytes()
    assert stats_a == stats_b


def test_the_gzip_member_carries_no_timestamp(tmp_path):
    dest = tmp_path / "out.tar.gz"
    write_deterministic_archive(_tree(tmp_path / "tree"), dest)
    # gzip header bytes 4-7 are the mtime; they must be zero.
    header = dest.read_bytes()[:8]
    assert header[4:8] == b"\x00\x00\x00\x00"


def test_stats_report_the_uncompressed_size_and_entry_count(tmp_path):
    stats = write_deterministic_archive(
        _tree(tmp_path / "tree"), tmp_path / "out.tar.gz"
    )
    # lib, lib/data.pak, node, lib/current = 4 entries.
    assert stats.entry_count == 4
    # Only file bytes count toward the uncompressed size.
    expected = len(b"resource bytes") + len(b"#!/bin/sh\n")
    assert stats.uncompressed_size == expected
    assert len(stats.archive_sha256) == 64
    assert len(stats.table_sha256) == 64


def test_the_table_digest_matches_the_embedded_table(tmp_path):
    stats = write_deterministic_archive(
        _tree(tmp_path / "tree"), tmp_path / "out.tar.gz"
    )
    with tarfile.open(tmp_path / "out.tar.gz", "r:gz") as archive:
        table = archive.extractfile(TABLE_NAME).read()
    assert stats.table_sha256 == hashlib.sha256(table).hexdigest()
