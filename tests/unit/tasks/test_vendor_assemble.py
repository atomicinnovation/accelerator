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


def _package_json(path, version="1.55.1"):
    path.write_text('{"dependencies": {"playwright": "' + version + '"}}\n')
    return path


def _browsers_json(path, revision="1181"):
    path.write_text(
        '{"browsers": ['
        '{"name": "chromium", "revision": "' + revision + '"},'
        '{"name": "chromium-headless-shell", "revision": "' + revision + '"},'
        '{"name": "ffmpeg", "revision": "1011"}'
        "]}\n"
    )
    return path


def test_the_pinned_playwright_version_is_read_from_dependencies(tmp_path):
    from tasks.vendor.assemble import read_pinned_playwright_version

    version = read_pinned_playwright_version(
        _package_json(tmp_path / "package.json")
    )
    assert version == "1.55.1"


def test_a_caret_ranged_playwright_pin_is_refused(tmp_path):
    from tasks.vendor.assemble import read_pinned_playwright_version

    with pytest.raises(ValueError, match="exact"):
        read_pinned_playwright_version(
            _package_json(tmp_path / "package.json", "^1.55.1")
        )


def test_the_headless_shell_revision_is_read_from_browsers_json(tmp_path):
    from tasks.vendor.assemble import browser_revision

    revision = browser_revision(
        _browsers_json(tmp_path / "browsers.json"),
        "chromium-headless-shell",
    )
    assert revision == "1181"


def test_a_matching_pairing_passes_the_guard(tmp_path):
    from tasks.vendor.assemble import assert_version_pairing

    assert_version_pairing(
        fetched_playwright_version="1.55.1",
        expected_playwright_version="1.55.1",
        fetched_chromium_revision="1181",
        expected_chromium_revision="1181",
    )


def test_a_playwright_version_mismatch_fails_the_release(tmp_path):
    from tasks.vendor.assemble import assert_version_pairing

    with pytest.raises(ValueError, match="playwright"):
        assert_version_pairing(
            fetched_playwright_version="1.55.2",
            expected_playwright_version="1.55.1",
            fetched_chromium_revision="1181",
            expected_chromium_revision="1181",
        )


def test_a_chromium_revision_mismatch_fails_the_release(tmp_path):
    from tasks.vendor.assemble import assert_version_pairing

    with pytest.raises(ValueError, match="Chromium"):
        assert_version_pairing(
            fetched_playwright_version="1.55.1",
            expected_playwright_version="1.55.1",
            fetched_chromium_revision="1180",
            expected_chromium_revision="1181",
        )


def _executable(path, body="#!/bin/sh\necho v1.0\n"):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body)
    path.chmod(0o755)
    return path


def _licence(path, text="MIT licence text"):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    return path


def test_notices_get_one_directory_per_component(tmp_path):
    from tasks.vendor.assemble import NoticeSource, write_notices

    tree = tmp_path / "tree"
    tree.mkdir()
    write_notices(
        tree,
        [
            NoticeSource("node", (_licence(tmp_path / "node.LICENSE"),)),
            NoticeSource(
                "playwright-core", (_licence(tmp_path / "pw.LICENSE"),)
            ),
        ],
    )
    assert (tree / "NOTICES" / "node" / "node.LICENSE").read_text()
    assert (tree / "NOTICES" / "playwright-core" / "pw.LICENSE").read_text()


def test_a_component_with_no_licence_files_fails(tmp_path):
    from tasks.vendor.assemble import NoticeSource, write_notices

    tree = tmp_path / "tree"
    tree.mkdir()
    with pytest.raises(ValueError, match="no licence"):
        write_notices(tree, [NoticeSource("node", ())])


def test_staging_a_tree_places_files_and_keeps_the_exec_bit(tmp_path):
    from tasks.vendor.assemble import (
        NoticeSource,
        TreePlacement,
        TreeSpec,
        stage_tree,
    )

    node = _executable(tmp_path / "in" / "node")
    core = tmp_path / "in" / "playwright-core"
    _licence(core / "index.js", "module.exports = {}")
    spec = TreeSpec(
        artifact="driver",
        placements=(
            TreePlacement(node, "node"),
            TreePlacement(core, "node_modules/playwright-core"),
        ),
        notices=(NoticeSource("node", (_licence(tmp_path / "L"),)),),
    )
    dest = tmp_path / "driver"
    stage_tree(spec, dest)
    assert (dest / "node").stat().st_mode & 0o111
    assert (dest / "node_modules" / "playwright-core" / "index.js").exists()
    assert (dest / "NOTICES" / "node" / "L").exists()


def test_the_structural_check_passes_a_well_formed_tree(tmp_path):
    from tasks.vendor.assemble import structural_check

    tree = tmp_path / "tree"
    _executable(tree / "node")
    _licence(tree / "NOTICES" / "node" / "L")
    structural_check(tree, executables=("node",), notice_components=("node",))


def test_the_structural_check_fails_a_non_executable_binary(tmp_path):
    from tasks.vendor.assemble import structural_check

    tree = tmp_path / "tree"
    (tree).mkdir()
    (tree / "node").write_text("plain")
    _licence(tree / "NOTICES" / "node" / "L")
    with pytest.raises(ValueError, match="executable"):
        structural_check(
            tree, executables=("node",), notice_components=("node",)
        )


def test_the_structural_check_fails_an_empty_notices_component(tmp_path):
    from tasks.vendor.assemble import structural_check

    tree = tmp_path / "tree"
    _executable(tree / "node")
    (tree / "NOTICES" / "node").mkdir(parents=True)
    with pytest.raises(ValueError, match="NOTICES"):
        structural_check(
            tree, executables=("node",), notice_components=("node",)
        )


def test_the_smoke_check_runs_each_executable(tmp_path):
    from tasks.vendor.assemble import smoke_check

    tree = tmp_path / "tree"
    _executable(tree / "node", "#!/bin/sh\necho v20\n")
    smoke_check(tree, executables=("node",))


def test_the_smoke_check_fails_an_unrunnable_binary(tmp_path):
    from tasks.vendor.assemble import smoke_check

    tree = tmp_path / "tree"
    (tree).mkdir()
    (tree / "node").write_bytes(b"\x00not a program")
    (tree / "node").chmod(0o755)
    with pytest.raises(ValueError, match="did not execute"):
        smoke_check(tree, executables=("node",))
