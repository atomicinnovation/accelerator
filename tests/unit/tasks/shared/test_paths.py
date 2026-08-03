"""Path helpers and the two release registries."""

from pathlib import Path

from tasks.shared.paths import (
    BIN_DIR,
    DEBUG_ARCHIVE_DIRS,
    cli_binary_path,
    debug_archive_path,
    vendored_shim_path,
)
from tasks.shared.sources import repo_root

REPO_ROOT = repo_root()


class TestCliPathHelpers:
    def test_cli_binary_path_default_staging(self) -> None:
        path = cli_binary_path("accelerator", "linux-x64")
        assert path.name == "accelerator-linux-x64"
        assert path.parent == REPO_ROOT / "dist" / "release"

    def test_cli_binary_path_custom_dir(self, tmp_path: Path) -> None:
        path = cli_binary_path("accelerator-verify", "darwin-arm64", tmp_path)
        assert path == tmp_path / "accelerator-verify-darwin-arm64"

    def test_vendored_shim_path(self) -> None:
        path = vendored_shim_path("linux-arm64")
        assert path == REPO_ROOT / "bin/accelerator-verify-linux-arm64"


def test_debug_archive_path_files_under_the_given_directory(
    tmp_path: Path,
) -> None:
    path = debug_archive_path("alpha", "linux-x64", tmp_path)
    assert path == tmp_path / "accelerator-alpha-linux-x64.debug.tar.gz"


def test_the_debug_archive_registry_is_the_visualiser_alone() -> None:
    # The only control against an emptied registry: _debug_archive_targets
    # raises on an undispatched key and a non-`bin` value but not on an empty
    # mapping, and the SLSA coverage loop's non-empty guard is satisfied by the
    # launchers and manifest alone.
    assert dict(DEBUG_ARCHIVE_DIRS) == {"visualiser": BIN_DIR}
