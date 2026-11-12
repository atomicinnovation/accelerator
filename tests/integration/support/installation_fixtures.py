from pathlib import Path
from typing import Protocol

import pytest

from tests.integration.support.installation import (
    Installation,
    generate_keys,
    make_installation,
    write_downloader,
)
from tests.integration.support.installation import (
    host_platform as resolve_host_platform,
)
from tests.support.artefacts import build_shim


class NewInstallation(Protocol):
    def __call__(
        self,
        label: str = "self",
        *,
        secret: str = "release",
        launcher_source: Path | None = None,
    ) -> Installation: ...


@pytest.fixture(scope="module")
def host_platform() -> str:
    return resolve_host_platform()


@pytest.fixture(scope="module")
def shim_bin() -> Path:
    return build_shim()


@pytest.fixture(scope="module")
def keys(tmp_path_factory: pytest.TempPathFactory) -> Path:
    return generate_keys(tmp_path_factory.mktemp("keys"))


@pytest.fixture
def downloader(tmp_path: Path) -> Path:
    return write_downloader(tmp_path / "downloader.py")


@pytest.fixture
def new_installation(
    tmp_path: Path, keys: Path, shim_bin: Path, host_platform: str
) -> NewInstallation:
    built = 0

    def _new(
        label: str = "self",
        *,
        secret: str = "release",
        launcher_source: Path | None = None,
    ) -> Installation:
        nonlocal built
        built += 1
        return make_installation(
            tmp_path / f"root{built}",
            tmp_path / f"server{built}",
            keys=keys,
            shim=shim_bin,
            alias=host_platform,
            label=label,
            secret=secret,
            launcher_source=launcher_source,
        )

    return _new
