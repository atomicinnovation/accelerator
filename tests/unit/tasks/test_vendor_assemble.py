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


def _miniature_specs(tmp_path):
    from tasks.vendor.assemble import NoticeSource, TreePlacement, TreeSpec

    node = _executable(tmp_path / "in" / "node", "#!/bin/sh\necho v20\n")
    core = tmp_path / "in" / "playwright-core"
    _licence(core / "index.js", "module.exports = {}")
    shell = _executable(
        tmp_path / "in" / "chrome-headless-shell", "#!/bin/sh\necho v1181\n"
    )
    lic = _licence(tmp_path / "in" / "LICENSE")
    driver = TreeSpec(
        artifact="driver",
        placements=(
            TreePlacement(node, "node"),
            TreePlacement(core, "node_modules/playwright-core"),
        ),
        notices=(
            NoticeSource("node", (lic,)),
            NoticeSource("playwright-core", (lic,)),
        ),
    )
    browser = TreeSpec(
        artifact="browser",
        placements=(TreePlacement(shell, "chrome-headless-shell"),),
        notices=(NoticeSource("chromium", (lic,)),),
    )
    return (driver, browser)


def test_assembly_produces_flat_named_archives(tmp_path):
    from tasks.vendor.assemble import assemble_specs

    stats = assemble_specs(
        _miniature_specs(tmp_path),
        platform="linux-x64",
        staging_dir=tmp_path / "staging",
        dist_dir=tmp_path / "dist",
    )
    assert (tmp_path / "dist" / "accelerator-driver-linux-x64.tar.gz").exists()
    assert (tmp_path / "dist" / "accelerator-browser-linux-x64.tar.gz").exists()
    assert set(stats) == {"driver", "browser"}


def test_assembling_the_same_specs_twice_is_byte_identical(tmp_path):
    from tasks.vendor.assemble import assemble_specs

    assemble_specs(
        _miniature_specs(tmp_path),
        platform="linux-x64",
        staging_dir=tmp_path / "s1",
        dist_dir=tmp_path / "d1",
    )
    assemble_specs(
        _miniature_specs(tmp_path),
        platform="linux-x64",
        staging_dir=tmp_path / "s2",
        dist_dir=tmp_path / "d2",
    )
    for name in ("driver", "browser"):
        asset = f"accelerator-{name}-linux-x64.tar.gz"
        assert (tmp_path / "d1" / asset).read_bytes() == (
            tmp_path / "d2" / asset
        ).read_bytes()


def _pins_toml(path, digests):
    lines = []
    for artifact, platforms in digests.items():
        lines.append(f"[assembled_sha256.{artifact}]")
        for platform, digest in platforms.items():
            lines.append(f'{platform} = "{digest}"')
    path.write_text("\n".join(lines) + "\n")
    return path


def test_a_matching_archive_passes_the_pin_gate(tmp_path):
    from tasks.vendor.assemble import assemble_specs, assert_matches_pin

    stats = assemble_specs(
        _miniature_specs(tmp_path),
        platform="linux-x64",
        staging_dir=tmp_path / "staging",
        dist_dir=tmp_path / "dist",
    )
    pins = _pins_toml(
        tmp_path / "pins.toml",
        {
            "driver": {"linux-x64": stats["driver"].archive_sha256},
            "browser": {"linux-x64": stats["browser"].archive_sha256},
        },
    )
    assert_matches_pin(
        tmp_path / "dist" / "accelerator-driver-linux-x64.tar.gz",
        artifact="driver",
        platform="linux-x64",
        pins_path=pins,
    )


def test_a_mismatched_archive_fails_the_pin_gate(tmp_path):
    from tasks.vendor.assemble import assemble_specs, assert_matches_pin

    assemble_specs(
        _miniature_specs(tmp_path),
        platform="linux-x64",
        staging_dir=tmp_path / "staging",
        dist_dir=tmp_path / "dist",
    )
    pins = _pins_toml(
        tmp_path / "pins.toml", {"driver": {"linux-x64": "00" * 32}}
    )
    with pytest.raises(ValueError, match="!= pinned"):
        assert_matches_pin(
            tmp_path / "dist" / "accelerator-driver-linux-x64.tar.gz",
            artifact="driver",
            platform="linux-x64",
            pins_path=pins,
        )


