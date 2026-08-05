"""Every `config` command the skills invoke, run in the production shape.

Production shape means the correct absolute path with an *empty* environment —
the one configuration that matters in practice and the one no other suite
exercises, because they all inject `ACCELERATOR_BIN` or a plugin root. That gap
is what let the bootstrap ship requiring a variable Claude Code never exports to
a `!` shell.

The commands run through the real bootstrap against a fixture installation: the
freshly built launcher served by the stubbed release server, traversing the
genuine fetch → verify → cache → exec chain with no network. Substituting the
prefix to the *repo* root instead would self-locate there, pass every gate, and
fetch the real GitHub release for a version that is not yet published.

Scope: the 204 `config` commands, not all 206 `!`-site launcher invocations.
Both remainders are in `skills/visualisation/visualise/SKILL.md` — one is a
`printf` rendering a recipe rather than invoking anything, and the other starts
the visualiser daemon. That one stays excluded: with
`ACCELERATOR_VISUALISER_BIN` set the resolver short-circuits before the cache
root is consulted, and without it it verifies `manifest.json` against the real
release key compiled into the launcher, which a locally signed fixture manifest
cannot satisfy. External-subcommand dispatch is therefore a known gap here,
covered instead by `cli/launcher/tests/version.rs` and `cache_root.rs`.
"""

from collections.abc import Callable, Iterator
from pathlib import Path

import pytest

from tasks.lint.skill_permissions import EXPECTED_INJECTION_SKILLS
from tests.integration.support import skill_corpus as corpus
from tests.integration.support.installation import (
    REPO_BIN,
    REPO_ROOT,
    Installation,
    build_launcher,
    build_shim,
    copy_bootstrap,
    generate_keys,
    host_platform,
    make_installation,
    run_bootstrap,
    write_downloader,
)

_SKILLS = REPO_ROOT / "skills"

# Collected at import time because parametrisation needs it before fixtures
# run. One case per *distinct* command: the 204 occurrences carry 122 distinct
# texts, and re-running a duplicate exercises nothing new at ~0.1s a go.
_ALL = corpus.extract(_SKILLS)
_DISTINCT = sorted({c.raw: c for c in _ALL}.values(), key=lambda c: c.tail)


@pytest.fixture(scope="module")
def commands() -> list[corpus.Command]:
    return _ALL


@pytest.fixture(scope="session", autouse=True)
def repo_bin_is_untouched() -> Iterator[None]:
    before = {entry.name for entry in REPO_BIN.iterdir()}
    yield
    added = sorted({entry.name for entry in REPO_BIN.iterdir()} - before)
    assert not added, f"the suite wrote into the shipped bin/: {added}"


@pytest.fixture(scope="module")
def installation(tmp_path_factory: pytest.TempPathFactory) -> Installation:
    base = tmp_path_factory.mktemp("conformance")
    keys = base / "keys"
    keys.mkdir()
    return make_installation(
        base / "root",
        base / "server",
        keys=generate_keys(keys),
        shim=build_shim(),
        bootstrap=copy_bootstrap(base / "bootstrap-accelerator"),
        alias=host_platform(),
        launcher_source=build_launcher(),
        templates=REPO_ROOT / "templates",
    )


@pytest.fixture(scope="module")
def project(tmp_path_factory: pytest.TempPathFactory) -> Path:
    return corpus.write_project(
        tmp_path_factory.mktemp("conformance-project"), _ALL
    )


@pytest.fixture(scope="module")
def invoke(
    tmp_path_factory: pytest.TempPathFactory,
    installation: Installation,
    project: Path,
) -> Callable[[corpus.Command], object]:
    downloader = write_downloader(
        tmp_path_factory.mktemp("downloader") / "downloader.py"
    )

    def _invoke(command: corpus.Command):
        argv = command.argv(installation.root)
        assert argv[0] == str(installation.root / "bin/accelerator"), argv
        return run_bootstrap(
            installation.root,
            installation.server,
            downloader,
            args=tuple(argv[1:]),
            cwd=project,
        )

    return _invoke


class TestCorpusIntegrity:
    """Structural invariants, not a magic floor.

    A `>= 200` floor fires on routine consolidation while staying quiet on the
    loss it exists to detect: dropping one skill's two commands still clears 200
    if skills are added elsewhere.
    """

    def test_every_skill_with_a_config_site_contributes_a_command(
        self, commands: list[corpus.Command]
    ) -> None:
        # Derived by a second, line-level scan rather than by reusing the
        # extractor, so a file the extractor's walk or filter drops is visible.
        # It must not simply grep for the marker: `allowed-tools` rules and
        # documentation code blocks name the command without invoking it.
        assert corpus.source_files(commands) == corpus.files_with_config_sites(
            _SKILLS
        )

    def test_instructions_count_matches_the_injection_census(
        self, commands: list[corpus.Command]
    ) -> None:
        # Cross-checked against a constant that is already single-sourced and
        # already an exact equality — asserting the family is merely non-empty
        # would be satisfied by one command while 41 vanished.
        assert (
            len(corpus.instructions_skills(commands))
            == EXPECTED_INJECTION_SKILLS
        )

    def test_context_count_matches_the_injection_census(
        self, commands: list[corpus.Command]
    ) -> None:
        assert len(corpus.context_skills(commands)) == EXPECTED_INJECTION_SKILLS

    def test_every_named_skill_exists(
        self, commands: list[corpus.Command]
    ) -> None:
        # The fixture derives its overrides from the corpus, so without this a
        # `!` site naming a nonexistent skill would have an override created
        # for it and pass.
        named = corpus.instructions_skills(commands) | corpus.context_skills(
            commands
        )
        assert named <= corpus.declared_skill_names(_SKILLS)

    def test_nothing_is_declined(self, commands: list[corpus.Command]) -> None:
        # The permissions guard already forbids shell metacharacters in a `!`
        # command, so a non-empty decline list means the corpus changed shape.
        assert [c.raw for c in commands if c.declined] == []

    def test_the_corpus_size_is_reported(
        self, commands: list[corpus.Command], capsys: pytest.CaptureFixture
    ) -> None:
        # A shrinking scan should be visible as well as fatal.
        with capsys.disabled():
            print(
                f"\ncorpus: {len(commands)} commands, "
                f"{len({c.raw for c in commands})} distinct, "
                f"{len(corpus.source_files(commands))} SKILL.md files"
            )
        assert commands


@pytest.mark.parametrize("command", _DISTINCT, ids=[c.tail for c in _DISTINCT])
def test_command_runs_in_the_production_shape(
    invoke: Callable[[corpus.Command], object], command: corpus.Command
) -> None:
    result = invoke(command)
    assert result.returncode == 0, (
        f"{command.tail} exited {result.returncode}\n{result.stderr}"
    )
    # `accelerator:` is `fail()`'s own prefix, so this detects bootstrap-layer
    # aborts specifically — the launcher's diagnostics carry no such prefix.
    assert "accelerator:" not in result.stderr, result.stderr

    expected = corpus.expected_stdout(command)
    if expected is None:
        # Every other family renders a plugin or built-in default, so an empty
        # answer means a degraded read rather than a legitimate blank.
        assert result.stdout.strip(), f"{command.tail} rendered nothing"
    else:
        assert expected in result.stdout, (
            f"{command.tail} did not render the fixture override\n"
            f"{result.stdout!r}"
        )
