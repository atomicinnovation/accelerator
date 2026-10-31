from pathlib import Path

import pytest

from tasks.probe import claude_permissions

RELEASE = "9.9.9"


@pytest.fixture
def measured(monkeypatch: pytest.MonkeyPatch) -> list[str]:
    commands: list[str] = []

    def measure(
        claude: Path, config_dir: Path, flags: list[str], command: str
    ) -> str:
        commands.append(command)
        return "allowed"

    monkeypatch.setattr(claude_permissions, "release_of", lambda _: RELEASE)
    monkeypatch.setattr(claude_permissions, "measure", measure)
    return commands


def _scratch_fixture(tmp_path: Path) -> Path:
    fixture = tmp_path / "scratch.tsv"
    fixture.write_text(
        "command\n./probe 'a $(touch smuggled) b'\n./probe 'What s it?'\n",
        encoding="utf-8",
    )
    return fixture


def _probe(tmp_path: Path, *options: str) -> int:
    return claude_permissions.main(
        [
            "--claude",
            str(tmp_path / "claude"),
            "--config-dir",
            str(tmp_path / "config"),
            *options,
        ]
    )


def test_a_fixture_option_reads_and_updates_that_file(
    tmp_path: Path, measured: list[str]
) -> None:
    fixture = _scratch_fixture(tmp_path)

    _probe(tmp_path, "--fixture", str(fixture), "--update")

    assert measured == [
        "./probe 'a $(touch smuggled) b'",
        "./probe 'What s it?'",
    ]
    assert fixture.read_text(encoding="utf-8").splitlines() == [
        f"command\t{RELEASE}",
        "./probe 'a $(touch smuggled) b'\tallowed",
        "./probe 'What s it?'\tallowed",
    ]


def test_the_committed_fixture_is_untouched_by_a_scratch_run(
    tmp_path: Path, measured: list[str]
) -> None:
    committed = claude_permissions.FIXTURE.read_bytes()

    _probe(tmp_path, "--fixture", str(_scratch_fixture(tmp_path)), "--update")

    assert claude_permissions.FIXTURE.read_bytes() == committed


def test_the_committed_fixture_is_the_default(
    tmp_path: Path, measured: list[str]
) -> None:
    _probe(tmp_path)

    committed = claude_permissions.Fixture.read(claude_permissions.FIXTURE)
    assert measured == [
        claude_permissions.unescape(row[0]) for row in committed.rows
    ]