def _tgz(path, tree):
    import tarfile

    with tarfile.open(path, "w:gz") as archive:
        archive.add(tree, arcname=".", recursive=True)
    return path


def _zip_dir(path, tree):
    import stat as _stat
    import zipfile

    with zipfile.ZipFile(path, "w") as archive:
        for item in sorted(tree.rglob("*")):
            rel = item.relative_to(tree).as_posix()
            info = zipfile.ZipInfo(rel)
            mode = item.stat().st_mode
            info.external_attr = (mode & 0o7777) << 16
            archive.writestr(info, item.read_bytes() if item.is_file() else b"")
            if item.is_file() and mode & 0o111:
                info.external_attr = (_stat.S_IFREG | 0o755) << 16
    return path


def _miniature_inputs(tmp_path):
    node_tree = tmp_path / "node-src"
    _executable(node_tree / "node", "#!/bin/sh\necho v20\n")
    pw_tree = tmp_path / "pw-src"
    _licence(pw_tree / "index.js", "module.exports = {}")
    chromium_tree = tmp_path / "chromium-src"
    _executable(
        chromium_tree / "chrome-headless-shell", "#!/bin/sh\necho v1181\n"
    )
    return (
        _tgz(tmp_path / "node.tar.gz", node_tree),
        _tgz(tmp_path / "pw.tgz", pw_tree),
        _zip_dir(tmp_path / "chromium.zip", chromium_tree),
    )


def _mini_spec_builder(tmp_path):
    from tasks.vendor.assemble import NoticeSource, TreePlacement, TreeSpec

    lic = _licence(tmp_path / "LICENSE")

    def build(extracted):
        driver = TreeSpec(
            artifact="driver",
            placements=(
                TreePlacement(extracted.node / "node", "node"),
                TreePlacement(
                    extracted.playwright_core, "node_modules/playwright-core"
                ),
            ),
            notices=(
                NoticeSource("node", (lic,)),
                NoticeSource("playwright-core", (lic,)),
            ),
            executables=("node",),
        )
        browser = TreeSpec(
            artifact="browser",
            placements=(
                TreePlacement(
                    extracted.chromium / "chrome-headless-shell",
                    "chrome-headless-shell",
                ),
            ),
            notices=(NoticeSource("chromium", (lic,)),),
            executables=("chrome-headless-shell",),
        )
        return (driver, browser)

    return build


def test_assemble_tree_artifacts_produces_archives_and_attestations(tmp_path):
    from tasks.vendor.assemble import assemble_tree_artifacts

    node_tar, pw_tar, chromium_zip = _miniature_inputs(tmp_path)
    stats = assemble_tree_artifacts(
        playwright_tarball=pw_tar,
        node_tarball=node_tar,
        chromium_archive=chromium_zip,
        platform="linux-x64",
        staging_dir=tmp_path / "staging",
        dist_dir=tmp_path / "dist",
        spec_builder=_mini_spec_builder(tmp_path),
    )
    assert set(stats) == {"driver", "browser"}
    for name in ("driver", "browser"):
        archive = tmp_path / "dist" / f"accelerator-{name}-linux-x64.tar.gz"
        assert archive.exists()
        assert archive.with_name(archive.name + ".sealed").exists()


def test_assemble_tree_artifacts_runs_the_structural_and_smoke_gates(tmp_path):
    from tasks.vendor.assemble import assemble_tree_artifacts

    node_tar, pw_tar, _chromium_zip = _miniature_inputs(tmp_path)
    # A browser whose shell will not execute must fail the assembly.
    broken = tmp_path / "chromium-broken"
    (broken).mkdir()
    (broken / "chrome-headless-shell").write_bytes(b"\x00not a program")
    (broken / "chrome-headless-shell").chmod(0o755)
    broken_zip = _zip_dir(tmp_path / "broken.zip", broken)

    with pytest.raises(ValueError, match="did not execute"):
        assemble_tree_artifacts(
            playwright_tarball=pw_tar,
            node_tarball=node_tar,
            chromium_archive=broken_zip,
            platform="linux-x64",
            staging_dir=tmp_path / "staging",
            dist_dir=tmp_path / "dist",
            spec_builder=_mini_spec_builder(tmp_path),
        )
