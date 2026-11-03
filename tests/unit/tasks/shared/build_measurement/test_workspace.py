import contextlib
import os
from dataclasses import dataclass
from pathlib import Path

import pytest

from tasks.shared.build_measurement.cache_states import BuildCacheState
from tasks.shared.build_measurement.errors import (
    TouchTargetNotAFileError,
    UnsafeDeletionError,
)
from tasks.shared.build_measurement.locations import RUN_DIRECTORY_ROOT
from tasks.shared.build_measurement.workspace import (
    FilesystemWorkspace,
    delete_from_disk,
)
from tests.unit.tasks.shared.build_measurement.doubles import FakeSession

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
    _sentinel(root, TARGET)
    _sentinel(root, FRONTEND_MODULES, "react/index.js")
    _sentinel(root, DOCS_MODULES, "astro/index.js")


@pytest.fixture
def run_directory(tmp_path):
    _checkout(tmp_path)
    run_directory = tmp_path / RUN_DIRECTORY_ROOT / "build_run"
    run_directory.mkdir(parents=True)
    return run_directory


def _fails_without_removing(path):
    del path
    return PermissionError(13, "Permission denied", "cli/target/debug")


@contextlib.contextmanager
def _read_only_frontend(root, _monkeypatch):
    frontend = (root / FRONTEND_MODULES).parent
    frontend.chmod(0o555)
    try:
        yield PermissionError
    finally:
        frontend.chmod(0o755)


@contextlib.contextmanager
def _interrupted_frontend(_root, monkeypatch):
    rename = Path.rename

    def interrupted(source, target):
        if source.name == "node_modules":
            raise KeyboardInterrupt
        return rename(source, target)

    monkeypatch.setattr(Path, "rename", interrupted)
    yield KeyboardInterrupt


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
            TouchTargetNotAFileError,
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
        workspace = FilesystemWorkspace(tmp_path)
        workspace.prepare_timed_run(BuildCacheState.COLD, trash=trash)
        assert workspace.remove_trash(trash) is None
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

    def test_a_cold_target_is_moved_aside_until_the_trash_is_removed(
        self, tmp_path, run_directory
    ):
        trash = run_directory / "cold-trash-1"
        FilesystemWorkspace(tmp_path).prepare_timed_run(
            BuildCacheState.COLD, trash=trash
        )
        assert not (tmp_path / TARGET).exists()
        assert (trash / TARGET / "debug/accelerator").read_bytes() == (
            b"x" * 100
        )

    def test_trash_whose_removal_fails_is_unremoved(
        self, tmp_path, run_directory
    ):
        trash = run_directory / "cold-trash-1"
        FilesystemWorkspace(tmp_path).prepare_timed_run(
            BuildCacheState.COLD, trash=trash
        )
        unremoved = FilesystemWorkspace(
            tmp_path, remove_tree=_fails_without_removing
        ).remove_trash(trash)
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
        FilesystemWorkspace(tmp_path).prepare_timed_run(
            BuildCacheState.COLD, trash=trash
        )
        unremoved = FilesystemWorkspace(
            tmp_path, remove_tree=lambda path: None
        ).remove_trash(trash)
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

    def test_removing_trash_outside_the_run_root_is_refused(
        self, tmp_path, run_directory
    ):
        with pytest.raises(UnsafeDeletionError, match="outside"):
            FilesystemWorkspace(tmp_path).remove_trash(tmp_path / TARGET)
        assert (tmp_path / TARGET / "debug/accelerator").exists()

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

    @pytest.mark.parametrize(
        "obstruct",
        [
            pytest.param(
                _read_only_frontend,
                marks=pytest.mark.skipif(
                    hasattr(os, "geteuid") and os.geteuid() == 0,
                    reason="no permission stops root",
                ),
                id="refused",
            ),
            pytest.param(_interrupted_frontend, id="interrupted"),
        ],
    )
    def test_a_failed_second_move_puts_the_first_back(
        self, tmp_path, run_directory, monkeypatch, obstruct
    ):
        trash = run_directory / "cold-trash-1"
        workspace = FilesystemWorkspace(tmp_path)
        with (
            obstruct(tmp_path, monkeypatch) as expected,
            pytest.raises(expected),
        ):
            workspace.prepare_timed_run(BuildCacheState.COLD, trash=trash)
        assert (tmp_path / TARGET / "debug/accelerator").read_bytes() == (
            b"x" * 100
        )
        assert (tmp_path / FRONTEND_MODULES / "react/index.js").exists()
        assert not trash.exists()
        assert workspace.leftover_trash() == ()


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


