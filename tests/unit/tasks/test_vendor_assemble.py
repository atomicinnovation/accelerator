"""Assembly: zip extraction that preserves Unix modes and symlinks.

Python's ``zipfile`` does not apply the Unix bits stored in ``external_attr``
and materialises symlink entries as regular files, so a browser tree extracted
naively would lose its executable bit and pass every downstream check before
failing at ``execve``. These assert the reconstruction the launcher's seal then
depends on.
"""

import stat
import zipfile

import pytest

from tasks.vendor.assemble import extract_zip


def _add_file(archive, name, data, mode):
    info = zipfile.ZipInfo(name)
    info.external_attr = mode << 16
    archive.writestr(info, data)


def _add_symlink(archive, name, target):
    info = zipfile.ZipInfo(name)
    info.external_attr = (stat.S_IFLNK | 0o777) << 16
    archive.writestr(info, target)


def _browser_zip(path):
    with zipfile.ZipFile(path, "w") as archive:
        _add_file(archive, "chrome-headless-shell", b"\x7fELF binary", 0o755)
        _add_file(archive, "resources/data.pak", b"resource bytes", 0o644)
        _add_symlink(archive, "libEGL.so", "resources/data.pak")
    return path


def test_the_executable_bit_survives_extraction(tmp_path):
    extract_zip(_browser_zip(tmp_path / "b.zip"), tmp_path / "out")
    shell = tmp_path / "out" / "chrome-headless-shell"
    assert shell.stat().st_mode & 0o111


def test_a_non_executable_file_keeps_its_mode(tmp_path):
    extract_zip(_browser_zip(tmp_path / "b.zip"), tmp_path / "out")
    pak = tmp_path / "out" / "resources" / "data.pak"
    assert not pak.stat().st_mode & 0o111


def test_a_symlink_is_reconstructed_not_flattened(tmp_path):
    extract_zip(_browser_zip(tmp_path / "b.zip"), tmp_path / "out")
    link = tmp_path / "out" / "libEGL.so"
    assert link.is_symlink()
    assert link.readlink().as_posix() == "resources/data.pak"


def test_an_absolute_member_path_is_refused(tmp_path):
    with zipfile.ZipFile(tmp_path / "evil.zip", "w") as archive:
        _add_file(archive, "/etc/passwd", b"x", 0o644)
    with pytest.raises(ValueError, match="escapes"):
        extract_zip(tmp_path / "evil.zip", tmp_path / "out")


def test_a_parent_traversal_member_is_refused(tmp_path):
    with zipfile.ZipFile(tmp_path / "evil.zip", "w") as archive:
        _add_file(archive, "../escape", b"x", 0o644)
    with pytest.raises(ValueError, match="escapes"):
        extract_zip(tmp_path / "evil.zip", tmp_path / "out")


def test_an_escaping_symlink_target_is_refused(tmp_path):
    with zipfile.ZipFile(tmp_path / "evil.zip", "w") as archive:
        _add_symlink(archive, "link", "../../etc/passwd")
    with pytest.raises(ValueError, match="escapes"):
        extract_zip(tmp_path / "evil.zip", tmp_path / "out")
