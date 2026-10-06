from collections.abc import Callable, Iterable, Mapping
from pathlib import Path

import pytest

from tests.integration.support.characterisation import (
    Binaries,
    CollectedCase,
    Hermetic,
    Loopback,
    Mask,
    Repository,
    Run,
    assert_matches_golden,
    assert_outside_any_repository,
    golden_path,
    masked,
)
from tests.support.tools import require

PLUGIN_ROOT = Path(__file__).resolve().parent / "fixtures" / "plugin-root"

VCS_KINDS = ("git", "jj")

_COLLECTED = pytest.StashKey[list[CollectedCase]]()


def pytest_configure(config: pytest.Config) -> None:
    config.addinivalue_line(
        "markers",
        "vcs_specific: the case characterises one VCS layout on purpose",
    )


def _collected_case(item: pytest.Item) -> CollectedCase:
    callspec = getattr(item, "callspec", None)
    params = dict(callspec.params) if callspec else {}
    vcs = params.pop("vcs", None)
    name = item.nodeid.split("[", 1)[0]
    return CollectedCase(
        case=(name, tuple(sorted((k, repr(v)) for k, v in params.items()))),
        vcs=vcs,
        vcs_specific=item.get_closest_marker("vcs_specific") is not None,
        runs_a_binary="run" in getattr(item, "fixturenames", ()),
    )


@pytest.hookimpl(tryfirst=True)
def pytest_collection_modifyitems(
    config: pytest.Config, items: list[pytest.Item]
) -> None:
    config.stash[_COLLECTED] = [_collected_case(item) for item in items]


@pytest.fixture
def collected_cases(request: pytest.FixtureRequest) -> list[CollectedCase]:
    return request.config.stash[_COLLECTED]


@pytest.fixture(scope="session")
def binaries() -> Binaries:
    return Binaries.characterisation()


@pytest.fixture(scope="session")
def loopback() -> Iterable[Loopback]:
    server = Loopback()
    yield server
    server.close()


@pytest.fixture
def fixture_root(tmp_path: Path) -> Path:
    root = tmp_path.resolve()
    assert_outside_any_repository(root)
    return root


@pytest.fixture
def hermetic(fixture_root: Path) -> Hermetic:
    require("git")
    require("jj")
    return Hermetic.rooted_at(fixture_root / "hermetic")


@pytest.fixture(params=VCS_KINDS)
def vcs(request: pytest.FixtureRequest) -> str:
    return request.param


@pytest.fixture
def repository(vcs: str, hermetic: Hermetic, fixture_root: Path) -> Repository:
    return Repository.create(vcs, fixture_root / "repository", hermetic)


@pytest.fixture
def default_masks(fixture_root: Path) -> list[Mask]:
    return [Mask(str(fixture_root), "ROOT")]


Runner = Callable[..., Run]


def _loopback_urls(base: str) -> dict[str, str]:
    return {
        "ACCELERATOR_RELEASE_BASE_URL": base,
        "ACCELERATOR_JIRA_API_URL": base,
        "ACCELERATOR_LINEAR_API_URL": f"{base}/graphql",
        "ACCELERATOR_COLLABORATION_GITHUB_API_URL": base,
        "ACCELERATOR_OPENALEX_API_URL": base,
        "ACCELERATOR_ARXIV_API_URL": base,
        "ACCELERATOR_ARXIV_OAI_URL": base,
    }


@pytest.fixture
def run(hermetic: Hermetic, loopback: Loopback, fixture_root: Path) -> Runner:
    cache = fixture_root / "launcher-cache"

    def runner(
        program: Path,
        *args: str,
        cwd: Path,
        env: Mapping[str, str] = {},
    ) -> Run:
        before = len(loopback.hits)
        completed = hermetic.run(
            [str(program), *args],
            cwd,
            {
                **_loopback_urls(loopback.url),
                "ACCELERATOR_CACHE_DIR": str(cache),
                "ACCELERATOR_PLUGIN_ROOT": str(PLUGIN_ROOT),
                **env,
            },
        )
        return Run(
            completed.returncode,
            completed.stdout,
            completed.stderr,
            len(loopback.hits) - before,
        )

    return runner


@pytest.fixture
def matches_golden(
    request: pytest.FixtureRequest, default_masks: list[Mask]
) -> Callable[..., None]:
    module = Path(str(request.node.path)).stem.removeprefix("test_")

    def check(run: Run, masks: Iterable[Mask] = ()) -> None:
        rendered = masked(run.render(), [*masks, *default_masks])
        assert_matches_golden(golden_path(request.node.name, module), rendered)

    return check