@dataclass(frozen=True)
class Observation:
    kernel_mtime: float
    kernel_bytes: bytes
    target_built: bool
    frontend_installed: bool
    docs_installed: bool


class ObservedBuild:
    """Records what each build sees, then rebuilds as a real build would."""

    def __init__(self, root, during=None):
        self.root = root
        self.during = during or {}
        self.seen: dict[str, Observation] = {}

    def __call__(self, log):
        root = self.root
        kernel = root / KERNEL
        self.seen[log.name] = Observation(
            kernel_mtime=kernel.stat().st_mtime,
            kernel_bytes=kernel.read_bytes(),
            target_built=(root / TARGET / "sentinel").exists(),
            frontend_installed=(root / FRONTEND_MODULES).exists(),
            docs_installed=(root / DOCS_MODULES / "astro/index.js").exists(),
        )
        os.utime(kernel, (0, 0))
        _sentinel(root, TARGET)
        _sentinel(root, FRONTEND_MODULES, "react/index.js")
        if log.name in self.during:
            self.during[log.name](root)


def _observed_session(tmp_path, *, state, runs=2, during=None):
    _checkout(tmp_path)
    os.utime(tmp_path / KERNEL, (0, 0))
    observed = ObservedBuild(tmp_path, during)
    session = FakeSession(
        tmp_path, observe=observed, workspace=FilesystemWorkspace(tmp_path)
    )
    return (
        session,
        observed,
        lambda: session.measure(runs=runs, cache_state=state),
    )


class TestObservedSessions:
    def test_incremental_touches_the_kernel_before_each_timed_run(
        self, tmp_path
    ):
        _, observed, measure = _observed_session(tmp_path, state="incremental")
        measure()
        seen = observed.seen
        assert seen["priming.log"].kernel_mtime == 0
        assert seen["run-1.log"].kernel_mtime > 0
        assert seen["run-2.log"].kernel_mtime > 0
        assert {each.kernel_bytes for each in seen.values()} == {
            b"pub fn kernel() {}\n"
        }

    def test_incremental_primes_first(self, tmp_path):
        session, _, measure = _observed_session(tmp_path, state="incremental")
        measure()
        assert session.build.calls[0][1].name == "priming.log"

    def test_cold_deletes_the_target_and_frontend_node_modules_before_each_run(
        self, tmp_path
    ):
        _, observed, measure = _observed_session(tmp_path, state="cold")
        measure()
        for log in ("run-1.log", "run-2.log"):
            assert not observed.seen[log].target_built
            assert not observed.seen[log].frontend_installed

    def test_a_cold_session_leaves_no_trash_behind(self, tmp_path):
        _, _, measure = _observed_session(tmp_path, state="cold")
        measure()
        assert FilesystemWorkspace(tmp_path).leftover_trash() == ()

    def test_cold_never_primes(self, tmp_path):
        _, observed, measure = _observed_session(tmp_path, state="cold")
        measure()
        assert list(observed.seen) == ["run-1.log", "run-2.log"]

    def test_cold_leaves_docs_site_node_modules_alone(self, tmp_path):
        _, observed, measure = _observed_session(tmp_path, state="cold")
        measure()
        assert all(each.docs_installed for each in observed.seen.values())
        assert (tmp_path / DOCS_MODULES / "astro/index.js").exists()

    @pytest.mark.parametrize("state", ["warm-no-op", "incremental"])
    def test_without_cold_a_sentinel_in_cli_target_survives(
        self, tmp_path, state
    ):
        _, observed, measure = _observed_session(tmp_path, state=state)
        measure()
        assert all(each.target_built for each in observed.seen.values())

    def test_each_deletion_rechecks_containment(self, tmp_path):
        elsewhere = tmp_path / "elsewhere"
        elsewhere.mkdir()

        def relink_target(root):
            _replace_with_symlink(root / TARGET, elsewhere)

        _, observed, measure = _observed_session(
            tmp_path, state="cold", during={"run-1.log": relink_target}
        )
        with pytest.raises(UnsafeDeletionError, match="cli/target"):
            measure()
        assert list(observed.seen) == ["run-1.log"]
        assert (tmp_path / FRONTEND_MODULES / "react/index.js").exists()

    def test_the_record_states_the_cache_state(self, tmp_path):
        _, _, measure = _observed_session(tmp_path, state="cold", runs=1)
        conditions = measure().measured.conditions
        assert conditions.cache_state is BuildCacheState.COLD
