import os

import pytest

from tasks.shared.build_measurement.cache_states import BuildCacheState
from tasks.shared.build_measurement.errors import (
    MissingTouchTargetError,
    UnsafeDeletionError,
)
from tasks.shared.build_measurement.locations import RUN_DIRECTORY_ROOT
from tasks.shared.build_measurement.workspace import (
    FilesystemWorkspace,
    delete_from_disk,
)

KERNEL = "cli/kernel/src/lib.rs"
TARGET = "cli/target"
FRONTEND_MODULES = "cli/visualiser/frontend/node_modules"
DOCS_MODULES = "docs-site/node_modules"


def _sentinel(root, directory, name="sentinel", content=b"built"):
    path = root / directory / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(content)
    return path


def _checkout(root):
    _sentinel(root, "cli/kernel/src", "lib.rs", b"pub fn kernel() {}\n")
    _sentinel(root, TARGET, "debug/accelerator", b"x" * 100)
    _sentinel(root, FRONTEND_MODULES, "react/index.js")
    _sentinel(root, DOCS_MODULES, "astro/index.js")
    run_directory = root / RUN_DIRECTORY_ROOT / "build_run"
    run_directory.mkdir(parents=True)
    return run_directory


@pytest.fixture
def run_directory(tmp_path):
    return _checkout(tmp_path)


def _fails_without_removing(path):
    del path
    return PermissionError(13, "Permission denied", "cli/target/debug")


class TestVerification:
    @pytest.mark.parametrize("state", list(BuildCacheState))
    def test_a_well_formed_checkout_verifies(
        self, tmp_path, run_directory, state
    ):
        FilesystemWorkspace(tmp_path).verify(state)

    def test_verifying_a_warm_state_checks_nothing(self, tmp_path):
        (tmp_path / "cli").mkdir()
        (tmp_path / TARGET).symlink_to(tmp_path)
        FilesystemWorkspace(tmp_path).verify(BuildCacheState.WARM_NO_OP)

    @pytest.mark.parametrize("kind", ["missing", "symlink"])
    def test_incremental_refuses_a_missing_touch_target(
        self, tmp_path, run_directory, kind
    ):
        kernel = tmp_path / KERNEL
        kernel.unlink()
        if kind == "symlink":
            real = tmp_path / "elsewhere.rs"
            real.write_text("")
            kernel.symlink_to(real)
        with pytest.raises(
            MissingTouchTargetError,
            match=(
                r"^cli/kernel/src/lib\.rs is missing or not a regular file; "
                r"--cache-state incremental touches it before each timed run$"
            ),
        ):
            FilesystemWorkspace(tmp_path).verify(BuildCacheState.INCREMENTAL)
        assert kernel.is_symlink() or not kernel.exists()

    def test_a_refused_deletion_names_the_symlink_and_its_target(
        self, tmp_path, run_directory
    ):
        elsewhere = tmp_path / "elsewhere"
        elsewhere.mkdir()
        _replace_with_symlink(tmp_path / TARGET, elsewhere)
        with pytest.raises(UnsafeDeletionError) as refusal:
            FilesystemWorkspace(tmp_path).verify(BuildCacheState.COLD)
        assert str(refusal.value) == (
            f"refusing to delete cli/target: it is a symlink to {elsewhere}; "
            "nothing was deleted"
        )

    def test_a_target_reached_through_a_symlinked_ancestor_is_refused(
        self, tmp_path, run_directory, tmp_path_factory
    ):
        outside = tmp_path_factory.mktemp("outside")
        _sentinel(outside, "frontend/node_modules", "react/index.js")
        _replace_with_symlink(tmp_path / "cli/visualiser", outside)
        with pytest.raises(UnsafeDeletionError) as refusal:
            FilesystemWorkspace(tmp_path).verify(BuildCacheState.COLD)
        assert str(refusal.value) == (
            "refusing to delete cli/visualiser/frontend/node_modules: it "
            f"resolves outside {tmp_path.resolve()}; nothing was deleted"
        )


def _replace_with_symlink(path, target):
    _remove(path)
    path.symlink_to(target)


def _remove(path):
    error = delete_from_disk(path)
    assert error is None


class TestPreparation:
    def test_incremental_touches_the_kernel_leaving_its_bytes(
        self, tmp_path, run_directory
    ):
        kernel = tmp_path / KERNEL
        before = kernel.read_bytes()
        os.utime(kernel, (0, 0))
        trash = run_directory / "cold-trash-1"
        assert (
            FilesystemWorkspace(tmp_path).prepare_timed_run(
                BuildCacheState.INCREMENTAL, trash=trash
            )
            is None
        )
        assert kernel.stat().st_mtime > 0
        assert kernel.read_bytes() == before
        assert not trash.exists()

    def test_a_warm_state_prepares_nothing(self, tmp_path, run_directory):
        kernel = tmp_path / KERNEL
        os.utime(kernel, (0, 0))
        FilesystemWorkspace(tmp_path).prepare_timed_run(
            BuildCacheState.WARM_NO_OP, trash=run_directory / "cold-trash-1"
        )
        assert kernel.stat().st_mtime == 0
        assert (tmp_path / TARGET / "debug/accelerator").exists()

    def test_cold_deletes_the_target_and_frontend_node_modules(
        self, tmp_path, run_directory
    ):
        trash = run_directory / "cold-trash-1"
        assert (
            FilesystemWorkspace(tmp_path).prepare_timed_run(
                BuildCacheState.COLD, trash=trash
            )
            is None
        )
        assert not (tmp_path / TARGET).exists()
        assert not (tmp_path / FRONTEND_MODULES).exists()
        assert (tmp_path / DOCS_MODULES / "astro/index.js").exists()
        assert not trash.exists()

    def test_a_missing_directory_is_not_an_error(self, tmp_path, run_directory):
        _remove(tmp_path / TARGET)
        FilesystemWorkspace(tmp_path).prepare_timed_run(
            BuildCacheState.COLD, trash=run_directory / "cold-trash-1"
        )
        assert not (tmp_path / FRONTEND_MODULES).exists()

    def test_a_symlinked_target_is_refused_before_anything_is_deleted(
        self, tmp_path, run_directory
    ):
        elsewhere = tmp_path / "elsewhere"
        elsewhere.mkdir()
        _replace_with_symlink(tmp_path / FRONTEND_MODULES, elsewhere)
        with pytest.raises(UnsafeDeletionError, match="node_modules"):
            FilesystemWorkspace(tmp_path).prepare_timed_run(
                BuildCacheState.COLD, trash=run_directory / "cold-trash-1"
            )
        assert (tmp_path / TARGET / "debug/accelerator").exists()
        assert not (run_directory / "cold-trash-1").exists()

    def test_a_cold_target_is_moved_aside_before_it_is_removed(
        self, tmp_path, run_directory
    ):
        trash = run_directory / "cold-trash-1"
        unremoved = FilesystemWorkspace(
            tmp_path, remove_tree=_fails_without_removing
        ).prepare_timed_run(BuildCacheState.COLD, trash=trash)
        assert not (tmp_path / TARGET).exists()
        assert (trash / TARGET / "debug/accelerator").read_bytes() == (
            b"x" * 100
        )
        assert unremoved is not None
        assert unremoved.path == trash.absolute()
        assert unremoved.size_bytes >= 100
        assert unremoved.cause == (
            "[Errno 13] Permission denied: 'cli/target/debug'"
        )

    def test_trash_that_remains_without_a_reported_failure_is_unremoved(
        self, tmp_path, run_directory
    ):
        trash = run_directory / "cold-trash-1"
        unremoved = FilesystemWorkspace(
            tmp_path, remove_tree=lambda path: None
        ).prepare_timed_run(BuildCacheState.COLD, trash=trash)
        assert unremoved is not None
        assert unremoved.cause == "the directory remained after removal"

    def test_a_trash_directory_outside_the_run_root_is_refused(
        self, tmp_path, run_directory
    ):
        with pytest.raises(UnsafeDeletionError, match="outside"):
            FilesystemWorkspace(tmp_path).prepare_timed_run(
                BuildCacheState.COLD, trash=tmp_path / "cold-trash-1"
            )
        assert (tmp_path / TARGET).exists()

    def test_an_existing_trash_directory_is_never_reused(
        self, tmp_path, run_directory
    ):
        trash = run_directory / "cold-trash-1"
        trash.mkdir()
        with pytest.raises(FileExistsError):
            FilesystemWorkspace(tmp_path).prepare_timed_run(
                BuildCacheState.COLD, trash=trash
            )
        assert (tmp_path / TARGET).exists()


@pytest.mark.skipif(
    hasattr(os, "geteuid") and os.geteuid() == 0,
    reason="no permission stops root",
)
def test_delete_from_disk_returns_the_first_failure(tmp_path):
    locked = tmp_path / "tree" / "locked"
    _sentinel(locked.parent, "locked", "file")
    locked.chmod(0o500)
    try:
        failure = delete_from_disk(tmp_path / "tree")
    finally:
        locked.chmod(0o700)
    assert isinstance(failure, PermissionError)


class TestLeftoverTrash:
    def test_leftover_trash_is_found_under_every_run_directory(self, tmp_path):
        earlier = tmp_path / RUN_DIRECTORY_ROOT / "build_earlier"
        later = tmp_path / RUN_DIRECTORY_ROOT / "build_later"
        _sentinel(earlier, "cold-trash-2/cli/target", "a", b"x" * 10)
        _sentinel(later, "cold-trash-1/cli/target", "b", b"x" * 20)
        _sentinel(later, "logs", "run-1.log")
        found = FilesystemWorkspace(tmp_path).leftover_trash()
        assert [each.path for each in found] == [
            (earlier / "cold-trash-2").absolute(),
            (later / "cold-trash-1").absolute(),
        ]
        assert all(
            each.size_bytes >= size
            for each, size in zip(found, (10, 20), strict=True)
        )

    def test_no_run_directory_means_no_trash(self, tmp_path):
        assert FilesystemWorkspace(tmp_path).leftover_trash() == ()

    @pytest.mark.skipif(
        hasattr(os, "geteuid") and os.geteuid() == 0,
        reason="no permission stops root",
    )
    def test_an_unreadable_entry_is_skipped_in_the_size(self, tmp_path):
        trash = tmp_path / RUN_DIRECTORY_ROOT / "build_run" / "cold-trash-1"
        _sentinel(trash, "readable", "a", b"x" * 10)
        unreadable = trash / "unreadable"
        _sentinel(trash, "unreadable", "b", b"x" * 10)
        unreadable.chmod(0)
        try:
            [found] = FilesystemWorkspace(tmp_path).leftover_trash()
        finally:
            unreadable.chmod(0o700)
        assert found.size_bytes >= 10


def test_the_dev_launcher_marker_is_read_from_the_real_file(tmp_path):
    workspace = FilesystemWorkspace(tmp_path)
    assert not workspace.has_dev_launcher_marker()
    (tmp_path / ".accelerator-dev-launcher").touch()
    assert workspace.has_dev_launcher_marker()
